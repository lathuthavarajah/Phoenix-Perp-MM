use anchor_lang::prelude::*;

#[error_code]
pub enum MmError {
    #[msg("Oracle price is stale beyond max_oracle_staleness_secs")]
    StaleOracle,

    #[msg("Oracle returned invalid or non-positive price")]
    InvalidPrice,

    #[msg("Drawdown exceeds max_drawdown_quote_lots — emergency cancel")]
    MaxDrawdownExceeded,

    #[msg("Invalid configuration parameter")]
    InvalidConfig,

    #[msg("Arithmetic overflow in fixed-point computation")]
    MathOverflow,
}
