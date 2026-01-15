use riscv::register::mcause;

#[unsafe(no_mangle)]
pub extern "C" fn rust_trap_handler() {
    let _cause = mcause::read();

    // If it was a timer interrupt (bit 63 set, cause 7), clear it far into future
    if _cause.bits() == 0x8000000000000007 {
        const CLINT_BASE: usize = 0x200_0000;
        const MTIMECMP_ADDR: usize = CLINT_BASE + 0x4000;
        const MTIME_ADDR: usize = CLINT_BASE + 0xBFF8;
        unsafe {
            let now = (MTIME_ADDR as *const u64).read_volatile();
            (MTIMECMP_ADDR as *mut u64).write_volatile(now + 10_000_000);
        }
    }
}
