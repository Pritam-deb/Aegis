# Aegis Architecture Design

## A Synchronous On-Chain Circuit Breaker for Solana DeFi

## Overview

Aegis is a reusable Solana security protocol designed to prevent exploit-driven vault drains and abnormal token outflows before suspicious transactions can commit on-chain.

The protocol has three layers:

1. **Inline Guard Layer**
   - Uses a synchronous `snapshot()` -> protected action -> `evaluate()` pattern.
   - Runs in the same transaction as the protected protocol instruction.
   - Reverts suspicious actions atomically.

2. **Transfer Hook Layer**
   - Uses Token-2022 Transfer Hooks.
   - Evaluates token transfer velocity at the mint level.
   - Blocks abnormal token outflows before the transfer completes.

3. **Governance Layer**
   - Manages thresholds, pause state, reset controls, and simulation mode.
   - Allows protocols to tune circuit breaker behavior over time.

## Diagram Legend

```mermaid
flowchart LR
    Program["Solana Program"]
    PDA[("Program-Derived Address")]
    Token["Token Account / Vault"]
    Oracle{{"Oracle"}}
    External(["External Service"])
    Decision{"Decision Point"}

    classDef program fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef pda fill:#dcfce7,stroke:#15803d,color:#111827
    classDef token fill:#fef3c7,stroke:#b45309,color:#111827
    classDef oracle fill:#fce7f3,stroke:#be185d,color:#111827
    classDef external fill:#ede9fe,stroke:#6d28d9,color:#111827
    classDef decision fill:#fee2e2,stroke:#b91c1c,color:#111827

    class Program program
    class PDA pda
    class Token token
    class Oracle oracle
    class External external
    class Decision decision
```

## 1. High-Level System Architecture

```mermaid
flowchart TB
    User["Protocol User"]
    Protocol["Protocol Program<br/>Lending, vaults, staking, withdrawals"]
    Guard["Aegis Guard Program<br/>Snapshots, evaluation, trip state"]
    Hook["Aegis Transfer Hook Program<br/>Transfer velocity checks"]
    Token2022["Token-2022 Program<br/>Token transfers and hook dispatch"]

    Config[("CircuitBreakerConfig PDA")]
    State[("CircuitBreakerState PDA")]
    Snapshot[("Snapshot PDA")]
    Outflow[("OutflowHistory PDA")]
    Price[("PriceHistory PDA")]
    Extra[("ExtraAccountMeta PDA")]
    Vault["Protocol Vault Token Account"]

    Pyth{{"Pyth Oracle"}}
    Gov["Governance Multisig"]
    Monitor(["Off-Chain Monitoring CLI"])

    User -->|"deposit(), withdraw(), stake(), claim_rewards()"| Protocol
    Protocol -->|"CPI: snapshot()"| Guard
    Protocol -->|"Token transfer"| Token2022
    Protocol -->|"CPI: evaluate()"| Guard
    Protocol --> Vault

    Token2022 -->|"Execute transfer hook"| Hook
    Hook -->|"Read thresholds"| Config
    Hook -->|"Read and update transfer history"| Outflow
    Hook -->|"Read extra account metadata"| Extra

    Guard -->|"Read thresholds"| Config
    Guard -->|"Read and update trip state"| State
    Guard -->|"Create and read pre-state"| Snapshot
    Guard -->|"Read transfer baselines"| Outflow
    Guard -->|"Read price baselines"| Price

    Pyth -->|"Price feed update"| Price
    Monitor -->|"Analyze simulation logs"| Gov
    Gov -->|"update_config(), pause(), unpause()"| Config
    Gov -->|"reset()"| State

    classDef program fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef pda fill:#dcfce7,stroke:#15803d,color:#111827
    classDef token fill:#fef3c7,stroke:#b45309,color:#111827
    classDef oracle fill:#fce7f3,stroke:#be185d,color:#111827
    classDef external fill:#ede9fe,stroke:#6d28d9,color:#111827

    class Protocol,Guard,Hook,Token2022 program
    class Config,State,Snapshot,Outflow,Price,Extra pda
    class Vault token
    class Pyth oracle
    class Monitor external
```

