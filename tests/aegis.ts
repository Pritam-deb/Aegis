import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";

import type { AegisGuard } from "../target/types/aegis_guard";
import type { DemoVault } from "../target/types/demo_vault";

describe("aegis", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const aegisGuard = anchor.workspace.AegisGuard as Program<AegisGuard>;
  const demoVault = anchor.workspace.DemoVault as Program<DemoVault>;
  const systemProgram = anchor.web3.SystemProgram.programId;

  const bn = (value: number | string) => new anchor.BN(value);

  const deriveConfig = (authority: anchor.web3.PublicKey) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("config"), authority.toBuffer()],
      aegisGuard.programId,
    )[0];

  const deriveState = (config: anchor.web3.PublicKey) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state"), config.toBuffer()],
      aegisGuard.programId,
    )[0];

  const deriveSnapshot = (
    config: anchor.web3.PublicKey,
    protocol: anchor.web3.PublicKey,
    vault: anchor.web3.PublicKey,
  ) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("snapshot"),
        config.toBuffer(),
        protocol.toBuffer(),
        vault.toBuffer(),
      ],
      aegisGuard.programId,
    )[0];

  const deriveVault = (authority: anchor.web3.PublicKey) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("demo-vault"), authority.toBuffer()],
      demoVault.programId,
    )[0];

  const fundAuthority = async () => {
    const authority = anchor.web3.Keypair.generate();
    const latestBlockhash = await provider.connection.getLatestBlockhash();
    const signature = await provider.connection.requestAirdrop(
      authority.publicKey,
      5 * anchor.web3.LAMPORTS_PER_SOL,
    );

    await provider.connection.confirmTransaction(
      { signature, ...latestBlockhash },
      "confirmed",
    );

    return authority;
  };

  const expectFailure = async (action: Promise<unknown>, expected: string) => {
    try {
      await action;
    } catch (error) {
      const message = String(error);
      assert.include(
        message,
        expected,
        `Expected error to include "${expected}", got:\n${message}`,
      );
      return;
    }

    assert.fail(`Expected action to fail with "${expected}"`);
  };

  const initializeGuard = async ({
    authority,
    maxOutflowBps = 1_000,
    isSimulationMode = false,
  }: {
    authority: anchor.web3.Keypair;
    maxOutflowBps?: number;
    isSimulationMode?: boolean;
  }) => {
    const config = deriveConfig(authority.publicKey);
    const state = deriveState(config);

    await aegisGuard.methods
      .initializeConfig(200, maxOutflowBps, bn(10), isSimulationMode)
      .accountsStrict({
        authority: authority.publicKey,
        config,
        state,
        systemProgram,
      })
      .signers([authority])
      .rpc();

    return { config, state };
  };

  const initializeVault = async (authority: anchor.web3.Keypair) => {
    const vault = deriveVault(authority.publicKey);

    await demoVault.methods
      .initialize()
      .accountsStrict({
        authority: authority.publicKey,
        vault,
        systemProgram,
      })
      .signers([authority])
      .rpc();

    return vault;
  };

  const createSnapshot = async ({
    authority,
    config,
    vault,
    preBalance,
  }: {
    authority: anchor.web3.Keypair;
    config: anchor.web3.PublicKey;
    vault: anchor.web3.PublicKey;
    preBalance: number;
  }) => {
    const snapshot = deriveSnapshot(config, authority.publicKey, vault);

    await aegisGuard.methods
      .snapshot(bn(preBalance))
      .accountsStrict({
        payer: authority.publicKey,
        protocol: authority.publicKey,
        config,
        authority: authority.publicKey,
        vault,
        snapshot,
        systemProgram,
      })
      .signers([authority])
      .rpc();

    return snapshot;
  };

  it("initializes both programs", async () => {
    const authority = await fundAuthority();
    const { config } = await initializeGuard({ authority });
    const vault = await initializeVault(authority);

    const configAccount =
      await aegisGuard.account.circuitBreakerConfig.fetch(config);
    const vaultAccount = await demoVault.account.demoVault.fetch(vault);

    assert.strictEqual(configAccount.maxOutflowBps, 1_000);
    assert.strictEqual(vaultAccount.balance.toNumber(), 0);
  });

  it("rejects invalid guard thresholds", async () => {
    const zeroVelocityAuthority = await fundAuthority();
    const zeroVelocityConfig = deriveConfig(zeroVelocityAuthority.publicKey);
    const zeroVelocityState = deriveState(zeroVelocityConfig);

    await expectFailure(
      aegisGuard.methods
        .initializeConfig(0, 1_000, bn(10), false)
        .accountsStrict({
          authority: zeroVelocityAuthority.publicKey,
          config: zeroVelocityConfig,
          state: zeroVelocityState,
          systemProgram,
        })
        .signers([zeroVelocityAuthority])
        .rpc(),
      "InvalidThreshold",
    );

    const excessiveOutflowAuthority = await fundAuthority();
    const excessiveOutflowConfig = deriveConfig(
      excessiveOutflowAuthority.publicKey,
    );
    const excessiveOutflowState = deriveState(excessiveOutflowConfig);

    await expectFailure(
      aegisGuard.methods
        .initializeConfig(200, 10_001, bn(10), false)
        .accountsStrict({
          authority: excessiveOutflowAuthority.publicKey,
          config: excessiveOutflowConfig,
          state: excessiveOutflowState,
          systemProgram,
        })
        .signers([excessiveOutflowAuthority])
        .rpc(),
      "InvalidThreshold",
    );
  });

  it("blocks snapshots while paused", async () => {
    const authority = await fundAuthority();
    const { config } = await initializeGuard({ authority });
    const vault = await initializeVault(authority);

    await aegisGuard.methods
      .updateConfig(200, 1_000, bn(10), false, true)
      .accountsStrict({
        authority: authority.publicKey,
        config,
      })
      .signers([authority])
      .rpc();

    await expectFailure(
      createSnapshot({
        authority,
        config,
        vault,
        preBalance: 1_000,
      }),
      "GuardPaused",
    );
  });

  it("rejects evaluating a snapshot against the wrong vault", async () => {
    const authority = await fundAuthority();
    const { config, state } = await initializeGuard({ authority });
    const vault = await initializeVault(authority);
    const snapshot = await createSnapshot({
      authority,
      config,
      vault,
      preBalance: 1_000,
    });
    const wrongVault = anchor.web3.Keypair.generate().publicKey;

    await expectFailure(
      aegisGuard.methods
        .evaluate(bn(900))
        .accountsStrict({
          protocol: authority.publicKey,
          config,
          authority: authority.publicKey,
          state,
          vault: wrongVault,
          snapshot,
        })
        .signers([authority])
        .rpc(),
      "ConstraintSeeds",
    );
  });

  it("allows outflow exactly at the threshold", async () => {
    const authority = await fundAuthority();
    const { config, state } = await initializeGuard({
      authority,
      maxOutflowBps: 1_000,
    });
    const vault = await initializeVault(authority);
    const snapshot = await createSnapshot({
      authority,
      config,
      vault,
      preBalance: 1_000,
    });

    await aegisGuard.methods
      .evaluate(bn(900))
      .accountsStrict({
        protocol: authority.publicKey,
        config,
        authority: authority.publicKey,
        state,
        vault,
        snapshot,
      })
      .signers([authority])
      .rpc();

    const stateAccount =
      await aegisGuard.account.circuitBreakerState.fetch(state);
    assert.isFalse(stateAccount.isTripped);
  });

  it("records trip metadata without reverting in simulation mode", async () => {
    const authority = await fundAuthority();
    const { config, state } = await initializeGuard({
      authority,
      maxOutflowBps: 1_000,
      isSimulationMode: true,
    });
    const vault = await initializeVault(authority);
    const snapshot = await createSnapshot({
      authority,
      config,
      vault,
      preBalance: 1_000,
    });

    await aegisGuard.methods
      .evaluate(bn(899))
      .accountsStrict({
        protocol: authority.publicKey,
        config,
        authority: authority.publicKey,
        state,
        vault,
        snapshot,
      })
      .signers([authority])
      .rpc();

    const stateAccount =
      await aegisGuard.account.circuitBreakerState.fetch(state);

    assert.isTrue(stateAccount.isTripped);
    assert.deepStrictEqual(stateAccount.tripReason, { vaultOutflow: {} });
    assert.strictEqual(stateAccount.deltaValueAtTrip.toNumber(), 101);
    assert.strictEqual(stateAccount.thresholdAtTrip.toNumber(), 100);
  });

  it("rolls back a protected withdrawal when live evaluation trips", async () => {
    const authority = await fundAuthority();
    const { config, state } = await initializeGuard({
      authority,
      maxOutflowBps: 1_000,
      isSimulationMode: false,
    });
    const vault = await initializeVault(authority);
    const snapshot = deriveSnapshot(config, authority.publicKey, vault);

    await demoVault.methods
      .deposit(bn(1_000))
      .accountsStrict({
        authority: authority.publicKey,
        vault,
      })
      .signers([authority])
      .rpc();

    const snapshotIx = await aegisGuard.methods
      .snapshot(bn(1_000))
      .accountsStrict({
        payer: authority.publicKey,
        protocol: authority.publicKey,
        config,
        authority: authority.publicKey,
        vault,
        snapshot,
        systemProgram,
      })
      .instruction();

    const withdrawIx = await demoVault.methods
      .withdraw(bn(101))
      .accountsStrict({
        authority: authority.publicKey,
        vault,
      })
      .instruction();

    const evaluateIx = await aegisGuard.methods
      .evaluate(bn(899))
      .accountsStrict({
        protocol: authority.publicKey,
        config,
        authority: authority.publicKey,
        state,
        vault,
        snapshot,
      })
      .instruction();

    await expectFailure(
      provider.sendAndConfirm(
        new anchor.web3.Transaction()
          .add(snapshotIx)
          .add(withdrawIx)
          .add(evaluateIx),
        [authority],
      ),
      "CircuitBreakerTripped",
    );

    const vaultAccount = await demoVault.account.demoVault.fetch(vault);
    assert.strictEqual(vaultAccount.balance.toNumber(), 1_000);

    await expectFailure(
      aegisGuard.account.snapshot.fetch(snapshot),
      "Account does not exist",
    );
  });

  it("resets recorded trip state", async () => {
    const authority = await fundAuthority();
    const { config, state } = await initializeGuard({
      authority,
      maxOutflowBps: 1_000,
      isSimulationMode: true,
    });
    const vault = await initializeVault(authority);
    const snapshot = await createSnapshot({
      authority,
      config,
      vault,
      preBalance: 1_000,
    });

    await aegisGuard.methods
      .evaluate(bn(899))
      .accountsStrict({
        protocol: authority.publicKey,
        config,
        authority: authority.publicKey,
        state,
        vault,
        snapshot,
      })
      .signers([authority])
      .rpc();

    await aegisGuard.methods
      .reset()
      .accountsStrict({
        authority: authority.publicKey,
        config,
        state,
      })
      .signers([authority])
      .rpc();

    const stateAccount =
      await aegisGuard.account.circuitBreakerState.fetch(state);

    assert.isFalse(stateAccount.isTripped);
    assert.deepStrictEqual(stateAccount.tripReason, { none: {} });
    assert.strictEqual(stateAccount.trippedAtSlot.toNumber(), 0);
    assert.strictEqual(stateAccount.deltaValueAtTrip.toNumber(), 0);
    assert.strictEqual(stateAccount.thresholdAtTrip.toNumber(), 0);
  });

  it("rejects demo vault withdrawals above available balance", async () => {
    const authority = await fundAuthority();
    const vault = await initializeVault(authority);

    await demoVault.methods
      .deposit(bn(50))
      .accountsStrict({
        authority: authority.publicKey,
        vault,
      })
      .signers([authority])
      .rpc();

    await expectFailure(
      demoVault.methods
        .withdraw(bn(51))
        .accountsStrict({
          authority: authority.publicKey,
          vault,
        })
        .signers([authority])
        .rpc(),
      "InsufficientBalance",
    );

    const vaultAccount = await demoVault.account.demoVault.fetch(vault);
    assert.strictEqual(vaultAccount.balance.toNumber(), 50);
  });

  it("rejects demo vault deposit overflow", async () => {
    const authority = await fundAuthority();
    const vault = await initializeVault(authority);

    await demoVault.methods
      .deposit(bn("18446744073709551615"))
      .accountsStrict({
        authority: authority.publicKey,
        vault,
      })
      .signers([authority])
      .rpc();

    await expectFailure(
      demoVault.methods
        .deposit(bn(1))
        .accountsStrict({
          authority: authority.publicKey,
          vault,
        })
        .signers([authority])
        .rpc(),
      "MathOverflow",
    );
  });
});
