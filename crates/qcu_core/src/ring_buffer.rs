//! Lock-free single-producer single-consumer ring buffer.
//!
//! Implements a circular buffer for efficient message passing between a single
//! producer thread and a single consumer thread without locks. Uses atomic
//! operations on head and tail pointers to coordinate access, enabling
//! low-latency communication in real-time systems.

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Lock-free ring buffer for single-producer single-consumer communication.
///
/// Provides a circular buffer where one thread can push items and another
/// thread can pop items concurrently without locks. The buffer uses atomic
/// head and tail pointers with acquire-release ordering to ensure correct
/// visibility of written data. Capacity must be a power of two to enable
/// efficient modulo operations via bit masking.
///
/// # Type Parameters
///
/// * `T` - Element type, must be Default and Copy for initialization
pub struct RingBuffer<T> {
    /// Backing storage for the circular buffer.
    ///
    /// Wrapped in UnsafeCell to allow mutable access from immutable references,
    /// which is safe because the SPSC contract ensures only one thread accesses
    /// each end of the buffer.
    buffer: UnsafeCell<Vec<T>>,

    /// Fixed capacity of the buffer (must be power of two).
    capacity: usize,

    /// Bit mask for efficient modulo operations (capacity - 1).
    ///
    /// Used instead of modulo operator since capacity is a power of two,
    /// enabling faster index calculation via bitwise AND.
    mask: usize,

    /// Atomic head pointer (producer index).
    ///
    /// Points to the next slot where data will be written. Incremented by
    /// the producer thread after writing an item.
    head: AtomicUsize,

    /// Atomic tail pointer (consumer index).
    ///
    /// Points to the next slot to read from. Incremented by the consumer
    /// thread after reading an item.
    tail: AtomicUsize,
}

/// RingBuffer is safe to share between threads under SPSC constraints.
///
/// The single-producer single-consumer contract ensures that only one thread
/// writes (via push) and only one thread reads (via pop), making the UnsafeCell
/// access safe despite the lack of explicit synchronization.
unsafe impl<T: Send> Sync for RingBuffer<T> {}
unsafe impl<T: Send> Send for RingBuffer<T> {}

impl<T: Default + Copy> RingBuffer<T> {
    /// Creates a new ring buffer with the specified capacity.
    ///
    /// The capacity must be a power of two to enable efficient modulo operations.
    /// All buffer slots are initialized to the default value of T. The buffer
    /// is ready for use after construction, with head and tail both at zero.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Buffer size (must be power of two and greater than zero)
    ///
    /// # Panics
    ///
    /// Panics if capacity is zero or not a power of two.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0 && capacity.is_power_of_two());
        let mut vec = Vec::with_capacity(capacity);
        vec.resize_with(capacity, Default::default);

        Self {
            buffer: UnsafeCell::new(vec),
            capacity,
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Pushes an item into the buffer (producer operation).
    ///
    /// Attempts to write the item at the current head position. Returns false
    /// if the buffer is full (head has wrapped around and caught up to tail).
    /// Uses acquire ordering when reading tail to ensure visibility of consumer
    /// updates, and release ordering when updating head to make the written
    /// data visible to the consumer.
    ///
    /// # Arguments
    ///
    /// * `item` - Item to enqueue
    ///
    /// # Returns
    ///
    /// True if the item was successfully enqueued, false if the buffer is full.
    pub fn push(&self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= self.capacity {
            return false;
        }

        unsafe {
            let ptr = (*self.buffer.get()).as_mut_ptr();
            ptr.add(head & self.mask).write(item);
        }

        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Pops an item from the buffer (consumer operation).
    ///
    /// Attempts to read an item from the current tail position. Returns None
    /// if the buffer is empty (tail has caught up to head). Uses acquire
    /// ordering when reading head to ensure visibility of producer updates,
    /// and release ordering when updating tail to signal that the slot is
    /// available for reuse.
    ///
    /// # Returns
    ///
    /// Some(item) if an item was dequeued, None if the buffer is empty.
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        let item = unsafe {
            let ptr = (*self.buffer.get()).as_ptr();
            ptr.add(tail & self.mask).read()
        };

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(item)
    }
}
