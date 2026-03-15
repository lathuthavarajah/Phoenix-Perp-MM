import { NextResponse } from "next/server";

export async function GET() {
  // Stub: will serve backtester results when available
  return NextResponse.json({
    available: false,
    message: "Backtester results not yet generated. Run: cd backtester && python simulate_basis.py",
    data: null,
  });
}
