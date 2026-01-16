//! Decoding graph representation for quantum error correction.
//!
//! Implements the graph structure that encodes the error model topology for
//! stabilizer codes. Nodes represent detectors (syndrome measurement points)
//! and edges represent possible error locations. The graph is used by the
//! union-find decoder to find minimum-weight correction paths.

use crate::QecError;
use alloc::alloc::Global;
use alloc::vec::Vec;
use core::alloc::Allocator;

/// Graph edge representation with target node and weight.
///
/// Stores a connection between two nodes in the decoding graph. The weight
/// field is currently unused but reserved for future weighted decoding
/// algorithms that consider error probabilities.
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    /// Target node index for this edge.
    ///
    /// In an undirected graph representation, this edge connects from an
    /// implicit source node to this target. The decoder processes edges
    /// bidirectionally, so the source/target distinction is arbitrary.
    pub target: usize,

    /// Edge weight for weighted decoding algorithms.
    ///
    /// Represents the negative log probability of an error occurring at this
    /// edge location. Currently unused by the union-find decoder but stored
    /// for compatibility with future minimum-weight matching decoders.
    pub weight: f64,
}

/// Decoding graph representing the error model topology.
///
/// Stores the connectivity structure of detector nodes and error locations
/// for a stabilizer code. The graph is built from error model descriptions
/// (e.g., .dem files) and used by decoders to find correction paths. Edges
/// are stored as (u, v) pairs for efficient iteration during decoding.
///
/// # Type Parameters
///
/// * `A` - Allocator type for edge storage. Defaults to Global for host-side
///   usage, but can be customized for firmware environments with custom allocators.
pub struct DecodingGraph<A: Allocator = Global> {
    /// Flat list of graph edges as (u, v) node pairs.
    ///
    /// Stored as u32 pairs to reduce memory footprint compared to usize pairs
    /// on 64-bit systems, while still supporting graphs with up to 4 billion
    /// nodes. The decoder iterates over this list to find edges connecting
    /// active syndrome clusters.
    pub fast_edges: Vec<(u32, u32), A>,

    /// Estimated capacity for node indices.
    ///
    /// Tracks the expected maximum node ID to guide memory pre-allocation.
    /// Updated dynamically as edges are added to accommodate graphs that
    /// grow beyond initial estimates.
    pub num_nodes_capacity: usize,

    /// Maximum node ID encountered in the graph.
    ///
    /// Tracks the highest node index referenced by any edge, used to determine
    /// the actual graph size for decoder initialization. This value determines
    /// how many nodes the decoder must allocate state for.
    pub max_node_id: usize,
}

impl DecodingGraph<Global> {
    /// Creates a new decoding graph with global allocator.
    ///
    /// Convenience constructor for host-side usage where the standard global
    /// allocator is available. Pre-allocates edge storage based on the capacity
    /// estimate to reduce reallocations during graph construction.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Estimated number of nodes (used for pre-allocation)
    pub fn new(capacity: usize) -> Self {
        Self::new_in(capacity, Global)
    }
}

impl<A: Allocator> DecodingGraph<A> {
    /// Creates a new decoding graph with a custom allocator.
    ///
    /// Allows graph construction in firmware environments where custom
    /// allocators (e.g., bump allocators) are required. Pre-allocates
    /// edge storage to reduce fragmentation and improve real-time performance.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Estimated number of nodes
    /// * `alloc` - Allocator instance for edge storage
    pub fn new_in(capacity: usize, alloc: A) -> Self {
        Self {
            fast_edges: Vec::with_capacity_in(capacity * 4, alloc),
            num_nodes_capacity: capacity,
            max_node_id: 0,
        }
    }

    /// Ensures the graph can accommodate nodes up to index n.
    ///
    /// Updates the capacity estimate if n exceeds the current value. This
    /// is a hint for memory management and does not allocate node storage
    /// (nodes are implicit in the edge list).
    ///
    /// # Arguments
    ///
    /// * `n` - Maximum node index that should be supported
    pub fn ensure_size(&mut self, n: usize) {
        if n > self.num_nodes_capacity {
            self.num_nodes_capacity = n;
        }
    }

    /// Adds an edge between nodes u and v to the graph.
    ///
    /// Records a connection in the error model topology. The weight parameter
    /// is currently ignored but stored for future use. Updates the maximum
    /// node ID to track the graph's actual size. Edges are stored as undirected,
    /// so (u, v) and (v, u) are equivalent.
    ///
    /// # Arguments
    ///
    /// * `u` - First node index
    /// * `v` - Second node index
    /// * `_weight` - Edge weight (currently unused)
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if memory allocation fails.
    pub fn add_edge(&mut self, u: usize, v: usize, _weight: f64) -> Result<(), QecError> {
        let max_idx = if u > v { u } else { v };
        self.ensure_size(max_idx + 1);

        if max_idx >= self.max_node_id {
            self.max_node_id = max_idx + 1;
        }

        self.fast_edges.push((u as u32, v as u32));

        Ok(())
    }

    /// Returns the number of nodes in the graph.
    ///
    /// Computed as the maximum node ID plus one, since node indices are
    /// zero-based. This value is used by decoders to allocate state vectors
    /// of the correct size.
    ///
    /// # Returns
    ///
    /// The number of nodes (max_node_id), representing the graph size.
    pub fn num_nodes(&self) -> usize {
        self.max_node_id
    }
}
