//! Stack-allocated vector with fixed capacity.
//!
//! Provides a vector-like interface with compile-time fixed capacity, enabling
//! heap-free collections in no_std environments. All storage is allocated on
//! the stack or in static memory, making this suitable for real-time firmware
//! where heap allocation is unavailable or undesirable.

use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::slice;

/// Fixed-capacity vector allocated on the stack or in static memory.
///
/// Maintains a contiguous array of elements with a length that can grow up
/// to the capacity N. Elements are stored as MaybeUninit<T> to handle
/// uninitialized memory safely. The vector provides push/pop operations and
/// can be used as a slice via Deref, making it compatible with standard
/// Rust collection APIs.
///
/// # Type Parameters
///
/// * `T` - Element type
/// * `N` - Maximum capacity (compile-time constant)
pub struct StaticVec<T, const N: usize> {
    /// Backing storage array of uninitialized elements.
    ///
    /// Elements are initialized on-demand when pushed, and the length field
    /// tracks how many elements are currently valid. Uninitialized elements
    /// beyond the length are never accessed.
    data: [MaybeUninit<T>; N],

    /// Current number of initialized elements in the vector.
    ///
    /// Always satisfies 0 <= len <= N. Elements in data[0..len] are
    /// initialized and valid, while elements in data[len..N] are uninitialized.
    len: usize,
}

impl<T, const N: usize> Default for StaticVec<T, N> {
    /// Creates a vector with default (empty) state.
    ///
    /// Equivalent to calling `new()`, provided for trait compatibility.
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> StaticVec<T, N> {
    /// Creates a new empty static vector.
    ///
    /// The backing array is left uninitialized until elements are pushed.
    /// The length is set to zero, indicating an empty vector ready for use.
    pub const fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    /// Clears the vector by resetting the length to zero.
    ///
    /// Does not drop or deallocate elements; they remain in memory but are
    /// marked as uninitialized. This is a constant-time operation that prepares
    /// the vector for reuse without reallocation.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Appends an element to the end of the vector.
    ///
    /// Writes the item to the first uninitialized slot and increments the
    /// length. Returns an error if the vector is at capacity, allowing the
    /// caller to handle overflow conditions gracefully.
    ///
    /// # Arguments
    ///
    /// * `item` - Element to append
    ///
    /// # Returns
    ///
    /// Ok(()) if the item was added, Err(item) if the vector is full.
    #[inline(always)]
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.len < N {
            unsafe {
                self.data.get_unchecked_mut(self.len).write(item);
            }
            self.len += 1;
            Ok(())
        } else {
            Err(item)
        }
    }

    /// Removes and returns the last element of the vector.
    ///
    /// Decrements the length and reads the element that was at the end.
    /// Returns None if the vector is empty. The element is moved out of
    /// the vector, so it is no longer considered initialized.
    ///
    /// # Returns
    ///
    /// Some(element) if the vector was non-empty, None otherwise.
    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            unsafe { Some(self.data.get_unchecked(self.len).assume_init_read()) }
        } else {
            None
        }
    }

    /// Returns the number of elements in the vector.
    ///
    /// This is the current length, which may be less than the capacity N.
    /// The length indicates how many elements are initialized and accessible.
    ///
    /// # Returns
    ///
    /// The number of elements currently in the vector.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the vector contains no elements.
    ///
    /// Equivalent to checking if len() == 0, but provided for clarity and
    /// compatibility with standard collection APIs.
    ///
    /// # Returns
    ///
    /// True if the vector is empty, false otherwise.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the maximum capacity of the vector.
    ///
    /// This is always equal to the compile-time constant N. The capacity
    /// cannot change after construction, as the storage is statically allocated.
    ///
    /// # Returns
    ///
    /// The maximum number of elements the vector can hold (N).
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        N
    }

    /// Returns a slice view of the initialized elements.
    ///
    /// Creates a slice covering elements [0..len], which are guaranteed to be
    /// initialized. The slice provides read-only access to the vector's contents
    /// and can be used with standard slice operations and iterators.
    ///
    /// # Returns
    ///
    /// A slice containing all initialized elements.
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.data.as_ptr() as *const T, self.len) }
    }

    /// Returns a mutable slice view of the initialized elements.
    ///
    /// Creates a mutable slice covering elements [0..len], which are guaranteed
    /// to be initialized. The slice provides mutable access for in-place
    /// modifications of vector elements.
    ///
    /// # Returns
    ///
    /// A mutable slice containing all initialized elements.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len) }
    }
}

