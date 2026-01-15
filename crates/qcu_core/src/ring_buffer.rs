use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct RingBuffer<T> {
    buffer: UnsafeCell<Vec<T>>,
    capacity: usize,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

unsafe impl<T: Send> Sync for RingBuffer<T> {}
unsafe impl<T: Send> Send for RingBuffer<T> {}

impl<T: Default + Copy> RingBuffer<T> {
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

    pub fn push(&self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= self.capacity {
            return false;
        }

        unsafe {
            // SAFETY: We are the only producer (SPSC).
            // Capacity is power of 2, so (head & mask) is within bounds.
            let ptr = (*self.buffer.get()).as_mut_ptr();
            ptr.add(head & self.mask).write(item);
        }

        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        let item = unsafe {
            // SAFETY: We are the only consumer (SPSC).
            // Data at tail is initialized and valid because head > tail.
            let ptr = (*self.buffer.get()).as_ptr();
            ptr.add(tail & self.mask).read()
        };

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(item)
    }
}
