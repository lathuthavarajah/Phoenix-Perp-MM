import { useMemo } from 'react';
import type { Venue, Orderbook, SlippageEstimate } from '../types/market';
import { estimateSlippage } from '../lib/metrics';

export function useSlippage(
  orderbooks: Partial<Record<Venue, Orderbook>>,
  side: 'buy' | 'sell',
  notionalUsd: number,
): Partial<Record<Venue, SlippageEstimate>> {
  return useMemo(() => {
    const result: Partial<Record<Venue, SlippageEstimate>> = {};
    for (const [venue, ob] of Object.entries(orderbooks)) {
      if (ob) {
        const est = estimateSlippage(ob, side, notionalUsd);
        if (est) result[venue as Venue] = est;
      }
    }
    return result;
  }, [orderbooks, side, notionalUsd]);
}
