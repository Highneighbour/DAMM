import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { HonoraryFeePosition } from "../target/types/honorary_fee_position";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram, 
  SYSVAR_RENT_PUBKEY,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert } from "chai";

describe("honorary_fee_position", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.HonoraryFeePosition as Program<HonoraryFeePosition>;

  let quoteMint: PublicKey;
  let baseMint: PublicKey;
  let pool: Keypair;
  let poolVaultA: PublicKey;
  let poolVaultB: PublicKey;
  let creator: Keypair;
  let creatorQuoteAta: PublicKey;
  let policyPda: PublicKey;
  let positionOwnerPda: PublicKey;
  let treasuryAta: PublicKey;
  let positionPda: PublicKey;

  before(async () => {
    // Create mints
    quoteMint = await createMint(
      provider.connection,
      provider.wallet.payer,
      provider.wallet.publicKey,
      null,
      6
    );

    baseMint = await createMint(
      provider.connection,
      provider.wallet.payer,
      provider.wallet.publicKey,
      null,
      6
    );

    // Create pool keypair
    pool = Keypair.generate();

    // Create pool vaults
    poolVaultA = await createAccount(
      provider.connection,
      provider.wallet.payer,
      baseMint,
      pool.publicKey
    );

    poolVaultB = await createAccount(
      provider.connection,
      provider.wallet.payer,
      quoteMint,
      pool.publicKey
    );

    // Create creator and ATA
    creator = Keypair.generate();
    const creatorAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      quoteMint,
      creator.publicKey
    );
    creatorQuoteAta = creatorAta.address;

    // Derive PDAs
    [policyPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), pool.publicKey.toBuffer()],
      program.programId
    );

    [positionOwnerPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("VAULT_SEED"),
        pool.publicKey.toBuffer(),
        Buffer.from("investor_fee_pos_owner"),
      ],
      program.programId
    );

    [positionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("honorary_pos"),
        pool.publicKey.toBuffer(),
        positionOwnerPda.toBuffer(),
      ],
      program.programId
    );

    // Derive treasury ATA
    [treasuryAta] = PublicKey.findProgramAddressSync(
      [
        positionOwnerPda.toBuffer(),
        TOKEN_PROGRAM_ID.toBuffer(),
        quoteMint.toBuffer(),
      ],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
  });

  it("Initializes honorary position successfully", async () => {
    const params = {
      lowerTick: -100,
      upperTick: 100,
      y0LockedLamports: new anchor.BN(1_000_000_000), // 1000 tokens
      investorFeeShareBps: new anchor.BN(7_000), // 70%
      dailyCapLamports: new anchor.BN(0), // No cap
      minPayoutLamports: new anchor.BN(1_000), // 0.001 tokens
      dustThreshold: new anchor.BN(100),
    };

    const tx = await program.methods
      .initializeHonoraryPosition(params)
      .accounts({
        payer: provider.wallet.publicKey,
        policy: policyPda,
        positionOwnerPda: positionOwnerPda,
        pool: pool.publicKey,
        poolVaultA: poolVaultA,
        poolVaultB: poolVaultB,
        quoteMint: quoteMint,
        treasuryAta: treasuryAta,
        creatorQuoteAta: creatorQuoteAta,
        position: positionPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Initialize tx:", tx);

    // Verify policy account
    const policyAccount = await program.account.policyConfig.fetch(policyPda);
    assert.equal(policyAccount.pool.toBase58(), pool.publicKey.toBase58());
    assert.equal(policyAccount.quoteMint.toBase58(), quoteMint.toBase58());
    assert.equal(
      policyAccount.y0LockedLamports.toNumber(),
      params.y0LockedLamports.toNumber()
    );
    assert.equal(
      policyAccount.investorFeeShareBps.toNumber(),
      params.investorFeeShareBps.toNumber()
    );
  });

  it("Fails initialization with invalid tick bounds", async () => {
    const pool2 = Keypair.generate();
    const [policyPda2] = PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), pool2.publicKey.toBuffer()],
      program.programId
    );

    const [positionOwnerPda2] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("VAULT_SEED"),
        pool2.publicKey.toBuffer(),
        Buffer.from("investor_fee_pos_owner"),
      ],
      program.programId
    );

    const [positionPda2] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("honorary_pos"),
        pool2.publicKey.toBuffer(),
        positionOwnerPda2.toBuffer(),
      ],
      program.programId
    );

    const [treasuryAta2] = PublicKey.findProgramAddressSync(
      [
        positionOwnerPda2.toBuffer(),
        TOKEN_PROGRAM_ID.toBuffer(),
        quoteMint.toBuffer(),
      ],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const params = {
      lowerTick: 100,
      upperTick: -100, // Invalid: lower > upper
      y0LockedLamports: new anchor.BN(1_000_000_000),
      investorFeeShareBps: new anchor.BN(7_000),
      dailyCapLamports: new anchor.BN(0),
      minPayoutLamports: new anchor.BN(1_000),
      dustThreshold: new anchor.BN(100),
    };

    try {
      await program.methods
        .initializeHonoraryPosition(params)
        .accounts({
          payer: provider.wallet.publicKey,
          policy: policyPda2,
          positionOwnerPda: positionOwnerPda2,
          pool: pool2.publicKey,
          poolVaultA: poolVaultA,
          poolVaultB: poolVaultB,
          quoteMint: quoteMint,
          treasuryAta: treasuryAta2,
          creatorQuoteAta: creatorQuoteAta,
          position: positionPda2,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();
      assert.fail("Should have failed with invalid tick bounds");
    } catch (err) {
      assert.include(err.toString(), "InvalidTickBounds");
    }
  });

  describe("Crank distribution", () => {
    let investor1: Keypair;
    let investor2: Keypair;
    let investor1Ata: PublicKey;
    let investor2Ata: PublicKey;
    let streamAccount1: Keypair;
    let streamAccount2: Keypair;

    before(async () => {
      // Create investors and their ATAs
      investor1 = Keypair.generate();
      investor2 = Keypair.generate();

      const ata1 = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        provider.wallet.payer,
        quoteMint,
        investor1.publicKey
      );
      investor1Ata = ata1.address;

      const ata2 = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        provider.wallet.payer,
        quoteMint,
        investor2.publicKey
      );
      investor2Ata = ata2.address;

      // Create mock Streamflow stream accounts
      streamAccount1 = Keypair.generate();
      streamAccount2 = Keypair.generate();

      // Initialize mock stream accounts with locked amounts
      // Format: [discriminator: 8][deposited: 8][start_ts: 8][end_ts: 8]
      const now = Math.floor(Date.now() / 1000);
      const streamData1 = Buffer.alloc(40);
      streamData1.writeBigUInt64LE(BigInt(600_000_000), 8); // 600 tokens deposited
      streamData1.writeBigInt64LE(BigInt(now - 86400), 16); // Started yesterday
      streamData1.writeBigInt64LE(BigInt(now + 86400 * 365), 24); // Ends in 1 year

      const streamData2 = Buffer.alloc(40);
      streamData2.writeBigUInt64LE(BigInt(400_000_000), 8); // 400 tokens deposited
      streamData2.writeBigInt64LE(BigInt(now - 86400), 16);
      streamData2.writeBigInt64LE(BigInt(now + 86400 * 365), 24);

      // Create stream accounts
      const rentExemption = await provider.connection.getMinimumBalanceForRentExemption(40);
      
      const createStreamTx1 = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: streamAccount1.publicKey,
          lamports: rentExemption,
          space: 40,
          programId: program.programId,
        })
      );
      await sendAndConfirmTransaction(provider.connection, createStreamTx1, [
        provider.wallet.payer,
        streamAccount1,
      ]);

      const createStreamTx2 = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: streamAccount2.publicKey,
          lamports: rentExemption,
          space: 40,
          programId: program.programId,
        })
      );
      await sendAndConfirmTransaction(provider.connection, createStreamTx2, [
        provider.wallet.payer,
        streamAccount2,
      ]);

      // Write stream data
      await provider.connection.confirmTransaction(
        await provider.connection.sendTransaction(
          new Transaction().add({
            keys: [
              { pubkey: streamAccount1.publicKey, isSigner: false, isWritable: true },
            ],
            programId: program.programId,
            data: streamData1,
          }),
          [provider.wallet.payer]
        )
      );

      // Simulate fees by minting to treasury
      await mintTo(
        provider.connection,
        provider.wallet.payer,
        quoteMint,
        treasuryAta,
        provider.wallet.publicKey,
        1_000_000_000 // 1000 tokens in fees
      );
    });

    it("Successfully distributes fees to investors", async () => {
      const now = Math.floor(Date.now() / 1000);
      const dayTs = Math.floor(now / 86400) * 86400;

      const [progressPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("progress"),
          pool.publicKey.toBuffer(),
          Buffer.from(dayTs.toString()),
        ],
        program.programId
      );

      const params = {
        investors: [
          {
            streamAccount: streamAccount1.publicKey,
            investorQuoteAta: investor1Ata,
          },
          {
            streamAccount: streamAccount2.publicKey,
            investorQuoteAta: investor2Ata,
          },
        ],
        expectedCursor: new anchor.BN(0),
        isFinalPage: true,
      };

      // Get balances before
      const treasuryBefore = await getAccount(provider.connection, treasuryAta);
      const investor1Before = await getAccount(provider.connection, investor1Ata);
      const investor2Before = await getAccount(provider.connection, investor2Ata);
      const creatorBefore = await getAccount(provider.connection, creatorQuoteAta);

      console.log("Treasury before:", treasuryBefore.amount.toString());
      console.log("Investor1 before:", investor1Before.amount.toString());
      console.log("Investor2 before:", investor2Before.amount.toString());
      console.log("Creator before:", creatorBefore.amount.toString());

      const tx = await program.methods
        .crankDistribute(params)
        .accounts({
          caller: provider.wallet.publicKey,
          policy: policyPda,
          progress: progressPda,
          positionOwnerPda: positionOwnerPda,
          position: positionPda,
          treasuryAta: treasuryAta,
          creatorQuoteAta: creatorQuoteAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts([
          { pubkey: streamAccount1.publicKey, isSigner: false, isWritable: false },
          { pubkey: streamAccount2.publicKey, isSigner: false, isWritable: false },
          { pubkey: investor1Ata, isSigner: false, isWritable: true },
          { pubkey: investor2Ata, isSigner: false, isWritable: true },
        ])
        .rpc();

      console.log("Crank tx:", tx);

      // Get balances after
      const treasuryAfter = await getAccount(provider.connection, treasuryAta);
      const investor1After = await getAccount(provider.connection, investor1Ata);
      const investor2After = await getAccount(provider.connection, investor2Ata);
      const creatorAfter = await getAccount(provider.connection, creatorQuoteAta);

      console.log("Treasury after:", treasuryAfter.amount.toString());
      console.log("Investor1 after:", investor1After.amount.toString());
      console.log("Investor2 after:", investor2After.amount.toString());
      console.log("Creator after:", creatorAfter.amount.toString());

      // Verify distributions occurred
      assert.isTrue(investor1After.amount > investor1Before.amount, "Investor1 should receive payout");
      assert.isTrue(investor2After.amount > investor2Before.amount, "Investor2 should receive payout");
      assert.isTrue(creatorAfter.amount > creatorBefore.amount, "Creator should receive remainder");
      assert.equal(treasuryAfter.amount, BigInt(0), "Treasury should be empty");
    });
  });
});
