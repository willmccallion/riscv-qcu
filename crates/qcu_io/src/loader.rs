use anyhow::{Context, Result};
use bitvec::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Loads a Stim .b8 file (binary measurement data).
pub fn load_b8_file<P: AsRef<Path>>(path: P) -> Result<BitVec<u8, Lsb0>> {
    let mut file = File::open(path).context("Failed to open .b8 file")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Convert bytes to BitVec (Little Endian, which Stim uses)
    let bits = BitVec::<u8, Lsb0>::from_vec(buffer);
    Ok(bits)
}

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
