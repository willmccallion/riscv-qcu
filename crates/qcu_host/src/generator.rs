//! Test data generator for quantum error correction benchmarks.
//!
//! Generates phenomenological noise model data by creating surface code
//! topologies and simulating error propagation. Outputs decoding graphs
//! (.dem files) and syndrome measurement data (.b8 files) for use in
//! performance benchmarks and correctness testing.

use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Generates phenomenological noise model test data.
///
/// Creates a surface code decoding graph and generates syndrome measurement
/// data with configurable error rates. The surface code is represented as
/// a grid of detector nodes connected by edges representing possible error
/// locations. Errors are applied probabilistically, and syndrome bits are
/// recorded when detectors fire. Optionally injects uncorrectable monopole
/// errors to test decoder robustness.
///
/// # Arguments
///
/// * `dem_path` - Output path for the decoding graph (.dem file)
/// * `b8_path` - Output path for the syndrome data (.b8 file)
/// * `size` - Surface code size (creates size x size grid)
/// * `num_shots` - Number of measurement shots to generate
/// * `p` - Physical error rate per edge (probability of error occurrence)
/// * `inject_failures` - If true, inject uncorrectable monopole errors in ~10% of shots
///
/// # Returns
///
/// Ok(()) on success, or an error if file I/O fails.
pub fn generate_phenomenological_data(
    dem_path: &str,
    b8_path: &str,
    size: usize,
    num_shots: usize,
    p: f64,
    inject_failures: bool,
) -> Result<()> {
    println!("Generating {}x{} Surface Code (p={})...", size, size, p);
    if inject_failures {
        println!("WARNING: Injecting ~10% invalid shots (Monopoles).");
    }

    let num_nodes = size * size;
    let mut dem_file = BufWriter::new(File::create(dem_path)?);
    let mut edges = Vec::new();

    for r in 0..size {
        for c in 0..size - 1 {
            let u = r * size + c;
            let v = u + 1;
            writeln!(dem_file, "error({}) D{} D{}", p, u, v)?;
            edges.push((u, v));
        }
    }
    for r in 0..size - 1 {
        for c in 0..size {
            let u = r * size + c;
            let v = u + size;
            writeln!(dem_file, "error({}) D{} D{}", p, u, v)?;
            edges.push((u, v));
        }
    }
    dem_file.flush()?;

    println!("Simulating {} shots...", num_shots);
    let mut b8_file = BufWriter::new(File::create(b8_path)?);

    let mut state: u64 = 12345;
    let mut rng_float = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let result = state.wrapping_mul(0x2545F4914F6CDD1D);
        (result as f64) / (u64::MAX as f64)
    };

    let bytes_per_shot = num_nodes.div_ceil(8);
    let mut buffer = Vec::with_capacity(bytes_per_shot * 1024);
    let mut detector_state = vec![false; num_nodes];

    for _ in 0..num_shots {
        detector_state.fill(false);

        for &(u, v) in &edges {
            if rng_float() < p {
                detector_state[u] = !detector_state[u];
                detector_state[v] = !detector_state[v];
            }
        }

        if inject_failures && rng_float() < 0.10 {
            detector_state[0] = !detector_state[0];
        }

        for chunk in detector_state.chunks(8) {
            let mut byte = 0u8;
            for (i, &triggered) in chunk.iter().enumerate() {
                if triggered {
                    byte |= 1 << i;
                }
            }
            buffer.push(byte);
        }

        if buffer.len() >= 1024 * 1024 {
            b8_file.write_all(&buffer)?;
            buffer.clear();
        }
    }
    b8_file.write_all(&buffer)?;

    println!("Done.");
    Ok(())
}
