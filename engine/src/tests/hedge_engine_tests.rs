use crate::types::EngineConfig;

#[test]
fn test_hedge_sizing() {
    let config = EngineConfig {
        position_size_usdc: 500.0,
        max_leverage: 3.0,
        ..Default::default()
    };

    let sol_price = 150.0;
    let spot_size = config.position_size_usdc / sol_price;
    let perp_size = -spot_size;
    let collateral = spot_size * sol_price / config.max_leverage;

    assert!((spot_size - 3.3333).abs() < 0.001, "spot_size should be ~3.333 SOL");
    assert!((perp_size + 3.3333).abs() < 0.001, "perp_size should be ~-3.333 SOL");
    assert!((collateral - 166.6667).abs() < 0.1, "collateral should be ~166.67 USDC");

    // Delta should be zero at entry
    let delta = spot_size + perp_size;
    assert!(delta.abs() < 1e-10, "delta should be zero at entry");
}

#[test]
fn test_hedge_sizing_different_prices() {
    let config = EngineConfig {
        position_size_usdc: 1000.0,
        max_leverage: 5.0,
        ..Default::default()
    };

    let sol_price = 200.0;
    let spot_size = config.position_size_usdc / sol_price;
    let perp_size = -spot_size;
    let collateral = spot_size * sol_price / config.max_leverage;

    assert!((spot_size - 5.0).abs() < 0.001);
    assert!((perp_size + 5.0).abs() < 0.001);
    assert!((collateral - 200.0).abs() < 0.1);
}

#[test]
fn test_delta_rebalance_threshold() {
    let config = EngineConfig::default();
    let sol_price = 150.0;
    let notional = 500.0;

    // 1% of notional in SOL terms
    let threshold_usdc = notional * config.rebalance_delta_threshold;
    let threshold_sol = threshold_usdc / sol_price;

    // If delta is 0.05 SOL and threshold is ~0.033 SOL, rebalance should fire
    let delta = 0.05;
    let delta_usdc = delta * sol_price;
    let needs_rebalance = delta_usdc.abs() > threshold_usdc;

    assert!(needs_rebalance, "Delta of 0.05 SOL ($7.50) should exceed 1% threshold ($5.00)");

    // Delta below threshold should not trigger
    let small_delta = 0.01;
    let small_delta_usdc = small_delta * sol_price;
    assert!(
        !( small_delta_usdc.abs() > threshold_usdc),
        "Delta of 0.01 SOL ($1.50) should NOT exceed 1% threshold ($5.00)"
    );

    let _ = threshold_sol;
}
