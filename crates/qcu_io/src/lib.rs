//! I/O utilities for loading quantum error correction data files.
//!
//! Provides functions for reading and parsing decoding graphs (.dem files)
//! and syndrome measurement data (.b8 files) used by the quantum error
//! correction system. These utilities handle file format parsing and
//! conversion to internal data structures.

/// File loading utilities for quantum error correction data formats.
///
/// Provides functions for reading binary syndrome data (.b8 files) and
/// converting them into internal data structures. Handles file I/O,
/// byte order conversion, and data validation.
pub mod loader;

/// Parser for decoding graph descriptions in DEM format.
///
/// Parses detector error model (.dem) files that describe the connectivity
/// structure of quantum error correction codes. Constructs DecodingGraph
/// instances from the parsed edge and node information.
pub mod parser;
