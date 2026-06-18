use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

pub use errors::*;
pub use instructions::*;
pub use state::*;

declare_id!("4gTAfDeL3ketKCwZUCRfFjBJMagxUUxpjgEu7KssUsNy");

#[program]
pub mod demo_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn deposit(ctx: Context<UpdateBalance>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    pub fn withdraw(ctx: Context<UpdateBalance>, amount: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, amount)
    }
}
