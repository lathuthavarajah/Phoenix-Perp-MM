use crate::types::{QuoteLevel, Side};

/// Generate multi-level quotes around a central bid/ask.
///
/// Creates `num_levels` on each side with:
/// - Progressive spacing from the inner price
/// - Geometrically decaying size per level
///
/// # Arguments
/// * `bid` - Inner bid price from quoting model
/// * `ask` - Inner ask price from quoting model
/// * `num_levels` - Number of levels per side
/// * `level_spacing_bps` - Spacing between levels in basis points
/// * `base_size` - Size of the innermost level
/// * `size_decay` - Multiplicative decay factor per level (0.0-1.0)
pub fn generate_levels(
    bid: f64,
    ask: f64,
    num_levels: u32,
    level_spacing_bps: f64,
    base_size: f64,
    size_decay: f64,
) -> Vec<QuoteLevel> {
    let mut levels = Vec::with_capacity((num_levels * 2) as usize);

    for i in 0..num_levels {
        let spacing_frac = (i as f64) * level_spacing_bps / 10_000.0;
        let size = base_size * size_decay.powi(i as i32);

        // Bid levels: price decreases with each level
        levels.push(QuoteLevel {
            side: Side::Bid,
            price: bid * (1.0 - spacing_frac),
            size,
            level: i,
        });

        // Ask levels: price increases with each level
        levels.push(QuoteLevel {
            side: Side::Ask,
            price: ask * (1.0 + spacing_frac),
            size,
            level: i,
        });
    }

    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correct_count() {
        let levels = generate_levels(99.0, 101.0, 3, 10.0, 1.0, 0.5);
        assert_eq!(levels.len(), 6, "3 levels * 2 sides = 6");

        let bids: Vec<_> = levels.iter().filter(|l| l.side == Side::Bid).collect();
        let asks: Vec<_> = levels.iter().filter(|l| l.side == Side::Ask).collect();
        assert_eq!(bids.len(), 3);
        assert_eq!(asks.len(), 3);
    }

    #[test]
    fn inner_level_matches_input() {
        let levels = generate_levels(99.0, 101.0, 3, 10.0, 1.0, 0.5);

        let inner_bid = levels.iter().find(|l| l.side == Side::Bid && l.level == 0).unwrap();
        let inner_ask = levels.iter().find(|l| l.side == Side::Ask && l.level == 0).unwrap();

        assert!((inner_bid.price - 99.0).abs() < 1e-10);
        assert!((inner_ask.price - 101.0).abs() < 1e-10);
    }

    #[test]
    fn bids_decrease_asks_increase() {
        let levels = generate_levels(99.0, 101.0, 3, 20.0, 1.0, 0.8);

        let mut bid_prices: Vec<f64> = levels
            .iter()
            .filter(|l| l.side == Side::Bid)
            .map(|l| l.price)
            .collect();
        bid_prices.sort_by(|a, b| b.partial_cmp(a).unwrap()); // descending

        for w in bid_prices.windows(2) {
            assert!(w[0] > w[1], "bid prices should decrease per level");
        }

        let mut ask_prices: Vec<f64> = levels
            .iter()
            .filter(|l| l.side == Side::Ask)
            .map(|l| l.price)
            .collect();
        ask_prices.sort_by(|a, b| a.partial_cmp(b).unwrap()); // ascending

        for w in ask_prices.windows(2) {
            assert!(w[0] < w[1], "ask prices should increase per level");
        }
    }

    #[test]
    fn size_decays_per_level() {
        let levels = generate_levels(99.0, 101.0, 4, 10.0, 2.0, 0.5);

        let bid_sizes: Vec<f64> = levels
            .iter()
            .filter(|l| l.side == Side::Bid)
            .map(|l| l.size)
            .collect();

        assert!((bid_sizes[0] - 2.0).abs() < 1e-10);
        assert!((bid_sizes[1] - 1.0).abs() < 1e-10);
        assert!((bid_sizes[2] - 0.5).abs() < 1e-10);
        assert!((bid_sizes[3] - 0.25).abs() < 1e-10);
    }

    #[test]
    fn single_level() {
        let levels = generate_levels(99.0, 101.0, 1, 10.0, 1.0, 0.5);
        assert_eq!(levels.len(), 2);
    }
}
