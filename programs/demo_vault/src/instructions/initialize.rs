use anchor_lang::prelude::*;

use crate::state::DemoVault;

pub(crate) fn handler(ctx: Context<Initialize>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.authority = ctx.accounts.authority.key();
    vault.balance = 0;
    vault.bump = ctx.bumps.vault;
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + DemoVault::INIT_SPACE,
        seeds = [b"demo-vault", authority.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, DemoVault>,
    pub system_program: Program<'info, System>,
}
