use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum MakerError {
    #[error("Oracle error: {0}")]
    Oracle(String),

    #[error("Phoenix client error: {0}")]
    Phoenix(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Risk limit breached: {0}")]
    RiskBreach(String),

    #[error("Solana RPC error: {0}")]
    Rpc(String),
}
