use anchor_lang::prelude::*;

use crate::errors::AegisError;
use crate::state::{CircuitBreakerConfig, CircuitBreakerState, Snapshot, TripReason};

pub(crate) fn handler(ctx: Context<Evaluate>, post_vault_balance: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(!config.is_paused, AegisError::GuardPaused);

    let snapshot = &ctx.accounts.snapshot;
    require_keys_eq!(snapshot.config, config.key(), AegisError::SnapshotMismatch);
    require_keys_eq!(
        snapshot.protocol,
        ctx.accounts.protocol.key(),
        AegisError::SnapshotMismatch
    );
    require_keys_eq!(
        snapshot.vault,
        ctx.accounts.vault.key(),
        AegisError::SnapshotMismatch
    );

    let outflow = snapshot
        .pre_vault_balance
        .saturating_sub(post_vault_balance);
    let threshold = snapshot
        .pre_vault_balance
        .checked_mul(config.max_outflow_bps as u64)
        .ok_or(AegisError::MathOverflow)?
        / 10_000;

    if outflow > threshold {
        let state = &mut ctx.accounts.state;
        state.is_tripped = true;
        state.tripped_at_slot = Clock::get()?.slot;
        state.trip_reason = TripReason::VaultOutflow;
        state.delta_value_at_trip = outflow;
        state.threshold_at_trip = threshold;

        require!(config.is_simulation_mode, AegisError::CircuitBreakerTripped);
    }

    Ok(())
}

#[derive(Accounts)]
pub struct Evaluate<'info> {
    pub protocol: Signer<'info>,
    #[account(has_one = authority)]
    pub config: Account<'info, CircuitBreakerConfig>,
    pub authority: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"state", config.key().as_ref()],
        bump = state.bump
    )]
    pub state: Account<'info, CircuitBreakerState>,
    /// CHECK: Checked against the snapshot vault key.
    pub vault: UncheckedAccount<'info>,
    #[account(
        seeds = [
            b"snapshot",
            config.key().as_ref(),
            protocol.key().as_ref(),
            vault.key().as_ref()
        ],
        bump = snapshot.bump
    )]
    pub snapshot: Account<'info, Snapshot>,
}
