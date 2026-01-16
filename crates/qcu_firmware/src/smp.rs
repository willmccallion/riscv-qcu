//! Symmetric multiprocessing synchronization primitives.
//!
//! Provides spinlock implementation for coordinating access to shared resources
//! between multiple hardware threads (harts) in a multi-core system. Used for
//! protecting critical sections when multiple cores need to access the same
//! data structures or hardware peripherals.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// Spinlock for mutual exclusion in no_std multi-core environments.
///
/// Provides lock-based synchronization using an atomic boolean flag and
/// busy-waiting. Used to serialize access to shared resources when multiple
/// hardware threads need to coordinate. The lock is released automatically
/// when the guard is dropped.
pub struct SpinLock<T> {
    /// Atomic flag indicating whether the lock is held.
    ///
    /// False means unlocked, true means locked. Modified via compare-and-swap
    /// to ensure atomic acquisition across multiple cores.
    lock: AtomicBool,

    /// Protected data wrapped in UnsafeCell for interior mutability.
    data: UnsafeCell<T>,
}

/// SpinLock is safe to share between hardware threads when T is Send.
///
/// The atomic lock flag ensures that only one hart can acquire the lock
/// at a time, making concurrent access to the protected data safe across
/// multiple cores.
unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new spinlock with the specified initial value.
    ///
    /// The lock starts in the unlocked state, ready for acquisition by any
    /// hardware thread.
    ///
    /// # Arguments
    ///
    /// * `data` - Initial value to protect
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquires the lock, returning a guard that releases it on drop.
    ///
    /// Spins in a loop using compare-and-swap until the lock is successfully
    /// acquired. The guard provides mutable access to the protected data
    /// and automatically releases the lock when dropped. Uses acquire ordering
    /// to ensure visibility of previous critical section operations.
    ///
    /// # Returns
    ///
    /// A guard that provides access to the protected data.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}

/// Guard that holds a spinlock and releases it on drop.
///
/// Provides mutable access to the protected data via Deref and DerefMut.
/// The lock is automatically released when the guard is dropped, ensuring
/// the lock is never held indefinitely and preventing deadlocks.
pub struct SpinLockGuard<'a, T> {
    /// Reference to the spinlock for releasing on drop.
    lock: &'a SpinLock<T>,
}

impl<'a, T> core::ops::Deref for SpinLockGuard<'a, T> {
    /// The target type for dereferencing operations.
    ///
    /// The guard dereferences directly to the protected data type T, enabling
    /// transparent access to the locked resource across multiple hardware threads.
    type Target = T;

    /// Returns a reference to the protected data.
    ///
    /// Provides read-only access to the data protected by the spinlock. The
    /// lock remains held while the returned reference exists, ensuring exclusive
    /// access until the guard is dropped. Safe for concurrent access from
    /// multiple hardware threads due to the atomic lock mechanism.
    ///
    /// # Returns
    ///
    /// An immutable reference to the protected data.
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinLockGuard<'a, T> {
    /// Returns a mutable reference to the protected data.
    ///
    /// Provides mutable access to the data protected by the spinlock. The
    /// lock remains held while the returned reference exists, ensuring exclusive
    /// access until the guard is dropped. This enables in-place modifications
    /// of the protected data across multiple hardware threads with proper
    /// synchronization guarantees.
    ///
    /// # Returns
    ///
    /// A mutable reference to the protected data.
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    /// Releases the lock by setting the flag to false.
    ///
    /// Uses release ordering to ensure all writes to the protected data
    /// are visible to the next hart that acquires the lock.
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
    }
}
