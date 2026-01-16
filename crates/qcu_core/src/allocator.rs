//! Bump allocator for fixed-size memory regions.
//!
//! Provides a thread-safe, lock-free allocator that manages a contiguous
//! region of memory using a simple bump pointer. Allocations are never freed,
//! making this suitable for firmware environments where memory is pre-allocated
//! and lifetime management is explicit. The allocator uses atomic operations
//! to support concurrent allocation from multiple threads or interrupt handlers.

use crate::QecError;
use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Thread-safe bump allocator for fixed memory regions.
///
/// Manages a contiguous block of memory by maintaining an atomic offset
/// pointer that advances with each allocation. Allocations are aligned to
/// the requested alignment boundary, and the allocator never deallocates
/// memory, making it suitable for long-lived data structures in firmware.
/// The atomic offset ensures thread-safe allocation without locks.
pub struct BumpAllocator {
    /// Base address of the managed memory region.
    start: usize,

    /// Total size of the memory region in bytes.
    len: usize,

    /// Current allocation offset, atomically updated.
    ///
    /// Tracks the next available byte in the region. Incremented atomically
    /// during allocation to prevent races between concurrent allocators.
    offset: AtomicUsize,
}

impl BumpAllocator {
    /// Creates a new bump allocator managing the specified memory region.
    ///
    /// The allocator takes ownership of the memory range [ptr, ptr + len) and
    /// will allocate from this region until exhaustion. The region must be
    /// valid, writable memory for the lifetime of the allocator.
    ///
    /// # Arguments
    ///
    /// * `ptr` - Base address of the memory region
    /// * `len` - Size of the region in bytes
    pub fn new(ptr: usize, len: usize) -> Self {
        Self {
            start: ptr,
            len,
            offset: AtomicUsize::new(0),
        }
    }

    /// Allocates a zero-initialized slice of the specified type and length.
    ///
    /// Convenience method that allocates memory for a slice, ensures proper
    /// alignment for type T, and zero-initializes all elements. This is
    /// commonly used for allocating arrays of Pauli frame registers or
    /// decoder state vectors.
    ///
    /// # Arguments
    ///
    /// * `len` - Number of elements to allocate
    ///
    /// # Returns
    ///
    /// A mutable slice of zero-initialized elements, or an error if allocation
    /// fails due to insufficient memory or overflow.
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice<T>(&self, len: usize) -> Result<&mut [T], QecError> {
        let layout = Layout::array::<T>(len).map_err(|_| QecError::OutOfMemory)?;
        let ptr = self.allocate(layout).map_err(|_| QecError::OutOfMemory)?;
        unsafe {
            let slice_ptr = ptr.as_ptr() as *mut T;
            core::ptr::write_bytes(slice_ptr, 0, len);
            Ok(core::slice::from_raw_parts_mut(slice_ptr, len))
        }
    }
}

unsafe impl Allocator for BumpAllocator {
    /// Allocates memory matching the requested layout.
    ///
    /// Performs alignment and size calculations, then atomically updates the
    /// offset pointer to reserve the memory. Uses compare-and-swap to handle
    /// concurrent allocations safely. Returns an error if the allocation
    /// would exceed the region bounds.
    ///
    /// # Arguments
    ///
    /// * `layout` - Memory layout specifying size and alignment requirements
    ///
    /// # Returns
    ///
    /// A pointer to the allocated memory, or AllocError if allocation fails.
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let size = layout.size();
        let align = layout.align();

        loop {
            let current_offset = self.offset.load(Ordering::Relaxed);
            let current_ptr = self.start + current_offset;
            let aligned_ptr = (current_ptr + align - 1) & !(align - 1);
            let padding = aligned_ptr - current_ptr;
            let new_offset = current_offset + padding + size;

            if new_offset > self.len {
                return Err(AllocError);
            }

            if self
                .offset
                .compare_exchange(
                    current_offset,
                    new_offset,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                let ptr = unsafe { NonNull::new_unchecked(aligned_ptr as *mut u8) };
                return Ok(NonNull::slice_from_raw_parts(ptr, size));
            }
        }
    }

    /// No-op deallocation function.
    ///
    /// Bump allocators do not support deallocation. Memory is reclaimed only
    /// when the entire allocator is reset or destroyed. This function exists
    /// to satisfy the Allocator trait contract but performs no operation.
    ///
    /// # Arguments
    ///
    /// * `_ptr` - Pointer to deallocate (ignored)
    /// * `_layout` - Layout of the allocation (ignored)
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

/// BumpAllocator is safe to share between threads.
///
/// The atomic offset pointer ensures that concurrent allocations are handled
/// correctly without data races. Multiple threads can allocate from the same
/// BumpAllocator instance safely.
unsafe impl Sync for BumpAllocator {}
