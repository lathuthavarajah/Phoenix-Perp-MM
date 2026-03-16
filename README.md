# Phoenix Inventory-Aware Market Maker

Off-chain market-making bot for [Phoenix DEX](https://www.phoenix.trade/) on Solana, implementing Avellaneda-Stoikov inventory-skewed quoting with volatility-adjusted spreads, multi-level orders, and automated risk management.

## Why This Exists

The reference [`phoenix-onchain-market-maker`](https://github.com/Ellipsis-Labs/phoenix-onchain-mm) uses fixed spreads. This bot advances it with real quoting theory — the same inventory management techniques used at professional trading firms.

## Architecture

```
Coinbase REST ──► Oracle ──► Fair Price + Volatility
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │   Quoting Engine     │
                          │                      │
                          │  Avellaneda-Stoikov   │
                          │  inventory skew       │
                          │  + multi-level quotes │
                          └─────────┬─────────────┘
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │   Risk Manager       │
                          │                      │
                          │  Position limits      │
                          │  Drawdown protection  │
                          │  Reduce-only mode     │
                          └─────────┬─────────────┘
                                    │
                                    ▼
                          ┌─────────────────────┐
                          │   Phoenix Client     │
                          │                      │
                          │  Cancel + Place       │
                          │  via Phoenix SDK      │
                          └─────────────────────┘
```

## Quoting Algorithm

**Avellaneda-Stoikov with inventory skew:**

```
q = (position / max_position).clamp(-1, 1)

reservation_price = fair_price * (1 - γ * q * σ²)
half_spread       = max(base_spread_bps, σ * 10000 * γ) * fair_price / 10000

bid = reservation_price - half_spread
ask = reservation_price + half_spread
```

| State | Effect |
|-------|--------|
| **Flat** (q=0) | Symmetric quotes around fair price |
| **Long** (q>0) | Reservation drops below fair → ask tightens → sells to reduce |
| **Short** (q<0) | Reservation rises above fair → bid tightens → buys to reduce |
| **High vol** | Spread widens to compensate for adverse selection |

The `gamma` (γ) parameter controls risk aversion — higher values produce wider spreads and stronger inventory skew.

## Quick Start

```bash
cd server

# Configure
cp .env.example .env
# Edit .env with your Solana keypair

# Validate config
cargo run -- config

# Run the market maker (simulated mode by default)
cargo run -- run
```

### Configuration

All parameters in `config.toml`:

```toml
[strategy]
base_spread_bps = 15.0     # Minimum half-spread
gamma = 0.1                # Risk aversion parameter
num_levels = 3             # Quote levels per side
level_spacing_bps = 10.0   # Spacing between levels
base_size = 0.1            # Inner level size (SOL)
size_decay = 0.5           # Size decay per level

[risk]
max_position = 5.0         # Max absolute position (SOL)
max_drawdown_usd = 50.0    # Emergency cancel threshold
```

## Testing

```bash
cd server
cargo test
```

30 unit tests covering:
- **Quoting**: symmetric at flat, skew direction, vol widens spread, position clamp
- **Multi-level**: correct count, price ordering, size decay
- **Inventory**: PnL calculation, weighted avg entry, drawdown tracking
- **Risk**: position limits, drawdown triggers, order filtering

## Project Structure

```
server/src/
  main.rs              # CLI entry point
  cli.rs               # Clap subcommands
  config.rs            # TOML config parsing + validation
  oracle.rs            # Coinbase fair price + rolling volatility
  strategy/
    quoting.rs         # Avellaneda-Stoikov inventory skew
    multi_level.rs     # Multi-level quote generation
  inventory.rs         # Position tracking, PnL calculation
  risk.rs              # Position limits, drawdown protection
  engine.rs            # Main quote-risk-place loop
  phoenix_client.rs    # Phoenix SDK wrapper (trait-based)
  metrics.rs           # Runtime counters
  types.rs             # Shared types
  error.rs             # Error types
```

## Key Design Decisions

- **Trait-based Phoenix client**: Core logic compiles and tests without the full Phoenix SDK dependency tree. Live SDK behind `--features live`.
- **Pure quoting functions**: `compute_quotes()` and `generate_levels()` are pure functions — easy to test, easy to reason about.
- **Risk as filter**: Risk manager produces an action (Continue/ReduceOnly/EmergencyCancel) that filters quotes, keeping the quoting logic clean.
- **Oracle-driven fair price**: Uses Coinbase spot as the reference price rather than on-chain book mid, avoiding stale or manipulated on-chain prices.