## 2. Program Structure

```mermaid
flowchart LR
    subgraph ProtocolProgram["Protocol Program"]
        P1["deposit()"]
        P2["withdraw()"]
        P3["liquidate()"]
        P4["stake()"]
        P5["claim_rewards()"]
        P6["emergency_withdraw()"]
    end

    subgraph GuardProgram["Aegis Guard Program"]
        G1["initialize_config()"]
        G2["snapshot()"]
        G3["evaluate()"]
        G4["update_config()"]
        G5["reset()"]
    end

    subgraph HookProgram["Aegis Transfer Hook Program"]
        H1["execute()"]
        H2["validate_transfer_velocity()"]
        H3["update_outflow_history()"]
    end

    subgraph TokenProgram["Token-2022 Program"]
        T1["transfer_checked()"]
        T2["dispatch_transfer_hook()"]
    end

    P2 -->|"CPI before protected action"| G2
    P3 -->|"CPI before protected action"| G2
    P6 -->|"CPI before protected action"| G2
    P2 -->|"CPI after protected action"| G3
    P3 -->|"CPI after protected action"| G3
    P6 -->|"CPI after protected action"| G3
    P1 -->|"Token movement"| T1
    P2 -->|"Token movement"| T1
    P5 -->|"Reward token movement"| T1
    T1 --> T2
    T2 -->|"Hook callback"| H1

    classDef program fill:#dbeafe,stroke:#1d4ed8,color:#111827
    class ProtocolProgram,GuardProgram,HookProgram,TokenProgram program
```

### Program Responsibilities

| Program | Primary Responsibilities | Important Instructions |
| --- | --- | --- |
| Protocol Program | Lending, vault management, staking, user operations | `deposit()`, `withdraw()`, `liquidate()`, `stake()`, `claim_rewards()`, `emergency_withdraw()` |
| Aegis Guard Program | Snapshot creation, delta evaluation, trip management, simulation mode, governance controls | `initialize_config()`, `snapshot()`, `evaluate()`, `update_config()`, `reset()` |
| Aegis Transfer Hook Program | Transfer velocity monitoring, transfer validation, outflow history updates | `execute()` |
| Token-2022 Program | Token transfers, transfer hook dispatch, mint-level extension enforcement | `transfer_checked()`, transfer hook callback |

## 3. Account Structure Mapping

```mermaid
flowchart TB
    Guard["Aegis Guard Program"]
    Hook["Aegis Transfer Hook Program"]
    Token2022["Token-2022 Program"]
    Protocol["Protocol Program"]

    Config[("CircuitBreakerConfig PDA<br/>Owner: Guard<br/>Type: PDA<br/>Stores: thresholds, pause state, simulation mode")]
    State[("CircuitBreakerState PDA<br/>Owner: Guard<br/>Type: PDA<br/>Stores: tripped state and reason")]
    Snapshot[("Snapshot PDA<br/>Owner: Guard<br/>Type: PDA<br/>Stores: pre-action vault, price, liquidation state")]
    Outflow[("OutflowHistory PDA<br/>Owner: Guard or Hook<br/>Type: PDA<br/>Stores: ring buffer and avg_slot_outflow")]
    Price[("PriceHistory PDA<br/>Owner: Guard<br/>Type: PDA<br/>Stores: EMA, price samples, timestamps")]
    Extra[("ExtraAccountMeta PDA<br/>Owner: Hook<br/>Type: PDA<br/>Stores: hook-required account metadata")]
    Vault["Vault Token Account<br/>Owner: Protocol vault authority<br/>Type: Token account<br/>Stores: protected assets"]
    Mint["Token-2022 Mint<br/>Owner: Token-2022<br/>Type: Mint account<br/>Stores: transfer hook extension"]
    UserAta["User Token Account<br/>Owner: User<br/>Type: Token account<br/>Stores: user assets"]

    Guard --> Config
    Guard --> State
    Guard --> Snapshot
    Guard --> Outflow
    Guard --> Price
    Hook --> Extra
    Hook --> Outflow
    Hook --> Config
    Token2022 --> Mint
    Protocol --> Vault
    UserAta -->|"Transfers through Token-2022"| Mint

    classDef program fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef pda fill:#dcfce7,stroke:#15803d,color:#111827
    classDef token fill:#fef3c7,stroke:#b45309,color:#111827

    class Guard,Hook,Token2022,Protocol program
    class Config,State,Snapshot,Outflow,Price,Extra pda
    class Vault,Mint,UserAta token
```

