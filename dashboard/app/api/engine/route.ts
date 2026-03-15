import { NextRequest, NextResponse } from "next/server";
import { EngineState } from "@/lib/types";

const ENGINE_URL = process.env.NEXT_PUBLIC_ENGINE_URL || "http://localhost:8080";

function mockEngineState(): EngineState {
  return {
    position: null,
    margin: null,
    current_funding: [],
    sol_price: 135.42,
    last_signal: { type: "Hold" },
    uptime_seconds: Math.floor(Date.now() / 1000) % 86400,
    alerts: [],
    mode: "Paper",
  };
}

export async function GET() {
  try {
    const res = await fetch(`${ENGINE_URL}/state`, {
      signal: AbortSignal.timeout(2000),
    });
    if (!res.ok) throw new Error(`Engine returned ${res.status}`);
    const data = await res.json();
    return NextResponse.json(data, {
      headers: { "Cache-Control": "no-store" },
    });
  } catch {
    // Engine not running — return mock state
    return NextResponse.json(mockEngineState(), {
      headers: { "Cache-Control": "no-store" },
    });
  }
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const action = body.action as string;

  try {
    const res = await fetch(`${ENGINE_URL}/${action}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      signal: AbortSignal.timeout(5000),
    });
    if (!res.ok) throw new Error(`Engine returned ${res.status}`);
    const data = await res.json();
    return NextResponse.json(data);
  } catch {
    return NextResponse.json(
      { success: false, error: "Engine not connected" },
      { status: 503 }
    );
  }
}
