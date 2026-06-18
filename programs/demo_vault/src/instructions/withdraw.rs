use anchor_lang::prelude::*;

use crate::errors::DemoVaultError;
use crate::instructions::UpdateBalance;

pub(crate) fn handler(ctx: Context<UpdateBalance>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    require!(vault.balance >= amount, DemoVaultError::InsufficientBalance);
    vault.balance = vault
        .balance
        .checked_sub(amount)
        .ok_or(DemoVaultError::MathOverflow)?;
    Ok(())
}
