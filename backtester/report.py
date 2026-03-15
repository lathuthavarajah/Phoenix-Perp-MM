"""Generate a PDF report with backtest results and visualizations."""

import os
import pandas as pd
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages

from simulate_basis import backtest, DEFAULT_CONFIG, BacktestResult
from stress_test import simulate_crash_scenario, generate_crash_path

DATA_DIR = os.path.join(os.path.dirname(__file__), "data")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "output")


def generate_report(output_path: str = None):
    """Generate the full PDF report."""
    if output_path is None:
        output_path = os.path.join(OUTPUT_DIR, "report.pdf")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load data
    binance_path = os.path.join(DATA_DIR, "binance_sol_funding.csv")
    if not os.path.exists(binance_path):
        print("No data found. Run fetch_funding.py first.")
        return

    funding_df = pd.read_csv(binance_path, parse_dates=["timestamp"])
    result = backtest(funding_df)

    # Run stress tests
    initial_price = 150.0
    stress_results = []
    for name, target in [("SOL +20%", 1.20), ("SOL +30%", 1.30), ("SOL -20%", 0.80)]:
        path = generate_crash_path(initial_price, initial_price * target, 60)
        sr = simulate_crash_scenario(name=name, initial_sol_price=initial_price, price_path=path)
        stress_results.append(sr)

    with PdfPages(output_path) as pdf:
        # Page 1: Equity curve
        fig, ax = plt.subplots(figsize=(11, 8.5))
        fig.suptitle("Phoenix Arb Engine — Backtest Report", fontsize=16, fontweight="bold")

        headline = (
            f"Strategy Return: {result.cumulative_return*100:+.2f}% | "
            f"Ann. Return: {result.annualized_return*100:+.2f}% | "
            f"Sharpe: {result.sharpe_ratio:.2f} | "
            f"Max DD: {result.max_drawdown*100:.2f}%"
        )
        ax.set_title(headline, fontsize=11, pad=10)

        if len(result.equity_curve) > 0:
            ax.plot(result.equity_curve.values, color="#3b82f6", linewidth=1.5)

            # Mark trades
            if not result.trade_log.empty:
                for _, trade in result.trade_log.iterrows():
                    color = "green" if trade["pnl"] > 0 else "red"
                    marker = "^" if trade.get("exit_reason") != "emergency_close" else "v"
                    if trade["exit_idx"] < len(result.equity_curve):
                        ax.scatter(trade["exit_idx"], result.equity_curve.iloc[int(trade["exit_idx"])],
                                   color=color, marker=marker, s=30, zorder=5)

        ax.set_xlabel("Period (8h intervals)")
        ax.set_ylabel("Portfolio Value (USDC)")
        ax.axhline(y=DEFAULT_CONFIG["position_size_usdc"], color="gray", linestyle="--", alpha=0.5, label="Initial")
        ax.legend()
        ax.grid(True, alpha=0.3)
        plt.tight_layout()
        pdf.savefig(fig)
        plt.close()

        # Page 2: Funding rates over time
        fig, ax = plt.subplots(figsize=(11, 8.5))
        fig.suptitle("Historical Funding Rates", fontsize=14, fontweight="bold")

        apys = funding_df["rate_8h"] * 1095 * 100
        ax.plot(funding_df["timestamp"], apys, color="#3b82f6", linewidth=0.5, alpha=0.7, label="Binance SOL")
        ax.axhline(y=15, color="red", linestyle="--", alpha=0.7, label="Entry threshold (15%)")
        ax.axhline(y=2, color="orange", linestyle="--", alpha=0.7, label="Exit threshold (2%)")
        ax.axhline(y=0, color="gray", linestyle="-", alpha=0.3)

        ax.set_xlabel("Date")
        ax.set_ylabel("Annualized APY (%)")
        ax.legend()
        ax.grid(True, alpha=0.3)
        plt.tight_layout()
        pdf.savefig(fig)
        plt.close()

        # Page 3: Trade analysis
        fig, axes = plt.subplots(1, 2, figsize=(11, 8.5))
        fig.suptitle("Trade Analysis", fontsize=14, fontweight="bold")

        if not result.trade_log.empty:
            # Histogram of per-trade returns
            pnls = result.trade_log["pnl"]
            axes[0].hist(pnls, bins=20, color="#3b82f6", edgecolor="white", alpha=0.8)
            axes[0].axvline(x=0, color="red", linestyle="--")
            axes[0].set_xlabel("Trade PnL (USDC)")
            axes[0].set_ylabel("Count")
            axes[0].set_title("PnL Distribution")

            # Scatter: hold time vs return
            axes[1].scatter(
                result.trade_log["hold_hours"],
                result.trade_log["pnl"],
                c=["green" if p > 0 else "red" for p in pnls],
                alpha=0.6,
                s=40,
            )
            axes[1].axhline(y=0, color="gray", linestyle="--")
            axes[1].set_xlabel("Hold Time (hours)")
            axes[1].set_ylabel("PnL (USDC)")
            axes[1].set_title("Hold Time vs Return")
        else:
            axes[0].text(0.5, 0.5, "No trades executed", ha="center", va="center", transform=axes[0].transAxes)
            axes[1].text(0.5, 0.5, "No trades executed", ha="center", va="center", transform=axes[1].transAxes)

        plt.tight_layout()
        pdf.savefig(fig)
        plt.close()

        # Page 4: Summary table + stress test
        fig, axes = plt.subplots(2, 1, figsize=(11, 8.5))
        fig.suptitle("Summary & Stress Tests", fontsize=14, fontweight="bold")

        # Summary table
        axes[0].axis("off")
        summary_data = [
            ["Metric", "Value"],
            ["Cumulative Return", f"{result.cumulative_return*100:+.2f}%"],
            ["Annualized Return", f"{result.annualized_return*100:+.2f}%"],
            ["Sharpe Ratio", f"{result.sharpe_ratio:.2f}"],
            ["Max Drawdown", f"{result.max_drawdown*100:.2f}%"],
            ["Win Rate", f"{result.win_rate*100:.1f}%"],
            ["Total Trades", f"{result.n_trades}"],
            ["Emergency Closes", f"{result.n_emergency_closes}"],
            ["Funding Collected", f"${result.total_funding_collected:.2f}"],
            ["Fees Paid", f"${result.total_fees_paid:.2f}"],
        ]
        table = axes[0].table(cellText=summary_data, loc="center", cellLoc="center")
        table.auto_set_font_size(False)
        table.set_fontsize(10)
        table.scale(0.8, 1.5)

        # Stress test results
        axes[1].axis("off")
        stress_data = [["Scenario", "Emergency Close", "Liquidated", "Net PnL", "Min Margin"]]
        for sr in stress_results:
            stress_data.append([
                sr.name,
                f"${sr.emergency_close_price:.2f}" if sr.emergency_close_triggered else "No",
                "YES" if sr.liquidated else "No",
                f"${sr.net_pnl:.2f}",
                f"{sr.min_margin_ratio:.4f}",
            ])
        stress_table = axes[1].table(cellText=stress_data, loc="center", cellLoc="center")
        stress_table.auto_set_font_size(False)
        stress_table.set_fontsize(10)
        stress_table.scale(0.8, 1.5)

        plt.tight_layout()
        pdf.savefig(fig)
        plt.close()

    print(f"Report saved to {output_path}")


def main():
    generate_report()


if __name__ == "__main__":
    main()
