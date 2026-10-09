# Testnet deployment

> **Status: deployed and verified on Stellar testnet.** The IDs below come from a
> real deploy via
> [`scripts/deploy-testnet.sh`](https://github.com/PulseRun-Labs/pulserun-core/blob/main/scripts/deploy-testnet.sh)
> and are reproducible from `main`. `MockToken` is test-only — never deploy it as
> real money.

## Deployed contracts

| Contract | Network | Contract ID | Stellar Expert |
| --- | --- | --- | --- |
| `PulseEscrow` | testnet | `CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB` | [view](https://stellar.expert/explorer/testnet/contract/CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB) |
| `MockToken` | testnet | `CCCNLWIQ2MZVXDNC2BTTHNGFRBP65EEJHEHCNFR5TUSLTLVDYO6HHVF6` | [view](https://stellar.expert/explorer/testnet/contract/CCCNLWIQ2MZVXDNC2BTTHNGFRBP65EEJHEHCNFR5TUSLTLVDYO6HHVF6) |

| Setting | Value |
| --- | --- |
| Admin | `GDBDQRK2AJM2CTJ3HPYWS33E3OET5J5ZTU5Q45V34VYHAWIURQBAKZEA` |
| Dispute window | `60` seconds (shortened for demo; production default is `86400`) |
| WASM hash (`pulserun_escrow`) | `79b505b1329f421f2355e0002d81dccc0a52a3b3cd68d9439876ce083f8db2dd` |
| WASM hash (`mock_token`) | `6764571d73cabb024b39b6376201a68b44cea3176fab5c461385c9d4088d511c` |

The machine-readable record is
[`testnet.env`](testnet.env); it is regenerated on each deploy.

## Initialization

The escrow was initialized with:

```bash
stellar contract invoke \
  --id CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB \
  --network testnet --source pulserun-deployer \
  -- init \
  --admin GDBDQRK2AJM2CTJ3HPYWS33E3OET5J5ZTU5Q45V34VYHAWIURQBAKZEA \
  --dispute_window_secs 60
```

Read back: `admin` → the admin address above; `dispute_window` → `60`.

## Verified end-to-end settlement

Run after deploy, over the public testnet RPC, to prove the full flow:

1. Minted `1000` `MockToken` to the requester.
2. `create_job` with `max_budget = 1000`, `rate_per_second = 10`,
   `max_duration_secs = 100` → `job_id = 1`. Contract token balance became
   `1000` (collateral locked).
3. `submit_proof` (by the runner) with `duration_secs = 20`, `exit_code = 0`,
   `output_hash = 0x0707…07`. Job moved to `Completed`; `output_hash` and
   `completed_at` were recorded.
   tx: [`0139ecfe…`](https://stellar.expert/explorer/testnet/tx/0139ecfe545a053e59904dfedd705599a80fbf8bef79c2ca867eb7c9187f15b8)
4. `claim_payout` **before** the window closed → rejected with
   `Error #11` (`DisputeWindowActive`). This is the expected, correct behavior.
5. `claim_payout` **after** the window → returned `200`
   (= `rate_per_second * duration_secs` = `10 * 20`).
   tx: [`a6c17ad8…`](https://stellar.expert/explorer/testnet/tx/a6c17ad89e7d6166ea82c438da974da7cef547e3a103915c394268156ac8b5a0)

Final balances confirm the settlement split exactly:

| Account | Balance |
| --- | --- |
| Runner | `200` (earnings) |
| Requester | `800` (unspent refund) |
| Escrow | `0` (fully drained) |

`200 + 800 = 1000 = max_budget`, and the escrow holds nothing — the core
invariant, observed on-chain.

## Mainnet

Do **not** reuse `MockToken` on mainnet. Deploy `pulserun_escrow` and pass the
SEP-41 token or the Stellar Asset Contract (SAC) of the asset you settle in
(e.g. USDC on Stellar) as each job's `payment_token`. Raise `dispute_window_secs`
to a production value. Use `stellar contract build` for the canonical artifact.
