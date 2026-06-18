use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

pub use errors::*;
pub use instructions::*;
pub use state::*;

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
        instructions::initialize_config::handler(
            ctx,
            velocity_multiplier_bps,
            max_outflow_bps,
            cooldown_slots,
            is_simulation_mode,
        )
    }

    pub fn snapshot(ctx: Context<CreateSnapshot>, vault_balance: u64) -> Result<()> {
        instructions::snapshot::handler(ctx, vault_balance)
    }

    pub fn evaluate(ctx: Context<Evaluate>, post_vault_balance: u64) -> Result<()> {
        instructions::evaluate::handler(ctx, post_vault_balance)
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        velocity_multiplier_bps: u16,
        max_outflow_bps: u16,
        cooldown_slots: u64,
        is_simulation_mode: bool,
        is_paused: bool,
    ) -> Result<()> {
        instructions::update_config::handler(
            ctx,
            velocity_multiplier_bps,
            max_outflow_bps,
            cooldown_slots,
            is_simulation_mode,
            is_paused,
        )
    }

    pub fn reset(ctx: Context<Reset>) -> Result<()> {
        instructions::reset::handler(ctx)
    }
}
