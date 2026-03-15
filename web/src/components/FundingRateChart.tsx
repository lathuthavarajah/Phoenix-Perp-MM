import { useState } from 'react';
import {
  BarChart,
  Bar,
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
  CartesianGrid,
} from 'recharts';
import type { Venue, FundingRateSnapshot, TradingPair } from '../types/market';
import { VENUES, VENUE_LABELS, VENUE_COLORS } from '../types/market';
import { useFundingHistory } from '../hooks/useFundingHistory';

interface Props {
  funding: Partial<Record<Venue, FundingRateSnapshot>>;
  pair: TradingPair;
}

type Tab = 'live' | '7d' | '30d';

export function FundingRateChart({ funding, pair }: Props) {
  const [tab, setTab] = useState<Tab>('live');
  const coin = pair === 'SOL-PERP' ? 'SOL' : 'BTC';
  const days = tab === '7d' ? 7 : 30;
  const { data: histData, loading } = useFundingHistory(coin, days);

  const liveData = VENUES.filter((v) => funding[v]).map((v) => ({
    venue: VENUE_LABELS[v],
    venueKey: v,
    annualized: funding[v]!.rate_annualized * 100,
    hourly: funding[v]!.rate_hourly * 100,
  }));

  const histChart = histData.map((p) => ({
    time: new Date(p.time).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }),
    rate: p.rate * 100 * 8760, // annualize
  }));

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-5">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold text-zinc-100">Funding Rates</h2>
        <div className="flex gap-1">
          {(['live', '7d', '30d'] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`rounded-md px-3 py-1 text-xs font-medium transition-colors ${
                tab === t
                  ? 'bg-zinc-600 text-white'
                  : 'bg-zinc-800 text-zinc-400 hover:bg-zinc-700'
              }`}
            >
              {t === 'live' ? 'Live' : t.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      <div className="h-56">
        {tab === 'live' ? (
          liveData.length > 0 ? (
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={liveData}>
                <XAxis
                  dataKey="venue"
                  tick={{ fill: '#d4d4d8', fontSize: 12 }}
                />
                <YAxis
                  tick={{ fill: '#a1a1aa', fontSize: 11 }}
                  tickFormatter={(v: number) => `${v.toFixed(1)}%`}
                />
                <Tooltip
                  contentStyle={{
                    background: '#18181b',
                    border: '1px solid #3f3f46',
                    borderRadius: 8,
                    fontSize: 12,
                  }}
                  // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  formatter={(v: any) => [`${Number(v).toFixed(3)}% annualized`, 'Rate']}
                />
                <Bar dataKey="annualized" radius={[4, 4, 0, 0]}>
                  {liveData.map((d) => (
                    <Cell
                      key={d.venueKey}
                      fill={
                        d.annualized >= 0
                          ? VENUE_COLORS[d.venueKey as Venue]
                          : '#ef4444'
                      }
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex h-full items-center justify-center text-zinc-500">
              Waiting for funding data...
            </div>
          )
        ) : loading ? (
          <div className="flex h-full items-center justify-center text-zinc-500">
            Loading historical data...
          </div>
        ) : (
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={histChart}>
              <CartesianGrid stroke="#27272a" />
              <XAxis
                dataKey="time"
                tick={{ fill: '#a1a1aa', fontSize: 10 }}
                interval="preserveStartEnd"
              />
              <YAxis
                tick={{ fill: '#a1a1aa', fontSize: 11 }}
                tickFormatter={(v: number) => `${v.toFixed(1)}%`}
              />
              <Tooltip
                contentStyle={{
                  background: '#18181b',
                  border: '1px solid #3f3f46',
                  borderRadius: 8,
                  fontSize: 12,
                }}
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                formatter={(v: any) => [`${Number(v).toFixed(3)}%`, 'Ann. Rate']}
              />
              <Line
                type="monotone"
                dataKey="rate"
                stroke="#eab308"
                dot={false}
                strokeWidth={1.5}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </div>
    </div>
  );
}
