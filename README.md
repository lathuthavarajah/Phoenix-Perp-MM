# Phoenix On-Chain Market Maker

On-chain Avellaneda-Stoikov market maker for [Phoenix DEX](https://www.phoenix.trade/) on Solana. An Anchor program that computes inventory-skewed quotes directly on-chain via CPI into Phoenix, with a lightweight off-chain cranker.

## Why This Exists

The reference [`phoenix-onchain-market-maker`](https://github.com/Ellipsis-Labs/phoenix-onchain-mm) uses fixed spreads. This program advances it with real quoting theory — the same inventory management techniques used at professional trading firms.

## Architecture

```
Pyth Oracle ──► On-Chain Program ──► Phoenix DEX
                    │                    (CPI)
                    │
                ┌───┴────────────────┐
                │  Fixed-Point A-S   │
                │  Quoting Engine    │
                │                    │
                │  MmConfig PDA      │
                │  MmState PDA       │
                └───┬────────────────┘
                    │
              Off-chain Cranker
              (calls update_quotes every 2s)
```

## Quoting Algorithm

**Avellaneda-Stoikov with inventory skew (fixed-point i128):**

```
q = (position / max_position).clamp(-1, 1)

reservation_price = fair_price * (1 - γ * q * σ²)
half_spread       = max(base_spread_bps, σ * γ) * fair_price / 10000

bid = reservation_price - half_spread
ask = reservation_price + half_spread
```

All math uses `i128` with two scale factors — no floats on Solana:
- **PRICE_SCALE** = 1e10 for prices
- **PARAM_SCALE** = 1e6 for parameters (gamma, size_decay)

| State | Effect |
|-------|--------|
| **Flat** (q=0) | Symmetric quotes around fair price |
| **Long** (q>0) | Reservation drops below fair → ask tightens → sells to reduce |
| **Short** (q<0) | Reservation rises above fair → bid tightens → buys to reduce |
| **High vol** | Spread widens to compensate for adverse selection |

## Instructions

| Instruction | Description |
|-------------|-------------|
| `initialize` | Create MmConfig + MmState PDAs with strategy/risk params |
| `update_quotes` | Read Pyth → compute A-S quotes → risk check → CPI cancel all → CPI place bids+asks |
| `close` | Close accounts, reclaim rent |

## Quick Start

**Run tests (no validator needed):**
```bash
cargo test -p phoenix-mm-onchain
```

**Build the program:**
```bash
cargo check -p phoenix-mm-onchain   # quick check
anchor build                         # full BPF compile (needs Anchor CLI)
```

**Run the cranker (needs deployed program + devnet setup):**
```bash
cargo run -p phoenix-mm-cranker -- \
  --keypair ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  --market <PHOENIX_MARKET_PUBKEY> \
  --pyth-feed <PYTH_FEED_PUBKEY> \
  --interval 2
```

## Account Layout

**MmConfig PDA** — seeds: `[b"mm_config", authority, phoenix_market]`
- Strategy: base_spread_bps, gamma_scaled, num_levels, level_spacing_bps, base_size_lots, size_decay_scaled
- Risk: max_position_lots, max_drawdown_quote_lots, max_oracle_staleness_secs
- volatility_bps (config override, or 0 to derive from Pyth confidence)

**MmState PDA** — seeds: `[b"mm_state", config]`
- position_lots, avg_entry_price_scaled, realized_pnl_atoms, peak_pnl_atoms
- total_volume_lots, crank_count, last_crank_ts

## Testing

17 unit tests covering:
- **A-S quoting**: symmetric at flat, skew direction, vol widens spread, position clamp, base spread floor
- **Cross-validation**: fixed-point results verified against f64 reference (flat, long, short+high vol)
- **Pyth conversion**: price scaling, exponent handling, negative rejection
- **Level computation**: size decay, price spacing, minimum sizes

## Project Structure

```
programs/phoenix-mm-onchain/src/
  lib.rs           # Program entry: initialize, update_quotes, close
  state.rs         # MmConfig + MmState account structs (PDAs)
  fixed_math.rs    # Fixed-point A-S quoting (i128 scaled integers)
  phoenix_cpi.rs   # CPI wrappers for Phoenix cancel/place
  errors.rs        # Custom error enum
  instructions/    # Instruction handlers
cranker/src/
  main.rs          # Off-chain loop: call update_quotes every 2s
```

## Key Design Decisions

- **No floats**: All on-chain math uses i128 fixed-point, cross-validated against f64 reference implementation.
- **Raw CPI**: Phoenix instructions built manually to avoid dependency version conflicts with Anchor.
- **Post-only orders**: Uses `PlaceMultiplePostOnlyOrdersWithFreeFunds` for efficient batch quoting.
- **Reduce-only mode**: Skips bids at max long, skips asks at max short — position naturally decays.
- **Pyth oracle**: Uses Pyth push-oracle for fair price; confidence interval can derive volatility.
