use anchor_lang::prelude::*;

use crate::state::{CircuitBreakerConfig, CircuitBreakerState, TripReason};

pub(crate) fn handler(ctx: Context<Reset>) -> Result<()> {
    let state = &mut ctx.accounts.state;
    state.is_tripped = false;
    state.tripped_at_slot = 0;
    state.trip_reason = TripReason::None;
    state.delta_value_at_trip = 0;
    state.threshold_at_trip = 0;

    Ok(())
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
