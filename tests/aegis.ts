import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";

import { AegisGuard } from "../target/types/aegis_guard";
import { DemoVault } from "../target/types/demo_vault";

describe("aegis", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const aegisGuard = anchor.workspace.AegisGuard as Program<AegisGuard>;
  const demoVault = anchor.workspace.DemoVault as Program<DemoVault>;

  it("initializes both programs", async () => {
    const authority = provider.wallet.publicKey;

    const [config] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("config"), authority.toBuffer()],
      aegisGuard.programId,
    );
    const [state] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state"), config.toBuffer()],
      aegisGuard.programId,
    );
    const [vault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("demo-vault"), authority.toBuffer()],
      demoVault.programId,
    );

    await aegisGuard.methods
      .initializeConfig(200, 1_000, new anchor.BN(10), false)
      .rpc();

    await demoVault.methods.initialize().rpc();

    const configAccount = await aegisGuard.account.circuitBreakerConfig.fetch(
      config,
    );
    const vaultAccount = await demoVault.account.demoVault.fetch(vault);

    assert.strictEqual(configAccount.maxOutflowBps, 1_000);
    assert.strictEqual(vaultAccount.balance.toNumber(), 0);
  });
});
