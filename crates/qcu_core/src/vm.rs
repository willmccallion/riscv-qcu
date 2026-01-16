//! Virtual machine for executing quantum error correction instructions.
//!
//! Implements a virtual machine that processes quantum instructions, maintains
//! Pauli frame state, collects syndrome measurements, and triggers decoding
//! operations. The VM orchestrates the interaction between gate applications,
//! measurements, and error correction to simulate a complete quantum error
//! correction cycle.

use crate::QecError;
use crate::allocator::BumpAllocator;
use crate::decoder::UnionFindDecoder;
use crate::graph::DecodingGraph;
use crate::pauli_frame::PauliFrame;
use alloc::vec::Vec;
use qcu_common::isa::{Instruction, Opcode};

/// Virtual machine for quantum error correction instruction execution.
///
/// Maintains the complete state needed to execute quantum error correction
/// programs: Pauli frame tracking, decoder instance, decoding graph reference,
/// and buffers for syndrome collection and correction output. The VM processes
/// instructions sequentially, updating state and triggering decoding when
/// requested.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the decoding graph reference
/// * `N` - Maximum number of nodes the decoder can handle (must satisfy
///   the constraint that N.div_ceil(64) is a valid array size)
pub struct VirtualMachine<'a, const N: usize>
where
    [(); N.div_ceil(64)]:,
{
    /// Pauli frame tracking X and Z errors on logical qubits.
    ///
    /// Maintains the stabilizer state representation, updated by gate
    /// operations and used to predict measurement outcomes for syndrome
    /// generation.
    pub frame: PauliFrame,

    /// Union-Find decoder instance for processing syndrome data.
    ///
    /// Used to find correction paths when a decode instruction is executed.
    /// The decoder's capacity N must be large enough to handle all nodes
    /// in the decoding graph.
    pub decoder: UnionFindDecoder<N>,

    /// Reference to the decoding graph defining the error model topology.
    ///
    /// Used by the decoder to find correction paths. The graph is shared
    /// and not modified by the VM, so a reference is sufficient.
    pub graph: &'a DecodingGraph,

    /// Buffer accumulating detector indices that fired (syndrome bits).
    ///
    /// Collects detector IDs from measurement instructions where the actual
    /// measurement differs from the Pauli frame prediction. This buffer is
    /// passed to the decoder when a decode instruction is executed.
    pub syndrome_buffer: Vec<usize>,

    /// Buffer receiving correction edge pairs from the decoder.
    ///
    /// Populated by the decoder's solve_into method with (u, v) edge pairs
    /// that should be flipped to correct detected errors. The corrections
    /// are applied to the Pauli frame to restore the logical state.
    pub correction_buffer: Vec<(usize, usize)>,
}

impl<'a, const N: usize> VirtualMachine<'a, N>
where
    [(); N.div_ceil(64)]:,
{
    /// Creates a new virtual machine with the specified configuration.
    ///
    /// Allocates a Pauli frame for the given number of qubits, initializes
    /// an empty decoder, and sets up syndrome and correction buffers with
    /// pre-allocated capacity. The graph reference is stored for use during
    /// decoding operations.
    ///
    /// # Arguments
    ///
    /// * `alloc` - Allocator for Pauli frame storage
    /// * `graph` - Decoding graph reference (must outlive the VM)
    /// * `num_qubits` - Number of logical qubits to track in the Pauli frame
    pub fn new(alloc: &BumpAllocator, graph: &'a DecodingGraph, num_qubits: usize) -> Self {
        Self {
            frame: PauliFrame::new(alloc, num_qubits),
            decoder: UnionFindDecoder::new(),
            graph,
            syndrome_buffer: Vec::with_capacity(256),
            correction_buffer: Vec::with_capacity(256),
        }
    }

    /// Executes a single quantum error correction instruction.
    ///
    /// Dispatches to the appropriate handler based on the instruction opcode:
    /// gate operations update the Pauli frame, measurements compare against
    /// the frame to generate syndromes, decode triggers the decoder, and
    /// reset clears all state. The hw_measure parameter provides the actual
    /// measurement result from hardware, which is compared against the
    /// Pauli frame prediction to detect errors.
    ///
    /// # Arguments
    ///
    /// * `instr` - Instruction to execute
    /// * `hw_measure` - Actual measurement result from hardware (for Measure
    ///   instructions)
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if decoding fails or buffers overflow.
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
