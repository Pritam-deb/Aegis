# MVP Scope

## Project Name

**Aegis — MVP**

## One-Line Summary

A synchronous on-chain circuit breaker for Solana protocols that snapshots sensitive account state before a critical action and atomically reverts the transaction if the post-action delta crosses a configured safety threshold.

---

## MVP Goal

The goal of this MVP is to prove the core security primitive:

> A Solana protocol can wrap a sensitive action with an inline guard that records pre-action state, evaluates post-action state inside the same transaction, and prevents dangerous state changes from committing.

The MVP will focus on **basic threshold-based atomic reversion** first, instead of advanced velocity-based anomaly detection. This keeps the implementation achievable within two weeks while still demonstrating the most important value: **detection and prevention happen synchronously, before the transaction commits**.

---

## Problem

Solana DeFi protocols can lose funds through atomic exploits where an attacker drains a vault or manipulates protocol state inside a single transaction or tightly bundled execution flow.

Most monitoring systems are asynchronous:

1. A transaction executes.
2. An off-chain bot or monitor detects something suspicious.
3. Governance, multisig, or an admin pauses the protocol later.

This is too slow for atomic attacks. By the time the alert is generated, the damage has already committed.

The MVP addresses this by moving the guard **inside the transaction path**.

---

## Core MVP Idea

The protected protocol performs a sensitive action using this pattern:

```text
snapshot()
protocol_action()
evaluate()
```

Where:

- `snapshot()` records important pre-action state.
- `protocol_action()` performs the actual protocol operation.
- `evaluate()` compares the post-action state against the snapshot.
- If the delta exceeds the allowed threshold, the instruction returns an error and the whole Solana transaction reverts.

---

## MVP Scope

The MVP will implement the **inline guard layer only**.

The Token-2022 Transfer Hook layer is intentionally out of scope for the first MVP because it adds integration complexity. The first version should prove that the atomic revert model works with a protocol-integrated CPI guard.

### Included in MVP

#### 1. Guard Configuration PDA

A configuration account controlled by a protocol/governance authority.

Stores:

```text
governance_authority
max_tvl_delta_bps
cooldown_slots
is_paused
is_simulation_mode
is_tripped
tripped_at_slot
trip_reason
delta_value_at_trip
threshold_at_trip
```

Purpose:

- Defines the allowed safety threshold.
- Tracks whether the guard is paused or tripped.
- Allows simulation mode for logging without reverting.
- Allows governance/admin reset after cooldown.

---

#### 2. Snapshot PDA

A per-user, per-slot snapshot account.

Seed pattern:

```text
[b"snapshot", user_pubkey, slot_as_bytes]
```

Stores:

```text
user
snapshot_slot
pre_vault_balance
```

Purpose:

- Captures vault balance before a sensitive action.
- Prevents the guard from relying on off-chain state.
- Keeps pre-action and post-action comparison fully on-chain.

---

#### 3. Snapshot Instruction

Instruction name:

```text
snapshot
```

Responsibilities:

- Check that the guard is not paused.
- Read the protected vault/token account balance.
- Store the pre-action vault balance in a Snapshot PDA.
- Store the current slot.

Example flow:

```text
User transaction:
1. snapshot()
2. withdraw / swap / drain-like test action
3. evaluate()
```

---

#### 4. Evaluate Instruction

Instruction name:

```text
evaluate
```

Responsibilities:

- Read the Snapshot PDA.
- Read the current post-action vault balance.
- Calculate the percentage outflow from the vault.
- Compare the delta with the configured threshold.
- Revert if the delta exceeds the allowed limit.

Core formula:

```text
tvl_delta_bps = ((pre_vault_balance - post_vault_balance) * 10_000) / pre_vault_balance
```

Decision:

```text
if tvl_delta_bps > max_tvl_delta_bps:
    trip circuit breaker
    revert transaction
```

Because Solana transactions are atomic, returning an error from `evaluate()` reverts the earlier protocol action too.

---

#### 5. Simulation Mode

The MVP will support simulation mode.

When:

```text
is_simulation_mode = true
```

The guard should:

- Calculate the delta.
- Emit/log what would have happened.
- Not revert the transaction.

When:

```text
is_simulation_mode = false
```

The guard should:

- Revert if the threshold is exceeded.
- Mark the circuit breaker as tripped.
- Store trip metadata.

