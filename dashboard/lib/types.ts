export type Side = "Buy" | "Sell";

export interface BasisPosition {
  spot_size: number;
  perp_size: number;
  spot_entry_price: number;
  perp_entry_price: number;
  collateral_usdc: number;
  is_open: boolean;
  opened_at_ts: number;
}

export interface MarginState {
  collateral_usdc: number;
  unrealized_pnl: number;
  total_equity: number;
  maintenance_margin: number;
  margin_ratio: number;
  liquidation_price: number;
  distance_to_liq_pct: number;
}

export interface VenueFundingRate {
  venue: string;
  symbol: string;
  rate_8h: number;
  annualized_apy: number;
  mark_price: number;
  index_price: number;
  open_interest_long: number;
  open_interest_short: number;
  fetched_at_ts: number;
  is_simulated: boolean;
}

export type FundingSignal =
  | { type: "Enter"; funding_rate_8h: number; annualized_apy: number; reason: string }
  | { type: "Exit"; funding_rate_8h: number; annualized_apy: number; reason: string }
  | { type: "Hold" };

export type EngineMode = "Paper" | "Devnet" | "Mainnet";

export interface EngineState {
  position: BasisPosition | null;
  margin: MarginState | null;
  current_funding: VenueFundingRate[];
  sol_price: number;
  last_signal: FundingSignal | null;
  uptime_seconds: number;
  alerts: string[];
  mode: EngineMode;
}

export interface FundingRateResponse {
  venues: VenueFundingRate[];
  fetchedAt: number;
}
