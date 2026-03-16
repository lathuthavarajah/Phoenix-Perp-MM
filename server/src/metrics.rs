use std::time::Instant;

/// Runtime counters for observability.
pub struct Metrics {
    pub start_time: Instant,
    pub cycles: u64,
    pub oracle_fetches: u64,
    pub oracle_errors: u64,
    pub orders_placed: u64,
    pub orders_cancelled: u64,
    pub fills_processed: u64,
    pub risk_reduce_only: u64,
    pub risk_emergency: u64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            cycles: 0,
            oracle_fetches: 0,
            oracle_errors: 0,
            orders_placed: 0,
            orders_cancelled: 0,
            fills_processed: 0,
            risk_reduce_only: 0,
            risk_emergency: 0,
        }
    }

    /// Uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "uptime:          {}s", self.uptime_secs())?;
        writeln!(f, "cycles:          {}", self.cycles)?;
        writeln!(f, "oracle_fetches:  {}", self.oracle_fetches)?;
        writeln!(f, "oracle_errors:   {}", self.oracle_errors)?;
        writeln!(f, "orders_placed:   {}", self.orders_placed)?;
        writeln!(f, "orders_cancelled:{}", self.orders_cancelled)?;
        writeln!(f, "fills_processed: {}", self.fills_processed)?;
        writeln!(f, "risk_reduce_only:{}", self.risk_reduce_only)?;
        writeln!(f, "risk_emergency:  {}", self.risk_emergency)?;
        Ok(())
    }
}
