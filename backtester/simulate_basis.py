"""Backtest the delta-neutral funding rate arbitrage strategy."""

import os
from dataclasses import dataclass, field
import numpy as np
import pandas as pd

from utils import annualize_8h_rate, compute_margin_ratio, max_drawdown, sharpe_ratio

DATA_DIR = os.path.join(os.path.dirname(__file__), "data")

DEFAULT_CONFIG = {
    "entry_threshold_apy": 0.15,
    "exit_threshold_apy": 0.02,
    "position_size_usdc": 500.0,
    "max_leverage": 3.0,
    "entry_fee_rate": 0.0005,
    "exit_fee_rate": 0.0005,
    "rebalance_fee_rate": 0.0001,
    "emergency_close_ratio": 0.12,
    "maintenance_margin_rate": 0.05,
}


@dataclass
class BacktestResult:
    cumulative_return: float = 0.0
    annualized_return: float = 0.0
    sharpe_ratio: float = 0.0
    max_drawdown: float = 0.0
    win_rate: float = 0.0
    avg_hold_time_hours: float = 0.0
    n_trades: int = 0
    n_emergency_closes: int = 0
    total_funding_collected: float = 0.0
    total_fees_paid: float = 0.0
    trade_log: pd.DataFrame = field(default_factory=pd.DataFrame)
    equity_curve: pd.Series = field(default_factory=lambda: pd.Series(dtype=float))


def backtest(funding_df: pd.DataFrame, config: dict = None) -> BacktestResult:
    """Run the basis trade backtest on historical funding data."""
    if config is None:
        config = DEFAULT_CONFIG

    result = BacktestResult()

    if funding_df.empty:
        print("No data to backtest")
        return result

    portfolio_value = config["position_size_usdc"]
    initial_value = portfolio_value
    position_open = False
    entry_idx = 0
    entry_price = 0.0
    position_sol = 0.0
    collateral = 0.0
    funding_collected = 0.0
    fees_paid = 0.0

    equity_values = []
    trades = []

    for i, row in funding_df.iterrows():
        rate_8h = row["rate_8h"]
        price = row.get("mark_price", 0) or row.get("index_price", 0)
        if price <= 0:
            equity_values.append(portfolio_value)
            continue

        apy = annualize_8h_rate(rate_8h)

        if position_open:
            # Collect funding (shorts receive when rate is positive)
            funding_payment = abs(position_sol) * price * rate_8h
            funding_collected += funding_payment
            portfolio_value += funding_payment

            # Check margin
            unrealized_pnl = -abs(position_sol) * (price - entry_price)
            margin_ratio = compute_margin_ratio(collateral, unrealized_pnl, position_sol, price)

            # Emergency close
            if margin_ratio < config["emergency_close_ratio"]:
                exit_fee = abs(position_sol) * price * config["exit_fee_rate"]
                fees_paid += exit_fee
                portfolio_value -= exit_fee
                portfolio_value += unrealized_pnl

                hold_time = (i - entry_idx) * 8  # hours
                trade_pnl = portfolio_value - initial_value
                trades.append({
                    "entry_idx": entry_idx,
                    "exit_idx": i,
                    "entry_price": entry_price,
                    "exit_price": price,
                    "hold_hours": hold_time,
                    "pnl": trade_pnl,
                    "exit_reason": "emergency_close",
                })
                result.n_emergency_closes += 1
                position_open = False

            # Check exit conditions
            elif apy < config["exit_threshold_apy"] or rate_8h < 0:
                exit_fee = abs(position_sol) * price * config["exit_fee_rate"]
                fees_paid += exit_fee
                portfolio_value -= exit_fee
                portfolio_value += unrealized_pnl

                hold_time = (i - entry_idx) * 8
                trade_pnl = portfolio_value - initial_value
                trades.append({
                    "entry_idx": entry_idx,
                    "exit_idx": i,
                    "entry_price": entry_price,
                    "exit_price": price,
                    "hold_hours": hold_time,
                    "pnl": trade_pnl,
                    "exit_reason": "signal",
                })
                position_open = False

        else:
            # Check entry
            if apy > config["entry_threshold_apy"] and rate_8h > 0:
                position_sol = portfolio_value / price
                collateral = portfolio_value / config["max_leverage"]
                entry_price = price
                entry_idx = i
                entry_fee = position_sol * price * config["entry_fee_rate"]
                fees_paid += entry_fee
                portfolio_value -= entry_fee
                position_open = True
                initial_value = portfolio_value

        equity_values.append(portfolio_value)

    # Close any open position at end
    if position_open and len(funding_df) > 0:
        last_price = funding_df.iloc[-1].get("mark_price", 0) or funding_df.iloc[-1].get("index_price", 0)
        if last_price > 0:
            unrealized_pnl = -abs(position_sol) * (last_price - entry_price)
            portfolio_value += unrealized_pnl

    equity_curve = pd.Series(equity_values, dtype=float)
    result.equity_curve = equity_curve

    # Compute metrics
    starting_value = config["position_size_usdc"]
    result.cumulative_return = (portfolio_value - starting_value) / starting_value
    n_periods = len(funding_df)
    if n_periods > 0:
        periods_per_year = 1095
        result.annualized_return = result.cumulative_return * (periods_per_year / n_periods)

    # Sharpe from equity curve returns
    if len(equity_curve) > 1:
        returns = equity_curve.pct_change().dropna().values
        result.sharpe_ratio = sharpe_ratio(returns)

    result.max_drawdown = max_drawdown(equity_curve.values) if len(equity_curve) > 0 else 0.0
    result.trade_log = pd.DataFrame(trades)
    result.n_trades = len(trades)
    result.total_funding_collected = funding_collected
    result.total_fees_paid = fees_paid

    if trades:
        winning = [t for t in trades if t["pnl"] > 0]
        result.win_rate = len(winning) / len(trades)
        result.avg_hold_time_hours = np.mean([t["hold_hours"] for t in trades])

    return result


