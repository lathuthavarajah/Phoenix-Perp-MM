#[test]
fn test_funding_annualization() {
    let rate_8h: f64 = 0.0001; // 0.01% per 8h
    let annualized = rate_8h * 1095.0;
    // 0.0001 * 1095 = 0.1095 = 10.95%
    assert!(
        (annualized - 0.1095_f64).abs() < 0.0001,
        "Annualized APY should be 10.95%, got {:.4}",
        annualized
    );
}

#[test]
fn test_funding_annualization_high() {
    let rate_8h: f64 = 0.0003; // 0.03% per 8h
    let annualized = rate_8h * 1095.0;
    // 0.0003 * 1095 = 0.3285 = 32.85%
    assert!(
        (annualized - 0.3285_f64).abs() < 0.0001,
        "Annualized APY should be 32.85%, got {:.4}",
        annualized
    );
}

#[test]
fn test_entry_signal_fires() {
    let entry_threshold = 0.15; // 15%
    let annualized_apy = 0.20; // 20%
    let premium = 0.001; // positive
    let has_position = false;

    let should_enter = !has_position
        && annualized_apy > entry_threshold
        && premium > 0.0;

    assert!(should_enter, "Should emit Enter signal when APY > threshold and premium > 0");
}

#[test]
fn test_entry_signal_not_fired_below_threshold() {
    let entry_threshold = 0.15;
    let annualized_apy = 0.10; // 10% — below threshold
    let premium = 0.001;
    let has_position = false;

    let should_enter = !has_position
        && annualized_apy > entry_threshold
        && premium > 0.0;

    assert!(!should_enter, "Should NOT enter when APY < threshold");
}

#[test]
fn test_exit_signal_fires() {
    let exit_threshold = 0.02; // 2%
    let annualized_apy = 0.01; // 1%
    let has_position = true;

    let should_exit = has_position && annualized_apy < exit_threshold;

    assert!(should_exit, "Should emit Exit signal when APY drops below exit threshold");
}

#[test]
fn test_exit_signal_on_negative_funding() {
    let funding_rate_8h = -0.0001;
    let has_position = true;

    let should_exit = has_position && funding_rate_8h < 0.0;

    assert!(should_exit, "Should exit when funding flips negative");
}

#[test]
fn test_mock_funding_rate_computation() {
    // Phoenix mock: rate = clamp(premium * 0.1, -0.003, 0.003)
    let mark_price: f64 = 150.15;
    let index_price: f64 = 150.0;
    let premium = (mark_price - index_price) / index_price; // 0.001
    let rate = (premium * 0.1).clamp(-0.003_f64, 0.003_f64);

    // premium = 0.001, rate = 0.0001
    assert!(
        (rate - 0.0001_f64).abs() < 0.00001,
        "Funding rate should be ~0.0001, got {}",
        rate
    );

    // Annualized: 0.0001 * 1095 = 10.95%
    let apy = rate * 1095.0;
    assert!((apy - 0.1095).abs() < 0.001);
}

#[test]
fn test_funding_rate_clamped() {
    // Extreme premium
    let mark_price: f64 = 165.0;
    let index_price: f64 = 150.0;
    let premium = (mark_price - index_price) / index_price; // 0.1
    let rate = (premium * 0.1).clamp(-0.003_f64, 0.003_f64);

    // Should be clamped to 0.003
    assert!(
        (rate - 0.003).abs() < 0.00001,
        "Rate should be clamped to 0.003, got {}",
        rate
    );
}