/// Enables indexing and slice operations via Deref.
///
/// Allows StaticVec to be used like a standard slice, enabling indexing
/// syntax (vec[i]) and automatic coercion to &[T] in function arguments.
impl<T, const N: usize> Deref for StaticVec<T, N> {
    /// The target type for dereferencing operations.
    ///
    /// StaticVec dereferences to a slice, enabling all slice methods and
    /// operations to work transparently on StaticVec instances.
    type Target = [T];

    /// Returns a reference to the initialized elements as a slice.
    ///
    /// Provides read-only access to elements [0..len], which are guaranteed
    /// to be initialized. This enables indexing syntax and automatic coercion
    /// to &[T] in function arguments.
    ///
    /// # Returns
    ///
    /// A slice reference covering all initialized elements.
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

/// Enables mutable indexing and slice operations via DerefMut.
///
/// Allows mutable indexing syntax (vec[i] = value) and automatic coercion
/// to &mut [T] in function arguments.
impl<T, const N: usize> DerefMut for StaticVec<T, N> {
    /// Returns a mutable reference to the initialized elements as a slice.
    ///
    /// Provides mutable access to elements [0..len], which are guaranteed
    /// to be initialized. This enables mutable indexing syntax and automatic
    /// coercion to &mut [T] in function arguments.
    ///
    /// # Returns
    ///
    /// A mutable slice reference covering all initialized elements.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

/// Enables iteration over vector elements via IntoIterator.
///
/// Allows StaticVec to be used in for loops and with iterator adapters.
/// The iterator consumes the vector, moving elements out as it iterates.
impl<T, const N: usize> IntoIterator for StaticVec<T, N> {
    /// The type of element yielded by the iterator.
    ///
    /// Elements are moved out of the vector during iteration, consuming
    /// the vector in the process.
    type Item = T;

    /// The iterator type that consumes the vector.
    ///
    /// A custom iterator that moves elements out of the vector as it
    /// progresses, ensuring proper ownership transfer.
    type IntoIter = StaticVecIntoIter<T, N>;

    /// Consumes the vector and returns an iterator over its elements.
    ///
    /// The iterator will yield elements in order from index 0 to len-1.
    /// After iteration completes, the vector is consumed and cannot be used.
    fn into_iter(self) -> Self::IntoIter {
        StaticVecIntoIter {
            vec: self,
            index: 0,
        }
    }
}

/// Iterator that consumes a StaticVec and yields its elements.
///
/// Moves elements out of the vector as it iterates, consuming the vector
/// in the process. Elements are yielded in order from first to last.
pub struct StaticVecIntoIter<T, const N: usize> {
    /// The vector being iterated over.
    vec: StaticVec<T, N>,

    /// Current iteration index.
    ///
    /// Points to the next element to yield. When index >= vec.len, iteration
    /// is complete.
    index: usize,
}

impl<T, const N: usize> Iterator for StaticVecIntoIter<T, N> {
    /// The type of element yielded by the iterator.
    ///
    /// Elements are moved out of the vector during iteration, transferring
    /// ownership to the caller.
    type Item = T;

    /// Returns the next element in the iteration.
    ///
    /// Moves the element out of the vector and advances the index. Returns
    /// None when all elements have been consumed.
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
