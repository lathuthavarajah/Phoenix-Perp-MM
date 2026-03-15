# Phoenix Perps Execution Quality Tracker

Live benchmark comparing Phoenix Perps execution quality against Hyperliquid,
Drift, and Binance — spread, depth, and funding cost in real time.

## What It Measures

| Metric | Description |
|--------|-------------|
| Spread (bps) | Best bid-ask across Phoenix, Hyperliquid, Drift, Binance |
| Slippage | Walk-the-book impact at $1k / $10k / $50k / $100k notional |
| Funding Rate | Live and 30d historical hourly rates, annualized |
| EQS | Composite Execution Quality Score (0-100) per venue |

## Architecture

```
Solana RPC ──────► Phoenix feed ──┐
Hyperliquid WS ───► HL feed ──────┤                     ┌── React (Vercel)
Drift DLOB API ───► Drift feed ───┼──► Axum + Tokio ───►│   TypeScript
Binance REST ─────► Binance feed ─┘   (Rust, Fly.io)    └── Recharts
```

**Backend**: Rust — Axum + Tokio + tokio-tungstenite
**Frontend**: React 18 + TypeScript + Vite + Recharts + Tailwind CSS
**Data**: Phoenix Legacy (Solana RPC), Hyperliquid WS, Drift DLOB, Binance Futures REST

## Quick Start

### Backend

```bash
cd server
cp .env.example .env
# Edit .env with your Solana RPC URL
cargo run
```

Server starts at `http://localhost:8080`. Health check: `GET /health`.

### Frontend

```bash
cd web
npm install
npm run dev
```

Dashboard at `http://localhost:5173`. Connects to backend WebSocket automatically.

### Run Tests

```bash
cd server && cargo test
```

## Note on Phoenix Data

Phoenix Perps is in private beta. The Phoenix panel uses Phoenix Legacy
spot orderbook data via direct Solana RPC account decoding as a live proxy.
The integration uses the same binary deserialization pattern the Phoenix Perps
SDK will expose publicly — swapping in Perps data requires a single adapter change.
