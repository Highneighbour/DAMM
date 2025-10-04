/**
 * Example client for integrating with the Honorary Fee Position program
 * 
 * This demonstrates how to:
 * 1. Initialize an honorary position
 * 2. Call the crank to distribute fees
 * 3. Handle pagination for large investor sets
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { 
  Connection, 
  PublicKey, 
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
} from "@solana/spl-token";

// Import your program IDL
import { HonoraryFeePosition } from "../target/types/honorary_fee_position";
import idl from "../target/idl/honorary_fee_position.json";

const PROGRAM_ID = new PublicKey("HonF33Po5itioN1111111111111111111111111111");

/**
 * Initialize an honorary fee position
 */
export async function initializePosition(
  provider: AnchorProvider,
  poolPubkey: PublicKey,
  poolVaultA: PublicKey,
  poolVaultB: PublicKey,
  quoteMint: PublicKey,
  creatorQuoteAta: PublicKey,
  params: {
    lowerTick: number;
    upperTick: number;
    y0LockedLamports: number;
    investorFeeShareBps: number;
    dailyCapLamports: number;
    minPayoutLamports: number;
    dustThreshold: number;
  }
): Promise<string> {
  const program = new Program(
    idl as any,
    PROGRAM_ID,
    provider
  ) as Program<HonoraryFeePosition>;

  // Derive PDAs
  const [policyPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("policy"), poolPubkey.toBuffer()],
    program.programId
  );

  const [positionOwnerPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("VAULT_SEED"),
      poolPubkey.toBuffer(),
      Buffer.from("investor_fee_pos_owner"),
    ],
    program.programId
  );

  const [positionPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("honorary_pos"),
      poolPubkey.toBuffer(),
      positionOwnerPda.toBuffer(),
    ],
    program.programId
  );

  const treasuryAta = await getAssociatedTokenAddress(
    quoteMint,
    positionOwnerPda,
    true
  );

  console.log("Initializing honorary position...");
  console.log("Pool:", poolPubkey.toBase58());
  console.log("Policy PDA:", policyPda.toBase58());
  console.log("Position Owner PDA:", positionOwnerPda.toBase58());
  console.log("Position PDA:", positionPda.toBase58());
  console.log("Treasury ATA:", treasuryAta.toBase58());

  const tx = await program.methods
    .initializeHonoraryPosition({
      lowerTick: params.lowerTick,
      upperTick: params.upperTick,
      y0LockedLamports: new anchor.BN(params.y0LockedLamports),
      investorFeeShareBps: new anchor.BN(params.investorFeeShareBps),
      dailyCapLamports: new anchor.BN(params.dailyCapLamports),
      minPayoutLamports: new anchor.BN(params.minPayoutLamports),
      dustThreshold: new anchor.BN(params.dustThreshold),
    })
    .accounts({
      payer: provider.wallet.publicKey,
      policy: policyPda,
      positionOwnerPda: positionOwnerPda,
      pool: poolPubkey,
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

  console.log("✅ Position initialized! Tx:", tx);
  return tx;
}

/**
 * Crank distribution for a single page of investors
 */
export async function crankDistribute(
  provider: AnchorProvider,
  poolPubkey: PublicKey,
  investors: Array<{
    streamAccount: PublicKey;
    investorQuoteAta: PublicKey;
  }>,
  expectedCursor: number,
  isFinalPage: boolean
): Promise<string> {
  const program = new Program(
    idl as any,
    PROGRAM_ID,
    provider
  ) as Program<HonoraryFeePosition>;

  // Derive PDAs
  const [policyPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("policy"), poolPubkey.toBuffer()],
    program.programId
  );

  const policyAccount = await program.account.policyConfig.fetch(policyPda);

  const now = Math.floor(Date.now() / 1000);
  const dayTs = Math.floor(now / 86400) * 86400;

  const [progressPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("progress"),
      poolPubkey.toBuffer(),
      Buffer.from(dayTs.toString()),
    ],
    program.programId
  );

  const [positionOwnerPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("VAULT_SEED"),
      poolPubkey.toBuffer(),
      Buffer.from("investor_fee_pos_owner"),
    ],
    program.programId
  );

  console.log("Cranking distribution...");
  console.log("Pool:", poolPubkey.toBase58());
  console.log("Investors:", investors.length);
  console.log("Expected cursor:", expectedCursor);
  console.log("Is final page:", isFinalPage);

  // Build remaining accounts: [stream1, stream2, ..., ata1, ata2, ...]
  const remainingAccounts = [
    ...investors.map(inv => ({
      pubkey: inv.streamAccount,
      isSigner: false,
      isWritable: false,
    })),
    ...investors.map(inv => ({
      pubkey: inv.investorQuoteAta,
      isSigner: false,
      isWritable: true,
    })),
  ];

  const tx = await program.methods
    .crankDistribute({
      investors: investors,
      expectedCursor: new anchor.BN(expectedCursor),
      isFinalPage: isFinalPage,
    })
    .accounts({
      caller: provider.wallet.publicKey,
      policy: policyPda,
      progress: progressPda,
      positionOwnerPda: positionOwnerPda,
      position: policyAccount.position,
      treasuryAta: policyAccount.treasuryAta,
      creatorQuoteAta: policyAccount.creatorQuoteAta,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .remainingAccounts(remainingAccounts)
    .rpc();

  console.log("✅ Crank completed! Tx:", tx);
  return tx;
}

/**
 * Crank with automatic pagination
 * Splits large investor list into pages and processes sequentially
 */
export async function crankDistributeWithPagination(
  provider: AnchorProvider,
  poolPubkey: PublicKey,
  allInvestors: Array<{
    streamAccount: PublicKey;
    investorQuoteAta: PublicKey;
  }>,
  pageSize: number = 10
): Promise<string[]> {
  const txs: string[] = [];
  let cursor = 0;

  // Split investors into pages
  const pages: typeof allInvestors[] = [];
  for (let i = 0; i < allInvestors.length; i += pageSize) {
    pages.push(allInvestors.slice(i, i + pageSize));
  }

  console.log(`Processing ${allInvestors.length} investors in ${pages.length} pages...`);

  for (let i = 0; i < pages.length; i++) {
    const page = pages[i];
    const isFinalPage = i === pages.length - 1;

    console.log(`\nPage ${i + 1}/${pages.length}`);
    
    const tx = await crankDistribute(
      provider,
      poolPubkey,
      page,
      cursor,
      isFinalPage
    );

    txs.push(tx);
    cursor += page.length;

    // Wait a bit between pages
    if (!isFinalPage) {
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }

  console.log(`\n✅ All ${pages.length} pages processed!`);
  return txs;
}

/**
 * Query policy configuration
 */
export async function getPolicy(
  provider: AnchorProvider,
  poolPubkey: PublicKey
): Promise<any> {
  const program = new Program(
    idl as any,
    PROGRAM_ID,
    provider
  ) as Program<HonoraryFeePosition>;

  const [policyPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("policy"), poolPubkey.toBuffer()],
    program.programId
  );

  return await program.account.policyConfig.fetch(policyPda);
}

/**
 * Query distribution progress for today
 */
export async function getProgress(
  provider: AnchorProvider,
  poolPubkey: PublicKey
): Promise<any> {
  const program = new Program(
    idl as any,
    PROGRAM_ID,
    provider
  ) as Program<HonoraryFeePosition>;

  const now = Math.floor(Date.now() / 1000);
  const dayTs = Math.floor(now / 86400) * 86400;

  const [progressPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("progress"),
      poolPubkey.toBuffer(),
      Buffer.from(dayTs.toString()),
    ],
    program.programId
  );

  try {
    return await program.account.distributionProgress.fetch(progressPda);
  } catch (err) {
    // Progress account doesn't exist yet (first crank not called today)
    return null;
  }
}

