use anchor_lang::prelude::*;

declare_id!("Bzt5J5Vw7KHQPp9ZEu9yuPf6GcVaMbMrrUk6m8GkpPSN");

#[program]
pub mod aegis_guard {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        velocity_multiplier_bps: u16,
        max_outflow_bps: u16,
        cooldown_slots: u64,
        is_simulation_mode: bool,
    ) -> Result<()> {
        require!(velocity_multiplier_bps > 0, AegisError::InvalidThreshold);
        require!(max_outflow_bps <= 10_000, AegisError::InvalidThreshold);

        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.velocity_multiplier_bps = velocity_multiplier_bps;
        config.max_outflow_bps = max_outflow_bps;
        config.cooldown_slots = cooldown_slots;
        config.is_simulation_mode = is_simulation_mode;
        config.is_paused = false;
        config.bump = ctx.bumps.config;

        let state = &mut ctx.accounts.state;
        state.is_tripped = false;
        state.tripped_at_slot = 0;
        state.trip_reason = TripReason::None;
        state.delta_value_at_trip = 0;
        state.threshold_at_trip = 0;
        state.bump = ctx.bumps.state;

        Ok(())
    }

    pub fn snapshot(ctx: Context<CreateSnapshot>, vault_balance: u64) -> Result<()> {
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

    pub fn evaluate(ctx: Context<Evaluate>, post_vault_balance: u64) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.is_paused, AegisError::GuardPaused);

        let snapshot = &ctx.accounts.snapshot;
        require_keys_eq!(snapshot.config, config.key(), AegisError::SnapshotMismatch);
        require_keys_eq!(snapshot.protocol, ctx.accounts.protocol.key(), AegisError::SnapshotMismatch);
        require_keys_eq!(snapshot.vault, ctx.accounts.vault.key(), AegisError::SnapshotMismatch);

        let outflow = snapshot.pre_vault_balance.saturating_sub(post_vault_balance);
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

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        velocity_multiplier_bps: u16,
        max_outflow_bps: u16,
        cooldown_slots: u64,
        is_simulation_mode: bool,
        is_paused: bool,
    ) -> Result<()> {
        require!(velocity_multiplier_bps > 0, AegisError::InvalidThreshold);
        require!(max_outflow_bps <= 10_000, AegisError::InvalidThreshold);

        let config = &mut ctx.accounts.config;
        config.velocity_multiplier_bps = velocity_multiplier_bps;
        config.max_outflow_bps = max_outflow_bps;
        config.cooldown_slots = cooldown_slots;
        config.is_simulation_mode = is_simulation_mode;
        config.is_paused = is_paused;

        Ok(())
    }

    pub fn reset(ctx: Context<Reset>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.is_tripped = false;
        state.tripped_at_slot = 0;
        state.trip_reason = TripReason::None;
        state.delta_value_at_trip = 0;
        state.threshold_at_trip = 0;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + CircuitBreakerConfig::INIT_SPACE,
        seeds = [b"config", authority.key().as_ref()],
        bump
    )]
    pub config: Account<'info, CircuitBreakerConfig>,
    #[account(
        init,
        payer = authority,
        space = 8 + CircuitBreakerState::INIT_SPACE,
        seeds = [b"state", config.key().as_ref()],
        bump
    )]
    pub state: Account<'info, CircuitBreakerState>,
    pub system_program: Program<'info, System>,
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

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub config: Account<'info, CircuitBreakerConfig>,
}

#[derive(Accounts)]
pub struct Reset<'info> {
    pub authority: Signer<'info>,
    #[account(has_one = authority)]
    pub config: Account<'info, CircuitBreakerConfig>,
    #[account(
        mut,
        seeds = [b"state", config.key().as_ref()],
        bump = state.bump
    )]
    pub state: Account<'info, CircuitBreakerState>,
}

#[account]
#[derive(InitSpace)]
pub struct CircuitBreakerConfig {
    pub authority: Pubkey,
    pub velocity_multiplier_bps: u16,
    pub max_outflow_bps: u16,
    pub cooldown_slots: u64,
    pub is_simulation_mode: bool,
    pub is_paused: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct CircuitBreakerState {
    pub is_tripped: bool,
    pub tripped_at_slot: u64,
    pub trip_reason: TripReason,
    pub delta_value_at_trip: u64,
    pub threshold_at_trip: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Snapshot {
    pub config: Pubkey,
    pub protocol: Pubkey,
    pub vault: Pubkey,
    pub pre_vault_balance: u64,
    pub snapshot_slot: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum TripReason {
    None,
    VaultOutflow,
}

#[error_code]
pub enum AegisError {
    #[msg("The guard is paused")]
    GuardPaused,
    #[msg("The provided threshold is invalid")]
    InvalidThreshold,
    #[msg("The snapshot does not match the evaluated accounts")]
    SnapshotMismatch,
    #[msg("The circuit breaker tripped")]
    CircuitBreakerTripped,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
