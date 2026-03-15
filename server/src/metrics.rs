use crate::types::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Compute bid-ask spread from a live orderbook snapshot.
pub fn compute_spread(ob: &Orderbook) -> Option<SpreadSnapshot> {
    let best_bid = ob.bids.first()?.price;
    let best_ask = ob.asks.first()?.price;
    let mid = (best_bid + best_ask) / 2.0;
    let spread_abs = best_ask - best_bid;
    let spread_bps = if mid > 0.0 {
        (spread_abs / mid) * 10_000.0
    } else {
        0.0
    };

    Some(SpreadSnapshot {
        venue: ob.venue,
        pair: ob.pair,
        bid_price: best_bid,
        ask_price: best_ask,
        mid_price: mid,
        spread_absolute: spread_abs,
        spread_bps,
        timestamp_ms: now_ms(),
    })
}

/// Walk the orderbook to estimate average fill price and slippage for a notional size.
pub fn estimate_slippage(
    ob: &Orderbook,
    side: Side,
    notional_usd: f64,
) -> Option<SlippageEstimate> {
    let best_bid = ob.bids.first()?.price;
    let best_ask = ob.asks.first()?.price;
    let mid = (best_bid + best_ask) / 2.0;

    let levels: &[OrderbookLevel] = match side {
        Side::Buy => &ob.asks,
        Side::Sell => &ob.bids,
    };

    let mut remaining_usd = notional_usd;
    let mut total_cost = 0.0_f64;
    let mut total_size = 0.0_f64;

    for level in levels {
        if remaining_usd <= 0.0 {
            break;
        }
        let level_usd = level.notional.min(remaining_usd);
        let level_size = level_usd / level.price;
        total_cost += level.price * level_size;
        total_size += level_size;
        remaining_usd -= level_usd;
    }

    if total_size == 0.0 {
        return None;
    }

    let avg_fill_price = total_cost / total_size;
    let slippage_bps = if mid > 0.0 {
        ((avg_fill_price - mid).abs() / mid) * 10_000.0
    } else {
        0.0
    };

    Some(SlippageEstimate {
        venue: ob.venue,
        pair: ob.pair,
        side,
        notional_usd,
        avg_fill_price,
        mid_price: mid,
        slippage_bps,
        depth_consumed_usd: notional_usd - remaining_usd.max(0.0),
        timestamp_ms: now_ms(),
    })
}

