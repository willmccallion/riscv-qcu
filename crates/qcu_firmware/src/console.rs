use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for SpinLock<T> {}
impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
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
pub struct SpinLockGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a UnsafeCell<T>,
}
impl<'a, T> core::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.data.get() }
    }
}
impl<'a, T> core::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

const UART0: *mut u8 = 0x1000_0000 as *mut u8;
static CONSOLE_LOCK: SpinLock<()> = SpinLock::new(());

pub struct Uart;
impl fmt::Write for Uart {
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

pub fn init() {}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    let _guard = CONSOLE_LOCK.lock();
    let _ = Uart.write_fmt(args);
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::console::_print(format_args!($($arg)*));
        $crate::console::_print(format_args!("\n"));
    });
}
pub use println;
