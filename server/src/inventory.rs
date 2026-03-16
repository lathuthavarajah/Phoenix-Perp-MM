use crate::types::{Fill, Side};

/// Tracks position, average entry price, and PnL.
#[derive(Debug)]
#[allow(dead_code)]
pub struct InventoryManager {
    /// Current position in base units (positive = long).
    pub position: f64,
    /// Volume-weighted average entry price.
    pub avg_entry_price: f64,
    /// Cumulative realized PnL in quote currency.
    pub realized_pnl: f64,
    /// Total base volume filled.
    pub total_volume: f64,
    /// Number of fills processed.
    pub fill_count: u64,
    /// Peak PnL for drawdown tracking.
    pub peak_pnl: f64,
}

impl InventoryManager {
    pub fn new() -> Self {
        Self {
            position: 0.0,
            avg_entry_price: 0.0,
            realized_pnl: 0.0,
            total_volume: 0.0,
            fill_count: 0,
            peak_pnl: 0.0,
        }
    }

    /// Process a fill and update position/PnL.
    #[allow(dead_code)]
    pub fn process_fill(&mut self, fill: &Fill) {
        let signed_size = match fill.side {
            Side::Bid => fill.size,  // Bought → position increases
            Side::Ask => -fill.size, // Sold → position decreases
        };

        let new_position = self.position + signed_size;

        // Check if this fill reduces or flips position
        if self.position.abs() > 1e-12 && self.position.signum() != signed_size.signum() {
            // Closing (or partially closing) position
            let close_size = fill.size.min(self.position.abs());
            let pnl = match fill.side {
                // Selling to close a long: profit = (sell_price - avg_entry) * size
                Side::Ask => (fill.price - self.avg_entry_price) * close_size,
                // Buying to close a short: profit = (avg_entry - buy_price) * size
                Side::Bid => (self.avg_entry_price - fill.price) * close_size,
            };
            self.realized_pnl += pnl;
        }

        // Update average entry price
        if new_position.abs() < 1e-12 {
            // Position fully closed
            self.avg_entry_price = 0.0;
        } else if new_position.signum() == signed_size.signum() && self.position.signum() == signed_size.signum() {
            // Adding to same-direction position: weighted average
            let old_cost = self.avg_entry_price * self.position.abs();
            let new_cost = fill.price * fill.size;
            self.avg_entry_price = (old_cost + new_cost) / new_position.abs();
        } else if new_position.signum() != self.position.signum() {
            // Position flipped: new avg entry is the fill price
            self.avg_entry_price = fill.price;
        }

        self.position = new_position;
        self.total_volume += fill.size;
        self.fill_count += 1;

        // Track peak PnL
        if self.realized_pnl > self.peak_pnl {
            self.peak_pnl = self.realized_pnl;
        }
    }

    /// Unrealized PnL at a given mark price.
    #[allow(dead_code)]
    pub fn unrealized_pnl(&self, mark_price: f64) -> f64 {
        if self.position.abs() < 1e-12 {
            return 0.0;
        }
        (mark_price - self.avg_entry_price) * self.position
    }

    /// Total PnL (realized + unrealized) at a given mark price.
    #[allow(dead_code)]
    pub fn total_pnl(&self, mark_price: f64) -> f64 {
        self.realized_pnl + self.unrealized_pnl(mark_price)
    }

    /// Current drawdown from peak realized PnL.
    pub fn drawdown(&self) -> f64 {
        self.peak_pnl - self.realized_pnl
    }
}

impl Default for InventoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_fill(side: Side, price: f64, size: f64) -> Fill {
        Fill {
            side,
            price,
            size,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn buy_increases_position() {
        let mut inv = InventoryManager::new();
        inv.process_fill(&make_fill(Side::Bid, 100.0, 1.0));
        assert!((inv.position - 1.0).abs() < 1e-10);
        assert!((inv.avg_entry_price - 100.0).abs() < 1e-10);
    }

    #[test]
    fn sell_decreases_position() {
        let mut inv = InventoryManager::new();
        inv.process_fill(&make_fill(Side::Ask, 100.0, 1.0));
        assert!((inv.position - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn round_trip_pnl() {
        let mut inv = InventoryManager::new();
        // Buy at 100, sell at 105 → profit = 5
        inv.process_fill(&make_fill(Side::Bid, 100.0, 1.0));
        inv.process_fill(&make_fill(Side::Ask, 105.0, 1.0));

        assert!((inv.realized_pnl - 5.0).abs() < 1e-10, "realized PnL should be 5.0");
        assert!(inv.position.abs() < 1e-10, "position should be flat");
    }

    #[test]
    fn short_round_trip_pnl() {
        let mut inv = InventoryManager::new();
        // Sell at 105, buy at 100 → profit = 5
        inv.process_fill(&make_fill(Side::Ask, 105.0, 1.0));
        inv.process_fill(&make_fill(Side::Bid, 100.0, 1.0));

        assert!((inv.realized_pnl - 5.0).abs() < 1e-10, "short round trip PnL should be 5.0");
    }

    #[test]
    fn unrealized_pnl() {
        let mut inv = InventoryManager::new();
        inv.process_fill(&make_fill(Side::Bid, 100.0, 2.0));

        let upnl = inv.unrealized_pnl(110.0);
        assert!((upnl - 20.0).abs() < 1e-10, "unrealized PnL should be 20.0");
    }

    #[test]
    fn weighted_avg_entry() {
        let mut inv = InventoryManager::new();
        inv.process_fill(&make_fill(Side::Bid, 100.0, 1.0));
        inv.process_fill(&make_fill(Side::Bid, 110.0, 1.0));

        // Avg entry = (100*1 + 110*1) / 2 = 105
        assert!((inv.avg_entry_price - 105.0).abs() < 1e-10);
    }

    #[test]
    fn drawdown_tracking() {
        let mut inv = InventoryManager::new();
        // Win some
        inv.process_fill(&make_fill(Side::Bid, 100.0, 1.0));
        inv.process_fill(&make_fill(Side::Ask, 110.0, 1.0)); // +10
        assert!((inv.peak_pnl - 10.0).abs() < 1e-10);

        // Lose some
        inv.process_fill(&make_fill(Side::Bid, 110.0, 1.0));
        inv.process_fill(&make_fill(Side::Ask, 105.0, 1.0)); // -5, total = 5
        assert!((inv.drawdown() - 5.0).abs() < 1e-10, "drawdown should be 5.0");
    }

    #[test]
    fn volume_and_count() {
        let mut inv = InventoryManager::new();
        inv.process_fill(&make_fill(Side::Bid, 100.0, 2.0));
        inv.process_fill(&make_fill(Side::Ask, 101.0, 1.5));

        assert!((inv.total_volume - 3.5).abs() < 1e-10);
        assert_eq!(inv.fill_count, 2);
    }
}
