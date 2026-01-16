//! Loader for binary syndrome measurement data files.
//!
//! Provides functions for reading Stim .b8 files, which contain packed binary
//! data representing syndrome measurements from multiple quantum shots. The
//! loader unpacks the binary data into per-shot boolean vectors for processing
//! by the decoder.

use anyhow::{Context, Result};
use bitvec::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Loads a Stim .b8 file containing binary measurement data.
///
/// Reads the entire file into memory and converts it to a BitVec for efficient
/// bit-level access. The .b8 format uses little-endian byte order, with bits
/// packed 8 per byte. Each bit represents one detector measurement result.
///
/// # Arguments
///
/// * `path` - Path to the .b8 file
///
/// # Returns
///
/// A BitVec containing all measurement bits, or an error if the file cannot
/// be read.
pub fn load_b8_file<P: AsRef<Path>>(path: P) -> Result<BitVec<u8, Lsb0>> {
    let mut file = File::open(path).context("Failed to open .b8 file")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let bits = BitVec::<u8, Lsb0>::from_vec(buffer);
    Ok(bits)
}

/// Splits raw bit data into per-shot boolean vectors.
///
/// Divides the packed bit vector into individual shots, where each shot
/// contains bits_per_shot detector measurements. Shots are stored as
/// separate boolean vectors for easy iteration and processing by the decoder.
///
/// # Arguments
///
/// * `raw_bits` - Packed bit vector from load_b8_file
/// * `bits_per_shot` - Number of detector bits per measurement shot
///
/// # Returns
///
/// A vector of boolean vectors, where each inner vector represents one shot's
/// detector measurements.
pub fn slice_shots(raw_bits: &BitVec<u8, Lsb0>, bits_per_shot: usize) -> Vec<Vec<bool>> {
    let bytes_per_shot = bits_per_shot.div_ceil(8);
    let stride_bits = bytes_per_shot * 8;

    let num_shots = raw_bits.len() / stride_bits;
    let mut shots = Vec::with_capacity(num_shots);

    for i in 0..num_shots {
        let start = i * stride_bits;
        let end = start + bits_per_shot;

        let slice = &raw_bits[start..end];
        let shot_bools: Vec<bool> = slice.iter().map(|b| *b).collect();
        shots.push(shot_bools);
    }

    shots
}
