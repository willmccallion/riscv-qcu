/// Base address of the CLINT (Core Local Interruptor) in QEMU 'virt' machine.
pub const CLINT_BASE: usize = 0x200_0000;
pub const MTIMECMP_ADDR: usize = CLINT_BASE + 0x4000;
pub const MTIME_ADDR: usize = CLINT_BASE + 0xBFF8;

/// Base address for our simulated QPU MMIO.
/// In a real FPGA, this would be 0x4000_0000 or similar.
/// We will map this to a static buffer in firmware for simulation.
pub const QPU_BASE_ADDR: usize = 0x8000_0000; // High RAM

#[repr(C)]
pub struct QpuInterface {
    /// Write 1 to trigger measurement
    pub trigger: u32,
    /// Read status (1 = data ready)
    pub status: u32,
    /// Read measurement data (32 bits at a time)
    pub data_fifo: u32,
}

impl QpuInterface {
    /// Safety: Must be called only if the memory region is mapped.
    pub fn get() -> &'static mut Self {
        unsafe { &mut *(QPU_BASE_ADDR as *mut Self) }
    }
}
