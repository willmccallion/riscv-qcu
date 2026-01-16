//! Hardware interface definitions for quantum processing unit access.
//!
//! Provides memory-mapped I/O structures and constants for interacting with
//! quantum hardware peripherals. These definitions are used by firmware to
//! trigger measurements, read syndrome data, and control quantum operations
//! through MMIO registers.

/// Base address of the CLINT (Core Local Interruptor) in QEMU 'virt' machine.
///
/// Standard address for the RISC-V Core Local Interruptor on QEMU's virt
/// platform. Used for timer interrupt management and inter-hart communication.
pub const CLINT_BASE: usize = 0x200_0000;

/// Memory-mapped address for machine timer compare register.
///
/// Writing a value to this register schedules a timer interrupt when the
/// machine timer counter reaches that value. Offset is 0x4000 for hart 0,
/// with 8-byte increments per additional hardware thread.
pub const MTIMECMP_ADDR: usize = CLINT_BASE + 0x4000;

/// Memory-mapped address for machine timer counter register.
///
/// 64-bit read-only register that increments at a fixed frequency (typically
/// 10 MHz in QEMU). Used for timestamping and scheduling periodic events
/// in real-time firmware.
pub const MTIME_ADDR: usize = CLINT_BASE + 0xBFF8;

/// Base address for simulated quantum processing unit MMIO interface.
///
/// Memory-mapped region for accessing quantum hardware control registers.
/// In simulation, this maps to a static buffer in firmware memory. On real
/// FPGA hardware, this would correspond to the physical MMIO base address
/// of the quantum accelerator. Located in high RAM region starting at 0x8000_0000.
pub const QPU_BASE_ADDR: usize = 0x8000_0000;

/// Memory-mapped interface structure for quantum processing unit control.
///
/// Defines the register layout for triggering measurements, reading status,
/// and accessing measurement data from quantum hardware. The structure is
/// packed to match hardware register layout and must be accessed via volatile
/// operations to prevent compiler optimizations.
#[repr(C)]
pub struct QpuInterface {
    /// Measurement trigger register.
    ///
    /// Writing 1 to this register initiates a measurement cycle on the
    /// quantum hardware. The firmware must wait for the status register
    /// to indicate completion before reading results.
    pub trigger: u32,

    /// Status register indicating measurement completion.
    ///
    /// Read as 1 when measurement data is ready in the data FIFO, 0 otherwise.
    /// The firmware polls this register after triggering a measurement to
    /// determine when results are available.
    pub status: u32,

    /// Measurement data FIFO register.
    ///
    /// Contains 32 bits of measurement results when status indicates data
    /// is ready. For measurements exceeding 32 bits, multiple reads are
    /// required to drain the FIFO. Results are packed with least significant
    /// bits corresponding to lower-indexed qubits.
    pub data_fifo: u32,
}

impl QpuInterface {
    /// Returns a mutable reference to the QPU interface at the MMIO base address.
    ///
    /// Provides unsafe access to the memory-mapped hardware registers. The
    /// caller must ensure that the memory region is properly mapped and that
    /// all accesses use volatile operations to prevent reordering or elision
    /// by the compiler.
    ///
    /// # Safety
    ///
    /// The memory region at QPU_BASE_ADDR must be mapped to valid hardware
    /// registers or a simulation buffer. Concurrent access from multiple
    /// threads or interrupt handlers requires external synchronization.
    ///
    /// # Returns
    ///
    /// A mutable reference to the QPU interface structure.
    pub fn get() -> &'static mut Self {
        unsafe { &mut *(QPU_BASE_ADDR as *mut Self) }
    }
}
