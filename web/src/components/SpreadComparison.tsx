import { useMemo } from 'react';
import type { Venue, Orderbook } from '../types/market';
import { VENUES, VENUE_LABELS } from '../types/market';
import { computeSpread } from '../lib/metrics';
import { formatBps, formatPrice } from '../lib/formatters';

interface Props {
  orderbooks: Partial<Record<Venue, Orderbook>>;
}

export function SpreadComparison({ orderbooks }: Props) {
  const spreads = useMemo(() => {
    return VENUES.map((v) => {
      const ob = orderbooks[v];
      return { venue: v, spread: ob ? computeSpread(ob) : null };
    });
  }, [orderbooks]);

  const tightest = useMemo(() => {
    let min = Infinity;
    let best: Venue | null = null;
    for (const s of spreads) {
      if (s.spread && s.spread.spread_bps < min) {
        min = s.spread.spread_bps;
        best = s.venue;
      }
    }
    return best;
  }, [spreads]);

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-5">
      <h2 className="mb-4 text-lg font-semibold text-zinc-100">
        Live Spread Comparison
      </h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-zinc-800 text-zinc-400">
              <th className="pb-2 text-left font-medium">Venue</th>
              <th className="pb-2 text-right font-medium">Best Bid</th>
              <th className="pb-2 text-right font-medium">Best Ask</th>
              <th className="pb-2 text-right font-medium">Spread</th>
              <th className="pb-2 text-right font-medium">Spread ($)</th>
            </tr>
          </thead>
          <tbody>
            {spreads.map(({ venue, spread }) => (
              <tr
                key={venue}
                className={`border-b border-zinc-800/50 ${
                  venue === 'phoenix' ? 'bg-orange-950/20' : ''
                }`}
              >
                <td className="py-3 text-left">
                  <span
                    className={`font-medium ${
                      venue === 'phoenix' ? 'text-orange-400' : 'text-zinc-200'
                    }`}
                  >
                    {VENUE_LABELS[venue]}
                  </span>
                  {venue === 'phoenix' && !spread && (
                    <span className="ml-2 rounded bg-yellow-900/50 px-1.5 py-0.5 text-[10px] text-yellow-400">
                      CONNECTING
                    </span>
                  )}
                </td>
                <td className="py-3 text-right font-mono text-zinc-300">
                  {spread ? formatPrice(spread.bid_price) : '-'}
                </td>
                <td className="py-3 text-right font-mono text-zinc-300">
                  {spread ? formatPrice(spread.ask_price) : '-'}
                </td>
                <td className="py-3 text-right font-mono">
                  <span
                    className={
                      spread
                        ? venue === tightest
                          ? 'text-green-400'
                          : 'text-zinc-300'
                        : 'text-zinc-600'
                    }
                  >
                    {spread ? formatBps(spread.spread_bps) : '-'}
                  </span>
                </td>
                <td className="py-3 text-right font-mono text-zinc-400">
                  {spread ? `$${spread.spread_absolute.toFixed(4)}` : '-'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
