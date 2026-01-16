//! Kernel heap allocator for firmware dynamic memory allocation.
//!
//! Implements a simple bump allocator that manages a contiguous region of
//! memory from 0x8050_0000 to 0x8800_0000. Allocations are never freed,
//! making this suitable for long-lived data structures. The allocator uses
//! atomic operations to support concurrent allocation from multiple threads.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Bump allocator for kernel heap memory.
///
/// Maintains a single atomic pointer that advances with each allocation.
/// Allocations are aligned to the requested alignment boundary, and the
/// allocator never deallocates memory, making it suitable for firmware
/// where memory lifetime is well-controlled.
struct KernelHeap {
    /// Current allocation pointer, atomically updated.
    ///
    /// Points to the next available byte in the heap region. Incremented
    /// atomically during allocation to prevent races between concurrent
    /// allocators.
    heap_curr: AtomicUsize,
}

unsafe impl GlobalAlloc for KernelHeap {
    /// Allocates memory matching the requested layout.
    ///
    /// Performs alignment and size calculations, then atomically updates
    /// the heap pointer to reserve the memory. Uses compare-and-swap to
    /// handle concurrent allocations safely. Returns null if the allocation
    /// would exceed the heap bounds (0x8800_0000).
    ///
    /// # Arguments
    ///
    /// * `layout` - Memory layout specifying size and alignment requirements
    ///
    /// # Returns
    ///
    /// A pointer to the allocated memory, or null if allocation fails.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut prev = self.heap_curr.load(Ordering::Relaxed);
        loop {
            let aligned = (prev + layout.align() - 1) & !(layout.align() - 1);
            let next = aligned + layout.size();

            if next >= 0x8800_0000 {
                return core::ptr::null_mut();
            }

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

    /// No-op deallocation function.
    ///
    /// The bump allocator does not support deallocation. Memory is reclaimed
    /// only when the entire system is reset. This function exists to satisfy
    /// the GlobalAlloc trait contract but performs no operation.
    ///
    /// # Arguments
    ///
    /// * `_ptr` - Pointer to deallocate (ignored)
    /// * `_layout` - Layout of the allocation (ignored)
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
    }
}

/// Global heap allocator instance.
///
/// Manages the kernel heap region from 0x8050_0000 to 0x8800_0000.
/// Initialized with the heap start address, ready for allocations after
/// system boot.
#[global_allocator]
static HEAP: KernelHeap = KernelHeap {
    heap_curr: AtomicUsize::new(0x8050_0000),
};
