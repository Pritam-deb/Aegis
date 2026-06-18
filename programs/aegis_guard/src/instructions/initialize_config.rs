use anchor_lang::prelude::*;

use crate::errors::AegisError;
use crate::state::{CircuitBreakerConfig, CircuitBreakerState, TripReason};

pub(crate) fn handler(
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
