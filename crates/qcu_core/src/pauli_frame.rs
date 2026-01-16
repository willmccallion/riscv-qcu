//! Pauli frame tracking for stabilizer state representation.
//!
//! Implements the Pauli frame abstraction that tracks X and Z errors on
//! logical qubits without explicitly storing the full quantum state. This
//! enables efficient simulation of stabilizer circuits by tracking only
//! the error operators rather than exponentially large state vectors.

use crate::allocator::BumpAllocator;
use crate::bit_utils::BitPack;
use core::slice;

/// Pauli frame tracking X and Z errors on logical qubits.
///
/// Maintains two bit vectors (X and Z registers) where each bit indicates
/// whether the corresponding qubit has accumulated an X or Z error. Gates
/// are applied by updating these registers according to their conjugation
/// rules on Pauli operators. This representation is exact for stabilizer
/// circuits and avoids the exponential memory cost of full state simulation.
pub struct PauliFrame {
    /// Bit vector tracking X errors on each qubit.
    ///
    /// Packed as u64 words, with each bit indicating whether the corresponding
    /// qubit has an X error. Updated when X-type gates (e.g., CNOT) are applied
    /// or when X errors are introduced by noise or corrections.
    pub x_register: *mut u64,

    /// Bit vector tracking Z errors on each qubit.
    ///
    /// Packed as u64 words, with each bit indicating whether the corresponding
    /// qubit has a Z error. Updated when Z-type gates (e.g., phase gates) are
    /// applied or when Z errors are introduced by noise or corrections.
    pub z_register: *mut u64,

    /// Number of u64 words required to store the bit vectors.
    ///
    /// Computed as num_qubits.div_ceil(64) to accommodate all qubits with
    /// proper alignment. Used for bounds checking and slice construction.
    num_u64: usize,
}

impl PauliFrame {
    /// Allocates and initializes a new Pauli frame for the specified number of qubits.
    ///
    /// Allocates two bit vectors (X and Z registers) from the provided allocator,
    /// both initialized to zero (no errors). The frame is ready to track errors
    /// after construction. The allocator must provide sufficient memory for
    /// 2 * num_u64 words.
    ///
    /// # Arguments
    ///
    /// * `alloc` - Allocator for frame storage
    /// * `num_qubits` - Number of logical qubits to track
    pub fn new(alloc: &BumpAllocator, num_qubits: usize) -> Self {
        let num_u64 = num_qubits.div_ceil(64);
        let x_reg = alloc.alloc_slice::<u64>(num_u64).unwrap().as_mut_ptr();
        let z_reg = alloc.alloc_slice::<u64>(num_u64).unwrap().as_mut_ptr();

        Self {
            x_register: x_reg,
            z_register: z_reg,
            num_u64,
        }
    }

    /// Resets the Pauli frame to the all-zero state (no errors).
    ///
    /// Clears both X and Z registers, effectively resetting the tracked
    /// quantum state to the initial |0...0⟩ state. Used for initialization
    /// and recovery from uncorrectable errors. Does not deallocate memory.
    pub fn reset(&mut self) {
        unsafe {
            let x_slice = slice::from_raw_parts_mut(self.x_register, self.num_u64);
            let z_slice = slice::from_raw_parts_mut(self.z_register, self.num_u64);
            x_slice.fill(0);
            z_slice.fill(0);
        }
    }

    /// Applies a Hadamard gate to the specified qubit, updating the Pauli frame.
    ///
    /// The Hadamard gate conjugates X to Z and Z to X, so the frame update
    /// swaps the X and Z error bits for this qubit. This transformation
    /// maintains the stabilizer representation under the gate operation.
    ///
    /// # Arguments
    ///
    /// * `q` - Qubit index to apply the Hadamard gate to
    pub fn apply_hadamard(&mut self, q: usize) {
        unsafe {
            let x_slice = slice::from_raw_parts_mut(self.x_register, self.num_u64);
            let z_slice = slice::from_raw_parts_mut(self.z_register, self.num_u64);

            let has_x = BitPack::get(x_slice, q);
            let has_z = BitPack::get(z_slice, q);

            BitPack::set(x_slice, q, has_z);
            BitPack::set(z_slice, q, has_x);
        }
    }

    /// Applies a CNOT gate with control and target qubits, updating the Pauli frame.
    ///
    /// CNOT conjugates X_c to X_c X_t and Z_t to Z_c Z_t, where c is the control
    /// and t is the target. The frame update implements these conjugation rules:
    /// if the control has an X error, toggle the target's X error; if the target
    /// has a Z error, toggle the control's Z error.
    ///
    /// # Arguments
    ///
    /// * `c` - Control qubit index
    /// * `t` - Target qubit index
    pub fn apply_cnot(&mut self, c: usize, t: usize) {
        unsafe {
            let x_slice = slice::from_raw_parts_mut(self.x_register, self.num_u64);
            let z_slice = slice::from_raw_parts_mut(self.z_register, self.num_u64);

            if BitPack::get(x_slice, c) {
                BitPack::toggle(x_slice, t);
            }
            if BitPack::get(z_slice, t) {
                BitPack::toggle(z_slice, c);
            }
        }
    }

    /// Checks whether the specified qubit has an X error in the frame.
    ///
    /// Used to predict measurement outcomes: a qubit with an X error will
    /// produce a flipped measurement result compared to the expected value.
    /// This prediction is compared against actual measurements to generate
    /// syndrome bits.
    ///
    /// # Arguments
    ///
    /// * `q` - Qubit index to check
    ///
    /// # Returns
    ///
    /// True if the qubit has an X error, false otherwise.
    pub fn has_x_error(&self, q: usize) -> bool {
        unsafe {
            let x_slice = slice::from_raw_parts(self.x_register, self.num_u64);
            BitPack::get(x_slice, q)
        }
    }
}