### Account Details

| Account | Owner | Type | Primary Data |
| --- | --- | --- | --- |
| `CircuitBreakerConfig` | Aegis Guard Program | PDA | `velocity_multiplier`, `price_deviation_bps`, `liquidation_multiplier`, `cooldown_slots`, `is_simulation_mode`, `is_paused` |
| `CircuitBreakerState` | Aegis Guard Program | PDA | `is_tripped`, `tripped_at_slot`, `trip_reason`, `delta_value_at_trip`, `threshold_at_trip` |
| `Snapshot` | Aegis Guard Program | PDA | `pre_vault_balance`, `pre_pyth_price`, `pre_liquidation_count`, `snapshot_slot` |
| `OutflowHistory` | Aegis Guard Program or Hook Program | PDA | Ring buffer, `avg_slot_outflow`, historical transfer amounts |
| `PriceHistory` | Aegis Guard Program | PDA | EMA, historical prices, timestamps |
| `ExtraAccountMeta` | Aegis Transfer Hook Program | PDA | Required account metadata for Token-2022 transfer hook execution |
| Vault Token Account | Protocol vault authority | Token account | Protected protocol liquidity |
| User Token Account | User wallet | Token account | User-owned token balance |
| Token-2022 Mint | Token-2022 Program | Mint account | Mint configuration and transfer hook extension |

## 4. PDA Derivation Map

```mermaid
flowchart LR
    Authority["protocol_authority"]
    User["user_pubkey"]
    Slot["slot"]
    Mint["mint"]

    ConfigSeed["Seeds: b'config', protocol_authority"]
    SnapshotSeed["Seeds: b'snapshot', user_pubkey, slot"]
    ExtraSeed["Seeds: b'extra-account-metas', mint"]

    Config[("CircuitBreakerConfig PDA")]
    Snapshot[("Snapshot PDA")]
    Extra[("ExtraAccountMeta PDA")]

    Authority --> ConfigSeed --> Config
    User --> SnapshotSeed
    Slot --> SnapshotSeed
    SnapshotSeed --> Snapshot
    Mint --> ExtraSeed --> Extra

    classDef pda fill:#dcfce7,stroke:#15803d,color:#111827
    classDef seed fill:#f3f4f6,stroke:#6b7280,color:#111827

    class Config,Snapshot,Extra pda
    class ConfigSeed,SnapshotSeed,ExtraSeed seed
```

### PDA Seeds

```rust
// CircuitBreakerConfig
[b"config", protocol_authority]

// Snapshot
[b"snapshot", user_pubkey, slot]

// ExtraAccountMeta
[b"extra-account-metas", mint]
```

Additional PDAs such as `CircuitBreakerState`, `OutflowHistory`, and `PriceHistory` should use deterministic seeds tied to the protected protocol, mint, vault, or market so each protected deployment has isolated state.

## 5. Program Interaction Matrix

| Source | Target | Interaction Type | Instruction or Data | Purpose |
| --- | --- | --- | --- | --- |
| User | Protocol Program | Transaction instruction | `deposit()`, `withdraw()`, `stake()`, `claim_rewards()` | User-facing DeFi operations |
| Protocol Program | Aegis Guard Program | CPI | `snapshot()` | Capture pre-action state |
| Protocol Program | Token-2022 Program | CPI | `transfer_checked()` | Move protected assets |
| Token-2022 Program | Aegis Transfer Hook Program | Hook callback | `execute()` | Validate transfer before completion |
| Protocol Program | Aegis Guard Program | CPI | `evaluate()` | Compare post-action state against thresholds |
| Aegis Guard Program | Pyth Oracle / PriceHistory | Account read | Price and EMA data | Detect abnormal price movement |
| Aegis Guard Program | OutflowHistory | Account read | Historical transfer baselines | Detect abnormal outflow velocity |
| Governance Multisig | Aegis Guard Program | Admin instruction | `update_config()`, `reset()` | Tune thresholds and recover from trips |
| Off-Chain Monitor | Governance Multisig | Report | Simulation results | Support threshold calibration |

