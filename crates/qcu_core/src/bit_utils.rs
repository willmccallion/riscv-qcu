//! Bit manipulation utilities for packed bit arrays.
//!
//! Provides efficient operations on bit vectors stored as arrays of u64 words.
//! Used throughout the decoder for managing parity bits, Pauli frame registers,
//! and syndrome tracking. All operations are inlined for maximum performance
//! in hot paths.

/// Static utility functions for bit-level operations on u64 word arrays.
///
/// Encapsulates bit indexing, setting, clearing, and toggling operations
/// that operate on bit vectors stored as arrays of 64-bit words. The word
/// and bit indices are computed from the linear bit index to enable efficient
/// random access to individual bits in large bit vectors.
pub struct BitPack;

impl BitPack {
    /// Reads a single bit from a packed bit array.
    ///
    /// Computes the word index and bit offset within that word, then extracts
    /// the bit value. Used for reading parity flags, Pauli frame error bits,
    /// and syndrome indicators without unpacking the entire bit vector.
    ///
    /// # Arguments
    ///
    /// * `storage` - Array of u64 words containing the bit vector
    /// * `index` - Linear bit index (0-based)
    ///
    /// # Returns
    ///
    /// True if the bit is set, false otherwise.
    #[inline(always)]
    pub fn get(storage: &[u64], index: usize) -> bool {
        let word = storage[index / 64];
        let bit = index % 64;
        (word >> bit) & 1 == 1
    }

    /// Toggles a single bit in a packed bit array.
    ///
    /// Flips the bit value at the specified index using XOR. This is the
    /// preferred operation for updating parity bits during union-find operations,
    /// as it avoids branching and handles both set and clear operations
    /// uniformly.
    ///
    /// # Arguments
    ///
    /// * `storage` - Mutable array of u64 words containing the bit vector
    /// * `index` - Linear bit index to toggle
    #[inline(always)]
    pub fn toggle(storage: &mut [u64], index: usize) {
        let word_idx = index / 64;
        let bit_idx = index % 64;
        storage[word_idx] ^= 1 << bit_idx;
    }

    /// Sets a bit to a specific value in a packed bit array.
    ///
    /// Conditionally sets or clears the bit based on the provided value.
    /// Uses bitwise OR for setting and AND with complement for clearing.
    /// This operation is used when the desired state is known in advance,
    /// such as initializing Pauli frame registers or resetting decoder state.
    ///
    /// # Arguments
    ///
    /// * `storage` - Mutable array of u64 words containing the bit vector
    /// * `index` - Linear bit index to modify
    /// * `val` - Desired bit value (true to set, false to clear)
    #[inline(always)]
    pub fn set(storage: &mut [u64], index: usize, val: bool) {
        if val {
            let word_idx = index / 64;
            let bit_idx = index % 64;
            storage[word_idx] |= 1 << bit_idx;
        } else {
            let word_idx = index / 64;
            let bit_idx = index % 64;
            storage[word_idx] &= !(1 << bit_idx);
        }
    }
}
