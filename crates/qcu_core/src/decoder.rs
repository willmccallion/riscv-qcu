//! Union-Find decoder implementation for quantum error correction.
//!
//! Implements the union-find algorithm for decoding stabilizer codes by
//! finding minimum-weight correction paths through the decoding graph.
//! The decoder processes syndrome bits, groups them into clusters via
//! union-find operations, and outputs correction operations that restore
//! the logical state.

use crate::QecError;
use crate::bit_utils::BitPack;
use crate::dsu::UnionFind;
use crate::graph::DecodingGraph;
use crate::static_vec::StaticVec;
use core::alloc::Allocator;

/// Trait for buffers that accumulate correction operations.
///
/// Abstracts over different buffer types (heap-allocated vectors and
/// stack-allocated static vectors) to allow the decoder to work in both
/// firmware and host environments. Corrections are represented as edge
/// pairs (u, v) indicating which graph edges should be flipped.
pub trait CorrectionBuffer {
    /// Appends a correction operation to the buffer.
    ///
    /// Records that the edge between nodes u and v should be flipped to
    /// correct the detected error. Returns an error if the buffer cannot
    /// accommodate additional corrections.
    ///
    /// # Arguments
    ///
    /// * `u` - First node of the correction edge
    /// * `v` - Second node of the correction edge
    fn push_correction(&mut self, u: usize, v: usize) -> Result<(), QecError>;

    /// Clears all accumulated corrections from the buffer.
    ///
    /// Resets the buffer to empty state, typically called at the start
    /// of a new decoding cycle to prepare for fresh correction output.
    fn clear_buffer(&mut self);
}

impl<A: Allocator> CorrectionBuffer for alloc::vec::Vec<(usize, usize), A> {
    /// Pushes a correction to a heap-allocated vector buffer.
    ///
    /// Attempts to reserve additional capacity if needed, returning an
    /// error if memory allocation fails. This implementation is used in
    /// host-side tools where heap allocation is available.
    fn push_correction(&mut self, u: usize, v: usize) -> Result<(), QecError> {
        self.try_reserve(1).map_err(|_| QecError::OutOfMemory)?;
        self.push((u, v));
        Ok(())
    }

    /// Clears the vector buffer by removing all elements.
    ///
    /// Maintains the vector's capacity to avoid reallocation in subsequent
    /// decoding cycles.
    fn clear_buffer(&mut self) {
        self.clear();
    }
}

impl<const N: usize> CorrectionBuffer for StaticVec<(usize, usize), N> {
    /// Pushes a correction to a stack-allocated static vector buffer.
    ///
    /// Returns an error if the buffer has reached its fixed capacity.
    /// This implementation is used in firmware where heap allocation is
    /// not available or undesirable for real-time constraints.
    fn push_correction(&mut self, u: usize, v: usize) -> Result<(), QecError> {
        self.push((u, v)).map_err(|_| QecError::BufferOverflow)
    }

    /// Clears the static vector buffer by resetting its length.
    ///
    /// Does not deallocate memory, as static vectors have fixed storage.
    fn clear_buffer(&mut self) {
        self.clear();
    }
}

/// Union-Find decoder with compile-time node capacity limit.
///
/// Implements the union-find decoding algorithm using stack-allocated buffers
/// to avoid heap allocation in firmware. The decoder maintains disjoint sets
/// of graph nodes, tracks parity (odd/even syndrome count) for each set, and
/// outputs correction edges when sets with odd parity are merged. The capacity
/// N must be large enough to accommodate all nodes in the decoding graph.
///
/// # Type Parameters
///
/// * `N` - Maximum number of nodes the decoder can handle. Must satisfy
///   the constraint that `N.div_ceil(64)` is a valid array size for the
///   parity bit vector.
pub struct UnionFindDecoder<const N: usize>
where
    [(); N.div_ceil(64)]:,
{
    /// Parent pointers for the union-find forest.
    ///
    /// Each element points to its parent in the tree, with root nodes
    /// pointing to themselves. Used for path compression during find
    /// operations to maintain near-constant-time lookups.
    parent: StaticVec<usize, N>,

    /// Rank values for union-by-rank optimization.
    ///
    /// Tracks the approximate depth of each tree to ensure balanced unions.
    /// Prevents degenerate linear trees that would degrade find performance
    /// to O(n) in the worst case.
    rank: StaticVec<u8, N>,

    /// Parity bits for each disjoint set root.
    ///
    /// Packed as u64 words, with each bit indicating whether the corresponding
    /// set has odd parity (active syndrome). Used to determine which sets
    /// should be merged to form valid correction paths.
    parity: StaticVec<u64, { N.div_ceil(64) }>,

    /// Tracking array for nodes involved in the current decoding cycle.
    ///
    /// Marks which nodes have been touched by syndrome bits or correction
    /// operations, allowing early termination when processing graph edges
    /// that cannot affect the result.
    touched: StaticVec<usize, N>,
}

