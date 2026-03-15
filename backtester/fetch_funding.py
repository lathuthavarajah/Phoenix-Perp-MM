"""Fetch historical funding rate data from Binance and Drift."""

import os
import time
import requests
import pandas as pd

DATA_DIR = os.path.join(os.path.dirname(__file__), "data")


def fetch_binance_funding(symbol: str = "SOLUSDT", days_back: int = 180) -> pd.DataFrame:
    """Fetch historical 8h funding rates from Binance FAPI."""
    url = "https://fapi.binance.com/fapi/v1/fundingRate"
    all_records = []
    end_time = int(time.time() * 1000)
    start_time = end_time - (days_back * 24 * 3600 * 1000)

    current_start = start_time
    while current_start < end_time:
        params = {
            "symbol": symbol,
            "startTime": current_start,
            "limit": 1000,
        }
        resp = requests.get(url, params=params, timeout=10)
        resp.raise_for_status()
        data = resp.json()

        if not data:
            break

        all_records.extend(data)
        current_start = data[-1]["fundingTime"] + 1

        # Rate limit
        time.sleep(0.2)

    df = pd.DataFrame(all_records)
    if df.empty:
        return df

    df["timestamp"] = pd.to_datetime(df["fundingTime"], unit="ms")
    df["rate_8h"] = df["fundingRate"].astype(float)
    df["mark_price"] = df.get("markPrice", pd.Series([0.0] * len(df))).astype(float)
    df["index_price"] = df.get("indexPrice", pd.Series([0.0] * len(df))).astype(float)

    df = df[["timestamp", "rate_8h", "mark_price", "index_price"]].sort_values("timestamp")
    return df


def fetch_drift_funding(market_index: int = 0, days_back: int = 180) -> pd.DataFrame:
    """Fetch historical funding rates from Drift protocol."""
    url = "https://mainnet-beta.api.drift.trade/fundingRates"
    end_ts = int(time.time())
    start_ts = end_ts - (days_back * 24 * 3600)

    all_records = []
    page = 1

    while True:
        params = {
            "marketIndex": market_index,
            "from": start_ts,
            "to": end_ts,
            "page": page,
            "limit": 500,
        }
        try:
            resp = requests.get(url, params=params, timeout=10)
            resp.raise_for_status()
            data = resp.json()

            records = data.get("data", data) if isinstance(data, dict) else data
            if not records:
                break

            all_records.extend(records if isinstance(records, list) else [records])
            page += 1
            time.sleep(0.3)

            if len(records) < 500:
                break
        except Exception as e:
            print(f"Drift API error (page {page}): {e}")
            break

    if not all_records:
        print("No Drift data fetched, generating synthetic data from Binance rates")
        return pd.DataFrame(columns=["timestamp", "rate_8h", "mark_price", "index_price"])

    df = pd.DataFrame(all_records)

    # Drift returns 1h rates — resample to 8h by summing
    if "ts" in df.columns:
        df["timestamp"] = pd.to_datetime(df["ts"].astype(int), unit="s")
    elif "timestamp" in df.columns:
        df["timestamp"] = pd.to_datetime(df["timestamp"])

    rate_col = "fundingRate" if "fundingRate" in df.columns else "rate"
    if rate_col in df.columns:
        df["rate_1h"] = df[rate_col].astype(float) / 1e9
    else:
        df["rate_1h"] = 0.0

    price_col = "oraclePrice" if "oraclePrice" in df.columns else "price"
    if price_col in df.columns:
        df["price"] = df[price_col].astype(float) / 1e6
    else:
        df["price"] = 0.0

    df = df.set_index("timestamp").sort_index()
    resampled = df.resample("8h").agg({"rate_1h": "sum", "price": "last"}).dropna()
    resampled = resampled.reset_index()
    resampled.columns = ["timestamp", "rate_8h", "mark_price"]
    resampled["index_price"] = resampled["mark_price"]

    return resampled


def main():
    os.makedirs(DATA_DIR, exist_ok=True)

    print("Fetching Binance SOL funding rates...")
    binance_df = fetch_binance_funding()
    binance_path = os.path.join(DATA_DIR, "binance_sol_funding.csv")
    binance_df.to_csv(binance_path, index=False)
    print(f"  Saved {len(binance_df)} records to {binance_path}")

    print("Fetching Drift SOL funding rates...")
    drift_df = fetch_drift_funding()
    drift_path = os.path.join(DATA_DIR, "drift_sol_funding.csv")
    drift_df.to_csv(drift_path, index=False)
    print(f"  Saved {len(drift_df)} records to {drift_path}")

    # Summary
    if not binance_df.empty:
        avg_rate = binance_df["rate_8h"].mean()
        avg_apy = avg_rate * 1095
        print(f"\nBinance summary:")
        print(f"  Period: {binance_df['timestamp'].iloc[0]} to {binance_df['timestamp'].iloc[-1]}")
        print(f"  Avg 8h rate: {avg_rate*100:.4f}%")
        print(f"  Avg annualized APY: {avg_apy*100:.2f}%")


if __name__ == "__main__":
    main()
