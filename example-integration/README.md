# Example Integration

This directory contains TypeScript client code demonstrating how to integrate with the Honorary Fee Position program.

## Files

- `client.ts` - Main client library with helper functions
- `README.md` - This file

## Usage

### 1. Install Dependencies

```bash
cd /workspace
yarn install
```

### 2. Build the Program

```bash
anchor build
```

### 3. Import in Your Project

```typescript
import {
  initializePosition,
  crankDistribute,
  crankDistributeWithPagination,
  getPolicy,
  getProgress,
} from "./example-integration/client";
```

### 4. Initialize a Position

```typescript
const tx = await initializePosition(
  provider,
  poolPubkey,
  poolVaultA,
  poolVaultB,
  quoteMint,
  creatorQuoteAta,
  {
    lowerTick: -100,
    upperTick: 100,
    y0LockedLamports: 1_000_000_000,
    investorFeeShareBps: 7_000,
    dailyCapLamports: 0,
    minPayoutLamports: 1_000,
    dustThreshold: 100,
  }
);
```

### 5. Crank Distribution

#### Single Page

```typescript
const investors = [
  {
    streamAccount: streamPubkey1,
    investorQuoteAta: ataPubkey1,
  },
  {
    streamAccount: streamPubkey2,
    investorQuoteAta: ataPubkey2,
  },
];

const tx = await crankDistribute(
  provider,
  poolPubkey,
  investors,
  0, // cursor
  true // is final page
);
```

#### Automatic Pagination

```typescript
const allInvestors = [/* ... large list ... */];

const txs = await crankDistributeWithPagination(
  provider,
  poolPubkey,
  allInvestors,
  10 // page size
);
```

### 6. Query State

```typescript
// Get policy configuration
const policy = await getPolicy(provider, poolPubkey);
console.log("Y0:", policy.y0LockedLamports.toString());
console.log("Investor share:", policy.investorFeeShareBps.toString());

// Get today's progress
const progress = await getProgress(provider, poolPubkey);
if (progress) {
  console.log("Cursor:", progress.cursor.toString());
  console.log("Cumulative distributed:", progress.cumulativeDistributed.toString());
  console.log("Day completed:", progress.dayCompleted);
}
```

## Key Functions

### `initializePosition()`

Creates an honorary fee position with specified parameters.

**Parameters:**
- `provider`: Anchor provider
- `poolPubkey`: cp-amm pool address
- `poolVaultA`, `poolVaultB`: Pool token vaults
- `quoteMint`: Quote token mint
- `creatorQuoteAta`: Creator's ATA for receiving remainder
- `params`: Configuration object (ticks, Y0, caps, etc.)

**Returns:** Transaction signature

### `crankDistribute()`

Calls the crank for a single page of investors.

**Parameters:**
- `provider`: Anchor provider
- `poolPubkey`: Pool address
- `investors`: Array of `{streamAccount, investorQuoteAta}`
- `expectedCursor`: Current cursor (for idempotency)
- `isFinalPage`: Whether this is the last page

**Returns:** Transaction signature

### `crankDistributeWithPagination()`

Automatically splits large investor list into pages and cranks sequentially.

**Parameters:**
- `provider`: Anchor provider
- `poolPubkey`: Pool address
- `allInvestors`: Full investor list
- `pageSize`: Investors per page (default: 10)

**Returns:** Array of transaction signatures

### `getPolicy()`

Fetches the policy configuration account.

**Returns:** Policy account data

### `getProgress()`

Fetches today's distribution progress.

**Returns:** Progress account data or `null` if not yet initialized

## Notes

- Replace placeholder pubkeys with actual addresses
- Ensure wallet has sufficient SOL for transaction fees
- For production, add error handling and retries
- Consider using connection pools for high-throughput cranking
- Monitor events for distribution confirmation

## Example Output

```
Initializing honorary position...
Pool: PoolPubkey123...
Policy PDA: PolicyPDA456...
✅ Position initialized! Tx: 5Txn1...

Cranking distribution...
Investors: 2
Expected cursor: 0
Is final page: true
✅ Crank completed! Tx: 5Txn2...

Progress: {
  cursor: 2,
  cumulativeDistributed: 700000000,
  carry: 0,
  dayCompleted: true
}
```
