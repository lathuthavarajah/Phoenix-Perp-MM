"use client";

import useSWR from "swr";
import { fetchEngineState } from "@/lib/engine-api";
import AlertBanner from "@/components/AlertBanner";
import FundingRatePanel from "@/components/FundingRatePanel";
import PositionDashboard from "@/components/PositionDashboard";
import RiskGauge from "@/components/RiskGauge";
import FundingChart from "@/components/FundingChart";
import MarketHealthPanel from "@/components/MarketHealthPanel";
import EngineControls from "@/components/EngineControls";

export default function Home() {
  const { data: engine, mutate } = useSWR("engine", fetchEngineState, {
    refreshInterval: 3000,
  });

  return (
    <div className="min-h-screen flex flex-col">
      {/* Alert Banner */}
      {engine && <AlertBanner alerts={engine.alerts} />}

      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="max-w-7xl mx-auto flex items-center justify-between">
          <div>
            <h1 className="text-xl font-bold tracking-tight">
              Phoenix Arb Engine
            </h1>
            <p className="text-xs text-gray-500">
              Delta-neutral funding rate arbitrage
            </p>
          </div>
          <div className="flex items-center gap-3 text-xs text-gray-500">
            <span className="flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
              Live
            </span>
            {engine && (
              <span className="font-mono">
                SOL ${engine.sol_price.toFixed(2)}
              </span>
            )}
          </div>
        </div>
      </header>

      {/* Main Grid */}
      <main className="flex-1 max-w-7xl mx-auto w-full p-6">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Left column - spans 2 */}
          <div className="lg:col-span-2 space-y-6">
            <FundingRatePanel />
            <FundingChart />
          </div>

          {/* Right column */}
          <div className="space-y-6">
            {engine && (
              <>
                <EngineControls engine={engine} onAction={() => mutate()} />
                <PositionDashboard engine={engine} />
                <RiskGauge margin={engine.margin} />
              </>
            )}
            <MarketHealthPanel />
          </div>
        </div>
      </main>
    </div>
  );
}
