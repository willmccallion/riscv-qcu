use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

struct KernelHeap {
    heap_curr: AtomicUsize,
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut prev = self.heap_curr.load(Ordering::Relaxed);
        loop {
            // Align up
            let aligned = (prev + layout.align() - 1) & !(layout.align() - 1);
            let next = aligned + layout.size();

            // Hard limit at 0x8800_0000 (128MB mark)
            if next >= 0x8800_0000 {
                return core::ptr::null_mut();
            }

            // CAS loop for thread safety
            match self.heap_curr.compare_exchange_weak(
                prev,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(e) => prev = e,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Leaky allocator
    }
}

#[global_allocator]
static HEAP: KernelHeap = KernelHeap {
    heap_curr: AtomicUsize::new(0x8050_0000),
};
