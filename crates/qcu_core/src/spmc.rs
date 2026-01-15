use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A statically allocated, Lock-Free Single-Producer Multi-Consumer (SPMC) Queue.
pub struct StaticQueue<T, const N: usize> {
    buffer: [UnsafeCell<MaybeUninit<T>>; N],
    _pad0: [u8; 64],
    head: AtomicUsize,
    _pad1: [u8; 64],
    tail: AtomicUsize,
}

unsafe impl<T: Send, const N: usize> Sync for StaticQueue<T, N> {}
unsafe impl<T: Send, const N: usize> Send for StaticQueue<T, N> {}

impl<T: Copy, const N: usize> Default for StaticQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> StaticQueue<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            _pad0: [0; 64],
            head: AtomicUsize::new(0),
            _pad1: [0; 64],
            tail: AtomicUsize::new(0),
        }
    }

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
