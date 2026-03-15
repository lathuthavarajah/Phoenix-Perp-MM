"use client";

import { useState } from "react";
import clsx from "clsx";
import { openPosition, closePosition } from "@/lib/engine-api";
import { EngineState } from "@/lib/types";

interface Props {
  engine: EngineState;
  onAction: () => void;
}

export default function EngineControls({ engine, onAction }: Props) {
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);

  const hasPosition = engine.position?.is_open ?? false;

  async function handleOpen() {
    setLoading(true);
    setMessage(null);
    try {
      const res = await openPosition();
      if (res.success) {
        setMessage({ text: "Position opened", type: "success" });
      } else {
        setMessage({ text: res.error || "Failed to open", type: "error" });
      }
      onAction();
    } catch {
      setMessage({ text: "Engine not connected", type: "error" });
    } finally {
      setLoading(false);
      setTimeout(() => setMessage(null), 5000);
    }
  }

  async function handleClose() {
    setLoading(true);
    setMessage(null);
    try {
      const res = await closePosition();
      if (res.success) {
        setMessage({ text: "Position closed", type: "success" });
      } else {
        setMessage({ text: res.error || "Failed to close", type: "error" });
      }
      onAction();
    } catch {
      setMessage({ text: "Engine not connected", type: "error" });
    } finally {
      setLoading(false);
      setTimeout(() => setMessage(null), 5000);
    }
  }

  const modeBadgeColor = {
    Paper: "bg-blue-500/20 text-blue-400",
    Devnet: "bg-purple-500/20 text-purple-400",
    Mainnet: "bg-green-500/20 text-green-400",
  }[engine.mode];

  return (
    <div className="bg-gray-900 rounded-xl border border-gray-800 p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">
          Engine Controls
        </h2>
        <span className={clsx("text-[10px] px-2 py-0.5 rounded font-bold", modeBadgeColor)}>
          {engine.mode.toUpperCase()}
        </span>
      </div>

      <div className="flex gap-3">
        <button
          onClick={handleOpen}
          disabled={hasPosition || loading}
          className={clsx(
            "flex-1 py-2 px-4 rounded-lg font-medium text-sm transition-colors",
            hasPosition || loading
              ? "bg-gray-800 text-gray-600 cursor-not-allowed"
              : "bg-green-600 hover:bg-green-500 text-white"
          )}
        >
          {loading ? "..." : "Open Position"}
        </button>
        <button
          onClick={handleClose}
          disabled={!hasPosition || loading}
          className={clsx(
            "flex-1 py-2 px-4 rounded-lg font-medium text-sm transition-colors",
            !hasPosition || loading
              ? "bg-gray-800 text-gray-600 cursor-not-allowed"
              : "bg-red-600 hover:bg-red-500 text-white"
          )}
        >
          {loading ? "..." : "Close Position"}
        </button>
      </div>

      {message && (
        <p
          className={clsx(
            "mt-3 text-xs text-center",
            message.type === "success" ? "text-green-400" : "text-red-400"
          )}
        >
          {message.text}
        </p>
      )}

      <div className="mt-4 text-xs text-gray-600">
        SOL: ${engine.sol_price.toFixed(2)} | Uptime: {Math.floor(engine.uptime_seconds / 60)}m
      </div>
    </div>
  );
}