def print_results(result: BacktestResult, venue: str):
    """Print backtest summary."""
    print(f"\n{'='*60}")
    print(f"  Backtest Results: {venue}")
    print(f"{'='*60}")
    print(f"  Cumulative Return:     {result.cumulative_return*100:+.2f}%")
    print(f"  Annualized Return:     {result.annualized_return*100:+.2f}%")
    print(f"  Sharpe Ratio:          {result.sharpe_ratio:.2f}")
    print(f"  Max Drawdown:          {result.max_drawdown*100:.2f}%")
    print(f"  Win Rate:              {result.win_rate*100:.1f}%")
    print(f"  Avg Hold Time:         {result.avg_hold_time_hours:.1f} hours")
    print(f"  Total Trades:          {result.n_trades}")
    print(f"  Emergency Closes:      {result.n_emergency_closes}")
    print(f"  Funding Collected:     ${result.total_funding_collected:.2f}")
    print(f"  Fees Paid:             ${result.total_fees_paid:.2f}")
    print(f"{'='*60}\n")


def main():
    binance_path = os.path.join(DATA_DIR, "binance_sol_funding.csv")

    if not os.path.exists(binance_path):
        print("No data found. Run fetch_funding.py first.")
        return

    print("Loading Binance funding data...")
    binance_df = pd.read_csv(binance_path, parse_dates=["timestamp"])

    result = backtest(binance_df)
    print_results(result, "Binance SOL-PERP")

    # Save equity curve
    output_dir = os.path.join(os.path.dirname(__file__), "output")
    os.makedirs(output_dir, exist_ok=True)
    result.equity_curve.to_csv(os.path.join(output_dir, "equity_curve.csv"), index=False)

    if not result.trade_log.empty:
        result.trade_log.to_csv(os.path.join(output_dir, "trade_log.csv"), index=False)
        print(f"Saved trade log ({len(result.trade_log)} trades)")


if __name__ == "__main__":
    main()
