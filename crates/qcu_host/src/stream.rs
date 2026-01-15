use crate::stats::LatencyStats;
use anyhow::Result;
use qcu_core::decoder::UnionFindDecoder;
use qcu_core::ring_buffer::RingBuffer;
use qcu_io::{loader, parser};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TaskPacket {
    pub syndrome_len: u32,
    pub syndrome_buffer: [u32; 64],
}

impl Default for TaskPacket {
    fn default() -> Self {
        Self {
            syndrome_len: 0,
            syndrome_buffer: [0; 64],
        }
    }
}

pub struct StreamStats {
    pub processed: Arc<AtomicU64>,
    pub generated: Arc<AtomicU64>,
    pub dropped: Arc<AtomicU64>,
    pub latency_us: Arc<AtomicU64>,
}

const MAX_NODES: usize = 4096;

pub fn run_stream(
    dem_path: &str,
    b8_path: Option<String>,
    freq: u64,
    duration_secs: u64,
    user_detectors: Option<usize>,
) -> Result<()> {
    println!("QEC STREAMING");
    println!("Graph: {}", dem_path);
    println!("Target Freq: {} Hz", freq);
    println!("Duration: {} s", duration_secs);
    println!("-------------------------------");

    let running = Arc::new(AtomicBool::new(true));
    let stats = StreamStats {
        processed: Arc::new(AtomicU64::new(0)),
        generated: Arc::new(AtomicU64::new(0)),
        dropped: Arc::new(AtomicU64::new(0)),
        latency_us: Arc::new(AtomicU64::new(0)),
    };

    let graph = parser::load_dem_file(dem_path)?;
    let num_detectors = user_detectors.unwrap_or(graph.num_nodes());
    println!(
        "Graph loaded. Nodes: {}, Edges: {}",
        graph.num_nodes(),
        graph.fast_edges.len()
    );

    let shots = if let Some(path) = b8_path {
        println!("Loading shots from {}...", path);
        let raw_bits = loader::load_b8_file(&path)?;
        loader::slice_shots(&raw_bits, num_detectors)
    } else {
        vec![vec![false; num_detectors]]
    };
    println!("Loaded {} unique error patterns.", shots.len());

    let graph_arc = Arc::new(graph);
    let ring_buffer = Arc::new(RingBuffer::<TaskPacket>::new(1024));

    let rb_cons = ring_buffer.clone();
    let s_cons = stats.processed.clone();
    let l_cons = stats.latency_us.clone();
    let r_cons = running.clone();

    let consumer = thread::spawn(move || {
        let mut decoder = UnionFindDecoder::<MAX_NODES>::new();
        let mut lat_stats = LatencyStats::new();
        let mut results = Vec::with_capacity(1024);

        while r_cons.load(Ordering::Relaxed) {
            if let Some(packet) = rb_cons.pop() {
                let len = packet.syndrome_len as usize;
                let mut indices = Vec::with_capacity(len);
                for i in 0..len {
                    indices.push(packet.syndrome_buffer[i] as usize);
                }

                let start = Instant::now();
                let _ = decoder.solve_into(&graph_arc, &indices, &mut results);
                let lat_ns = start.elapsed().as_nanos() as u64;

                s_cons.fetch_add(1, Ordering::Relaxed);
                l_cons.store(lat_ns / 1000, Ordering::Relaxed);
                lat_stats.update(lat_ns);
            } else {
                std::hint::spin_loop();
            }
        }
        lat_stats.print_report();
    });

    let rb_prod = ring_buffer.clone();
    let s_gen = stats.generated.clone();
    let s_drop = stats.dropped.clone();
    let r_prod = running.clone();
    let producer_shots = shots.clone();

    let producer = thread::spawn(move || {
        let interval = Duration::from_micros(1_000_000 / freq);
        let num_patterns = producer_shots.len();
        let mut idx = 0;

        while r_prod.load(Ordering::Relaxed) {
            let start = Instant::now();
            let mut packet = TaskPacket::default();
            let mut count = 0;

            if num_patterns > 0 {
                for (det_id, &triggered) in producer_shots[idx].iter().enumerate() {
                    if triggered && count < 64 {
                        packet.syndrome_buffer[count] = det_id as u32;
                        count += 1;
                    }
                }
                idx = (idx + 1) % num_patterns;
            }
            packet.syndrome_len = count as u32;

            if rb_prod.push(packet) {
                s_gen.fetch_add(1, Ordering::Relaxed);
            } else {
                s_drop.fetch_add(1, Ordering::Relaxed);
            }

            while start.elapsed() < interval {
                std::hint::spin_loop();
            }
        }
    });

    let start_time = Instant::now();
    let mut last_processed = 0;

    while start_time.elapsed().as_secs() < duration_secs {
        thread::sleep(Duration::from_secs(1));
        let proc = stats.processed.load(Ordering::Relaxed);
        let r#gen = stats.generated.load(Ordering::Relaxed);
        let drop = stats.dropped.load(Ordering::Relaxed);
        let lat = stats.latency_us.load(Ordering::Relaxed);

        let tput = proc - last_processed;
        last_processed = proc;

        println!(
            "T={:2}s | Gen: {:8} | Proc: {:8} ({:5}/s) | Drop: {:5} | Latency: {:3} us",
            start_time.elapsed().as_secs(),
            r#gen,
            proc,
            tput,
            drop,
            lat
        );
    }

    running.store(false, Ordering::Relaxed);
    thread::sleep(Duration::from_millis(100));
    consumer.join().unwrap();
    producer.join().unwrap();

    println!("Done.");
    Ok(())
}
