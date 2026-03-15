"""Compare funding rate yields across venues."""

import os
import pandas as pd
import numpy as np
from utils import annualize_8h_rate

DATA_DIR = os.path.join(os.path.dirname(__file__), "data")


def load_venue_data() -> dict[str, pd.DataFrame]:
    """Load all available venue funding data."""
    venues = {}

    binance_path = os.path.join(DATA_DIR, "binance_sol_funding.csv")
    if os.path.exists(binance_path):
        venues["Binance"] = pd.read_csv(binance_path, parse_dates=["timestamp"])

    drift_path = os.path.join(DATA_DIR, "drift_sol_funding.csv")
    if os.path.exists(drift_path):
        df = pd.read_csv(drift_path, parse_dates=["timestamp"])
        if not df.empty:
            venues["Drift"] = df

    return venues


def analyze_venue(name: str, df: pd.DataFrame) -> dict:
    """Compute summary statistics for a venue."""
    rates = df["rate_8h"].dropna()
    apys = rates.apply(annualize_8h_rate)

    return {
        "venue": name,
        "n_periods": len(rates),
        "mean_rate_8h": rates.mean(),
        "median_rate_8h": rates.median(),
        "std_rate_8h": rates.std(),
        "mean_apy": apys.mean(),
        "median_apy": apys.median(),
        "pct_positive": (rates > 0).mean(),
        "pct_above_15": (apys > 0.15).mean(),
        "max_rate_8h": rates.max(),
        "min_rate_8h": rates.min(),
        "start_date": df["timestamp"].iloc[0],
        "end_date": df["timestamp"].iloc[-1],
    }


def main():
    venues = load_venue_data()

    if not venues:
        print("No data found. Run fetch_funding.py first.")
        return

    print("=" * 70)
    print("  CROSS-VENUE FUNDING RATE COMPARISON")
    print("=" * 70)

    results = []
    for name, df in venues.items():
        stats = analyze_venue(name, df)
        results.append(stats)

        print(f"\n  {name}:")
        print(f"    Period: {stats['start_date']} to {stats['end_date']}")
        print(f"    Data points: {stats['n_periods']}")
        print(f"    Mean 8h rate: {stats['mean_rate_8h']*100:.4f}%")
        print(f"    Mean annualized: {stats['mean_apy']*100:.2f}%")
        print(f"    Median annualized: {stats['median_apy']*100:.2f}%")
        print(f"    Std dev (8h): {stats['std_rate_8h']*100:.4f}%")
        print(f"    % positive: {stats['pct_positive']*100:.1f}%")
        print(f"    % above 15% APY: {stats['pct_above_15']*100:.1f}%")
        print(f"    Max 8h rate: {stats['max_rate_8h']*100:.4f}%")
        print(f"    Min 8h rate: {stats['min_rate_8h']*100:.4f}%")

    if len(results) > 1:
        print(f"\n{'─' * 70}")
        print("  COMPARISON:")
        best_venue = max(results, key=lambda x: x["mean_apy"])
        print(f"    Highest avg APY: {best_venue['venue']} ({best_venue['mean_apy']*100:.2f}%)")

        most_consistent = max(results, key=lambda x: x["pct_positive"])
        print(f"    Most consistent: {most_consistent['venue']} ({most_consistent['pct_positive']*100:.1f}% positive)")

    print(f"\n{'=' * 70}")

    # Save comparison
    output_dir = os.path.join(os.path.dirname(__file__), "output")
    os.makedirs(output_dir, exist_ok=True)
    pd.DataFrame(results).to_csv(os.path.join(output_dir, "venue_comparison.csv"), index=False)
    print(f"Saved to output/venue_comparison.csv")


if __name__ == "__main__":
    main()
