use crate::QecError;
use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct BumpAllocator {
    start: usize,
    len: usize,
    offset: AtomicUsize,
}

impl BumpAllocator {
    pub fn new(ptr: usize, len: usize) -> Self {
        Self {
            start: ptr,
            len,
            offset: AtomicUsize::new(0),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn alloc_slice<T>(&self, len: usize) -> Result<&mut [T], QecError> {
        let layout = Layout::array::<T>(len).map_err(|_| QecError::OutOfMemory)?;
        let ptr = self.allocate(layout).map_err(|_| QecError::OutOfMemory)?;
        unsafe {
            // SAFETY: allocate() returns valid memory matching layout.
            let slice_ptr = ptr.as_ptr() as *mut T;
            core::ptr::write_bytes(slice_ptr, 0, len);
            Ok(core::slice::from_raw_parts_mut(slice_ptr, len))
        }
    }
}

unsafe impl Allocator for BumpAllocator {
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
                // SAFETY: We checked bounds and own the range via atomic increment.
                let ptr = unsafe { NonNull::new_unchecked(aligned_ptr as *mut u8) };
                return Ok(NonNull::slice_from_raw_parts(ptr, size));
            }
        }
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

unsafe impl Sync for BumpAllocator {}