Purpose:

- Makes demos easier.
- Shows how protocol teams can test thresholds safely.
- Reduces risk of false positives during integration.

---

#### 6. Trip State

If a threshold is exceeded, the config PDA records:

```text
is_tripped = true
tripped_at_slot = current_slot
trip_reason = "TVLDeltaExceeded"
delta_value_at_trip = tvl_delta_bps
threshold_at_trip = max_tvl_delta_bps
```

The guard should prevent future guarded actions while tripped.

---

#### 7. Reset Instruction

Instruction name:

```text
reset
```

Responsibilities:

- Only callable by governance authority.
- Requires cooldown period to have elapsed.
- Clears trip state.

Checks:

```text
current_slot >= tripped_at_slot + cooldown_slots
```

After reset:

```text
is_tripped = false
tripped_at_slot = 0
trip_reason = None
delta_value_at_trip = 0
threshold_at_trip = 0
```

---

#### 8. Update Config Instruction

Instruction name:

```text
update_config
```

Responsibilities:

- Only callable by governance authority.
- Update threshold values.
- Toggle simulation mode.
- Pause/unpause the guard.
- Update cooldown slots.

Configurable fields:

```text
max_tvl_delta_bps
cooldown_slots
is_simulation_mode
is_paused
```

---

#### 9. Demo Protocol / Test Harness

The MVP should include a small demo protocol or test harness that simulates a protected vault.

Demo actions:

```text
safe_withdraw
unsafe_withdraw
```

Expected behavior:

| Scenario                                    | Expected Result                       |
| ------------------------------------------- | ------------------------------------- |
| Withdraw below threshold                    | Transaction succeeds                  |
| Withdraw above threshold in simulation mode | Transaction succeeds and logs warning |
| Withdraw above threshold in active mode     | Transaction fails and reverts         |
| Guard is paused                             | Guarded action fails                  |
| Guard is tripped                            | Future guarded actions fail           |
| Governance resets after cooldown            | Guard becomes usable again            |

---

## Explicitly Out of Scope for MVP

The following are important, but not part of the two-week MVP.

### 1. Token-2022 Transfer Hook Layer

Not included in MVP.

Reason:

- Transfer Hooks are valuable but add complexity around mint configuration, extra account metas, and integration ergonomics.
- The first MVP should prove the core synchronous revert mechanism before adding mint-level automatic enforcement.

Planned later as Layer 2.

---

### 2. Advanced Velocity Metrics

Not included in MVP.

Examples out of scope:

```text
avg_slot_outflow
velocity_delta
rolling baseline
EMA-based outflow history
per-slot transfer velocity
```

Reason:

- Velocity systems can create false positives.
- They require careful baseline design.
- For the MVP, simple thresholding is easier to explain, test, and demo.

---

### 3. Price Deviation Guard

Not included in MVP.

Out of scope fields:

```text
pre_pyth_price
ema_price
price_delta
price_deviation_bps
```

Reason:

- Oracle integration adds additional moving parts.
- The MVP should focus on vault outflow protection first.

---

### 4. Liquidation Rate Guard

Not included in MVP.

Out of scope fields:

```text
pre_liq_count
liq_rate_this_window
liq_rate_baseline
liq_delta
record_liquidation
```

Reason:

- Liquidation tracking is protocol-specific.
- It is better suited for a later integration phase.

---

### 5. Production-Grade Protocol SDK

Not included in MVP.

The MVP may include basic TypeScript scripts for:

- initializing config
- toggling simulation mode
- running demo transactions
- resetting the guard

But a polished SDK is not required.

---

## MVP Success Criteria

The MVP is successful if it demonstrates the following:

### Functional Success

- A protected vault action can be wrapped using `snapshot -> action -> evaluate`.
- The guard can calculate vault outflow delta on-chain.
- The guard allows safe changes below the threshold.
- The guard reverts unsafe changes above the threshold.
- The revert is atomic: the unsafe vault action does not commit.
- Governance can update config.
- Governance can reset the circuit breaker after cooldown.
- Simulation mode works without reverting.

---

### Demo Success

The demo should clearly show:

```text
Before vault balance: 1000 tokens
Attempted withdrawal: 50 tokens
Threshold: 10%
Result: success
```

```text
Before vault balance: 1000 tokens
Attempted withdrawal: 400 tokens
Threshold: 10%
Result: transaction reverted
Final vault balance: 1000 tokens
```

