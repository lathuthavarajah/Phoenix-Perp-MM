use crate::inventory::InventoryManager;
use crate::types::{QuoteLevel, Side};

/// Risk manager decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskAction {
    /// Normal operation — place all quotes.
    Continue,
    /// Position near limit — only place orders that reduce position.
    ReduceOnly,
    /// Emergency — cancel all orders immediately.
    EmergencyCancel,
}

pub struct RiskManager {
    pub max_position: f64,
    pub max_drawdown_usd: f64,
}

impl RiskManager {
    pub fn new(max_position: f64, max_drawdown_usd: f64) -> Self {
        Self {
            max_position,
            max_drawdown_usd,
        }
    }

    /// Evaluate current risk state and return the appropriate action.
    pub fn check(&self, inventory: &InventoryManager) -> RiskAction {
        // Emergency: drawdown exceeds limit
        if inventory.drawdown() >= self.max_drawdown_usd {
            return RiskAction::EmergencyCancel;
        }

        // Reduce-only: position at or beyond limit
        if inventory.position.abs() >= self.max_position {
            return RiskAction::ReduceOnly;
        }

        RiskAction::Continue
    }

    /// Filter quote levels based on the risk action.
    /// - Continue: pass all through
    /// - ReduceOnly: only keep orders that reduce current position
    /// - EmergencyCancel: return empty (cancel all)
    pub fn filter_quotes(
        &self,
        quotes: Vec<QuoteLevel>,
        action: &RiskAction,
        position: f64,
    ) -> Vec<QuoteLevel> {
        match action {
            RiskAction::Continue => quotes,
            RiskAction::EmergencyCancel => Vec::new(),
            RiskAction::ReduceOnly => {
                quotes
                    .into_iter()
                    .filter(|q| {
                        if position > 0.0 {
                            // Long → only allow sells (asks)
                            q.side == Side::Ask
                        } else if position < 0.0 {
                            // Short → only allow buys (bids)
                            q.side == Side::Bid
                        } else {
                            // Flat → allow all (shouldn't hit reduce-only when flat)
                            true
                        }
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Fill;
    use chrono::Utc;

    fn make_fill(side: Side, price: f64, size: f64) -> Fill {
        Fill {
            side,
            price,
            size,
            timestamp: Utc::now(),
        }
    }

    fn make_quote(side: Side, price: f64, size: f64) -> QuoteLevel {
        QuoteLevel {
            side,
            price,
            size,
            level: 0,
        }
    }

    #[test]
    fn normal_operation() {
        let rm = RiskManager::new(10.0, 100.0);
        let inv = InventoryManager::new();
        assert_eq!(rm.check(&inv), RiskAction::Continue);
    }

    #[test]
    fn position_limit_triggers_reduce_only() {
        let rm = RiskManager::new(10.0, 100.0);
        let mut inv = InventoryManager::new();
        inv.process_fill(&make_fill(Side::Bid, 100.0, 10.0));

        assert_eq!(rm.check(&inv), RiskAction::ReduceOnly);
    }

    #[test]
    fn drawdown_triggers_emergency() {
        let rm = RiskManager::new(100.0, 50.0);
        let mut inv = InventoryManager::new();

        // Gain 100, then lose 60 → drawdown = 60 > 50
        inv.process_fill(&make_fill(Side::Bid, 100.0, 1.0));
        inv.process_fill(&make_fill(Side::Ask, 200.0, 1.0)); // +100
        inv.process_fill(&make_fill(Side::Bid, 200.0, 1.0));
        inv.process_fill(&make_fill(Side::Ask, 140.0, 1.0)); // -60, total = 40, peak = 100

        assert_eq!(rm.check(&inv), RiskAction::EmergencyCancel);
    }

    #[test]
    fn reduce_only_filters_when_long() {
        let rm = RiskManager::new(10.0, 100.0);

        let quotes = vec![
            make_quote(Side::Bid, 99.0, 1.0),
            make_quote(Side::Ask, 101.0, 1.0),
        ];

        let filtered = rm.filter_quotes(quotes, &RiskAction::ReduceOnly, 5.0);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].side, Side::Ask); // Only sell when long
    }

    #[test]
    fn reduce_only_filters_when_short() {
        let rm = RiskManager::new(10.0, 100.0);

        let quotes = vec![
            make_quote(Side::Bid, 99.0, 1.0),
            make_quote(Side::Ask, 101.0, 1.0),
        ];

        let filtered = rm.filter_quotes(quotes, &RiskAction::ReduceOnly, -5.0);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].side, Side::Bid); // Only buy when short
    }

    #[test]
    fn emergency_cancel_returns_empty() {
        let rm = RiskManager::new(10.0, 100.0);

        let quotes = vec![
            make_quote(Side::Bid, 99.0, 1.0),
            make_quote(Side::Ask, 101.0, 1.0),
        ];

        let filtered = rm.filter_quotes(quotes, &RiskAction::EmergencyCancel, 5.0);
        assert!(filtered.is_empty());
    }

    #[test]
    fn continue_passes_all() {
        let rm = RiskManager::new(10.0, 100.0);

        let quotes = vec![
            make_quote(Side::Bid, 99.0, 1.0),
            make_quote(Side::Ask, 101.0, 1.0),
        ];

        let filtered = rm.filter_quotes(quotes, &RiskAction::Continue, 5.0);
        assert_eq!(filtered.len(), 2);
    }
}
