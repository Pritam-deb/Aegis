# Aegis

Aegis is an Anchor-based Solana security protocol prototype for synchronous, on-chain circuit breaking around protected protocol actions.

The current workspace contains two Anchor programs:

- `aegis_guard`: a reusable guard program that stores circuit breaker config/state, records pre-action snapshots, evaluates post-action vault balance deltas, and trips when outflow exceeds the configured threshold.
- `demo_vault`: a small demo protocol program with an internal vault balance, intended as the first integration target for the guard flow.

See `docs/ARCHITECTURE.md` and `docs/mvp_scope.md` for the broader design and MVP notes.

## Programs

| Program       | Localnet Program ID                            |
| ------------- | ---------------------------------------------- |
| `aegis_guard` | `Bzt5J5Vw7KHQPp9ZEu9yuPf6GcVaMbMrrUk6m8GkpPSN` |
| `demo_vault`  | `4gTAfDeL3ketKCwZUCRfFjBJMagxUUxpjgEu7KssUsNy` |

## Repository Layout

```text
.
├── Anchor.toml
├── Cargo.toml
├── programs
│   ├── aegis_guard
│   │   └── src/lib.rs
│   └── demo_vault
│       └── src/lib.rs
├── tests
│   └── aegis.ts
└── docs
    ├── ARCHITECTURE.md
    └── mvp_scope.md
```

## Prerequisites

- Anchor CLI `0.32.1`
- Solana CLI / Agave toolchain
- Rust / Cargo
- Node.js and Yarn

## Setup

Install JavaScript dependencies:

```bash
yarn install
```

Build the Anchor programs:

```bash
NO_DNA=1 anchor build
```

Generated artifacts are written to:

- `target/deploy/*.so`
- `target/idl/*.json`
- `target/types/*.ts`

## Verification

Run Rust workspace tests:

```bash
NO_DNA=1 cargo test --workspace
```

Type-check the TypeScript test/client code:

```bash
NO_DNA=1 yarn tsc --noEmit
```

Run the full Anchor test suite:

```bash
NO_DNA=1 anchor test
```

If port `8899` is already occupied by another local validator, stop that validator or run against a separate local validator configuration before retrying.

## Current Guard Flow

The intended inline protection pattern is:

1. Protected protocol calls `aegis_guard::snapshot` before the sensitive action.
2. Protected protocol performs the action.
3. Protected protocol calls `aegis_guard::evaluate` with the post-action vault balance.
4. `aegis_guard` compares the outflow against `max_outflow_bps`.
5. If the outflow exceeds the threshold, the guard records trip state and either logs simulation behavior or aborts the transaction depending on `is_simulation_mode`.
