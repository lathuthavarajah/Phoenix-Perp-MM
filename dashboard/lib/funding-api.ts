import { FundingRateResponse } from "./types";

export async function fetchFundingRates(): Promise<FundingRateResponse> {
  const res = await fetch("/api/funding", { cache: "no-store" });
  if (!res.ok) throw new Error(`Funding API error: ${res.status}`);
  return res.json();
}
