# Phoenix Perps Funding Rate Arb Engine

A **delta-neutral funding rate arbitrage engine** targeting Ellipsis Labs' Phoenix products on Solana.

**Strategy**: Long SOL/USDC on Phoenix Legacy (spot LOB) + Short SOL-PERP on Phoenix Perpetuals, collecting funding payments when the perp funding rate is positive.

## Architecture

```
┌─────────────────────┐     ┌──────────────────────┐     ┌───────────────────┐
│   Next.js Dashboard │────▶│   Rust Engine (Axum)  │     │ Python Backtester │
│   localhost:3000    │◀────│   localhost:8080      │     │ Historical data   │
└─────────────────────┘     └──────────────────────┘     └───────────────────┘
        │                           │
        │ Live funding rates        │ Simulated Phoenix Perps
        ▼                           ▼
  Drift / Hyperliquid /       Pyth Oracle (real)
  Binance (real APIs)         Phoenix Legacy (real SDK)
```

## What's Real vs Simulated

| Component | Status | Notes |
|---|---|---|
| Drift/Hyperliquid/Binance funding rates | **Real** | Live REST API calls |
| Pyth SOL/USD oracle price | **Real** | On-chain price feed |
| Phoenix Legacy LOB | **Real SDK** | Can trade on devnet |
| Phoenix Perps trading | **Simulated** | Private beta, no public SDK |
| Phoenix Perps funding rate | **Simulated** | Derived from mark/index spread |

Phoenix Perps data is clearly tagged as `SIMULATED` in the dashboard. When the public SDK releases, the mock client is a drop-in replacement via the `PerpClientTrait`.

## Quick Start

### Prerequisites
- Rust (install via `rustup`)
- Node.js 18+
- Python 3.11+

### 1. Dashboard
```bash
cd dashboard
npm install
npm run dev
# Open http://localhost:3000
```

### 2. Rust Engine
```bash
cd engine
cargo run
# Serves state at http://localhost:8080
```

### 3. Python Backtester
```bash
cd backtester
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

python fetch_funding.py      # Download historical data
python simulate_basis.py     # Run backtest
python stress_test.py        # Crash scenarios
python report.py             # Generate PDF report
```

## Configuration

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
```

Key settings:
- `FUNDING_ENTRY_THRESHOLD_APY=0.15` — Enter when annualized funding > 15%
- `FUNDING_EXIT_THRESHOLD_APY=0.02` — Exit when funding < 2%
- `POSITION_SIZE_USDC=500` — Total notional per position
- `MAX_LEVERAGE=3` — Max perp leverage
- `EMERGENCY_CLOSE_RATIO=0.12` — Emergency close at 12% margin

## Testing

```bash
# Rust unit tests (14 tests)
cd engine && cargo test

# Dashboard build check
cd dashboard && npm run build
```

## Project Structure

```
├── dashboard/          Next.js frontend with live funding data
│   ├── app/api/        API routes (funding, engine proxy, history)
│   ├── components/     7 React components
│   └── lib/            TypeScript types and API clients
├── engine/             Rust execution engine
│   └── src/            10 modules + tests
├── backtester/         Python historical analysis
│   ├── notebooks/      Jupyter exploratory analysis
│   └── output/         Generated reports
├── .env.example        Environment variable template
└── README.md
```
