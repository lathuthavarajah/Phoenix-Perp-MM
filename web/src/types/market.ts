export type Venue = 'phoenix' | 'hyperliquid' | 'drift' | 'binance';
export type TradingPair = 'SOL-PERP' | 'BTC-PERP';

export interface OrderbookLevel {
  price: number;
  size: number;
  notional: number;
}

export interface Orderbook {
  venue: Venue;
  pair: TradingPair;
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  timestamp_ms: number;
}

export interface SpreadSnapshot {
  venue: Venue;
  pair: TradingPair;
  bid_price: number;
  ask_price: number;
  mid_price: number;
  spread_absolute: number;
  spread_bps: number;
  timestamp_ms: number;
}

export interface FundingRateSnapshot {
  venue: Venue;
  pair: TradingPair;
  rate_hourly: number;
  rate_annualized: number;
  next_funding_ms: number;
  timestamp_ms: number;
}

export interface SlippageEstimate {
  venue: Venue;
  pair: TradingPair;
  side: 'buy' | 'sell';
  notional_usd: number;
  avg_fill_price: number;
  mid_price: number;
  slippage_bps: number;
  depth_consumed_usd: number;
  timestamp_ms: number;
}

export interface ExecutionQualityScore {
  venue: Venue;
  pair: TradingPair;
  spread_score: number;
  depth_score: number;
  funding_score: number;
  composite_score: number;
  timestamp_ms: number;
}

export type WsMsg =
  | { type: 'orderbook'; data: Orderbook }
  | { type: 'funding'; data: FundingRateSnapshot }
  | { type: 'scores'; data: ExecutionQualityScore[] };

export const VENUES: Venue[] = ['phoenix', 'hyperliquid', 'drift', 'binance'];

export const VENUE_LABELS: Record<Venue, string> = {
  phoenix: 'Phoenix',
  hyperliquid: 'Hyperliquid',
  drift: 'Drift',
  binance: 'Binance',
};

export const VENUE_COLORS: Record<Venue, string> = {
  phoenix: '#f97316',
  hyperliquid: '#06b6d4',
  drift: '#a855f7',
  binance: '#eab308',
};