## 6. User Interaction Flows

### Deposit Flow

```mermaid
sequenceDiagram
    actor User
    participant Protocol
    participant Token2022
    participant Hook
    participant Vault

    User->>Protocol: deposit(amount)
    Protocol->>Token2022: transfer_checked(user_ata, vault, amount)
    Token2022->>Hook: execute()
    Hook->>Hook: Validate transfer velocity

    alt Transfer velocity breached
        Hook-->>Token2022: Error
        Token2022-->>Protocol: Revert transfer
        Protocol-->>User: Deposit fails
    else Transfer allowed
        Token2022->>Vault: Credit vault token account
        Token2022-->>Protocol: Success
        Protocol-->>User: Mint or update deposit position
    end
```

### Withdrawal Protection Flow

```mermaid
sequenceDiagram
    actor User
    participant Protocol
    participant Guard
    participant Token2022
    participant Hook
    participant Vault

    User->>Protocol: withdraw(amount)
    Protocol->>Guard: snapshot()
    Guard->>Guard: Store pre-vault balance and baseline data
    Protocol->>Token2022: transfer_checked(vault, user_ata, amount)
    Token2022->>Hook: execute()
    Hook->>Hook: Validate transfer velocity
    Token2022->>Vault: Debit vault token account
    Protocol->>Guard: evaluate()
    Guard->>Guard: Compute price_delta, tvl_delta, liq_delta

    alt Any threshold exceeded
        Guard-->>Protocol: Error
        Protocol-->>User: Entire transaction reverts
    else Within thresholds
        Guard-->>Protocol: Success
        Protocol-->>User: Withdrawal succeeds
    end
```

### Staking and Reward Claim Flow

```mermaid
sequenceDiagram
    actor User
    participant Protocol
    participant Token2022
    participant Hook
    participant RewardVault

    User->>Protocol: stake(amount)
    Protocol->>Protocol: Update staking position
    Protocol-->>User: Stake recorded

    User->>Protocol: claim_rewards()
    Protocol->>Token2022: transfer_checked(reward_vault, user_ata, rewards)
    Token2022->>Hook: execute()
    Hook->>Hook: Validate reward outflow velocity

    alt Reward outflow abnormal
        Hook-->>Token2022: Error
        Token2022-->>Protocol: Revert transfer
        Protocol-->>User: Claim fails
    else Reward outflow normal
        Token2022->>RewardVault: Debit reward vault
        Token2022-->>Protocol: Success
        Protocol-->>User: Rewards claimed
    end
```

## 7. Circuit Breaker Decision Tree

```mermaid
flowchart TD
    Start["Protected Action"]
    Paused{"Config paused?"}
    Snapshot["snapshot()"]
    Execute["Execute Protocol Action"]
    HookCheck{"Transfer Hook Threshold Breached?"}
    Evaluate["evaluate()"]
    DeltaCheck{"Guard Delta Threshold Exceeded?"}
    Simulation{"Simulation Mode?"}
    Log["Emit simulation log"]
    Trip["Set tripped state"]
    Revert["Revert transaction"]
    Commit["Commit transaction"]

    Start --> Paused
    Paused -->|"Yes"| Revert
    Paused -->|"No"| Snapshot
    Snapshot --> Execute
    Execute --> HookCheck
    HookCheck -->|"Yes"| Simulation
    HookCheck -->|"No"| Evaluate
    Evaluate --> DeltaCheck
    DeltaCheck -->|"Yes"| Simulation
    DeltaCheck -->|"No"| Commit
    Simulation -->|"Yes"| Log
    Log --> Commit
    Simulation -->|"No"| Trip
    Trip --> Revert

    classDef decision fill:#fee2e2,stroke:#b91c1c,color:#111827
    classDef process fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef terminal fill:#f3f4f6,stroke:#374151,color:#111827

    class Paused,HookCheck,DeltaCheck,Simulation decision
    class Snapshot,Execute,Evaluate,Log,Trip process
    class Start,Revert,Commit terminal
```

