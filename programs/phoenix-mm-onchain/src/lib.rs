use anchor_lang::prelude::*;

pub mod errors;
pub mod fixed_math;
pub mod instructions;
pub mod phoenix_cpi;
pub mod state;

// Re-export context structs at crate root — required by Anchor's #[program] macro.
pub use instructions::close::*;
pub use instructions::initialize::*;
pub use instructions::update_quotes::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod phoenix_mm_onchain {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        instructions::initialize::handler(ctx, params)
    }

    pub fn update_quotes(ctx: Context<UpdateQuotes>) -> Result<()> {
        instructions::update_quotes::handler(ctx)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        instructions::close::handler(ctx)
    }
}
