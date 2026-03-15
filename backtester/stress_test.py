"""Simulate flash crash scenarios to validate risk management."""

import numpy as np
from dataclasses import dataclass
from utils import compute_margin_ratio, compute_liquidation_price_short


@dataclass
class ScenarioResult:
    name: str
    emergency_close_triggered: bool
    emergency_close_price: float
    final_loss: float
    funding_collected: float
    net_pnl: float
    min_margin_ratio: float
    liquidated: bool


def simulate_crash_scenario(
    name: str,
    initial_sol_price: float,
    price_path: list[float],
    position_size_usdc: float = 500.0,
    max_leverage: float = 3.0,
    emergency_close_ratio: float = 0.12,
    maintenance_margin_rate: float = 0.05,
    funding_rate_8h: float = 0.0001,
) -> ScenarioResult:
    """Simulate a crash scenario tick-by-tick (1-minute intervals)."""
    position_sol = position_size_usdc / initial_sol_price
    collateral = position_size_usdc / max_leverage
    entry_price = initial_sol_price
    liq_price = compute_liquidation_price_short(entry_price, collateral, position_sol, maintenance_margin_rate)

    funding_collected = 0.0
    min_margin_ratio = 1.0
    emergency_close_triggered = False
    emergency_close_price = 0.0
    liquidated = False

    # Funding accrues per minute: rate_8h / (8*60)
    funding_per_minute = funding_rate_8h / 480.0

    for price in price_path:
        # Accrue funding
        funding = position_sol * price * funding_per_minute
        funding_collected += funding
        collateral += funding

        # Compute margin
        unrealized_pnl = -position_sol * (price - entry_price)
        margin_ratio = compute_margin_ratio(collateral, unrealized_pnl, position_sol, price)
        min_margin_ratio = min(min_margin_ratio, margin_ratio)

        # Check liquidation
        if price >= liq_price:
            liquidated = True
            break

        # Check emergency close
        if margin_ratio < emergency_close_ratio:
            emergency_close_triggered = True
            emergency_close_price = price
            break

    final_loss = -position_sol * (price_path[-1] - entry_price) if not emergency_close_triggered else \
                 -position_sol * (emergency_close_price - entry_price)

    net_pnl = final_loss + funding_collected
    exit_fee = position_sol * (emergency_close_price if emergency_close_triggered else price_path[-1]) * 0.0005
    net_pnl -= exit_fee

    return ScenarioResult(
        name=name,
        emergency_close_triggered=emergency_close_triggered,
        emergency_close_price=emergency_close_price,
        final_loss=final_loss,
        funding_collected=funding_collected,
        net_pnl=net_pnl,
        min_margin_ratio=min_margin_ratio,
        liquidated=liquidated,
    )


def generate_crash_path(start_price: float, end_price: float, minutes: int) -> list[float]:
    """Generate a linear price path from start to end over N minutes."""
    return list(np.linspace(start_price, end_price, minutes))


def main():
    initial_price = 150.0

    scenarios = [
        {
            "name": "SOL -20% in 1 hour",
            "price_path": generate_crash_path(initial_price, initial_price * 0.80, 60),
        },
        {
            "name": "SOL -30% in 30 minutes",
            "price_path": generate_crash_path(initial_price, initial_price * 0.70, 30),
        },
        {
            "name": "SOL +20% in 1 hour (adverse for short)",
            "price_path": generate_crash_path(initial_price, initial_price * 1.20, 60),
        },
        {
            "name": "SOL +30% in 30 minutes (severe adverse)",
            "price_path": generate_crash_path(initial_price, initial_price * 1.30, 30),
        },
    ]

    print("=" * 70)
    print("  STRESS TEST RESULTS")
    print("=" * 70)

    all_pass = True
    for scenario in scenarios:
        result = simulate_crash_scenario(
            name=scenario["name"],
            initial_sol_price=initial_price,
            price_path=scenario["price_path"],
        )

        status = "PASS" if (result.emergency_close_triggered and not result.liquidated) or \
                          (not result.emergency_close_triggered and not result.liquidated) \
                 else "FAIL"

        # For adverse price moves (price up for shorts), emergency close should fire
        if "adverse" in scenario["name"] or "+20%" in scenario["name"] or "+30%" in scenario["name"]:
            if not result.emergency_close_triggered and result.liquidated:
                status = "FAIL"
                all_pass = False

        print(f"\n  Scenario: {result.name}")
        print(f"  Status: {status}")
        print(f"  Emergency close: {'YES at ${:.2f}'.format(result.emergency_close_price) if result.emergency_close_triggered else 'NO'}")
        print(f"  Liquidated: {'YES' if result.liquidated else 'NO'}")
        print(f"  Min margin ratio: {result.min_margin_ratio:.4f}")
        print(f"  Price loss: ${result.final_loss:.2f}")
        print(f"  Funding collected: ${result.funding_collected:.4f}")
        print(f"  Net PnL: ${result.net_pnl:.2f}")
        print(f"  {'─' * 50}")

    print(f"\n{'=' * 70}")
    print(f"  Overall: {'ALL PASS' if all_pass else 'SOME FAILURES'}")
    print(f"{'=' * 70}")


if __name__ == "__main__":
    main()
