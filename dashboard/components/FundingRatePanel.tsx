"use client";

import useSWR from "swr";
import clsx from "clsx";
import { fetchFundingRates } from "@/lib/funding-api";
import { VenueFundingRate } from "@/lib/types";

function apyColor(apy: number): string {
  if (apy > 0.15) return "text-green-400";
  if (apy > 0.02) return "text-yellow-400";
  return "text-red-400";
}

function formatRate(rate: number): string {
  return (rate * 100).toFixed(4) + "%";
}

function formatApy(apy: number): string {
  return (apy * 100).toFixed(2) + "%";
}

function formatPrice(price: number): string {
  if (price === 0) return "-";
  return "$" + price.toFixed(2);
}

function formatOI(oi: number): string {
  if (oi === 0) return "-";
  if (oi >= 1_000_000) return (oi / 1_000_000).toFixed(1) + "M";
  if (oi >= 1_000) return (oi / 1_000).toFixed(0) + "K";
  return oi.toFixed(0);
}

function VenueRow({ venue }: { venue: VenueFundingRate }) {
  return (
    <tr className="border-b border-gray-800 hover:bg-gray-800/50">
      <td className="py-3 px-4 font-medium">
        {venue.venue}
        {venue.is_simulated && (
          <span className="ml-2 px-1.5 py-0.5 text-[10px] font-bold bg-yellow-500/20 text-yellow-400 rounded">
            SIMULATED
          </span>
        )}
      </td>
      <td className="py-3 px-4 font-mono text-sm">{formatRate(venue.rate_8h)}</td>
      <td className={clsx("py-3 px-4 font-mono text-sm font-bold", apyColor(venue.annualized_apy))}>
        {formatApy(venue.annualized_apy)}
      </td>
      <td className="py-3 px-4 font-mono text-sm">{formatPrice(venue.mark_price)}</td>
      <td className="py-3 px-4 font-mono text-sm text-gray-400">{formatOI(venue.open_interest_long)}</td>
      <td className="py-3 px-4 font-mono text-sm text-gray-400">{formatOI(venue.open_interest_short)}</td>
    </tr>
  );
}

export default function FundingRatePanel() {
  const { data, error } = useSWR("funding", fetchFundingRates, {
    refreshInterval: 15000,
  });

  return (
    <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
      <div className="px-4 py-3 border-b border-gray-800">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">
          Live Funding Rates
        </h2>
      </div>
      {error ? (
        <div className="p-4 text-red-400 text-sm">Failed to fetch funding rates</div>
      ) : !data ? (
        <div className="p-4 text-gray-500 text-sm">Loading...</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left">
            <thead>
              <tr className="text-xs text-gray-500 uppercase border-b border-gray-800">
                <th className="py-2 px-4">Venue</th>
                <th className="py-2 px-4">8h Rate</th>
                <th className="py-2 px-4">Ann. APY</th>
                <th className="py-2 px-4">Mark Price</th>
                <th className="py-2 px-4">OI Long</th>
                <th className="py-2 px-4">OI Short</th>
              </tr>
            </thead>
            <tbody>
              {data.venues.map((v) => (
                <VenueRow key={v.venue} venue={v} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
