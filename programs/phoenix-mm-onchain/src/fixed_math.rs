//! Fixed-point Avellaneda-Stoikov quoting for on-chain use.
//!
//! All prices are i128 scaled by PRICE_SCALE (1e10).
//! Parameters (gamma, size_decay) are scaled by PARAM_SCALE (1e6).
//! No floats — safe for BPF.

/// Price scaling factor: 1e10
pub const PRICE_SCALE: i128 = 10_000_000_000;

/// Parameter scaling factor: 1e6
pub const PARAM_SCALE: i128 = 1_000_000;

/// Basis points denominator: 10_000
pub const BPS_DENOM: i128 = 10_000;

/// Computed quote prices in fixed-point (scaled by PRICE_SCALE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedQuoteParams {
    /// Inventory-adjusted fair price (scaled by PRICE_SCALE).
    pub reservation_price: i128,
    /// Half spread (scaled by PRICE_SCALE).
    pub half_spread: i128,
    /// Bid price (scaled by PRICE_SCALE).
    pub bid: i128,
    /// Ask price (scaled by PRICE_SCALE).
    pub ask: i128,
}

/// Compute inventory-skewed bid/ask using fixed-point arithmetic.
///
/// # Arguments
/// * `fair_price` — scaled by PRICE_SCALE
/// * `position_lots` — current position (signed)
/// * `max_position_lots` — max position for normalization (positive)
/// * `volatility_bps` — annualized vol in basis points (e.g. 5000 = 50%)
/// * `gamma_scaled` — risk aversion × PARAM_SCALE (e.g. 0.1 → 100_000)
/// * `base_spread_bps` — minimum half-spread in basis points
pub fn compute_quotes_fixed(
    fair_price: i128,
    position_lots: i64,
    max_position_lots: i64,
    volatility_bps: i128,
    gamma_scaled: i128,
    base_spread_bps: i128,
) -> FixedQuoteParams {
    // q_scaled = (position_lots * PARAM_SCALE) / max_position_lots, clamped to [-PARAM_SCALE, PARAM_SCALE]
    let q_scaled = if max_position_lots == 0 {
        0i128
    } else {
        let raw = (position_lots as i128)
            .saturating_mul(PARAM_SCALE)
            / (max_position_lots as i128);
        raw.clamp(-PARAM_SCALE, PARAM_SCALE)
    };

    // Reservation price = fair_price - fair_price * gamma_scaled * q_scaled * vol_bps^2
    //                                  / (PARAM_SCALE * PARAM_SCALE * BPS_DENOM^2)
    //
    // skew_numer = gamma_scaled * q_scaled * vol_bps * vol_bps
    // skew_denom = PARAM_SCALE * PARAM_SCALE * BPS_DENOM * BPS_DENOM
    //
    // reservation = fair_price - fair_price * skew_numer / skew_denom
    let skew_numer = gamma_scaled
        .checked_mul(q_scaled)
        .unwrap()
        .checked_mul(volatility_bps)
        .unwrap()
        .checked_mul(volatility_bps)
        .unwrap();
    let skew_denom = PARAM_SCALE
        .checked_mul(PARAM_SCALE)
        .unwrap()
        .checked_mul(BPS_DENOM)
        .unwrap()
        .checked_mul(BPS_DENOM)
        .unwrap();

    let reservation_price = fair_price
        .checked_sub(
            fair_price
                .checked_mul(skew_numer)
                .unwrap()
                .checked_div(skew_denom)
                .unwrap(),
        )
        .unwrap();

    // vol_spread_bps = volatility_bps * gamma_scaled / PARAM_SCALE
    let vol_spread_bps = volatility_bps
        .checked_mul(gamma_scaled)
        .unwrap()
        .checked_div(PARAM_SCALE)
        .unwrap();

    let effective_spread_bps = base_spread_bps.max(vol_spread_bps);

    // half_spread = effective_spread_bps * fair_price / BPS_DENOM
    let half_spread = effective_spread_bps
        .checked_mul(fair_price)
        .unwrap()
        .checked_div(BPS_DENOM)
        .unwrap();

    let bid = reservation_price.checked_sub(half_spread).unwrap();
    let ask = reservation_price.checked_add(half_spread).unwrap();

    FixedQuoteParams {
        reservation_price,
        half_spread,
        bid,
        ask,
    }
}