// Example usage
async function main() {
  // Setup connection and provider
  const connection = new Connection("http://localhost:8899", "confirmed");
  const wallet = Wallet.local(); // Or load your wallet
  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });

  // Example pool and mint addresses (replace with actual)
  const poolPubkey = new PublicKey("YourPoolPubkeyHere");
  const quoteMint = new PublicKey("YourQuoteMintHere");
  const poolVaultA = new PublicKey("YourVaultAPubkeyHere");
  const poolVaultB = new PublicKey("YourVaultBPubkeyHere");
  const creatorQuoteAta = new PublicKey("YourCreatorAtaHere");

  // 1. Initialize position
  const initTx = await initializePosition(
    provider,
    poolPubkey,
    poolVaultA,
    poolVaultB,
    quoteMint,
    creatorQuoteAta,
    {
      lowerTick: -100,
      upperTick: 100,
      y0LockedLamports: 1_000_000_000, // 1000 tokens (6 decimals)
      investorFeeShareBps: 7_000, // 70%
      dailyCapLamports: 0, // No cap
      minPayoutLamports: 1_000, // 0.001 tokens
      dustThreshold: 100,
    }
  );

  console.log("\n=== Position initialized ===\n");

  // 2. Query policy
  const policy = await getPolicy(provider, poolPubkey);
  console.log("Policy:", policy);

  // 3. Crank distribution (example with 2 investors)
  const investors = [
    {
      streamAccount: new PublicKey("StreamAccount1Pubkey"),
      investorQuoteAta: new PublicKey("InvestorAta1Pubkey"),
    },
    {
      streamAccount: new PublicKey("StreamAccount2Pubkey"),
      investorQuoteAta: new PublicKey("InvestorAta2Pubkey"),
    },
  ];

  const crankTx = await crankDistribute(
    provider,
    poolPubkey,
    investors,
    0, // First page, cursor = 0
    true // Final page
  );

  console.log("\n=== Distribution cranked ===\n");

  // 4. Query progress
  const progress = await getProgress(provider, poolPubkey);
  console.log("Progress:", progress);
}

// Uncomment to run
// main().catch(console.error);

export {
  initializePosition,
  crankDistribute,
  crankDistributeWithPagination,
  getPolicy,
  getProgress,
};
