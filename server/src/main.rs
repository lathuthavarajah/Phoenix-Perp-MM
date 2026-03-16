mod cli;
mod config;
mod engine;
mod error;
mod inventory;
mod metrics;
mod oracle;
mod phoenix_client;
mod risk;
mod strategy;
mod types;

use anyhow::Result;
use clap::Parser;
use std::path::Path;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command};
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("phoenix_mm=info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config
    let config = Config::load(Path::new(&cli.config))?;

    match cli.command {
        Command::Config => {
            println!("{config}");
        }
        Command::Status => {
            println!("Status: market maker not running (use `run` to start)");
        }
        Command::Run => {
            info!("Phoenix Inventory-Aware Market Maker starting");
            let mut engine = engine::Engine::new(config);
            engine.run().await?;
        }
    }

    Ok(())
}
