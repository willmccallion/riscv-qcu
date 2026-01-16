//! Latency statistics tracking for performance analysis.
//!
//! Provides data structures and functions for collecting and reporting
//! decoding latency measurements. Tracks minimum, maximum, average, and
//! distribution statistics to enable performance profiling and bottleneck
//! identification.

/// Tracks latency statistics with minimal overhead.
///
/// Accumulates latency measurements and computes summary statistics including
/// min, max, average, and histogram distribution. Designed for high-frequency
/// updates in performance-critical paths, using simple arithmetic operations
/// to minimize measurement overhead.
pub struct LatencyStats {
    pub min: u64,
    pub max: u64,
    pub sum: u64,
    pub count: u64,
    pub buckets: [u64; 20],
}

impl LatencyStats {
    /// Creates a new latency statistics tracker with empty state.
    ///
    /// Initializes all counters to zero and min to u64::MAX so the first
    /// measurement becomes the minimum. Ready to start collecting measurements
    /// after construction.
    pub fn new() -> Self {
        Self {
            min: u64::MAX,
            max: 0,
            sum: 0,
            count: 0,
            buckets: [0; 20],
        }
    }

    /// Records a latency measurement in nanoseconds.
    ///
    /// Updates min, max, sum, and count statistics, and increments the
    /// appropriate histogram bucket. The measurement is assumed to be in
    /// nanoseconds, and buckets are sized at 10 microsecond intervals.
    ///
    /// # Arguments
    ///
    /// * `nanos` - Latency measurement in nanoseconds
    pub fn update(&mut self, nanos: u64) {
        if nanos < self.min {
            self.min = nanos;
        }
        if nanos > self.max {
            self.max = nanos;
        }
        self.sum += nanos;
        self.count += 1;

        let idx = (nanos / 10_000).min(19) as usize;
        self.buckets[idx] += 1;
    }

    /// Computes the average latency from accumulated statistics.
    ///
    /// Divides the sum of all latencies by the count of measurements.
    /// Returns 0.0 if no measurements have been recorded to avoid division
    /// by zero.
    ///
    /// # Returns
    ///
    /// The average latency in nanoseconds, or 0.0 if no measurements recorded.
    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum as f64 / self.count as f64
        }
    }

    /// Prints a formatted report of latency statistics.
    ///
    /// Displays count, min, average, and max latencies, with automatic unit
    /// conversion (nanoseconds to microseconds) for readability. Also
    /// prints a histogram showing the distribution of latencies across
    /// 10-microsecond buckets.
    pub fn print_report(&self) {
        println!("\nLatency Metrics (Service Time)");
        println!("Count: {}", self.count);

        let avg_ns = self.avg();
        if avg_ns < 1000.0 {
            println!("Min:   {:.2} ns", self.min as f64);
            println!("Avg:   {:.2} ns", avg_ns);
            println!("Max:   {:.2} ns", self.max as f64);
        } else {
            println!("Min:   {:.2} us", self.min as f64 / 1000.0);
            println!("Avg:   {:.2} us", avg_ns / 1000.0);
            println!("Max:   {:.2} us", self.max as f64 / 1000.0);
        }

        println!("Distribution (10us buckets):");
        for i in 0..20 {
            let count = self.buckets[i];
            if count > 0 {
                let range_end = if i == 19 { ">" } else { "" };
                let lower = i * 10;
                let upper = (i + 1) * 10;
                println!("[{:3}-{:3}{} us]: {}", lower, upper, range_end, count);
            }
        }
    }
}
