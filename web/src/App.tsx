import { useState } from 'react';
import type { TradingPair } from './types/market';
import { useMarketFeed } from './hooks/useMarketFeed';
import { LiveIndicator } from './components/LiveIndicator';
import { VenueSelector } from './components/VenueSelector';
import { SpreadComparison } from './components/SpreadComparison';
import { SlippageSimulator } from './components/SlippageSimulator';
import { FundingRateChart } from './components/FundingRateChart';
import { ExecutionQualityScorePanel } from './components/ExecutionQualityScore';

function App() {
  const [pair, setPair] = useState<TradingPair>('SOL-PERP');
  const { orderbooks, funding, scores, connected } = useMarketFeed(pair);

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100">
      {/* Header */}
      <header className="border-b border-zinc-800 px-6 py-4">
        <div className="mx-auto flex max-w-7xl items-center justify-between">
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-bold tracking-tight">
              <span className="text-orange-500">Phoenix</span> Execution Quality
            </h1>
            <LiveIndicator connected={connected} />
          </div>
          <VenueSelector pair={pair} onPairChange={setPair} />
        </div>
      </header>

      {/* Main content */}
      <main className="mx-auto max-w-7xl space-y-6 px-6 py-6">
        {/* EQS Gauges — most important, top of page */}
        <ExecutionQualityScorePanel scores={scores} />

        {/* Two-column layout for spread + slippage */}
        <div className="grid gap-6 lg:grid-cols-2">
          <SpreadComparison orderbooks={orderbooks} />
          <SlippageSimulator orderbooks={orderbooks} />
        </div>

        {/* Funding rate chart — full width */}
        <FundingRateChart funding={funding} pair={pair} />
      </main>

      {/* Footer */}
      <footer className="border-t border-zinc-800 px-6 py-4 text-center text-xs text-zinc-600">
        Phoenix Perps Execution Quality Tracker — Real-time benchmarking against Hyperliquid, Drift, Binance
      </footer>
    </div>
  );
}

export default App;