impl<const N: usize> Default for UnionFindDecoder<N>
where
    [(); N.div_ceil(64)]:,
{
    /// Creates a decoder with default (empty) state.
    ///
    /// Equivalent to calling `new()`, provided for trait compatibility.
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> UnionFindDecoder<N>
where
    [(); N.div_ceil(64)]:,
{
    /// Creates a new decoder with empty internal state.
    ///
    /// All internal buffers are initialized to empty. The decoder is ready
    /// to process syndrome data after this call, but no memory is allocated
    /// until `solve_into` is called with a graph.
    pub fn new() -> Self {
        Self {
            parent: StaticVec::new(),
            rank: StaticVec::new(),
            parity: StaticVec::new(),
            touched: StaticVec::new(),
        }
    }

    /// Solves the decoding problem and outputs corrections to the buffer.
    ///
    /// Processes the provided syndrome bits through the union-find algorithm,
    /// grouping nodes into clusters and generating correction operations for
    /// clusters with odd parity. The algorithm iterates until no further
    /// corrections can be found, ensuring all syndrome bits are matched.
    ///
    /// # Type Parameters
    ///
    /// * `GA` - Allocator type for the decoding graph's edge storage
    /// * `CB` - Correction buffer type for output
    ///
    /// # Arguments
    ///
    /// * `graph` - Decoding graph defining the error model topology
    /// * `syndrome_indices` - List of detector node indices that fired
    /// * `out_buffer` - Buffer to receive correction edge pairs
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if buffer overflow or invalid indices
    /// are encountered.
    pub fn solve_into<GA: Allocator, CB: CorrectionBuffer>(
        &mut self,
        graph: &DecodingGraph<GA>,
        syndrome_indices: &[usize],
        out_buffer: &mut CB,
    ) -> Result<(), QecError> {
        out_buffer.clear_buffer();

        let num_nodes = graph.num_nodes().min(N);

        self.parent.clear();
        self.rank.clear();
        self.touched.clear();
        self.parity.clear();

        for i in 0..num_nodes {
            let _ = self.parent.push(i);
            let _ = self.rank.push(0);
            let _ = self.touched.push(0);
        }

        let num_u64 = num_nodes.div_ceil(64);
        for _ in 0..num_u64 {
            let _ = self.parity.push(0);
        }

        let mut dsu = UnionFind::new(
            self.parent.as_mut_slice(),
            self.rank.as_mut_slice(),
            self.parity.as_mut_slice(),
        );

        for &idx in syndrome_indices {
            if idx < num_nodes {
                dsu.toggle_parity(idx);
                unsafe {
                    *self.touched.get_unchecked_mut(idx) = 1;
                }
            }
        }

        loop {
            let mut changed = false;
            for &(u32_u, u32_v) in &graph.fast_edges {
                let u = u32_u as usize;
                let v = u32_v as usize;

                if unsafe {
                    *self.touched.get_unchecked(u) == 0 && *self.touched.get_unchecked(v) == 0
                } {
                    continue;
                }

                let root_u = dsu.find(u);
                let root_v = dsu.find(v);

                if root_u != root_v {
                    let u_active = BitPack::get(dsu.parity, root_u);
                    let v_active = BitPack::get(dsu.parity, root_v);

                    if (u_active || v_active) && dsu.union(u, v) {
                        out_buffer.push_correction(u, v)?;
                        changed = true;
                        unsafe {
                            *self.touched.get_unchecked_mut(u) = 1;
                            *self.touched.get_unchecked_mut(v) = 1;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        Ok(())
    }
}
