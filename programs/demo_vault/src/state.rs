use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct DemoVault {
    pub authority: Pubkey,
    pub balance: u64,
    pub bump: u8,
}
