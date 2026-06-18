use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct CircuitBreakerConfig {
    pub authority: Pubkey,
    pub velocity_multiplier_bps: u16,
    pub max_outflow_bps: u16,
    pub cooldown_slots: u64,
    pub is_simulation_mode: bool,
    pub is_paused: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct CircuitBreakerState {
    pub is_tripped: bool,
    pub tripped_at_slot: u64,
    pub trip_reason: TripReason,
    pub delta_value_at_trip: u64,
    pub threshold_at_trip: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Snapshot {
    pub config: Pubkey,
    pub protocol: Pubkey,
    pub vault: Pubkey,
    pub pre_vault_balance: u64,
    pub snapshot_slot: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum TripReason {
    None,
    VaultOutflow,
}
