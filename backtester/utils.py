"""Shared math helpers for the backtester."""

import numpy as np


def annualize_8h_rate(rate_8h: float) -> float:
    """Convert 8-hour funding rate to annualized APY."""
    return rate_8h * 3 * 365  # 3 periods per day * 365 days


def compute_margin_ratio(
    collateral: float,
    unrealized_pnl: float,
    position_size: float,
    mark_price: float,
) -> float:
    """Compute margin ratio for a perp position."""
    equity = collateral + unrealized_pnl
    notional = abs(position_size) * mark_price
    if notional == 0:
        return 1.0
    return equity / notional


def compute_liquidation_price_short(
    entry_price: float,
    collateral: float,
    position_size: float,
    maintenance_margin_rate: float = 0.05,
) -> float:
    """Compute liquidation price for a short position."""
    notional = abs(position_size) * entry_price
    equity = collateral
    return entry_price * (1.0 + (equity / notional) - maintenance_margin_rate)


def compute_unrealized_pnl_short(
    position_size: float,
    entry_price: float,
    current_price: float,
) -> float:
    """Compute unrealized PnL for a short position (size is negative)."""
    return position_size * (current_price - entry_price)


def sharpe_ratio(returns: np.ndarray, risk_free_rate: float = 0.04) -> float:
    """Compute annualized Sharpe ratio from a series of periodic returns."""
    if len(returns) == 0 or np.std(returns) == 0:
        return 0.0
    # Assuming 8h periods: 1095 per year
    periods_per_year = 1095
    excess_returns = returns - risk_free_rate / periods_per_year
    return float(np.mean(excess_returns) / np.std(excess_returns) * np.sqrt(periods_per_year))


def max_drawdown(equity_curve: np.ndarray) -> float:
    """Compute maximum drawdown from an equity curve."""
    if len(equity_curve) == 0:
        return 0.0
    peak = np.maximum.accumulate(equity_curve)
    drawdown = (equity_curve - peak) / peak
    return float(np.min(drawdown))
