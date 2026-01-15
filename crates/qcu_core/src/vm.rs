use crate::QecError;
use crate::allocator::BumpAllocator;
use crate::decoder::UnionFindDecoder;
use crate::graph::DecodingGraph;
use crate::isa::{Instruction, Opcode};
use crate::pauli_frame::PauliFrame;
use alloc::vec::Vec;

pub struct VirtualMachine<'a, const N: usize>
where
    [(); N.div_ceil(64)]:,
{
    pub frame: PauliFrame,
    pub decoder: UnionFindDecoder<N>,
    pub graph: &'a DecodingGraph,
    pub syndrome_buffer: Vec<usize>,
    pub correction_buffer: Vec<(usize, usize)>,
}

impl<'a, const N: usize> VirtualMachine<'a, N>
where
    [(); N.div_ceil(64)]:,
{
    pub fn new(alloc: &BumpAllocator, graph: &'a DecodingGraph, num_qubits: usize) -> Self {
        Self {
            frame: PauliFrame::new(alloc, num_qubits),
            decoder: UnionFindDecoder::new(),
            graph,
            syndrome_buffer: Vec::with_capacity(256),
            correction_buffer: Vec::with_capacity(256),
        }
    }

    pub fn execute(&mut self, instr: &Instruction, hw_measure: bool) -> Result<(), QecError> {
        let op = instr.opcode;

        if op == Opcode::GateH as u8 {
            self.frame.apply_hadamard(instr.operand_1 as usize);
        } else if op == Opcode::GateCNOT as u8 {
            self.frame
                .apply_cnot(instr.operand_1 as usize, instr.operand_2 as usize);
        } else if op == Opcode::Measure as u8 {
            let expected = self.frame.has_x_error(instr.operand_1 as usize);
            let is_syndrome = expected ^ hw_measure;
            if is_syndrome {
                self.syndrome_buffer.push(instr.operand_2 as usize);
            }
        } else if op == Opcode::Decode as u8 {
            self.decoder.solve_into(
                self.graph,
                &self.syndrome_buffer,
                &mut self.correction_buffer,
            )?;
            self.syndrome_buffer.clear();
        } else if op == Opcode::Reset as u8 {
            self.frame.reset();
            self.syndrome_buffer.clear();
        }

        Ok(())
    }
}
