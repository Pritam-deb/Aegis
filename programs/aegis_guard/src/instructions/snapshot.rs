use anchor_lang::prelude::*;

use crate::errors::AegisError;
use crate::state::{CircuitBreakerConfig, Snapshot};

pub(crate) fn handler(ctx: Context<CreateSnapshot>, vault_balance: u64) -> Result<()> {
    let config = &ctx.accounts.config;
    require!(!config.is_paused, AegisError::GuardPaused);

    let snapshot = &mut ctx.accounts.snapshot;
    snapshot.config = config.key();
    snapshot.protocol = ctx.accounts.protocol.key();
    snapshot.vault = ctx.accounts.vault.key();
    snapshot.pre_vault_balance = vault_balance;
    snapshot.snapshot_slot = Clock::get()?.slot;
    snapshot.bump = ctx.bumps.snapshot;

    Ok(())
}

#[derive(Accounts)]
pub struct CreateSnapshot<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub protocol: Signer<'info>,
    #[account(has_one = authority)]
    pub config: Account<'info, CircuitBreakerConfig>,
    pub authority: SystemAccount<'info>,
    /// CHECK: The protected protocol owns the vault semantics.
    pub vault: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Snapshot::INIT_SPACE,
        seeds = [
            b"snapshot",
            config.key().as_ref(),
            protocol.key().as_ref(),
            vault.key().as_ref()
        ],
        bump
    )]
    pub snapshot: Account<'info, Snapshot>,
    pub system_program: Program<'info, System>,
}
