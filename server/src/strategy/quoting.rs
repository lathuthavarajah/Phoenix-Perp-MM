//! Avellaneda-Stoikov inventory-skewed quoting.
//!
//! The model adjusts the reservation price (fair value from the maker's perspective)
//! based on current inventory. When long, the reservation price drops below fair,
//! tightening the ask to encourage selling. Vice versa when short.
//!
//! Key parameters:
//! - `gamma`: risk aversion. Higher → wider spreads, stronger inventory skew.
//! - `base_spread_bps`: minimum half-spread in basis points.

/// Computed quote prices from the Avellaneda-Stoikov model.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QuoteParams {
    /// Inventory-adjusted fair price
    pub reservation_price: f64,
    /// Half spread (distance from reservation to bid/ask)
    pub half_spread: f64,
    /// Bid price
    pub bid: f64,
    /// Ask price
    pub ask: f64,
}

/// Compute inventory-skewed bid/ask from the Avellaneda-Stoikov model.
///
/// # Arguments
/// * `fair_price` - Current fair price from oracle
/// * `position` - Current position in base units (positive = long)
/// * `max_position` - Maximum allowed position for normalization
/// * `volatility` - Annualized volatility as decimal (e.g. 0.80 = 80%)
/// * `gamma` - Risk aversion parameter
/// * `base_spread_bps` - Minimum half-spread in basis points
pub fn compute_quotes(
    fair_price: f64,
    position: f64,
    max_position: f64,
    volatility: f64,
    gamma: f64,
    base_spread_bps: f64,
) -> QuoteParams {
    // Normalize position to [-1, 1]
    let q = (position / max_position).clamp(-1.0, 1.0);

    // Reservation price: skewed away from inventory direction
    // When long (q > 0): reservation < fair → ask tightens → encourages selling
    // When short (q < 0): reservation > fair → bid tightens → encourages buying
    let reservation_price = fair_price * (1.0 - gamma * q * volatility * volatility);

    // Half spread: wider with higher vol or higher gamma
    let vol_spread_bps = volatility * 10_000.0 * gamma;
    let effective_spread_bps = base_spread_bps.max(vol_spread_bps);
    let half_spread = effective_spread_bps * fair_price / 10_000.0;

    let bid = reservation_price - half_spread;
    let ask = reservation_price + half_spread;

    QuoteParams {
        reservation_price,
        half_spread,
        bid,
        ask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAIR: f64 = 100.0;
    const MAX_POS: f64 = 10.0;
    const VOL: f64 = 0.50;
    const GAMMA: f64 = 0.1;
    const BASE_BPS: f64 = 10.0;

    #[test]
    fn flat_position_symmetric_quotes() {
        let q = compute_quotes(FAIR, 0.0, MAX_POS, VOL, GAMMA, BASE_BPS);

        // Reservation should equal fair price
        assert!(
            (q.reservation_price - FAIR).abs() < 1e-10,
            "reservation should equal fair at flat position"
        );

        // Bid and ask should be symmetric around fair
        let bid_dist = FAIR - q.bid;
        let ask_dist = q.ask - FAIR;
        assert!(
            (bid_dist - ask_dist).abs() < 1e-10,
            "quotes should be symmetric at flat position"
        );
    }

    #[test]
    fn long_position_skews_to_sell() {
        let q = compute_quotes(FAIR, 5.0, MAX_POS, VOL, GAMMA, BASE_BPS);

        // When long, reservation should be below fair
        assert!(
            q.reservation_price < FAIR,
            "reservation should be below fair when long"
        );

        // Ask should be closer to fair than bid (tighter ask to sell)
        let bid_dist = FAIR - q.bid;
        let ask_dist = q.ask - FAIR;
        assert!(
            ask_dist < bid_dist,
            "ask should be tighter than bid when long: ask_dist={ask_dist}, bid_dist={bid_dist}"
        );
    }

    #[test]
    fn short_position_skews_to_buy() {
        let q = compute_quotes(FAIR, -5.0, MAX_POS, VOL, GAMMA, BASE_BPS);

        // When short, reservation should be above fair
        assert!(
            q.reservation_price > FAIR,
            "reservation should be above fair when short"
        );

        // Bid should be closer to fair than ask (tighter bid to buy)
        let bid_dist = FAIR - q.bid;
        let ask_dist = q.ask - FAIR;
        assert!(
            bid_dist < ask_dist,
            "bid should be tighter than ask when short: bid_dist={bid_dist}, ask_dist={ask_dist}"
        );
    }

    #[test]
    fn higher_vol_widens_spread() {
        let low_vol = compute_quotes(FAIR, 0.0, MAX_POS, 0.10, GAMMA, BASE_BPS);
        let high_vol = compute_quotes(FAIR, 0.0, MAX_POS, 1.00, GAMMA, BASE_BPS);

        assert!(
            high_vol.half_spread > low_vol.half_spread,
            "higher vol should widen spread"
        );
    }

    #[test]
    fn position_clamps_at_max() {
        let at_max = compute_quotes(FAIR, 10.0, MAX_POS, VOL, GAMMA, BASE_BPS);
        let over_max = compute_quotes(FAIR, 20.0, MAX_POS, VOL, GAMMA, BASE_BPS);

        assert!(
            (at_max.reservation_price - over_max.reservation_price).abs() < 1e-10,
            "position beyond max should clamp to max"
        );
    }

    #[test]
    fn base_spread_floors_half_spread() {
        // Very low vol so vol_spread_bps < base_spread_bps
        let q = compute_quotes(FAIR, 0.0, MAX_POS, 0.001, GAMMA, 50.0);
        let expected_half = 50.0 * FAIR / 10_000.0; // 0.50

        assert!(
            (q.half_spread - expected_half).abs() < 1e-10,
            "half_spread should be floored by base_spread_bps"
        );
    }

    #[test]
    fn symmetry_long_vs_short() {
        let long = compute_quotes(FAIR, 5.0, MAX_POS, VOL, GAMMA, BASE_BPS);
        let short = compute_quotes(FAIR, -5.0, MAX_POS, VOL, GAMMA, BASE_BPS);

        // The reservation price shift should be symmetric
        let long_shift = FAIR - long.reservation_price;
        let short_shift = short.reservation_price - FAIR;
        assert!(
            (long_shift - short_shift).abs() < 1e-10,
            "inventory skew should be symmetric"
        );
    }
}
