use crate::types::EngineConfig;

pub fn load_config() -> EngineConfig {
    EngineConfig {
        rpc_url: env_or(
            "SOLANA_RPC_URL",
            "https://api.mainnet-beta.solana.com",
        ),
        use_devnet: env_or("USE_DEVNET", "true").parse().unwrap_or(true),
        wallet_keypair_path: env_or(
            "WALLET_KEYPAIR_PATH",
            "~/.config/solana/id.json",
        ),
        funding_entry_threshold_apy: env_or("FUNDING_ENTRY_THRESHOLD_APY", "0.15")
            .parse()
            .unwrap_or(0.15),
        funding_exit_threshold_apy: env_or("FUNDING_EXIT_THRESHOLD_APY", "0.02")
            .parse()
            .unwrap_or(0.02),
        position_size_usdc: env_or("POSITION_SIZE_USDC", "500")
            .parse()
            .unwrap_or(500.0),
        max_leverage: env_or("MAX_LEVERAGE", "3")
            .parse()
            .unwrap_or(3.0),
        rebalance_delta_threshold: env_or("REBALANCE_DELTA_THRESHOLD", "0.01")
            .parse()
            .unwrap_or(0.01),
        margin_warning_ratio: env_or("MARGIN_WARNING_RATIO", "0.25")
            .parse()
            .unwrap_or(0.25),
        emergency_close_ratio: env_or("EMERGENCY_CLOSE_RATIO", "0.12")
            .parse()
            .unwrap_or(0.12),
        engine_http_port: env_or("ENGINE_HTTP_PORT", "8080")
            .parse()
            .unwrap_or(8080),
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
