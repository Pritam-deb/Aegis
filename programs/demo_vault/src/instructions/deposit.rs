use anchor_lang::prelude::*;

use crate::errors::DemoVaultError;
use crate::instructions::UpdateBalance;

pub(crate) fn handler(ctx: Context<UpdateBalance>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.balance = vault
        .balance
        .checked_add(amount)
        .ok_or(DemoVaultError::MathOverflow)?;
    Ok(())
}