/// Convert a Pyth price (price × 10^expo) to PRICE_SCALE representation.
///
/// Returns None if the price is non-positive or the exponent causes overflow.
pub fn pyth_price_to_scaled(price: i64, expo: i32) -> Option<i128> {
    if price <= 0 {
        return None;
    }
    let price_i128 = price as i128;

    // We need: price * 10^expo * PRICE_SCALE
    // = price * 10^(expo + 10)
    let target_exp = expo + 10; // since PRICE_SCALE = 10^10

    if target_exp >= 0 {
        let factor = 10i128.checked_pow(target_exp as u32)?;
        price_i128.checked_mul(factor)
    } else {
        let divisor = 10i128.checked_pow((-target_exp) as u32)?;
        Some(price_i128 / divisor)
    }
}

/// Compute sizes for each level with geometric decay.
///
/// Returns `num_levels` sizes where size[i] = base_size_lots * decay^i.
/// `size_decay_scaled` is the decay factor × PARAM_SCALE (e.g. 0.5 → 500_000).
pub fn compute_level_sizes(
    base_size_lots: u64,
    size_decay_scaled: u64,
    num_levels: u8,
) -> Vec<u64> {
    let mut sizes = Vec::with_capacity(num_levels as usize);
    let mut current = base_size_lots as i128 * PARAM_SCALE;

    for i in 0..num_levels {
        let size = current / PARAM_SCALE;
        // Minimum 1 lot per level
        sizes.push(if size < 1 && i == 0 {
            base_size_lots.max(1)
        } else {
            (size as u64).max(1)
        });
        current = current * (size_decay_scaled as i128) / PARAM_SCALE;
    }

    sizes
}

