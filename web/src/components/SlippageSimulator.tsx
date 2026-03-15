import { useState } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import type { Venue, Orderbook } from '../types/market';
import { VENUES, VENUE_LABELS, VENUE_COLORS } from '../types/market';
import { useSlippage } from '../hooks/useSlippage';
import { formatBps } from '../lib/formatters';

interface Props {
  orderbooks: Partial<Record<Venue, Orderbook>>;
}

const SIZES = [1_000, 10_000, 50_000, 100_000];

export function SlippageSimulator({ orderbooks }: Props) {
  const [size, setSize] = useState(10_000);
  const [side, setSide] = useState<'buy' | 'sell'>('buy');
  const slippages = useSlippage(orderbooks, side, size);

  const chartData = VENUES.filter((v) => slippages[v]).map((v) => ({
    venue: VENUE_LABELS[v],
    venueKey: v,
    slippage: slippages[v]!.slippage_bps,
    avgFill: slippages[v]!.avg_fill_price,
    depth: slippages[v]!.depth_consumed_usd,
  }));

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-5">
      <h2 className="mb-4 text-lg font-semibold text-zinc-100">
        Slippage Simulator
      </h2>

      <div className="mb-4 flex flex-wrap gap-2">
        <div className="flex gap-1">
          {(['buy', 'sell'] as const).map((s) => (
            <button
              key={s}
              onClick={() => setSide(s)}
              className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                side === s
                  ? s === 'buy'
                    ? 'bg-green-700 text-white'
                    : 'bg-red-700 text-white'
                  : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700'
              }`}
            >
              {s.toUpperCase()}
            </button>
          ))}
        </div>
        <div className="flex gap-1">
          {SIZES.map((s) => (
            <button
              key={s}
              onClick={() => setSize(s)}
              className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                size === s
                  ? 'bg-zinc-600 text-white'
                  : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700'
              }`}
            >
              ${s >= 1000 ? `${s / 1000}k` : s}
            </button>
          ))}
        </div>
      </div>

      <div className="h-56">
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData} layout="vertical">
              <XAxis
                type="number"
                tick={{ fill: '#a1a1aa', fontSize: 11 }}
                tickFormatter={(v: number) => formatBps(v)}
              />
              <YAxis
                type="category"
                dataKey="venue"
                tick={{ fill: '#d4d4d8', fontSize: 12 }}
                width={90}
              />
              <Tooltip
                contentStyle={{
                  background: '#18181b',
                  border: '1px solid #3f3f46',
                  borderRadius: 8,
                  fontSize: 12,
                }}
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                formatter={(value: any) => [
                  formatBps(Number(value)),
                  'Slippage',
                ]}
              />
              <Bar dataKey="slippage" radius={[0, 4, 4, 0]}>
                {chartData.map((d) => (
                  <Cell
                    key={d.venueKey}
                    fill={VENUE_COLORS[d.venueKey as Venue]}
                  />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex h-full items-center justify-center text-zinc-500">
            Waiting for orderbook data...
          </div>
        )}
      </div>
    </div>
  );
}
