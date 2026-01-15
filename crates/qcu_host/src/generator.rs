use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write};

/// Generates a Phenomenological Noise Model.
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

    // Horizontal edges
    for r in 0..size {
        for c in 0..size - 1 {
            let u = r * size + c;
            let v = u + 1;
            writeln!(dem_file, "error({}) D{} D{}", p, u, v)?;
            edges.push((u, v));
        }
    }
    // Vertical edges
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

    // Simple Xorshift RNG
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

        // Apply valid physical errors (Pairs)
        for &(u, v) in &edges {
            if rng_float() < p {
                detector_state[u] = !detector_state[u];
                detector_state[v] = !detector_state[v];
            }
        }

        // Inject FATAL error (Monopole) if requested
        if inject_failures && rng_float() < 0.10 {
            detector_state[0] = !detector_state[0];
        }

        // Pack bits
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
