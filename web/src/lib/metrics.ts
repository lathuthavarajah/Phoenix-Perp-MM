import type { Orderbook, SpreadSnapshot, SlippageEstimate, Venue, TradingPair } from '../types/market';

export function computeSpread(ob: Orderbook): SpreadSnapshot | null {
  if (!ob.bids.length || !ob.asks.length) return null;

  const bestBid = ob.bids[0].price;
  const bestAsk = ob.asks[0].price;
  const mid = (bestBid + bestAsk) / 2;
  const spreadAbs = bestAsk - bestBid;
  const spreadBps = mid > 0 ? (spreadAbs / mid) * 10_000 : 0;

  return {
    venue: ob.venue,
    pair: ob.pair,
    bid_price: bestBid,
    ask_price: bestAsk,
    mid_price: mid,
    spread_absolute: spreadAbs,
    spread_bps: spreadBps,
    timestamp_ms: Date.now(),
  };
}

export function estimateSlippage(
  ob: Orderbook,
  side: 'buy' | 'sell',
  notionalUsd: number,
): SlippageEstimate | null {
  if (!ob.bids.length || !ob.asks.length) return null;

  const mid = (ob.bids[0].price + ob.asks[0].price) / 2;
  const levels = side === 'buy' ? ob.asks : ob.bids;

  let remaining = notionalUsd;
  let totalCost = 0;
  let totalSize = 0;

  for (const level of levels) {
    if (remaining <= 0) break;
    const levelUsd = Math.min(level.notional, remaining);
    const levelSize = levelUsd / level.price;
    totalCost += level.price * levelSize;
    totalSize += levelSize;
    remaining -= levelUsd;
  }

  if (totalSize === 0) return null;

  const avgFill = totalCost / totalSize;
  const slippageBps = mid > 0 ? (Math.abs(avgFill - mid) / mid) * 10_000 : 0;

  return {
    venue: ob.venue,
    pair: ob.pair,
    side,
    notional_usd: notionalUsd,
    avg_fill_price: avgFill,
    mid_price: mid,
    slippage_bps: slippageBps,
    depth_consumed_usd: notionalUsd - Math.max(remaining, 0),
    timestamp_ms: Date.now(),
  };
}

export function computeEqs(
  spreads: SpreadSnapshot[],
  slippages: SlippageEstimate[],
  _funding: unknown[],
  _pair: TradingPair,
): { venue: Venue; spread_score: number; depth_score: number; composite_score: number }[] {
  const maxSpread = Math.max(...spreads.map(s => s.spread_bps), 1);
  const slipVals = slippages.map(s => s.slippage_bps);
  const maxSlip = Math.max(...slipVals, 1);
  const minSlip = Math.min(...slipVals, maxSlip);
  const range = maxSlip - minSlip;

  return spreads.map(s => {
    const spreadScore = Math.max((1 - s.spread_bps / maxSpread) * 100, 0);
    const slip = slippages.find(sl => sl.venue === s.venue);
    const depthScore = slip
      ? range > 0
        ? Math.max((1 - (slip.slippage_bps - minSlip) / range) * 100, 0)
        : 100
      : 0;
    return {
      venue: s.venue,
      spread_score: spreadScore,
      depth_score: depthScore,
      composite_score: 0.4 * spreadScore + 0.4 * depthScore + 0.2 * 50,
    };
  });
}
