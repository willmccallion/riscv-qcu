#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    /// Apply Hadamard Gate
    GateH = 0x01,
    /// Apply Phase Gate (S)
    GateS = 0x02,
    /// Apply CNOT (Control, Target)
    GateCNOT = 0x03,
    /// Measure Qubit.
    /// Compares hardware result against Pauli Frame to determine syndrome.
    Measure = 0x10,
    /// Run the Union-Find Decoder
    Decode = 0x20,
    /// Reset Pauli Frame and Decoder state
    Reset = 0x30,
    /// Halt execution
    Halt = 0xFF,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    pub opcode: u8,
    pub operand_1: u16, // Qubit Index or Control
    pub operand_2: u16, // Target or Detector ID
    pub _padding: u8,   // Alignment padding
}

impl Instruction {
    pub fn new(opcode: Opcode, op1: u16, op2: u16) -> Self {
        Self {
            opcode: opcode as u8,
            operand_1: op1,
            operand_2: op2,
            _padding: 0,
        }
    }
}