## 8. Transfer Hook Flow

```mermaid
flowchart TD
    Start["Token-2022 transfer starts"]
    LoadExtra["Load ExtraAccountMeta PDA"]
    ReadConfig["Read CircuitBreakerConfig"]
    ReadOutflow["Read OutflowHistory"]
    Compute["velocity_delta = transfer_amount / avg_slot_outflow"]
    Check{"velocity_delta > velocity_multiplier?"}
    Sim{"Simulation mode?"}
    Update["Update OutflowHistory"]
    Log["Emit simulation event"]
    Reject["Return error to Token-2022"]
    Allow["Allow transfer"]

    Start --> LoadExtra
    LoadExtra --> ReadConfig
    ReadConfig --> ReadOutflow
    ReadOutflow --> Compute
    Compute --> Check
    Check -->|"No"| Update
    Update --> Allow
    Check -->|"Yes"| Sim
    Sim -->|"Yes"| Log
    Log --> Update
    Sim -->|"No"| Reject

    classDef decision fill:#fee2e2,stroke:#b91c1c,color:#111827
    classDef process fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef terminal fill:#f3f4f6,stroke:#374151,color:#111827

    class Check,Sim decision
    class LoadExtra,ReadConfig,ReadOutflow,Compute,Update,Log process
    class Start,Reject,Allow terminal
```

## 9. Delta Calculations

### Price Delta

```text
price_delta =
abs(current_price - ema_price) / ema_price
```

### TVL Delta

```text
tvl_delta =
(pre_balance - post_balance) / pre_balance
```

### Liquidation Delta

```text
liq_delta =
current_liq_rate / historical_liq_rate
```

### Transfer Velocity Delta

```text
velocity_delta =
transfer_amount / avg_slot_outflow
```

## 10. External Dependencies and Integrations

```mermaid
flowchart LR
    Protocol["Protocol Program"]
    Guard["Aegis Guard Program"]
    Hook["Aegis Transfer Hook Program"]
    Token2022["Token-2022 Program"]
    Pyth{{"Pyth Oracle"}}
    Monitor(["Off-Chain Monitoring CLI"])
    Governance["Governance Multisig"]

    Token2022 -->|"Transfer Hook extension"| Hook
    Pyth -->|"Price account read"| Guard
    Monitor -->|"Simulation analysis and threshold reports"| Governance
    Governance -->|"Admin instructions"| Guard
    Protocol -->|"Protected CPIs"| Guard
    Protocol -->|"Token transfers"| Token2022

    classDef program fill:#dbeafe,stroke:#1d4ed8,color:#111827
    classDef oracle fill:#fce7f3,stroke:#be185d,color:#111827
    classDef external fill:#ede9fe,stroke:#6d28d9,color:#111827

    class Protocol,Guard,Hook,Token2022 program
    class Pyth oracle
    class Monitor,Governance external
```

### Dependency Summary

| Dependency | Shape in Diagrams | Integration Point | Purpose |
| --- | --- | --- | --- |
| Token-2022 | Program box | Transfer Hook extension | Mint-level transfer interception |
| Pyth Oracle | Hexagon | Price account reads | Price anomaly detection and EMA updates |
| Off-Chain Monitoring CLI | External service box | Simulation logs and reports | Threshold tuning and deployment validation |
| Governance Multisig | External actor box | Admin instructions | Pause, reset, threshold updates, simulation mode control |

## 11. Governance and Simulation Mode

