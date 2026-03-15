"use client";

import clsx from "clsx";
import { EngineState } from "@/lib/types";

function formatUsd(n: number): string {
  const sign = n >= 0 ? "+" : "";
  return sign + "$" + Math.abs(n).toFixed(2);
}

function formatSol(n: number): string {
  return n.toFixed(4) + " SOL";
}

function formatElapsed(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

interface Props {
  engine: EngineState;
}

export default function PositionDashboard({ engine }: Props) {
  const { position, sol_price } = engine;

  if (!position || !position.is_open) {
    return (
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4">
          Position
        </h2>
        <p className="text-gray-500">No active position</p>
        {engine.last_signal && engine.last_signal.type === "Enter" && (
          <p className="text-green-400 text-sm mt-2">
            Entry signal active: {engine.last_signal.reason}
          </p>
        )}
      </div>
    );
  }

  const spotPnl = position.spot_size * (sol_price - position.spot_entry_price);
  const perpPnl = position.perp_size * (sol_price - position.perp_entry_price);
  const totalPnl = spotPnl + perpPnl;
  const delta = position.spot_size + position.perp_size;
  const elapsed = Math.floor(Date.now() / 1000) - position.opened_at_ts;

  return (
    <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">
          Position
        </h2>
        <span className="text-[10px] px-2 py-0.5 bg-yellow-500/20 text-yellow-400 rounded font-bold">
          Perp data simulated
        </span>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {/* Spot Leg */}
        <div className="space-y-1">
          <p className="text-xs text-gray-500 uppercase">Spot Leg (Long)</p>
          <p className="font-mono text-sm">{formatSol(position.spot_size)}</p>
          <p className="text-xs text-gray-400">
            Entry: ${position.spot_entry_price.toFixed(2)}
          </p>
          <p className={clsx("text-xs font-mono", spotPnl >= 0 ? "text-green-400" : "text-red-400")}>
            PnL: {formatUsd(spotPnl)}
          </p>
        </div>

        {/* Perp Leg */}
        <div className="space-y-1">
          <p className="text-xs text-gray-500 uppercase">Perp Leg (Short)</p>
          <p className="font-mono text-sm">{formatSol(position.perp_size)}</p>
          <p className="text-xs text-gray-400">
            Entry: ${position.perp_entry_price.toFixed(2)}
          </p>
          <p className={clsx("text-xs font-mono", perpPnl >= 0 ? "text-green-400" : "text-red-400")}>
            PnL: {formatUsd(perpPnl)}
          </p>
        </div>

        {/* Delta */}
        <div className="space-y-1">
          <p className="text-xs text-gray-500 uppercase">Net Delta</p>
          <p className={clsx("font-mono text-sm font-bold", Math.abs(delta) < 0.01 ? "text-green-400" : "text-red-400")}>
            {delta.toFixed(4)} SOL
          </p>
        </div>

        {/* Total PnL */}
        <div className="space-y-1">
          <p className="text-xs text-gray-500 uppercase">Total PnL</p>
          <p className={clsx("font-mono text-sm font-bold", totalPnl >= 0 ? "text-green-400" : "text-red-400")}>
            {formatUsd(totalPnl)}
          </p>
        </div>

        {/* Collateral */}
        <div className="space-y-1">
          <p className="text-xs text-gray-500 uppercase">Collateral</p>
          <p className="font-mono text-sm">${position.collateral_usdc.toFixed(2)}</p>
        </div>

        {/* Time Open */}
        <div className="space-y-1">
          <p className="text-xs text-gray-500 uppercase">Time Open</p>
          <p className="font-mono text-sm">{formatElapsed(elapsed)}</p>
        </div>
      </div>
    </div>
  );
}
