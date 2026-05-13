# Phoenix On-Chain Market Maker

The reference [`phoenix-onchain-mm`](https://github.com/Ellipsis-Labs/phoenix-onchain-mm) uses fixed spreads. This one uses Avellaneda-Stoikov inventory-skewed quoting instead: when you're long, it tightens the ask to sell. When you're short, it tightens the bid to buy. The spread widens with volatility. All the math runs on-chain in fixed-point i128 (no floats on Solana).

## How it works

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

A cranker on your machine sends a transaction every 2 seconds. The Solana program reads the Pyth price, runs the quoting math, does a risk check, then CPIs into Phoenix to cancel stale orders and place fresh ones.

Three instructions: `initialize` (set up config + state PDAs with your strategy params), `update_quotes` (the main loop), and `close` (tear down and reclaim rent).

## The math

```
q = (position / max_position).clamp(-1, 1)

reservation = fair_price * (1 - γ * q * σ²)
half_spread = max(base_spread_bps, σ * γ) * fair_price / 10000

bid = reservation - half_spread
ask = reservation + half_spread
```

Since Solana doesn't allow floats, everything uses i128 with two scale factors: `PRICE_SCALE = 1e10` for prices and `PARAM_SCALE = 1e6` for parameters like gamma and size decay. Intermediate products max out around 9.6e29, well within i128's range (~1.7e38).

The fixed-point output is cross-validated against an f64 reference implementation, the tests confirm they match within 0.01 tolerance.

## Try it

```bash
# run the 17 unit tests (no validator needed)
cargo test -p phoenix-mm-onchain

# check it compiles
cargo check -p phoenix-mm-onchain
cargo check -p phoenix-mm-cranker

# full BPF build (needs Anchor CLI + Solana toolchain)
anchor build
```

To actually deploy and crank on devnet:

```bash
cargo run -p phoenix-mm-cranker -- \
  --keypair ~/.config/solana/id.json \
  --rpc-url https://api.devnet.solana.com \
  --market <PHOENIX_MARKET_PUBKEY> \
  --pyth-feed <PYTH_FEED_PUBKEY> \
  --interval 2
```

## What's in here

```
programs/phoenix-mm-onchain/src/
  lib.rs           # program entry — initialize, update_quotes, close
  state.rs         # MmConfig + MmState PDAs
  fixed_math.rs    # the i128 A-S quoting engine + 17 tests
  phoenix_cpi.rs   # raw CPI into Phoenix (cancel all, place batch post-only)
  errors.rs        # StaleOracle, InvalidPrice, MaxDrawdownExceeded, etc.
  instructions/    # handler for each instruction

cranker/src/
  main.rs          # off-chain loop that pokes update_quotes
```

## Design choices

**Raw CPI instead of importing phoenix-v1.** Phoenix v0.2.4 pins solana-program 1.14, which conflicts with Anchor 0.30's requirement for 1.17+. Rather than downgrading Anchor or forking Phoenix, I built the instruction data manually, it's just discriminant bytes + Borsh-serialized order packets.

**Post-only batch orders.** Uses `PlaceMultiplePostOnlyOrdersWithFreeFunds` (discriminant 17) to place all bid and ask levels in a single CPI. Cheaper than individual limit orders.

**Reduce-only at position limits.** When at max long, bids are skipped entirely (only asks remain). At max short, asks are skipped. Position decays naturally without a separate unwind mechanism.

**Volatility from Pyth confidence.** If you don't set a fixed `volatility_bps`, the program derives it from the Pyth confidence interval (`conf / price * 10000`), floored at 1%.

## Account layout

**MmConfig** `[b"mm_config", authority, market]` — strategy params (spread, gamma, levels, sizes) + risk params (max position, max drawdown, oracle staleness). Set once via `initialize`.

**MmState** `[b"mm_state", config]` — runtime tracking: position in lots, avg entry price, realized PnL, peak PnL watermark, volume, crank count, last crank timestamp.
