use anchor_lang::prelude::*;

#[error_code]
pub enum DemoVaultError {
    #[msg("The vault has insufficient balance")]
    InsufficientBalance,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
