/// Tracks latency statistics with minimal overhead.
pub struct LatencyStats {
    pub min: u64,
    pub max: u64,
    pub sum: u64,
    pub count: u64,
    pub buckets: [u64; 20],
}

impl LatencyStats {
    pub fn new() -> Self {
        Self {
            min: u64::MAX,
            max: 0,
            sum: 0,
            count: 0,
            buckets: [0; 20],
        }
    }

    /// Record a latency measurement in Nanoseconds.
    pub fn update(&mut self, nanos: u64) {
        if nanos < self.min {
            self.min = nanos;
        }
        if nanos > self.max {
            self.max = nanos;
        }
        self.sum += nanos;
        self.count += 1;

        // Bucketize (each bucket = 10us = 10,000ns)
        let idx = (nanos / 10_000).min(19) as usize;
        self.buckets[idx] += 1;
    }

    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum as f64 / self.count as f64
        }
    }

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
