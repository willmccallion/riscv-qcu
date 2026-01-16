//! Throughput benchmark for decoder performance evaluation.
//!
//! Measures the time required to decode a batch of syndrome measurements
//! using parallel processing. Reports throughput in shots per second and
//! validates that all shots are successfully decoded. Used for performance
//! regression testing and optimization validation.

use anyhow::Result;
use qcu_core::decoder::UnionFindDecoder;
use qcu_io::{loader, parser};
use rayon::prelude::*;
use std::time::Instant;

/// Maximum number of nodes supported by the benchmark decoder.
///
/// Large enough to accommodate typical surface code sizes used in benchmarks.
/// The decoder is instantiated with this capacity to handle graphs up to
/// this size.
const MAX_NODES: usize = 4096;

/// Runs a throughput benchmark on decoding performance.
///
/// Loads a decoding graph and syndrome data, then processes all shots in
/// parallel using Rayon. Measures the total time and computes throughput.
/// Reports results including total time, shots per second, and success rate.
///
/// # Arguments
///
/// * `dem_path` - Path to the decoding graph (.dem file)
/// * `b8_path` - Path to the syndrome data (.b8 file)
/// * `user_detectors` - Optional override for detector count (defaults to graph size)
///
/// # Returns
///
/// Ok(()) on success, or an error if file loading or decoding fails.
pub fn run_benchmark(dem_path: &str, b8_path: &str, user_detectors: Option<usize>) -> Result<()> {
    println!("Loading Graph from {}...", dem_path);
    let start_load = Instant::now();
    let graph = parser::load_dem_file(dem_path)?;
    println!(
        "Graph loaded in {:?}. Nodes: {}, Edges: {}",
        start_load.elapsed(),
        graph.num_nodes(),
        graph.fast_edges.len()
    );

    let num_detectors = user_detectors.unwrap_or(graph.num_nodes());

    println!("Loading Shots from {}...", b8_path);
    let raw_bits = loader::load_b8_file(b8_path)?;
    let shots = loader::slice_shots(&raw_bits, num_detectors);
    println!("Loaded {} shots.", shots.len());

    println!("Starting Benchmark (Parallel - Rayon)...");
    let start_bench = Instant::now();

    let solved_count: usize = shots
        .par_iter()
        .map(|shot| {
            let mut local_decoder = UnionFindDecoder::<MAX_NODES>::new();
            let mut local_results = Vec::with_capacity(128);

            let syndrome: Vec<usize> = shot
                .iter()
                .enumerate()
                .filter_map(|(i, &triggered)| if triggered { Some(i) } else { None })
                .collect();

            if local_decoder
                .solve_into(&graph, &syndrome, &mut local_results)
                .is_ok()
            {
                1
            } else {
                0
            }
        })
        .sum();

    let duration = start_bench.elapsed();
    let seconds = duration.as_secs_f64();
    let throughput = shots.len() as f64 / seconds;

    println!("Results");
    println!("Time: {:.4} s", seconds);
    println!("Throughput: {:.2} shots/s", throughput);
    println!("Solved: {}/{}", solved_count, shots.len());

    Ok(())
}
