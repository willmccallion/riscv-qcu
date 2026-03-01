//! Parser for decoding graph description files.
//!
//! Provides functions for parsing Stim .dem (Detector Error Model) files,
//! which describe the error model topology for stabilizer codes. The parser
//! extracts edges between detector nodes and constructs a DecodingGraph structure
//! for use by the decoder.

use anyhow::{Context, Result};
use qcu_core::graph::DecodingGraph;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Loads a Stim .dem file and constructs a DecodingGraph.
///
/// Parses the DEM file format, which specifies error probabilities and detector
/// node connections. Each "error" line defines an edge in the decoding graph
/// with an associated error probability. The probability is converted to a
/// weight using negative log probability for use in weighted decoding algorithms.
///
/// # Arguments
///
/// * `path` - Path to the .dem file
///
/// # Returns
///
/// A DecodingGraph containing all edges from the file, or an error if parsing fails.
#[allow(clippy::collapsible_if)]
pub fn load_dem_file<P: AsRef<Path>>(path: P) -> Result<DecodingGraph> {
    let file = File::open(path).context("Failed to open .dem file")?;
    let reader = BufReader::new(file);

    let mut graph = DecodingGraph::new(1024);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("error") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let p_part = parts[0];
            let start = p_part.find('(');
            let end = p_part.find(')');

            if let (Some(s), Some(e)) = (start, end) {
                if let Ok(p) = p_part[s + 1..e].parse::<f64>() {
                    let weight = -p.ln();

                    let mut detectors = Vec::new();
                    for part in &parts[1..] {
                        if let Some(Ok(idx)) = part.strip_prefix('D').map(|s| s.parse::<usize>()) {
                            detectors.push(idx);
                        }
                    }

                    if detectors.len() >= 2 {
                        for i in 0..detectors.len() - 1 {
                            let _ = graph.add_edge(detectors[i], detectors[i + 1], weight);
                        }
                    }
                }
            }
        }
    }

    graph.build_adjacency();

    Ok(graph)
}
