#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

extern crate alloc;

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use qcu_core::allocator::BumpAllocator;
use qcu_core::decoder::UnionFindDecoder;
use qcu_core::graph::DecodingGraph;
#[cfg(feature = "use_fpga_mmio")]
use qcu_core::hw_accel::DecoderAccelerator;
use qcu_core::spmc::StaticQueue;
use qcu_core::static_vec::StaticVec;

mod console;
mod trap;

mod bench_data {
    include!(concat!(env!("OUT_DIR"), "/bench_data.rs"));
}

static DEM_DATA: &str = include_str!("../../../output/bench.dem");

use core::alloc::{GlobalAlloc, Layout};
struct DummyAlloc;
unsafe impl GlobalAlloc for DummyAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
#[global_allocator]
static GLOBAL: DummyAlloc = DummyAlloc;

const WORDS_PER_SHOT: usize = 160;
const MAX_NODES: usize = 10240;

#[derive(Clone, Copy)]
pub struct SyndromePacket {
    pub shot_id: u64,
    pub timestamp: u64,
    pub syndromes: [u64; WORDS_PER_SHOT],
}

pub static JOB_QUEUE: StaticQueue<SyndromePacket, 512> = StaticQueue::new();
pub static QUEUE_DEPTH: AtomicI64 = AtomicI64::new(0);
static SYSTEM_READY: AtomicBool = AtomicBool::new(false);
static TOTAL_PROCESSED: AtomicU64 = AtomicU64::new(0);
static LATENCY_SUM: AtomicU64 = AtomicU64::new(0);
static LATENCY_MAX: AtomicU64 = AtomicU64::new(0);
static LATENCY_MIN: AtomicU64 = AtomicU64::new(u64::MAX);

struct GlobalCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for GlobalCell<T> {}

impl<T> GlobalCell<T> {
    const fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }

    unsafe fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }
}

static GRAPH_ALLOC: GlobalCell<Option<BumpAllocator>> = GlobalCell::new(None);
static GRAPH_REF: GlobalCell<Option<&'static DecodingGraph<&'static BumpAllocator>>> =
    GlobalCell::new(None);

use core::arch::global_asm;
global_asm!(include_str!("entry.S"));

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    let hartid: usize;
    unsafe {
        core::arch::asm!("csrr {}, mhartid", out(reg) hartid);
    }

    if hartid == 0 {
        primary_main();
    } else {
        worker_main(hartid);
    }
}

fn primary_main() -> ! {
    console::init();
    console::println!("[BOOT] Core 0 Online");

    unsafe {
        *GRAPH_ALLOC.get_mut() = Some(BumpAllocator::new(0x8400_0000, 0x400000));
        let alloc_ref = GRAPH_ALLOC.get().as_ref().unwrap();

        let (graph, _) = parse_graph_dem(alloc_ref);
        let leaked_graph = alloc::boxed::Box::leak(alloc::boxed::Box::new_in(graph, alloc_ref));

        *GRAPH_REF.get_mut() = Some(leaked_graph);
    }

    SYSTEM_READY.store(true, Ordering::Release);

    const MTIME_ADDR: usize = 0x200_BFF8;
    const TARGET_INTERVAL: u64 = 222;

    let mut data_idx = 0;
    let mut last_print_time = unsafe { (MTIME_ADDR as *const u64).read_volatile() };
    let mut last_processed = 0;
    let mut next_shot_time = unsafe { (MTIME_ADDR as *const u64).read_volatile() };

    loop {
        let now = unsafe { (MTIME_ADDR as *const u64).read_volatile() };

        if now < next_shot_time {
            core::hint::spin_loop();
            continue;
        }
        next_shot_time += TARGET_INTERVAL;

        let offset = data_idx * bench_data::WORDS_PER_SHOT;
        let mut syndromes = [0u64; WORDS_PER_SHOT];

        if offset + WORDS_PER_SHOT <= bench_data::BENCH_DATA.len() {
            syndromes.copy_from_slice(&bench_data::BENCH_DATA[offset..offset + WORDS_PER_SHOT]);
        } else {
            data_idx = 0;
        }

        let packet = SyndromePacket {
            shot_id: data_idx as u64,
            timestamp: now,
            syndromes,
        };

        if JOB_QUEUE.push(packet).is_ok() {
            QUEUE_DEPTH.fetch_add(1, Ordering::Relaxed);
            data_idx = (data_idx + 1) % bench_data::TOTAL_SHOTS;
        }

        if now.wrapping_sub(last_print_time) >= 10_000_000 {
            let total = TOTAL_PROCESSED.load(Ordering::Relaxed);
            let depth = QUEUE_DEPTH.load(Ordering::Relaxed);
            let sum = LATENCY_SUM.swap(0, Ordering::Relaxed);
            let max = LATENCY_MAX.swap(0, Ordering::Relaxed);
            let min = LATENCY_MIN.swap(u64::MAX, Ordering::Relaxed);

            let delta = total.wrapping_sub(last_processed);
            let avg = if delta > 0 { sum / delta } else { 0 };

            console::println!(
                "T={:3}s | Rate: {:6}/s | Lat: {:4}/{:4}/{:4} | Q: {:4}",
                now / 10_000_000,
                delta,
                min,
                avg,
                max,
                depth
            );

            last_print_time = now;
            last_processed = total;
        }
    }
}

