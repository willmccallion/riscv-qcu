//! Host-side tools for quantum error correction testing and benchmarking.
//!
//! Provides command-line utilities for generating test data, running
//! benchmarks, streaming syndrome data, and hardware-in-the-loop testing.
//! These tools are used during development and validation of the quantum
//! error correction system.

#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

/// Test data generation for quantum error correction benchmarks.
///
/// Generates phenomenological noise models and syndrome measurement data
/// for testing and benchmarking the decoder. Creates surface code error
/// models with configurable error rates and outputs .dem and .b8 files.
mod generator;

/// Hardware-in-the-loop interface for real-time quantum hardware simulation.
///
/// Provides TCP-based communication with Verilator simulations for demonstrating
/// closed-loop error correction. Enables real-time monitoring and control of
/// qubit states, error detection, and correction operations.
mod hil;

/// Statistics tracking and reporting for decoder performance metrics.
///
/// Collects and analyzes latency, throughput, and error rate statistics
/// from decoder operations. Provides formatted reporting for benchmark
/// results and performance analysis.
mod stats;

/// Streaming decoder simulation with real-time throughput monitoring.
///
/// Implements a producer-consumer architecture for continuous syndrome
/// processing. Monitors queue depth, latency, and throughput to evaluate
/// decoder performance under sustained load conditions.
mod stream;

/// Throughput benchmarking for decoder performance evaluation.
///
/// Measures decoding throughput by processing large batches of syndrome
/// data in parallel. Reports shots per second and latency statistics
/// for performance characterization.
mod throughput;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Command-line interface structure.
///
/// Parses command-line arguments and dispatches to the appropriate subcommand
/// handler. Uses clap for argument parsing and validation.
#[derive(Parser)]
struct Cli {
    /// Subcommand to execute (gen, run, stream, or hil).
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the host tools.
///
/// Each variant represents a different operation mode: data generation,
/// benchmark execution, streaming simulation, or hardware-in-the-loop testing.
#[derive(Subcommand)]
enum Commands {
    /// Generate phenomenological noise model test data.
    ///
    /// Creates a surface code error model and generates syndrome measurement
    /// data with configurable error rates. Outputs a .dem file (decoding graph)
    /// and a .b8 file (binary syndrome data) for use in benchmarks.
    Gen {
        /// Output path for the decoding graph (.dem file).
        #[arg(long, default_value = "bench.dem")]
        dem: String,

        /// Output path for the syndrome data (.b8 file).
        #[arg(long, default_value = "bench.b8")]
        b8: String,

        /// Surface code size (creates size x size code).
        #[arg(long, default_value_t = 21)]
        size: usize,

        /// Number of measurement shots to generate.
        #[arg(long, default_value_t = 100_000)]
        shots: usize,

        /// Physical error rate per edge (probability of error occurrence).
        #[arg(long, default_value_t = 0.005)]
        p: f64,

        /// Inject uncorrectable monopole errors in ~10% of shots.
        #[arg(long)]
        inject_failures: bool,
    },

    /// Run a throughput benchmark on decoding performance.
    ///
    /// Loads a decoding graph and syndrome data, then measures the time
    /// required to decode all shots using parallel processing. Reports
    /// throughput in shots per second.
    Run {
        /// Path to the decoding graph (.dem file).
        #[arg(short, long)]
        dem: String,

        /// Path to the syndrome data (.b8 file).
        #[arg(short, long)]
        b8: String,

        /// Override the number of detectors (defaults to graph node count).
        #[arg(short, long)]
        detectors: Option<usize>,
    },

    /// Run a streaming simulation with real-time throughput monitoring.
    ///
    /// Continuously generates or loads syndrome data and processes it through
    /// the decoder at a specified frequency. Monitors queue depth, latency,
    /// and throughput over the specified duration.
    Stream {
        /// Path to the decoding graph (.dem file).
        #[arg(short, long)]
        dem: String,

        /// Optional path to pre-generated syndrome data (.b8 file).
        #[arg(short, long)]
        b8: Option<String>,

        /// Target frequency for syndrome generation (Hz).
        #[arg(short, long, default_value_t = 10_000)]
        freq: u64,

        /// Duration of the streaming test in seconds.
        #[arg(short, long, default_value_t = 10)]
        duration: u64,

        /// Override the number of detectors (defaults to graph node count).
        #[arg(long)]
        detectors: Option<usize>,
    },

    /// Run hardware-in-the-loop demonstration.
    ///
    /// Connects to a Verilator simulation via TCP and demonstrates real-time
    /// error detection and correction on a simulated quantum hardware system.
    /// Displays a live dashboard of qubit states and correction operations.
    Hil,
}

/// Main entry point for host-side tools.
///
/// Parses command-line arguments and dispatches to the appropriate subcommand
/// handler. Each subcommand performs a different operation: data generation,
/// benchmarking, streaming simulation, or hardware-in-the-loop testing.
///
/// # Returns
///
/// Ok(()) on success, or an error if any operation fails.
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Gen {
            dem,
            b8,
            size,
            shots,
            p,
            inject_failures,
        } => {
            generator::generate_phenomenological_data(&dem, &b8, size, shots, p, inject_failures)?;
        }
        Commands::Run { dem, b8, detectors } => {
            throughput::run_benchmark(&dem, &b8, detectors)?;
        }
        Commands::Stream {
            dem,
            b8,
            freq,
            duration,
            detectors,
        } => {
            stream::run_stream(&dem, b8, freq, duration, detectors)?;
        }
        Commands::Hil => {
            hil::run_hil_demo()?;
        }
    }
    Ok(())
}
