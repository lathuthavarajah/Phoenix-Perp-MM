use crate::types::{BasisPosition, PerpMarketState, MarginState};

fn compute_margin_state(
    position: &BasisPosition,
    perp_state: &PerpMarketState,
    sol_price: f64,
) -> MarginState {
    let unrealized_pnl = position.perp_size * (sol_price - position.perp_entry_price);
    let total_equity = position.collateral_usdc + unrealized_pnl;
    let abs_notional = position.perp_size.abs() * perp_state.mark_price;
    let margin_ratio = if abs_notional > 0.0 {
        total_equity / abs_notional
    } else {
        1.0
    };
    let maintenance_margin = abs_notional * 0.05;

    let liq_price = if position.perp_size.abs() > 0.0 {
        position.perp_entry_price
            + (total_equity / position.perp_size.abs())
            - (maintenance_margin / position.perp_size.abs())
    } else {
        0.0
    };

    let distance_to_liq_pct = if sol_price > 0.0 {
        (liq_price - sol_price) / sol_price
    } else {
        0.0
    };

    MarginState {
        collateral_usdc: position.collateral_usdc,
        unrealized_pnl,
        total_equity,
        maintenance_margin,
        margin_ratio,
        liquidation_price: liq_price,
        distance_to_liq_pct,
    }
}

#[test]
fn test_margin_state_at_entry() {
    // Short 3.333 SOL at $150, collateral $166.67
    let position = BasisPosition {
        spot_size: 3.333,
        perp_size: -3.333,
        spot_entry_price: 150.0,
        perp_entry_price: 150.0,
        collateral_usdc: 166.67,
        is_open: true,
        opened_at_ts: 0,
    };

    let perp_state = PerpMarketState {
        mark_price: 150.0,
        index_price: 150.0,
        funding_rate_8h: 0.0001,
        open_interest_long: 1_000_000.0,
        open_interest_short: 1_000_000.0,
        premium: 0.001,
        is_simulated: true,
    };

    let margin = compute_margin_state(&position, &perp_state, 150.0);

    // At entry: unrealized PnL = 0, equity = collateral
    assert!((margin.unrealized_pnl).abs() < 0.01);
    assert!((margin.total_equity - 166.67).abs() < 0.01);
    // margin_ratio = 166.67 / (3.333 * 150) = 166.67 / 499.95 ≈ 0.3334
    assert!((margin.margin_ratio - 0.3334).abs() < 0.01);
}

#[test]
fn test_margin_state_price_increase() {
    // Price rises to $160 — short position loses money
    let position = BasisPosition {
        spot_size: 3.333,
        perp_size: -3.333,
        spot_entry_price: 150.0,
        perp_entry_price: 150.0,
        collateral_usdc: 166.67,
        is_open: true,
        opened_at_ts: 0,
    };

    let perp_state = PerpMarketState {
        mark_price: 160.0,
        index_price: 160.0,
        funding_rate_8h: 0.0001,
        open_interest_long: 1_000_000.0,
        open_interest_short: 1_000_000.0,
        premium: 0.0,
        is_simulated: true,
    };

    let margin = compute_margin_state(&position, &perp_state, 160.0);

    // Unrealized PnL = -3.333 * (160 - 150) = -33.33
    assert!((margin.unrealized_pnl + 33.33).abs() < 0.1);
    // Equity = 166.67 - 33.33 = 133.34
    assert!((margin.total_equity - 133.34).abs() < 0.1);
    // Margin ratio = 133.34 / (3.333 * 160) = 133.34 / 533.28 ≈ 0.25
    assert!((margin.margin_ratio - 0.25).abs() < 0.01);
}

#[test]
fn test_emergency_close_triggers() {
    let position = BasisPosition {
        spot_size: 3.333,
        perp_size: -3.333,
        spot_entry_price: 150.0,
        perp_entry_price: 150.0,
        collateral_usdc: 166.67,
        is_open: true,
        opened_at_ts: 0,
    };

    // Price at $185 — severe loss on short
    let perp_state = PerpMarketState {
        mark_price: 185.0,
        index_price: 185.0,
        funding_rate_8h: 0.0001,
        open_interest_long: 1_000_000.0,
        open_interest_short: 1_000_000.0,
        premium: 0.0,
        is_simulated: true,
    };

    let margin = compute_margin_state(&position, &perp_state, 185.0);

    // Unrealized PnL = -3.333 * (185 - 150) = -116.655
    // Equity = 166.67 - 116.655 = 50.015
    // Margin ratio = 50.015 / (3.333 * 185) = 50.015 / 616.605 ≈ 0.081
    assert!(
        margin.margin_ratio < 0.12,
        "Margin ratio {:.4} should be below emergency threshold 0.12",
        margin.margin_ratio
    );
}
