//! Common definitions and constants shared across the quantum control unit system.
//!
//! This module provides memory-mapped I/O addresses for hardware peripherals,
//! instruction set architecture definitions for quantum operations, and other
//! shared constants used by firmware, host tools, and hardware simulation.

#![no_std]

// Memory-mapped I/O address space definitions for the system-on-chip.
//
// Defines the physical address layout for peripherals including interrupt
// controllers, quantum processing accelerators, and memory regions. These
// addresses must match the hardware memory map and are used by both firmware
// and host-side drivers for MMIO access.
pub mod mmio {
    /// Base address of the CLINT (Core Local Interruptor) in QEMU 'virt' machine.
    ///
    /// The CLINT provides per-hart machine-mode timer interrupts and software
    /// interrupt generation. This address is standard for QEMU's RISC-V virt
    /// platform and must match the device tree configuration.
    pub const CLINT_BASE: usize = 0x200_0000;

    /// Memory-mapped address for the machine timer compare register.
    ///
    /// When the machine timer (MTIME) reaches this value, a timer interrupt
    /// is generated. Writing to this register schedules the next interrupt.
    /// Offset from CLINT_BASE is 0x4000 for hart 0, with 8-byte increments
    /// per additional hart.
    pub const MTIMECMP_ADDR: usize = CLINT_BASE + 0x4000;

    /// Memory-mapped address for the machine timer counter register.
    ///
    /// This 64-bit read-only register increments at a fixed frequency (typically
    /// 10 MHz in QEMU). Used for timestamping and scheduling periodic events
    /// in the firmware scheduler.
    pub const MTIME_ADDR: usize = CLINT_BASE + 0xBFF8;

    /// Base address for the Union-Find Decoder Accelerator.
    ///
    /// Memory-mapped interface to the hardware-accelerated union-find data
    /// structure operations. The accelerator performs path compression and
    /// union operations in hardware to reduce decoder latency. This address
    /// must match the Verilog module's MMIO base address.
    pub const ACCELERATOR_BASE: usize = 0x4000_0000;

    /// Base address of high RAM region.
    ///
    /// Start of the main system memory region where firmware code, data
    /// structures, and heap allocations reside. This address is standard for
    /// QEMU virt machine and marks the transition from device memory space
    /// to general-purpose RAM.
    pub const RAM_BASE: usize = 0x8000_0000;
}

/// Instruction Set Architecture definitions for quantum error correction operations.
///
/// Defines the binary encoding of quantum operations, measurements, and control
/// instructions that the virtual machine executes. The instruction format is
/// designed for efficient decoding in firmware and compatibility with hardware
/// instruction fetch units.
pub mod isa {
    /// Opcode enumeration for quantum error correction instructions.
    ///
    /// Each opcode represents a distinct operation in the quantum error
    /// correction pipeline: gate applications, measurements, decoding, and
    /// system control. The numeric values are chosen to allow efficient
    /// dispatch via jump tables or match statements.
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Opcode {
        /// Apply Hadamard gate to the specified qubit.
        ///
        /// Transforms the qubit between X and Z eigenbases, requiring
        /// corresponding updates to the Pauli frame's X and Z registers.
        /// This is a Clifford gate and preserves the stabilizer formalism.
        GateH = 0x01,

        /// Apply Phase gate (S gate) to the specified qubit.
        ///
        /// Applies a π/2 rotation around the Z axis. Like Hadamard, this
        /// gate requires Pauli frame updates to maintain the stabilizer
        /// representation of the quantum state.
        GateS = 0x02,

        /// Apply CNOT gate with control and target qubits.
        ///
        /// Performs a controlled-NOT operation that entangles the control
        /// and target qubits. The Pauli frame must be updated to reflect
        /// the transformation of both X and Z operators under conjugation
        /// by CNOT.
        GateCNOT = 0x03,

        /// Measure a qubit and record the result.
        ///
        /// Performs a destructive measurement in the Z basis and compares
        /// the result against the Pauli frame's X error prediction. If the
        /// measurement differs from expectation, a syndrome bit is recorded
        /// for the associated detector.
        Measure = 0x10,

        /// Execute the Union-Find decoder on accumulated syndrome data.
        ///
        /// Triggers the decoding algorithm to process all syndrome bits
        /// collected since the last decode operation. The decoder computes
        /// correction operations that will restore the logical state, and
        /// these corrections are applied to the Pauli frame.
        Decode = 0x20,

        /// Reset the Pauli frame and clear all syndrome buffers.
        ///
        /// Clears all tracked X and Z errors in the Pauli frame, effectively
        /// resetting the quantum state to the initial |0...0⟩ state. Also
        /// clears the syndrome buffer to prepare for a new error correction
        /// cycle. Used for initialization and recovery from uncorrectable errors.
        Reset = 0x30,

        /// Halt execution and enter idle state.
        ///
        /// Stops instruction processing and places the system in a low-power
        /// state. Used for graceful shutdown or when all quantum operations
        /// have completed. The system remains responsive to interrupts but
        /// does not execute further instructions.
        Halt = 0xFF,
    }

    /// Binary instruction format for quantum error correction operations.
    ///
    /// Encodes a single quantum operation with opcode and operands in a
    /// compact 6-byte format suitable for instruction fetch and decode
    /// pipelines. The packed representation ensures efficient memory usage
    /// and cache-friendly instruction streams.
    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy)]
    pub struct Instruction {
        /// Operation code identifying the instruction type.
        ///
        /// Must match one of the values defined in the Opcode enumeration.
        /// Invalid opcodes are treated as no-ops or trigger error handling
        /// depending on the execution context.
        pub opcode: u8,

        /// First operand: qubit index, control qubit, or detector identifier.
        ///
        /// Interpretation depends on the opcode: for gates, this is the
        /// target qubit index; for CNOT, this is the control qubit; for
        /// measurements, this is the qubit being measured.
        pub operand_1: u16,

        /// Second operand: target qubit, detector ID, or unused.
        ///
        /// For CNOT instructions, this specifies the target qubit. For
        /// measurement instructions, this identifies the detector that
        /// records the syndrome bit. For single-qubit gates, this field
        /// is ignored.
        pub operand_2: u16,

        /// Padding byte to maintain 8-byte alignment.
        ///
        /// Ensures the instruction structure aligns to word boundaries
        /// for efficient memory access patterns. Always set to zero and
        /// ignored during instruction execution.
        pub _padding: u8,
    }

    impl Instruction {
        /// Constructs a new instruction with the specified opcode and operands.
        ///
        /// Initializes all fields including zeroing the padding byte to ensure
        /// deterministic instruction encoding. The opcode is cast to u8 to
        /// match the binary format, and operands are truncated to u16 if
        /// necessary.
        ///
        /// # Arguments
        ///
        /// * `opcode` - The operation to perform
        /// * `op1` - First operand (qubit index, control, etc.)
        /// * `op2` - Second operand (target qubit, detector ID, etc.)
        pub fn new(opcode: Opcode, op1: u16, op2: u16) -> Self {
            Self {
                opcode: opcode as u8,
                operand_1: op1,
                operand_2: op2,
                _padding: 0,
            }
        }
    }
}