/// Compute bid/ask prices for each level.
///
/// Level 0 = inner price (bid or ask from A-S model).
/// Level i: bid *= (1 - i * spacing_bps / 10000), ask *= (1 + i * spacing_bps / 10000).
///
/// Prices are in PRICE_SCALE.
pub fn compute_level_prices(
    inner_bid: i128,
    inner_ask: i128,
    num_levels: u8,
    level_spacing_bps: i128,
) -> (Vec<i128>, Vec<i128>) {
    let mut bids = Vec::with_capacity(num_levels as usize);
    let mut asks = Vec::with_capacity(num_levels as usize);

    for i in 0..num_levels as i128 {
        // bid_i = inner_bid * (BPS_DENOM - i * spacing) / BPS_DENOM
        let bid_price = inner_bid
            .checked_mul(BPS_DENOM - i * level_spacing_bps)
            .unwrap()
            .checked_div(BPS_DENOM)
            .unwrap();
        bids.push(bid_price);

        // ask_i = inner_ask * (BPS_DENOM + i * spacing) / BPS_DENOM
        let ask_price = inner_ask
            .checked_mul(BPS_DENOM + i * level_spacing_bps)
            .unwrap()
            .checked_div(BPS_DENOM)
            .unwrap();
        asks.push(ask_price);
    }

    (bids, asks)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mirror the f64 test constants from server/src/strategy/quoting.rs
    const FAIR_F64: f64 = 100.0;
    const MAX_POS: i64 = 10;
    const VOL_BPS: i128 = 5000; // 0.50 = 50% annual vol = 5000 bps
    const GAMMA_SCALED: i128 = 100_000; // 0.1 * 1e6
    const BASE_BPS: i128 = 10;

    fn fair_scaled() -> i128 {
        (FAIR_F64 * PRICE_SCALE as f64) as i128
    }

    /// Convert fixed-point scaled price back to f64 for comparison.
    fn to_f64(scaled: i128) -> f64 {
        scaled as f64 / PRICE_SCALE as f64
    }

    // -----------------------------------------------------------------------
    // Cross-validation tests against server/src/strategy/quoting.rs
    // -----------------------------------------------------------------------

    #[test]
    fn flat_position_symmetric_quotes() {
        let q = compute_quotes_fixed(fair_scaled(), 0, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);

        let res_f64 = to_f64(q.reservation_price);
        assert!(
            (res_f64 - FAIR_F64).abs() < 1e-4,
            "reservation should equal fair at flat position, got {res_f64}"
        );

        let bid_dist = FAIR_F64 - to_f64(q.bid);
        let ask_dist = to_f64(q.ask) - FAIR_F64;
        assert!(
            (bid_dist - ask_dist).abs() < 1e-4,
            "quotes should be symmetric at flat position"
        );
    }

    #[test]
    fn long_position_skews_to_sell() {
        let q = compute_quotes_fixed(fair_scaled(), 5, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);

        assert!(
            to_f64(q.reservation_price) < FAIR_F64,
            "reservation should be below fair when long"
        );

        let bid_dist = FAIR_F64 - to_f64(q.bid);
        let ask_dist = to_f64(q.ask) - FAIR_F64;
        assert!(
            ask_dist < bid_dist,
            "ask should be tighter than bid when long: ask_dist={ask_dist}, bid_dist={bid_dist}"
        );
    }

    #[test]
    fn short_position_skews_to_buy() {
        let q = compute_quotes_fixed(fair_scaled(), -5, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);

        assert!(
            to_f64(q.reservation_price) > FAIR_F64,
            "reservation should be above fair when short"
        );

        let bid_dist = FAIR_F64 - to_f64(q.bid);
        let ask_dist = to_f64(q.ask) - FAIR_F64;
        assert!(
            bid_dist < ask_dist,
            "bid should be tighter than ask when short"
        );
    }

    #[test]
    fn higher_vol_widens_spread() {
        let low = compute_quotes_fixed(fair_scaled(), 0, MAX_POS, 1000, GAMMA_SCALED, BASE_BPS);
        let high = compute_quotes_fixed(fair_scaled(), 0, MAX_POS, 10000, GAMMA_SCALED, BASE_BPS);

        assert!(
            to_f64(high.half_spread) > to_f64(low.half_spread),
            "higher vol should widen spread"
        );
    }

    #[test]
    fn position_clamps_at_max() {
        let at_max = compute_quotes_fixed(fair_scaled(), 10, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);
        let over_max = compute_quotes_fixed(fair_scaled(), 20, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);

        assert_eq!(
            at_max.reservation_price, over_max.reservation_price,
            "position beyond max should clamp to max"
        );
    }

    #[test]
    fn base_spread_floors_half_spread() {
        // Very low vol so vol_spread_bps < base_spread_bps
        let q = compute_quotes_fixed(fair_scaled(), 0, MAX_POS, 10, GAMMA_SCALED, 50);
        let expected_half = 50.0 * FAIR_F64 / 10_000.0;

        assert!(
            (to_f64(q.half_spread) - expected_half).abs() < 1e-4,
            "half_spread should be floored by base_spread_bps, got {} expected {}",
            to_f64(q.half_spread),
            expected_half
        );
    }

    #[test]
    fn symmetry_long_vs_short() {
        let long = compute_quotes_fixed(fair_scaled(), 5, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);
        let short = compute_quotes_fixed(fair_scaled(), -5, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);

        let long_shift = FAIR_F64 - to_f64(long.reservation_price);
        let short_shift = to_f64(short.reservation_price) - FAIR_F64;
        assert!(
            (long_shift - short_shift).abs() < 1e-4,
            "inventory skew should be symmetric"
        );
    }

    // -----------------------------------------------------------------------
    // Numerical cross-validation: compare fixed-point to f64 implementation
    // -----------------------------------------------------------------------

    /// Reference f64 implementation (copied from server/src/strategy/quoting.rs)
    fn compute_quotes_f64(
        fair_price: f64,
        position: f64,
        max_position: f64,
        volatility: f64,
        gamma: f64,
        base_spread_bps: f64,
    ) -> (f64, f64, f64, f64) {
        let q = (position / max_position).clamp(-1.0, 1.0);
        let reservation_price = fair_price * (1.0 - gamma * q * volatility * volatility);
        let vol_spread_bps = volatility * 10_000.0 * gamma;
        let effective_spread_bps = base_spread_bps.max(vol_spread_bps);
        let half_spread = effective_spread_bps * fair_price / 10_000.0;
        let bid = reservation_price - half_spread;
        let ask = reservation_price + half_spread;
        (reservation_price, half_spread, bid, ask)
    }

    #[test]
    fn cross_validate_flat() {
        let fixed = compute_quotes_fixed(fair_scaled(), 0, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);
        let (res, hs, bid, ask) = compute_quotes_f64(100.0, 0.0, 10.0, 0.50, 0.1, 10.0);

        assert!((to_f64(fixed.reservation_price) - res).abs() < 1e-4, "reservation mismatch");
        assert!((to_f64(fixed.half_spread) - hs).abs() < 1e-4, "half_spread mismatch");
        assert!((to_f64(fixed.bid) - bid).abs() < 1e-4, "bid mismatch");
        assert!((to_f64(fixed.ask) - ask).abs() < 1e-4, "ask mismatch");
    }

    #[test]
    fn cross_validate_long() {
        let fixed = compute_quotes_fixed(fair_scaled(), 7, MAX_POS, VOL_BPS, GAMMA_SCALED, BASE_BPS);
        let (res, hs, bid, ask) = compute_quotes_f64(100.0, 7.0, 10.0, 0.50, 0.1, 10.0);

        assert!((to_f64(fixed.reservation_price) - res).abs() < 1e-4, "reservation mismatch");
        assert!((to_f64(fixed.half_spread) - hs).abs() < 1e-4, "half_spread mismatch");
        assert!((to_f64(fixed.bid) - bid).abs() < 1e-4, "bid mismatch");
        assert!((to_f64(fixed.ask) - ask).abs() < 1e-4, "ask mismatch");
    }

    #[test]
    fn cross_validate_short_high_vol() {
        // Short position, high vol, high gamma
        let fair = (150.0 * PRICE_SCALE as f64) as i128;
        let fixed = compute_quotes_fixed(fair, -8, 20, 8000, 200_000, 20);
        let (res, hs, bid, ask) = compute_quotes_f64(150.0, -8.0, 20.0, 0.80, 0.2, 20.0);

        assert!(
            (to_f64(fixed.reservation_price) - res).abs() < 0.01,
            "reservation mismatch: fixed={} f64={}",
            to_f64(fixed.reservation_price),
            res
        );
        assert!((to_f64(fixed.half_spread) - hs).abs() < 0.01, "half_spread mismatch");
        assert!((to_f64(fixed.bid) - bid).abs() < 0.01, "bid mismatch");
        assert!((to_f64(fixed.ask) - ask).abs() < 0.01, "ask mismatch");
    }

    // -----------------------------------------------------------------------
    // Pyth price conversion tests
    // -----------------------------------------------------------------------

    #[test]
    fn pyth_price_positive_expo() {
        // price=150, expo=-2 → $1.50 → 1.50 * 1e10 = 15_000_000_000
        let scaled = pyth_price_to_scaled(150, -2).unwrap();
        assert_eq!(scaled, 15_000_000_000);
    }

    #[test]
    fn pyth_price_large_expo() {
        // price=15000000000, expo=-8 → $150.00 → 150 * 1e10
        let scaled = pyth_price_to_scaled(15_000_000_000, -8).unwrap();
        assert_eq!(scaled, 1_500_000_000_000);
    }

    #[test]
    fn pyth_price_rejects_negative() {
        assert!(pyth_price_to_scaled(-100, -8).is_none());
        assert!(pyth_price_to_scaled(0, -8).is_none());
    }

    // -----------------------------------------------------------------------
    // Level computation tests
    // -----------------------------------------------------------------------

    #[test]
    fn level_sizes_decay() {
        let sizes = compute_level_sizes(100, 500_000, 4); // decay = 0.5
        assert_eq!(sizes, vec![100, 50, 25, 12]);
    }

    #[test]
    fn level_sizes_minimum_one() {
        let sizes = compute_level_sizes(1, 500_000, 5);
        // 1, 0.5->1, 0.25->1, 0.125->1, 0.0625->1 (all clamped to 1)
        assert!(sizes.iter().all(|&s| s >= 1));
    }

    #[test]
    fn level_prices_spread_correctly() {
        let inner_bid = 99 * PRICE_SCALE;
        let inner_ask = 101 * PRICE_SCALE;
        let (bids, asks) = compute_level_prices(inner_bid, inner_ask, 3, 10);

        // Level 0 = inner
        assert_eq!(bids[0], inner_bid);
        assert_eq!(asks[0], inner_ask);

        // Level 1: bid = 99 * (10000-10)/10000, ask = 101 * (10000+10)/10000
        assert!(bids[1] < bids[0], "bids should decrease per level");
        assert!(asks[1] > asks[0], "asks should increase per level");

        // Level 2
        assert!(bids[2] < bids[1]);
        assert!(asks[2] > asks[1]);
    }
}
