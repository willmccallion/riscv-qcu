//! Lock-free single-producer multi-consumer queue with static allocation.
//!
//! Implements a circular buffer that supports one producer thread and multiple
//! consumer threads concurrently. Uses compare-and-swap operations on the tail
//! pointer to handle concurrent consumers safely. The buffer is statically
//! allocated at compile time, making it suitable for no_std firmware environments.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Statically allocated lock-free single-producer multi-consumer queue.
///
/// Provides a circular buffer where one thread can push items and multiple
/// threads can pop items concurrently without locks. Uses compare-and-swap
/// on the tail pointer to coordinate between competing consumers. The buffer
/// capacity must be a power of two to enable efficient modulo via bit masking.
/// Cache line padding is included to reduce false sharing between head and
/// tail pointers.
///
/// # Type Parameters
///
/// * `T` - Element type, must be Copy for efficient reads
/// * `N` - Buffer capacity (must be power of two)
pub struct StaticQueue<T, const N: usize> {
    buffer: [UnsafeCell<MaybeUninit<T>>; N],
    _pad0: [u8; 64],
    head: AtomicUsize,
    _pad1: [u8; 64],
    tail: AtomicUsize,
}

/// StaticQueue is safe to share between threads under SPMC constraints.
///
/// The single-producer contract ensures only one thread writes, and the
/// compare-and-swap on tail ensures safe concurrent reads by multiple consumers.
unsafe impl<T: Send, const N: usize> Sync for StaticQueue<T, N> {}
unsafe impl<T: Send, const N: usize> Send for StaticQueue<T, N> {}

impl<T: Copy, const N: usize> Default for StaticQueue<T, N> {
    /// Creates a queue with default (empty) state.
    ///
    /// Equivalent to calling `new()`, provided for trait compatibility.
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> StaticQueue<T, N> {
    /// Creates a new static queue with empty state.
    ///
    /// The buffer slots are left uninitialized until items are pushed. Head
    /// and tail are both initialized to zero. The capacity N must be a power
    /// of two for correct operation.
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            _pad0: [0; 64],
            head: AtomicUsize::new(0),
            _pad1: [0; 64],
            tail: AtomicUsize::new(0),
        }
    }

    /// Pushes an item into the queue (producer operation).
    ///
    /// Writes the item at the current head position and increments head. Returns
    /// an error if the buffer is full (head has wrapped around and caught up
    /// to tail). Uses acquire ordering when reading tail to ensure visibility
    /// of consumer updates, and release ordering when updating head to make
    /// the written data visible to consumers.
    ///
    /// # Arguments
    ///
    /// * `item` - Item to enqueue
    ///
    /// # Returns
    ///
    /// Ok(()) if the item was enqueued, Err(item) if the buffer is full.
    #[inline(always)]
    pub fn push(&self, item: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= N {
            return Err(item);
        }

        unsafe {
            let slot = self.buffer[head & (N - 1)].get();
            (*slot).write(item);
        }

        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Pops an item from the queue (consumer operation).
    ///
    /// Attempts to claim and read an item from the current tail position using
    /// compare-and-swap to handle concurrent consumers. Returns None if the
    /// buffer is empty (tail has caught up to head). The compare-and-swap loop
    /// retries if another consumer claimed the slot first, ensuring each item
    /// is consumed exactly once.
    ///
    /// # Returns
    ///
    /// Some(item) if an item was dequeued, None if the buffer is empty.
    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let mut tail = self.tail.load(Ordering::Relaxed);
        loop {
            let head = self.head.load(Ordering::Acquire);

            if tail == head {
                return None;
            }

            match self.tail.compare_exchange_weak(
                tail,
                tail.wrapping_add(1),
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let item = unsafe {
                        let slot = self.buffer[tail & (N - 1)].get();
                        (*slot).assume_init()
                    };
                    return Some(item);
                }
                Err(actual_tail) => {
                    tail = actual_tail;
                }
            }
        }
    }
}
