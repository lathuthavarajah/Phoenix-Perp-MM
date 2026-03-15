import { useEffect, useState } from 'react';

interface FundingHistoryPoint {
  time: number;
  rate: number;
  venue: string;
}

export function useFundingHistory(coin: string, days: number) {
  const [data, setData] = useState<FundingHistoryPoint[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    async function fetchBinance() {
      try {
        const symbol = coin === 'SOL' ? 'SOLUSDT' : 'BTCUSDT';
        const limit = Math.min(days * 3, 500); // 3 funding events per day
        const res = await fetch(
          `https://fapi.binance.com/fapi/v1/fundingRate?symbol=${symbol}&limit=${limit}`,
        );
        const json = await res.json();
        if (!cancelled && Array.isArray(json)) {
          const points: FundingHistoryPoint[] = json.map(
            (item: { fundingTime: number; fundingRate: string }) => ({
              time: item.fundingTime,
              rate: parseFloat(item.fundingRate) / 8, // convert 8h to hourly
              venue: 'binance',
            }),
          );
          setData(points);
        }
      } catch {
        // Silently fail — funding history is supplementary
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    fetchBinance();
    const interval = setInterval(fetchBinance, 5 * 60_000); // refresh every 5min

    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [coin, days]);

  return { data, loading };
}