```mermaid
flowchart TB
    Gov["Governance Multisig"]
    Config[("CircuitBreakerConfig PDA")]
    State[("CircuitBreakerState PDA")]
    Logs[("Simulation Logs")]
    Monitor(["TypeScript Monitoring CLI"])

    Gov -->|"Enable simulation mode"| Config
    Config -->|"Simulation events emitted during checks"| Logs
    Logs -->|"Analyze false positives and threshold sensitivity"| Monitor
    Monitor -->|"Recommend threshold updates"| Gov
    Gov -->|"Enable live mode"| Config
    Gov -->|"Pause or unpause"| Config
    Gov -->|"Update thresholds"| Config
    Gov -->|"Reset tripped state"| State

    classDef pda fill:#dcfce7,stroke:#15803d,color:#111827
    classDef external fill:#ede9fe,stroke:#6d28d9,color:#111827

    class Config,State,Logs pda
    class Monitor external
```

## 12. Error Paths and Alternate Outcomes

| Scenario | Detection Point | Outcome in Live Mode | Outcome in Simulation Mode |
| --- | --- | --- | --- |
| Protocol is paused | Guard config check | Transaction reverts | Transaction reverts |
| Transfer velocity exceeds threshold | Transfer Hook `execute()` | Transfer reverts | Event emitted and transfer may continue |
| TVL delta exceeds threshold | Guard `evaluate()` | Transaction reverts and state may trip | Event emitted and transaction may continue |
| Price delta exceeds threshold | Guard `evaluate()` | Transaction reverts and state may trip | Event emitted and transaction may continue |
| Liquidation rate exceeds threshold | Guard `evaluate()` | Transaction reverts and state may trip | Event emitted and transaction may continue |
| Governance reset | Guard `reset()` | Tripped state clears | Tripped state clears |

## 13. Solana-Specific Design Considerations

- Program modularity is explicit: the Protocol Program owns business logic, the Guard Program owns synchronous circuit breaker evaluation, and the Transfer Hook Program owns mint-level transfer validation.
- CPIs are labeled separately from Token-2022 hook callbacks.
- PDAs are separated from token accounts and external dependencies.
- Protected account state is derived deterministically so each protocol deployment can have isolated configuration, trip state, and history.
- Token movement is routed through Token-2022 when mint-level protection is required.
- Error paths are part of the architecture because suspicious actions must revert before state changes commit.

## 14. Security Assumptions

1. The integrated protocol calls `snapshot()` before protected state changes and `evaluate()` after protected state changes.
2. The governance multisig remains uncompromised.
3. Oracle price feeds remain available and accurate enough for configured thresholds.
4. Historical baselines represent normal protocol behavior.
5. Token-2022 Transfer Hooks are configured on protected mints.
6. Required hook accounts are correctly registered through the `ExtraAccountMeta` PDA.

## 15. Known Limitations

1. Cross-bundle Jito analysis is out of scope.
2. Threshold calibration remains protocol-specific.
3. Transfer Hook protection applies only to Token-2022 assets.
4. False positives remain possible with poorly calibrated baselines.
5. Protocols must integrate the inline guard pattern correctly for atomic withdrawal protection.

## 16. Final Checklist

| Requirement | Status | Where Covered |
| --- | --- | --- |
| All programs represented | Complete | Sections 1 and 2 |
| Individual program responsibilities shown | Complete | Section 2 |
| Program interactions illustrated | Complete | Sections 1, 2, 5, and 6 |
| CPIs and instruction labels included | Complete | Sections 1, 2, and 5 |
| Account structures mapped | Complete | Section 3 |
| Account owners and types included | Complete | Section 3 |
| PDA derivation process shown | Complete | Section 4 |
| External dependencies shown | Complete | Section 10 |
| Decision points and alternate flows included | Complete | Sections 7, 8, and 12 |
| Clear labels and legend included | Complete | Diagram Legend and all Mermaid diagrams |
| Error paths included | Complete | Sections 6, 7, 8, and 12 |

## Summary

Aegis introduces a synchronous security primitive for Solana DeFi by combining an Inline Guard execution model with Token-2022 Transfer Hooks. The Guard Program protects protocol-level actions such as withdrawals and liquidations, while the Transfer Hook Program protects mint-level token movement. Governance and simulation mode allow the protocol to calibrate thresholds before enforcing live reverts, making the architecture suitable for capstone evaluation and real-world Solana protocol integration.
