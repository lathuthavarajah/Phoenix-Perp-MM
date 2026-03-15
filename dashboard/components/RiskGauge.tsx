"use client";

import clsx from "clsx";
import { MarginState } from "@/lib/types";

interface Props {
  margin: MarginState | null;
}

export default function RiskGauge({ margin }: Props) {
  if (!margin) {
    return (
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4">
          Risk / Margin
        </h2>
        <p className="text-gray-500 text-sm">No active position</p>
      </div>
    );
  }

  const ratio = margin.margin_ratio;
  const pct = Math.min(ratio * 100, 100);
  const isCritical = ratio < 0.12;
  const isWarning = ratio < 0.25;

  const barColor = isCritical
    ? "bg-red-500"
    : isWarning
    ? "bg-yellow-500"
    : "bg-green-500";

  const statusText = isCritical
    ? "CRITICAL"
    : isWarning
    ? "WARNING"
    : "SAFE";

  const statusColor = isCritical
    ? "text-red-400"
    : isWarning
    ? "text-yellow-400"
    : "text-green-400";

  return (
    <div
      className={clsx(
        "bg-gray-900 rounded-xl border p-6",
        isCritical ? "border-red-500 animate-pulse" : "border-gray-800"
      )}
    >
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">
          Risk / Margin
        </h2>
        <span className={clsx("text-xs font-bold", statusColor)}>
          {statusText}
        </span>
      </div>

      {/* Bar */}
      <div className="relative h-4 bg-gray-800 rounded-full overflow-hidden mb-2">
        {/* Zone markers */}
        <div className="absolute inset-0 flex">
          <div className="w-[12%] bg-red-900/30 border-r border-gray-700" />
          <div className="w-[13%] bg-yellow-900/20 border-r border-gray-700" />
          <div className="flex-1 bg-green-900/10" />
        </div>
        {/* Fill */}
        <div
          className={clsx("absolute inset-y-0 left-0 rounded-full transition-all duration-500", barColor)}
          style={{ width: `${pct}%` }}
        />
      </div>

      <div className="flex justify-between text-[10px] text-gray-600 mb-4">
        <span>0%</span>
        <span>12%</span>
        <span>25%</span>
        <span>100%</span>
      </div>

      <div className="grid grid-cols-2 gap-3 text-sm">
        <div>
          <p className="text-xs text-gray-500">Margin Ratio</p>
          <p className={clsx("font-mono font-bold", statusColor)}>
            {(ratio * 100).toFixed(1)}%
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-500">Liq. Distance</p>
          <p className="font-mono text-gray-300">
            {(margin.distance_to_liq_pct * 100).toFixed(1)}% move
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-500">Equity</p>
          <p className="font-mono text-gray-300">${margin.total_equity.toFixed(2)}</p>
        </div>
        <div>
          <p className="text-xs text-gray-500">Liq. Price</p>
          <p className="font-mono text-gray-300">${margin.liquidation_price.toFixed(2)}</p>
        </div>
      </div>
    </div>
  );
}
