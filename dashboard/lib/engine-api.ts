import { EngineState } from "./types";

const ENGINE_BASE = "/api/engine";

export async function fetchEngineState(): Promise<EngineState> {
  const res = await fetch(ENGINE_BASE, { cache: "no-store" });
  if (!res.ok) throw new Error(`Engine API error: ${res.status}`);
  return res.json();
}

export async function openPosition(): Promise<{ success: boolean; error?: string }> {
  const res = await fetch(`${ENGINE_BASE}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action: "open" }),
  });
  return res.json();
}

export async function closePosition(): Promise<{ success: boolean; error?: string }> {
  const res = await fetch(`${ENGINE_BASE}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action: "close" }),
  });
  return res.json();
}
