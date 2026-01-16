//! Core quantum error correction algorithms and data structures.
//!
//! This crate provides the fundamental components for quantum error correction
//! including the union-find decoder, Pauli frame tracking, decoding graph
//! representation, and supporting data structures. All modules are designed
//! for use in both firmware (no_std) and host-side simulation environments.

#![no_std]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

extern crate alloc;

/// Bump allocator for fixed-size memory regions in no_std environments.
///
/// Provides a simple linear allocator that allocates from a contiguous memory
/// region without support for deallocation. Used for allocating decoding graphs
/// and other long-lived data structures in firmware where heap allocation is
/// unavailable or undesirable.
pub mod allocator;

/// Bit manipulation utilities for syndrome packing and unpacking.
///
/// Provides efficient operations for converting between packed bit arrays
/// (u64 words) and sparse detector indices. Used throughout the decoder
/// pipeline to minimize memory usage and improve cache locality.
pub mod bit_utils;

/// Union-find decoder implementation for quantum error correction.
///
/// Implements the union-find algorithm for finding minimum-weight corrections
/// in decoding graphs. The decoder processes syndrome measurements and produces
/// correction operations that restore the quantum state to a valid code space.
pub mod decoder;

/// Disjoint set union (DSU) data structure for union-find operations.
///
/// Provides the underlying data structure for the union-find decoder, enabling
/// efficient set merging and root finding with path compression. Used internally
/// by the decoder to track connected components in the decoding graph.
pub mod dsu;

/// Decoding graph representation for quantum error correction.
///
/// Represents the connectivity structure of a quantum error correction code,
/// where nodes correspond to stabilizer measurements (detectors) and edges
/// represent error propagation paths. The graph is used by the decoder to
/// determine correction operations from syndrome measurements.
pub mod graph;

/// Pauli frame tracking for quantum state updates.
///
/// Maintains a representation of accumulated Pauli corrections applied to
/// the quantum state. Tracks X, Y, and Z Pauli operators without explicitly
/// storing the full quantum state, enabling efficient error correction
/// tracking in classical simulation.
pub mod pauli_frame;

/// Lock-free ring buffer for single-producer single-consumer communication.
///
/// Provides a fixed-capacity circular buffer for efficient data transfer
/// between threads or cores without requiring mutexes. Used for streaming
/// syndrome packets from producer to consumer threads in host-side simulations.
pub mod ring_buffer;

/// Single-producer multi-consumer queue for work distribution.
///
/// Enables one producer thread to push work items that are consumed by
/// multiple worker threads. Used in firmware to distribute decoding jobs
/// from the primary core to worker cores via a shared job queue.
pub mod spmc;

/// Stack-allocated vector with compile-time fixed capacity.
///
/// Provides a vector-like interface without heap allocation, suitable for
/// no_std environments. All storage is allocated on the stack or in static
/// memory, making it safe for real-time firmware where heap allocation is
/// unavailable or undesirable.
pub mod static_vec;

/// Virtual machine for executing quantum error correction operations.
///
/// Provides a high-level interface for running decoding algorithms on quantum
/// circuits. Manages state, coordinates decoder execution, and tracks correction
/// operations applied to the quantum state.
pub mod vm;

/// Error types returned by quantum error correction operations.
///
/// Encapsulates failure modes encountered during decoding, memory allocation,
/// and buffer management. These errors propagate through the decoding pipeline
/// and are handled by the virtual machine or firmware error recovery logic.
#[derive(Debug)]
pub enum QecError {
    /// A node index exceeds the bounds of the decoding graph.
    ///
    /// Indicates that a syndrome bit or detector ID references a node that
    /// does not exist in the current graph topology. This typically results
    /// from corrupted input data or a mismatch between the graph and syndrome
    /// data structures.
    NodeOutOfBounds,

    /// The decoder failed to produce a valid correction.
    ///
    /// Occurs when the union-find algorithm cannot find a valid correction
    /// path, often due to uncorrectable error patterns (e.g., monopole errors
    /// that violate the error model assumptions) or graph connectivity issues.
    DecodingFailed,

    /// Memory allocation request could not be satisfied.
    ///
    /// The allocator has exhausted available memory or the requested allocation
    /// size exceeds the allocator's capacity. This triggers error recovery
    /// or graceful degradation in the firmware.
    OutOfMemory,

    /// A fixed-size buffer has reached its capacity limit.
    ///
    /// A static buffer (e.g., StaticVec or ring buffer) cannot accommodate
    /// additional elements. The caller must either use a larger buffer or
    /// implement overflow handling logic.
    BufferOverflow,
}
