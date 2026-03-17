use anchor_lang::prelude::*;

use crate::state::{MmConfig, MmState};

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        close = authority,
        has_one = authority,
        seeds = [b"mm_config", authority.key().as_ref(), config.phoenix_market.as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, MmConfig>,

    #[account(
        mut,
        close = authority,
        has_one = config,
        seeds = [b"mm_state", config.key().as_ref()],
        bump = state.bump,
    )]
    pub state: Account<'info, MmState>,

    pub system_program: Program<'info, System>,
}

pub fn handler(_ctx: Context<Close>) -> Result<()> {
    msg!("MM config and state closed, rent reclaimed");
    Ok(())
}
