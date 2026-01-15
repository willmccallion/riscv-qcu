#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]

mod generator;
mod stats;
mod stream;
mod throughput;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Gen {
        #[arg(long, default_value = "bench.dem")]
        dem: String,
        #[arg(long, default_value = "bench.b8")]
        b8: String,
        #[arg(long, default_value_t = 21)]
        size: usize,
        #[arg(long, default_value_t = 100_000)]
        shots: usize,
        #[arg(long, default_value_t = 0.005)]
        p: f64,
        #[arg(long)]
        inject_failures: bool,
    },
    Run {
        #[arg(short, long)]
        dem: String,
        #[arg(short, long)]
        b8: String,
        #[arg(short, long)]
        detectors: Option<usize>,
    },
    Stream {
        #[arg(short, long)]
        dem: String,
        #[arg(short, long)]
        b8: Option<String>,
        #[arg(short, long, default_value_t = 10_000)]
        freq: u64,
        #[arg(short, long, default_value_t = 10)]
        duration: u64,
        #[arg(long)]
        detectors: Option<usize>,
    },
}

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
    }
    Ok(())
}
