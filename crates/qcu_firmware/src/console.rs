//! UART console implementation for firmware debugging output.
//!
//! Provides a simple console interface that writes to the QEMU UART device
//! at address 0x1000_0000. Uses a spinlock to ensure thread-safe output
//! when multiple cores attempt to print simultaneously.

use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

/// Spinlock for mutual exclusion in no_std environments.
///
/// Provides lock-based synchronization using an atomic boolean flag and
/// busy-waiting. Used to serialize access to the UART device when multiple
/// cores attempt to print simultaneously. The lock is released automatically
/// when the guard is dropped.
pub struct SpinLock<T> {
    /// Atomic flag indicating whether the lock is held.
    ///
    /// False means unlocked, true means locked. Modified via compare-and-swap
    /// to ensure atomic acquisition.
    lock: AtomicBool,

    /// Protected data wrapped in UnsafeCell for interior mutability.
    data: UnsafeCell<T>,
}

/// SpinLock is safe to share between threads when T is Send.
///
/// The atomic lock flag ensures that only one thread can acquire the lock
/// at a time, making concurrent access to the protected data safe.
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new spinlock with the specified initial value.
    ///
    /// The lock starts in the unlocked state, ready for acquisition.
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
    /// and automatically releases the lock when dropped.
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
        SpinLockGuard {
            lock: &self.lock,
            data: &self.data,
        }
    }
}

/// Guard that holds a spinlock and releases it on drop.
///
/// Provides mutable access to the protected data via Deref and DerefMut.
/// The lock is automatically released when the guard is dropped, ensuring
/// the lock is never held indefinitely.
pub struct SpinLockGuard<'a, T> {
    /// Reference to the lock flag for releasing on drop.
    lock: &'a AtomicBool,

    /// Reference to the protected data.
    data: &'a UnsafeCell<T>,
}

impl<'a, T> core::ops::Deref for SpinLockGuard<'a, T> {
    /// The target type for dereferencing operations.
    ///
    /// The guard dereferences directly to the protected data type T, enabling
    /// transparent access to the locked resource.
    type Target = T;

    /// Returns a reference to the protected data.
    ///
    /// Provides read-only access to the data protected by the spinlock. The
    /// lock remains held while the returned reference exists, ensuring exclusive
    /// access until the guard is dropped.
    ///
    /// # Returns
    ///
    /// An immutable reference to the protected data.
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinLockGuard<'a, T> {
    /// Returns a mutable reference to the protected data.
    ///
    /// Provides mutable access to the data protected by the spinlock. The
    /// lock remains held while the returned reference exists, ensuring exclusive
    /// access until the guard is dropped. This enables in-place modifications
    /// of the protected data.
    ///
    /// # Returns
    ///
    /// A mutable reference to the protected data.
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    /// Releases the lock by setting the flag to false.
    ///
    /// Uses release ordering to ensure all writes to the protected data
    /// are visible to the next thread that acquires the lock.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

/// Memory-mapped address of the QEMU UART device.
///
/// Standard address for the UART on QEMU's RISC-V virt platform. Writing
/// a byte to this address transmits it over the serial console.
const UART0: *mut u8 = 0x1000_0000 as *mut u8;

/// Global spinlock protecting UART access.
///
/// Ensures that only one core can write to the UART at a time, preventing
/// interleaved output from corrupting console messages.
static CONSOLE_LOCK: SpinLock<()> = SpinLock::new(());

/// UART device interface for formatted output.
///
/// Implements fmt::Write to enable formatted printing via the write! macro.
/// Handles newline conversion (LF to CRLF) for compatibility with serial
/// terminal expectations.
pub struct Uart;

impl fmt::Write for Uart {
    /// Writes a string to the UART device.
    ///
    /// Converts newline characters ('\n') to carriage return + line feed
    /// ("\r\n") for proper terminal display. All writes use volatile
    /// operations to prevent compiler optimizations from reordering or
    /// eliding the I/O.
    ///
    /// # Arguments
    ///
    /// * `s` - String to write
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            unsafe {
                if c == b'\n' {
                    core::ptr::write_volatile(UART0, b'\r');
                }
                core::ptr::write_volatile(UART0, c);
            }
        }
        Ok(())
    }
}

/// Initializes the console subsystem.
///
/// Currently a no-op, as the UART requires no initialization on QEMU.
/// Provided for API compatibility and future hardware-specific setup.
pub fn init() {}

/// Internal function for printing formatted arguments.
///
/// Acquires the console lock, formats the arguments to the UART, and
/// releases the lock. This function is called by the println! macro
/// and should not be called directly.
///
/// # Arguments
///
/// * `args` - Formatted arguments to print
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    let _guard = CONSOLE_LOCK.lock();
    let _ = Uart.write_fmt(args);
}

/// Macro for printing a line to the console.
///
/// Formats the arguments and prints them followed by a newline. Thread-safe
/// via the console lock, so multiple cores can print simultaneously without
/// corruption.
///
/// # Example
///
/// ```ignore
/// println!("Value: {}", 42);
/// ```
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::console::_print(format_args!($($arg)*));
        $crate::console::_print(format_args!("\n"));
    });
}
pub use println;
