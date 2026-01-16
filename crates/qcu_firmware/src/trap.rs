//! RISC-V trap and interrupt handler.
//!
//! Handles machine-mode traps and interrupts, including timer interrupts
//! from the CLINT (Core Local Interruptor). Timer interrupts are cleared
//! by scheduling the next interrupt far in the future, effectively disabling
//! periodic timer interrupts for this firmware.

use riscv::register::mcause;

/// Trap handler called from assembly trap vector.
///
/// Reads the machine cause register to determine the trap type, then handles
/// timer interrupts by clearing them. Other trap types are currently ignored.
/// This function is called with interrupts disabled and must preserve all
/// registers except those used for return values.
///
/// # Safety
///
/// This function is marked as no_mangle and extern "C" to match the calling
/// convention expected by the assembly trap vector. It must be called only
/// from the trap vector and must not panic or perform operations that could
/// cause nested traps.
#[unsafe(no_mangle)]
pub extern "C" fn rust_trap_handler() {
    let _cause = mcause::read();

    if _cause.bits() == 0x8000000000000007 {
        // Base address of the CLINT (Core Local Interruptor) peripheral.
        //
        // The CLINT provides timer interrupts and software interrupts for
        // RISC-V cores. All CLINT registers are accessed relative to this
        // base address. The address 0x200_0000 is the standard location
        // for CLINT on QEMU's RISC-V virt platform.
        const CLINT_BASE: usize = 0x200_0000;

        // Memory-mapped address of the machine timer compare register.
        //
        // When the machine timer (mtime) reaches the value stored in this
        // register, a timer interrupt is generated. Writing a value far in
        // the future effectively disables periodic timer interrupts. The
        // offset 0x4000 from CLINT_BASE is the standard location for the
        // first hart's mtimecmp register.
        const MTIMECMP_ADDR: usize = CLINT_BASE + 0x4000;

        // Memory-mapped address of the machine timer register.
        //
        // Contains the current 64-bit timer value that increments at a
        // fixed frequency. Used for timestamping and measuring elapsed time.
        // The offset 0xBFF8 from CLINT_BASE is the standard location for
        // the mtime register.
        const MTIME_ADDR: usize = CLINT_BASE + 0xBFF8;
        unsafe {
            let now = (MTIME_ADDR as *const u64).read_volatile();
            (MTIMECMP_ADDR as *mut u64).write_volatile(now + 10_000_000);
        }
    }
}
