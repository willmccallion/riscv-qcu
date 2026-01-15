use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::slice;

/// A fixed-capacity vector that lives on the stack or in static memory.
pub struct StaticVec<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Default for StaticVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> StaticVec<T, N> {
    pub const fn new() -> Self {
        Self {
            // Safety: Array of MaybeUninit is always safe to create uninitialized.
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline(always)]
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.len < N {
            unsafe {
                // Safety: We checked bounds.
                self.data.get_unchecked_mut(self.len).write(item);
            }
            self.len += 1;
            Ok(())
        } else {
            Err(item)
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            unsafe {
                // Safety: We checked bounds. Item is initialized.
                Some(self.data.get_unchecked(self.len).assume_init_read())
            }
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        N
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            // Safety: data[0..len] is initialized.
            slice::from_raw_parts(self.data.as_ptr() as *const T, self.len)
        }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            // Safety: data[0..len] is initialized.
            slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len)
        }
    }
}

// Allow indexing like a normal slice
impl<T, const N: usize> Deref for StaticVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> DerefMut for StaticVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

// Required for generic usage in loops
impl<T, const N: usize> IntoIterator for StaticVec<T, N> {
    type Item = T;
    type IntoIter = StaticVecIntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        StaticVecIntoIter {
            vec: self,
            index: 0,
        }
    }
}

pub struct StaticVecIntoIter<T, const N: usize> {
    vec: StaticVec<T, N>,
    index: usize,
}

impl<T, const N: usize> Iterator for StaticVecIntoIter<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.len {
            let item = unsafe { self.vec.data.get_unchecked(self.index).assume_init_read() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}
