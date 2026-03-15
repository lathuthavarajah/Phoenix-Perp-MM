"use client";

import useSWR from "swr";
import { fetchFundingRates } from "@/lib/funding-api";

interface MetricCardProps {
  label: string;
  value: string;
  subtitle?: string;
  simulated?: boolean;
}

function MetricCard({ label, value, subtitle, simulated }: MetricCardProps) {
  return (
    <div className="bg-gray-800/50 rounded-lg p-4">
      <p className="text-xs text-gray-500 uppercase mb-1">
        {label}
        {simulated && (
          <span className="ml-1 text-[9px] text-yellow-500">(simulated)</span>
        )}
      </p>
      <p className="text-lg font-mono font-bold text-gray-200">{value}</p>
      {subtitle && <p className="text-xs text-gray-500 mt-1">{subtitle}</p>}
    </div>
  );
}

export default function MarketHealthPanel() {
  const { data } = useSWR("funding", fetchFundingRates, {
    refreshInterval: 15000,
  });

  const phoenix = data?.venues.find((v) => v.venue === "Phoenix Perps");

  if (!phoenix) {
    return (
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4">
          Market Health
        </h2>
        <p className="text-gray-500 text-sm">Loading...</p>
      </div>
    );
  }

  const totalOI = phoenix.open_interest_long + phoenix.open_interest_short;
  const oiImbalance =
    totalOI > 0
      ? ((phoenix.open_interest_long - phoenix.open_interest_short) / totalOI) * 100
      : 0;
  const premiumBps =
    phoenix.index_price > 0
      ? ((phoenix.mark_price - phoenix.index_price) / phoenix.index_price) * 10000
      : 0;

  return (
    <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
      <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4">
        Market Health
      </h2>
      <div className="grid grid-cols-2 gap-3">
        <MetricCard
          label="OI Imbalance"
          value={`${oiImbalance >= 0 ? "+" : ""}${oiImbalance.toFixed(1)}%`}
          subtitle={oiImbalance > 0 ? "More longs (funding likely +)" : "More shorts"}
          simulated
        />
        <MetricCard
          label="Premium"
          value={`${premiumBps >= 0 ? "+" : ""}${premiumBps.toFixed(1)} bps`}
          subtitle="Mark vs Oracle spread"
          simulated
        />
        <MetricCard
          label="Mark Price"
          value={`$${phoenix.mark_price.toFixed(2)}`}
          simulated
        />
        <MetricCard
          label="Oracle Price"
          value={`$${phoenix.index_price.toFixed(2)}`}
          subtitle="Pyth SOL/USD"
        />
      </div>
    </div>
  );
}