/// Compute Execution Quality Scores for all venues on a given pair.
/// Scores are relative (best venue gets 100 on each dimension).
pub fn compute_eqs(
    spreads: &[SpreadSnapshot],
    slippages: &[SlippageEstimate],
    funding_rates: &[FundingRateSnapshot],
    pair: TradingPair,
) -> Vec<ExecutionQualityScore> {
    let pair_spreads: Vec<&SpreadSnapshot> = spreads.iter().filter(|s| s.pair == pair).collect();
    let pair_slippages: Vec<&SlippageEstimate> = slippages
        .iter()
        .filter(|s| s.pair == pair && matches!(s.side, Side::Buy))
        .collect();
    let pair_funding: Vec<&FundingRateSnapshot> =
        funding_rates.iter().filter(|f| f.pair == pair).collect();

    let max_spread = pair_spreads
        .iter()
        .map(|s| s.spread_bps)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(1.0);
    let max_slip = pair_slippages
        .iter()
        .map(|s| s.slippage_bps)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(1.0);
    let min_slip = pair_slippages
        .iter()
        .map(|s| s.slippage_bps)
        .fold(f64::INFINITY, f64::min)
        .min(max_slip);
    let max_fund = pair_funding
        .iter()
        .map(|f| f.rate_annualized.abs())
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0001);
    let ts = now_ms();

    [
        Venue::Phoenix,
        Venue::Hyperliquid,
        Venue::Drift,
        Venue::Binance,
    ]
    .iter()
    .map(|&venue| {
        let spread_score = pair_spreads
            .iter()
            .find(|s| s.venue == venue)
            .map(|s| (1.0 - s.spread_bps / max_spread) * 100.0)
            .unwrap_or(0.0)
            .max(0.0);

        let slip_range = max_slip - min_slip;
        let depth_score = pair_slippages
            .iter()
            .find(|s| s.venue == venue)
            .map(|s| {
                if slip_range > 0.0 {
                    (1.0 - (s.slippage_bps - min_slip) / slip_range) * 100.0
                } else {
                    100.0
                }
            })
            .unwrap_or(0.0)
            .max(0.0);

        let funding_score = pair_funding
            .iter()
            .find(|f| f.venue == venue)
            .map(|f| (1.0 - f.rate_annualized.abs() / max_fund) * 100.0)
            .unwrap_or(0.0)
            .max(0.0);

        ExecutionQualityScore {
            venue,
            pair,
            spread_score,
            depth_score,
            funding_score,
            composite_score: 0.4 * spread_score + 0.4 * depth_score + 0.2 * funding_score,
            timestamp_ms: ts,
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ob(venue: Venue, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> Orderbook {
        Orderbook {
            venue,
            pair: TradingPair::SolPerp,
            bids: bids
                .iter()
                .map(|&(p, s)| OrderbookLevel {
                    price: p,
                    size: s,
                    notional: p * s,
                })
                .collect(),
            asks: asks
                .iter()
                .map(|&(p, s)| OrderbookLevel {
                    price: p,
                    size: s,
                    notional: p * s,
                })
                .collect(),
            timestamp_ms: 0,
        }
    }

    #[test]
    fn test_spread_bps() {
        let ob = make_ob(Venue::Phoenix, vec![(99.9, 10.0)], vec![(100.1, 10.0)]);
        let snap = compute_spread(&ob).unwrap();
        assert!((snap.spread_bps - 20.0).abs() < 0.01);
        assert!((snap.mid_price - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_slippage_walk_book() {
        let ob = make_ob(
            Venue::Hyperliquid,
            vec![(99.9, 10.0)],
            vec![(100.0, 5.0), (100.5, 5.0)],
        );
        let slip = estimate_slippage(&ob, Side::Buy, 750.0).unwrap();
        assert!(slip.avg_fill_price > 100.0 && slip.avg_fill_price < 100.5);
        assert!(slip.slippage_bps > 0.0);
        assert!((slip.depth_consumed_usd - 750.0).abs() < 0.01);
    }

    #[test]
    fn test_slippage_insufficient_depth() {
        let ob = make_ob(Venue::Binance, vec![(99.9, 1.0)], vec![(100.0, 1.0)]);
        let slip = estimate_slippage(&ob, Side::Buy, 1000.0).unwrap();
        assert!(slip.depth_consumed_usd < 1000.0);
    }

    #[test]
    fn test_eqs_tighter_spread_wins() {
        let spreads = vec![
            SpreadSnapshot {
                venue: Venue::Phoenix,
                pair: TradingPair::SolPerp,
                spread_bps: 1.0,
                bid_price: 0.0,
                ask_price: 0.0,
                mid_price: 100.0,
                spread_absolute: 0.0,
                timestamp_ms: 0,
            },
            SpreadSnapshot {
                venue: Venue::Hyperliquid,
                pair: TradingPair::SolPerp,
                spread_bps: 5.0,
                bid_price: 0.0,
                ask_price: 0.0,
                mid_price: 100.0,
                spread_absolute: 0.0,
                timestamp_ms: 0,
            },
        ];
        let scores = compute_eqs(&spreads, &[], &[], TradingPair::SolPerp);
        let phoenix = scores.iter().find(|s| s.venue == Venue::Phoenix).unwrap();
        let hl = scores
            .iter()
            .find(|s| s.venue == Venue::Hyperliquid)
            .unwrap();
        assert!(phoenix.spread_score > hl.spread_score);
        // Phoenix: (1 - 1/5)*100 = 80, HL: (1 - 5/5)*100 = 0
        assert!((phoenix.spread_score - 80.0).abs() < 0.01);
        assert!((hl.spread_score - 0.0).abs() < 0.01);
    }
}