This proves the main pitch:

> The guard does not merely detect an exploit. It prevents the state transition from committing.

---

## Suggested Two-Week Build Plan

### Week 1 — Core Guard

#### Day 1

- Finalize Anchor project structure.
- Define config account.
- Define snapshot account.
- Write initialization instruction.

#### Day 2

- Implement `update_config`.
- Implement pause/unpause.
- Add authority checks.

#### Day 3

- Implement `snapshot`.
- Store pre-action vault balance.
- Add tests for snapshot account creation.

#### Day 4

- Implement `evaluate`.
- Calculate TVL delta in basis points.
- Add threshold comparison.

#### Day 5

- Implement trip state.
- Implement simulation mode.
- Add custom errors.

#### Day 6

- Implement reset with cooldown.
- Add unit/integration tests.

#### Day 7

- Build demo vault or test harness.
- Create safe and unsafe withdrawal scenarios.

---

### Week 2 — Polish, Tests, Demo, Pitch

#### Day 8

- Strengthen test coverage.
- Test edge cases:
  - zero pre-balance
  - already tripped
  - paused guard
  - invalid authority
  - cooldown not elapsed

#### Day 9

- Add TypeScript scripts for demo flow.
- Improve logs and output readability.

#### Day 10

- Write repo documentation.
- Add diagrams for transaction flow.

#### Day 11

- Prepare pitch deck.
- Explain problem, solution, demo, and future roadmap.

#### Day 12

- Record demo transaction flows.
- Capture successful and reverted transaction outputs.

#### Day 13

- Record 3-minute video presentation.
- Keep the story focused on atomic prevention.

#### Day 14

- Final polish.
- Clean README.
- Verify reproducible setup.
- Final test run.

---

## Technical Architecture

### Main Accounts

```text
CircuitBreakerConfig PDA
Snapshot PDA
Protected Vault Token Account
Governance Authority
User
```

---

### Main Instructions

```text
initialize_config
update_config
snapshot
evaluate
reset
```

---

### Custom Errors

```text
Unauthorized
GuardPaused
CircuitBreakerTripped
CooldownNotElapsed
InvalidSnapshot
ZeroPreBalance
TVLDeltaExceeded
MathOverflow
```

---

## Example Transaction Flow

### Safe Flow

```text
1. User calls snapshot
2. Protocol executes withdrawal
3. User/protocol calls evaluate
4. Delta is below threshold
5. Transaction succeeds
```

---

### Unsafe Flow

```text
1. User calls snapshot
2. Protocol executes withdrawal
3. User/protocol calls evaluate
4. Delta is above threshold
5. evaluate returns TVLDeltaExceeded
6. Entire transaction reverts
7. Vault balance remains unchanged
```

---

## Why This MVP Is Enough

The full project vision includes Transfer Hooks, velocity baselines, price deviation checks, and liquidation-rate protection.

However, the MVP only needs to prove the hardest and most valuable claim:

> Can a Solana protocol detect dangerous state changes inside the same transaction and atomically revert them?

If the answer is yes, then the project has a strong foundation.

Everything else can be layered later.

---

## Future Roadmap

### Phase 2 — Velocity Guard

Add historical outflow tracking:

```text
OutflowHistory PDA
avg_slot_outflow
velocity_delta
velocity_multiplier
```

This enables dynamic anomaly detection instead of fixed thresholds.

---

### Phase 3 — Price Deviation Guard

Add oracle-based checks:

```text
PriceHistory PDA
Pyth price
EMA price
price_delta
```

This protects against oracle manipulation and sudden price deviation.

---

### Phase 4 — Liquidation Rate Guard

Add liquidation monitoring:

```text
liq_rate_this_window
liq_rate_baseline
liq_delta
```

This protects lending/perps protocols against sudden liquidation cascades.

---

### Phase 5 — Token-2022 Transfer Hook Layer

Add mint-level automatic enforcement using Token-2022 Transfer Hooks.

This gives protocols protection even when individual instructions are not manually wrapped.

---

## Final MVP Positioning

This MVP is not trying to build a complete DeFi security platform in two weeks.

It is trying to prove one powerful primitive:

> Inline account-state delta checks can turn Solana’s atomic transaction model into a real-time exploit prevention mechanism.

That is the core value of the project.
