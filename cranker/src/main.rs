use anyhow::Result;
use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::Duration;

/// Program ID for phoenix-mm-onchain (must match deployed program).
const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

/// Phoenix DEX program ID.
const PHOENIX_PROGRAM_ID: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";

/// Phoenix log authority PDA.
const PHOENIX_LOG_AUTHORITY: &str = "7aDTsspkQNGKmrexAN7FLx9oxU3iPczSSvHNggyuqYkR";

/// Off-chain cranker for the Phoenix on-chain market maker.
///
/// Periodically calls update_quotes to refresh quotes on a Phoenix market.
#[derive(Parser, Debug)]
#[command(name = "phoenix-mm-cranker")]
struct Args {
    /// Path to the authority keypair JSON file.
    #[arg(short, long, default_value = "~/.config/solana/id.json")]
    keypair: String,

    /// Solana RPC URL.
    #[arg(short, long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Phoenix market pubkey (base58).
    #[arg(short, long)]
    market: String,

    /// Pyth price feed pubkey (base58).
    #[arg(short = 'f', long)]
    pyth_feed: String,

    /// Crank interval in seconds.
    #[arg(short, long, default_value_t = 2)]
    interval: u64,
}

/// Anchor instruction discriminator: sha256("global:update_quotes")[..8]
fn update_quotes_discriminator() -> [u8; 8] {
    let hash = <sha2::Sha256 as sha2::Digest>::digest(b"global:update_quotes");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Derive MmConfig PDA: seeds = ["mm_config", authority, phoenix_market]
fn derive_config_pda(authority: &Pubkey, market: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"mm_config", authority.as_ref(), market.as_ref()],
        program_id,
    )
}

/// Derive MmState PDA: seeds = ["mm_state", config]
fn derive_state_pda(config: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"mm_state", config.as_ref()], program_id)
}

/// Derive Phoenix seat PDA: seeds = ["seat", market, trader]
fn derive_seat(market: &Pubkey, trader: &Pubkey) -> (Pubkey, u8) {
    let phoenix_pid = Pubkey::from_str(PHOENIX_PROGRAM_ID).unwrap();
    Pubkey::find_program_address(
        &[b"seat", market.as_ref(), trader.as_ref()],
        &phoenix_pid,
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let keypair_path = shellexpand::tilde(&args.keypair).to_string();
    let authority = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair from {keypair_path}: {e}"))?;

    let rpc = RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    let program_id = Pubkey::from_str(PROGRAM_ID)?;
    let market = Pubkey::from_str(&args.market)?;
    let pyth_feed = Pubkey::from_str(&args.pyth_feed)?;
    let phoenix_pid = Pubkey::from_str(PHOENIX_PROGRAM_ID)?;
    let log_auth = Pubkey::from_str(PHOENIX_LOG_AUTHORITY)?;

    let (config_pda, _) = derive_config_pda(&authority.pubkey(), &market, &program_id);
    let (state_pda, _) = derive_state_pda(&config_pda, &program_id);
    let (seat, _) = derive_seat(&market, &authority.pubkey());

    tracing::info!(
        authority = %authority.pubkey(),
        market = %market,
        config = %config_pda,
        state = %state_pda,
        interval_secs = args.interval,
        "Starting cranker"
    );

    let interval = Duration::from_secs(args.interval);

    loop {
        match crank_once(
            &rpc,
            &program_id,
            &authority,
            &config_pda,
            &state_pda,
            &pyth_feed,
            &market,
            &phoenix_pid,
            &log_auth,
            &seat,
        ) {
            Ok(sig) => {
                tracing::info!(signature = %sig, "update_quotes sent");
            }
            Err(e) => {
                tracing::warn!(error = %e, "crank failed");
            }
        }

        tokio::time::sleep(interval).await;
    }
}

fn crank_once(
    rpc: &RpcClient,
    program_id: &Pubkey,
    authority: &dyn Signer,
    config: &Pubkey,
    state: &Pubkey,
    pyth_feed: &Pubkey,
    market: &Pubkey,
    phoenix_pid: &Pubkey,
    log_auth: &Pubkey,
    seat: &Pubkey,
) -> Result<solana_sdk::signature::Signature> {
    let disc = update_quotes_discriminator();

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), true), // cranker (signer)
            AccountMeta::new_readonly(*config, false),           // config
            AccountMeta::new_readonly(authority.pubkey(), false), // authority
            AccountMeta::new(*state, false),                     // state (mut)
            AccountMeta::new_readonly(*pyth_feed, false),        // pyth_price_feed
            AccountMeta::new(*market, false),                    // phoenix_market (mut)
            AccountMeta::new_readonly(*phoenix_pid, false),      // phoenix_program
            AccountMeta::new_readonly(*log_auth, false),         // log_authority
            AccountMeta::new_readonly(*seat, false),             // seat
        ],
        data: disc.to_vec(),
    };

    let recent_blockhash = rpc.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        recent_blockhash,
    );

    let sig = rpc.send_and_confirm_transaction(&tx)?;
    Ok(sig)
}
