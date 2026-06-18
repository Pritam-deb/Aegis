use anchor_lang::prelude::*;

use crate::state::DemoVault;

#[derive(Accounts)]
pub struct UpdateBalance<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub vault: Account<'info, DemoVault>,
}