fn worker_main(hartid: usize) -> ! {
    while !SYSTEM_READY.load(Ordering::Acquire) {
        core::hint::spin_loop();
    }

    let graph = unsafe { GRAPH_REF.get().as_ref().unwrap() };

    let mut decoder = UnionFindDecoder::<MAX_NODES>::new();
    let mut syndrome_indices: StaticVec<usize, 1024> = StaticVec::new();
    let mut corrections: StaticVec<(usize, usize), 1024> = StaticVec::new();

    console::println!("[WORKER] Core {} Ready", hartid);
    const MTIME_ADDR: usize = 0x200_BFF8;

    loop {
        if let Some(packet) = JOB_QUEUE.pop() {
            QUEUE_DEPTH.fetch_sub(1, Ordering::Relaxed);

            syndrome_indices.clear();
            for (i, &word) in packet.syndromes.iter().enumerate() {
                let mut w = word;
                let mut bit = 0;
                while w > 0 {
                    if w & 1 == 1 {
                        let _ = syndrome_indices.push(i * 64 + bit);
                    }
                    w >>= 1;
                    bit += 1;
                }
            }

            #[cfg(not(feature = "use_fpga_mmio"))]
            {
                if decoder
                    .solve_into(graph, &syndrome_indices, &mut corrections)
                    .is_ok()
                {
                    let now = unsafe { (MTIME_ADDR as *const u64).read_volatile() };
                    let latency = now.wrapping_sub(packet.timestamp);

                    TOTAL_PROCESSED.fetch_add(1, Ordering::Relaxed);
                    LATENCY_SUM.fetch_add(latency, Ordering::Relaxed);
                    LATENCY_MAX.fetch_max(latency, Ordering::Relaxed);
                    LATENCY_MIN.fetch_min(latency, Ordering::Relaxed);
                }
            }
        } else {
            core::hint::spin_loop();
        }
    }
}

fn parse_graph_dem(alloc: &BumpAllocator) -> (DecodingGraph<&BumpAllocator>, usize) {
    let mut graph = DecodingGraph::new_in(120_000, alloc);
    let mut max_node_id = 0;

    for line in DEM_DATA.split('\n') {
        let trimmed: &str = line.trim();
        if trimmed.starts_with("error") {
            let mut parts = trimmed.split_whitespace();
            parts.next();
            let mut u = usize::MAX;
            let mut v = usize::MAX;

            for part in parts {
                if let Some(Ok(idx)) = part.strip_prefix('D').map(core::str::FromStr::from_str) {
                    if idx > max_node_id {
                        max_node_id = idx;
                    }
                    if u == usize::MAX {
                        u = idx;
                    } else if v == usize::MAX {
                        v = idx;
                    }
                }
            }
            if u != usize::MAX && v != usize::MAX {
                let _ = graph.add_edge(u, v, 1.0);
            }
        }
    }
    (graph, max_node_id + 1)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    console::println!("PANIC: {:?}", info);
    unsafe {
        let qemu_exit = 0x100000 as *mut u32;
        qemu_exit.write_volatile(0x5555);
    }
    loop {}
}
