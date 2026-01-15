pub struct DecoderAccelerator;

impl DecoderAccelerator {
    pub const BASE_ADDR: usize = 0x4000_0000;

    /// # Safety
    /// * `syndrome_ptr` must be a valid pointer to a syndrome buffer in memory accessible by the accelerator.
    /// * `result_ptr` must be a valid pointer to a writable result buffer with sufficient capacity.
    /// * The hardware accelerator must be mapped at `BASE_ADDR`.
    #[inline(always)]
    pub unsafe fn trigger_decode(syndrome_ptr: *const usize, result_ptr: *mut usize) {
        let base = Self::BASE_ADDR as *mut u32;
        let s_addr = syndrome_ptr as usize;
        let r_addr = result_ptr as usize;

        unsafe {
            base.add(2).write_volatile(s_addr as u32);
            base.add(3).write_volatile((s_addr >> 32) as u32);
            base.add(4).write_volatile(r_addr as u32);
            base.add(5).write_volatile((r_addr >> 32) as u32);

            core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
            base.add(0).write_volatile(1);
        }
    }

    #[inline(always)]
    pub fn poll_complete() {
        let base = Self::BASE_ADDR as *const u32;
        unsafe {
            while base.add(1).read_volatile() & 1 == 1 {
                core::hint::spin_loop();
            }
        }
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
    }
}
