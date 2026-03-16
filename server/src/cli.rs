use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "phoenix-mm")]
#[command(about = "Inventory-aware market maker for Phoenix DEX")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the market maker
    Run,
    /// Print validated configuration
    Config,
    /// Print current status (placeholder)
    Status,
}
