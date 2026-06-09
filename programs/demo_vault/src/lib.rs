use anchor_lang::prelude::*;

declare_id!("4gTAfDeL3ketKCwZUCRfFjBJMagxUUxpjgEu7KssUsNy");

#[program]
pub mod demo_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.authority = ctx.accounts.authority.key();
        vault.balance = 0;
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    pub fn deposit(ctx: Context<UpdateBalance>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.balance = vault
            .balance
            .checked_add(amount)
            .ok_or(DemoVaultError::MathOverflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<UpdateBalance>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(vault.balance >= amount, DemoVaultError::InsufficientBalance);
        vault.balance = vault
            .balance
            .checked_sub(amount)
            .ok_or(DemoVaultError::MathOverflow)?;
        Ok(())
    }
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

#[derive(Accounts)]
pub struct UpdateBalance<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub vault: Account<'info, DemoVault>,
}

#[account]
#[derive(InitSpace)]
pub struct DemoVault {
    pub authority: Pubkey,
    pub balance: u64,
    pub bump: u8,
}

#[error_code]
pub enum DemoVaultError {
    #[msg("The vault has insufficient balance")]
    InsufficientBalance,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
