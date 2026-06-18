use anchor_lang::prelude::*;

#[error_code]
pub enum AegisError {
    #[msg("The guard is paused")]
    GuardPaused,
    #[msg("The provided threshold is invalid")]
    InvalidThreshold,
    #[msg("The snapshot does not match the evaluated accounts")]
    SnapshotMismatch,
    #[msg("The circuit breaker tripped")]
    CircuitBreakerTripped,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
