use anchor_lang::prelude::*;

use crate::errors::AegisError;
use crate::state::CircuitBreakerConfig;

pub(crate) fn handler(
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

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub config: Account<'info, CircuitBreakerConfig>,
}
