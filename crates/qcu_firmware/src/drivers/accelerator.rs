//! Hardware accelerator driver for union-find decoder operations.
//!
//! Provides an interface to the hardware-accelerated union-find data structure
//! operations. The accelerator performs path compression and union operations
//! in hardware to reduce decoder latency compared to software implementations.

use qcu_common::mmio::ACCELERATOR_BASE;

/// Hardware accelerator interface for decoder operations.
///
/// Encapsulates MMIO operations to communicate with the hardware union-find
/// accelerator. The accelerator accepts syndrome data and returns correction
/// results via memory-mapped registers.
pub struct DecoderAccelerator;

impl DecoderAccelerator {
    /// Triggers a hardware-accelerated decoding operation.
    ///
    /// Writes the syndrome buffer address and result buffer address to the
    /// accelerator's MMIO registers, then sets the trigger bit to start the
    /// operation. The accelerator will process the syndrome data and write
    /// corrections to the result buffer. A memory fence ensures all register
    /// writes are visible before the trigger is set.
    ///
    /// # Safety
    ///
    /// * `syndrome_ptr` must be a valid pointer to a syndrome buffer containing
    ///   detector indices that fired.
    /// * `result_ptr` must be a valid pointer to a writable result buffer large
    ///   enough to hold the correction edge pairs.
    /// * The hardware accelerator must be mapped at `ACCELERATOR_BASE` and
    ///   ready to accept commands.
    ///
    /// # Arguments
    ///
    /// * `syndrome_ptr` - Pointer to the syndrome data buffer
    /// * `result_ptr` - Pointer to the result buffer for corrections
    #[inline(always)]
    pub unsafe fn trigger_decode(syndrome_ptr: *const usize, result_ptr: *mut usize) {
        let base = ACCELERATOR_BASE as *mut u32;
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

    /// Polls the accelerator until the decode operation completes.
    ///
    /// Reads the status register in a loop until the accelerator indicates
    /// completion. Uses a memory fence after polling to ensure all results
    /// are visible before the caller accesses the result buffer. This is a
    /// blocking operation that spins until completion.
    #[inline(always)]
    pub fn poll_complete() {
        let base = ACCELERATOR_BASE as *const u32;
        unsafe {
            while base.add(1).read_volatile() & 1 == 1 {
                core::hint::spin_loop();
            }
        }
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
    }
}
