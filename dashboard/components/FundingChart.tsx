"use client";

import { useEffect, useRef, useState } from "react";
import useSWR from "swr";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ReferenceLine,
  ResponsiveContainer,
} from "recharts";
import { fetchFundingRates } from "@/lib/funding-api";

interface ChartPoint {
  time: string;
  timestamp: number;
  phoenix: number;
  drift: number;
  hyperliquid: number;
  binance: number;
}

const MAX_POINTS = 96; // 24h at 15s intervals

export default function FundingChart() {
  const [history, setHistory] = useState<ChartPoint[]>([]);
  const historyRef = useRef(history);
  historyRef.current = history;

  const { data } = useSWR("funding", fetchFundingRates, {
    refreshInterval: 15000,
  });

  useEffect(() => {
    if (!data) return;
    const now = new Date();
    const timeStr = now.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });

    const venueMap: Record<string, number> = {};
    for (const v of data.venues) {
      const key = v.venue.toLowerCase().replace(/\s+/g, "");
      venueMap[key] = v.annualized_apy * 100;
    }

    const point: ChartPoint = {
      time: timeStr,
      timestamp: Date.now(),
      phoenix: venueMap["phoenixperps"] ?? 0,
      drift: venueMap["drift"] ?? 0,
      hyperliquid: venueMap["hyperliquid"] ?? 0,
      binance: venueMap["binance"] ?? 0,
    };

    setHistory((prev) => {
      const next = [...prev, point];
      return next.length > MAX_POINTS ? next.slice(-MAX_POINTS) : next;
    });
  }, [data]);

  return (
    <div className="bg-gray-900 rounded-xl border border-gray-800 p-4">
      <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-4">
        Funding Rate History (Annualized %)
      </h2>
      {history.length < 2 ? (
        <div className="h-[250px] flex items-center justify-center text-gray-500 text-sm">
          Collecting data points... ({history.length}/2 minimum)
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={250}>
          <LineChart data={history}>
            <CartesianGrid strokeDasharray="3 3" stroke="#1f2937" />
            <XAxis dataKey="time" tick={{ fill: "#6b7280", fontSize: 10 }} />
            <YAxis tick={{ fill: "#6b7280", fontSize: 10 }} unit="%" />
            <Tooltip
              contentStyle={{
                backgroundColor: "#111827",
                border: "1px solid #374151",
                borderRadius: "8px",
                fontSize: 12,
              }}
            />
            <Legend wrapperStyle={{ fontSize: 11 }} />
            <ReferenceLine
              y={15}
              stroke="#ef4444"
              strokeDasharray="5 5"
              label={{ value: "Entry 15%", fill: "#ef4444", fontSize: 10 }}
            />
            <ReferenceLine
              y={2}
              stroke="#eab308"
              strokeDasharray="5 5"
              label={{ value: "Exit 2%", fill: "#eab308", fontSize: 10 }}
            />
            <Line
              type="monotone"
              dataKey="phoenix"
              stroke="#f97316"
              strokeDasharray="5 5"
              dot={false}
              name="Phoenix (sim)"
            />
            <Line type="monotone" dataKey="drift" stroke="#3b82f6" dot={false} name="Drift" />
            <Line type="monotone" dataKey="hyperliquid" stroke="#a855f7" dot={false} name="Hyperliquid" />
            <Line type="monotone" dataKey="binance" stroke="#22c55e" dot={false} name="Binance" />
          </LineChart>
        </ResponsiveContainer>
      )}
    </div>
  );
}
