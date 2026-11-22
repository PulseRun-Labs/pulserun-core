# Testnet deployment

> **Status: the v0.1.0 contract below is live and historically verified.** The
> current **v0.3.0** build has not been redeployed yet. v0.3.0 settles in a
> **Stellar Asset Contract (SAC)**, whereas the v0.1.0 instance settled in the
> retired test-only token that was removed from the tree. Run
> [`scripts/deploy-testnet.sh`](https://github.com/PulseRun-Labs/pulserun-core/blob/main/scripts/deploy-testnet.sh)
> with your own funded identity to create a current instance; the script
> resolves the native XLM SAC and writes the new record to
> [`testnet.env`](testnet.env). The transaction hashes in the historical section
> remain valid evidence of the original settlement flow.

## Historical v0.1.0 deployment

| Contract | Network | Contract ID | Stellar Expert |
| --- | --- | --- | --- |
| `PulseEscrow` (v0.1.0) | testnet | `CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB` | [view](https://stellar.expert/explorer/testnet/contract/CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB) |
| Retired test-only token | testnet | `CCCNLWIQ2MZVXDNC2BTTHNGFRBP65EEJHEHCNFR5TUSLTLVDYO6HHVF6` | [view](https://stellar.expert/explorer/testnet/contract/CCCNLWIQ2MZVXDNC2BTTHNGFRBP65EEJHEHCNFR5TUSLTLVDYO6HHVF6) |

| Setting | Value |
| --- | --- |
| Admin | `GDBDQRK2AJM2CTJ3HPYWS33E3OET5J5ZTU5Q45V34VYHAWIURQBAKZEA` |
| Dispute window | `60` seconds (shortened for the demo; production default is `86400`) |
| WASM hash (`pulserun_escrow`) | `79b505b1329f421f2355e0002d81dccc0a52a3b3cd68d9439876ce083f8db2dd` |
| WASM hash (retired token) | `6764571d73cabb024b39b6376201a68b44cea3176fab5c461385c9d4088d511c` |

The machine-readable record is [`testnet.env`](testnet.env). It is marked as a
legacy record; each new deploy rewrites it in the SAC format.

### Initialization

The v0.1.0 escrow was initialized with:

```bash
stellar contract invoke \
  --id CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB \
  --network testnet --source pulserun-deployer \
  -- init \
  --admin GDBDQRK2AJM2CTJ3HPYWS33E3OET5J5ZTU5Q45V34VYHAWIURQBAKZEA \
  --dispute_window_secs 60
```

Read back: `admin` → the admin address above; `dispute_window` → `60`.

### Verified end-to-end settlement (v0.1.0)

1. Minted `1000` units of the retired test-only token to the requester.
2. `create_job` with `max_budget = 1000`, `rate_per_second = 10`,
   `max_duration_secs = 100` → `job_id = 1`. The contract's token balance became
   `1000` (collateral locked).
3. `submit_proof` (by the runner) with `duration_secs = 20`, `exit_code = 0`,
   `output_hash = 0x0707…07`. The job moved to `Completed`; `output_hash` and
   `completed_at` were recorded.
   tx: [`0139ecfe…`](https://stellar.expert/explorer/testnet/tx/0139ecfe545a053e59904dfedd705599a80fbf8bef79c2ca867eb7c9187f15b8)
4. `claim_payout` **before** the window closed → rejected with
   `Error #11` (`DisputeWindowActive`). This is the expected, correct behavior.
5. `claim_payout` **after** the window → returned `200`
   (= `rate_per_second * duration_secs` = `10 * 20`).
   tx: [`a6c17ad8…`](https://stellar.expert/explorer/testnet/tx/a6c17ad89e7d6166ea82c438da974da7cef547e3a103915c394268156ac8b5a0)

Final balances confirmed the settlement split exactly:

| Account | Balance |
| --- | --- |
| Runner | `200` (earnings) |
| Requester | `800` (unspent refund) |
| Escrow | `0` (fully drained) |

`200 + 800 = 1000 = max_budget`, and the escrow held nothing — the core
invariant, observed on-chain.

## Current build (v0.3.0): settle in a Stellar Asset Contract

```bash
PAYMENT_ASSET=native DISPUTE_WINDOW_SECS=86400 ./scripts/deploy-testnet.sh
```

The script:

1. Builds with `stellar contract build` (the supported Soroban build path).
2. Creates and funds the deployer identity on testnet if needed.
3. Resolves the SAC address for `PAYMENT_ASSET` with
   `stellar contract id asset` — no token contract is deployed.
4. Deploys `pulserun_escrow`, calls `init`, and writes
   `PAYMENT_ASSET` / `PAYMENT_TOKEN_ID` / `PULSEESCROW_ID` to
   [`testnet.env`](testnet.env).

Use `PAYMENT_ASSET="CODE:ISSUER"` to settle in an issued Stellar asset instead of
XLM. The native XLM SAC id on testnet (and on every Stellar network) is:

```text
CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

After deploying, exercise `create_job` → `submit_proof` → `dispute_job` →
`resolve_dispute` (or let the window elapse and call `claim_payout`) and confirm
the final SAC balances. Record the new contract id and wasm hash above; do not
mix ids from different builds.

## Mainnet

Deploy `pulserun_escrow` with `stellar contract build` and pass the **SAC of a
trusted asset** (for example native XLM or USDC on Stellar) as each job's
`payment_token`. Raise `dispute_window_secs` to a production value and use a
multisignature or governed admin account. There is no test token to retire: the
escrow only ever calls the SEP-41 interface of the SAC it is given.
