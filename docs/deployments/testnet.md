# Testnet deployment

> **Status: v0.3.0 is deployed and verified on Stellar testnet.** It settles in
> the **native XLM Stellar Asset Contract (SAC)** — there is no custom token in
> the deployment. A full `create_job` → `submit_proof` → `claim_payout` flow was
> executed on the public network and the escrow drained to zero.

## Current v0.3.0 deployment

| Contract | Network | Contract ID | Stellar Expert |
| --- | --- | --- | --- |
| `PulseEscrow` (v0.3.0) | testnet | `CAPNQ4LWFOVVJG76UHGAOUT4RL4OH2LJKQOY36KYU5JQ7SEJ5F4DSI2D` | [view](https://stellar.expert/explorer/testnet/contract/CAPNQ4LWFOVVJG76UHGAOUT4RL4OH2LJKQOY36KYU5JQ7SEJ5F4DSI2D) |
| Native XLM SAC | testnet | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` | [view](https://stellar.expert/explorer/testnet/contract/CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC) |

| Setting | Value |
| --- | --- |
| Admin | `GAFRQAM4VZDKUY5TL4PJFFLCAN5E7PPDXP3QCELHHMBP4QZZY54Q7IQP` |
| Dispute window | `60` seconds (shortened for the demo; production default is `86400`) |
| Payment asset | `native` (XLM), resolved to the SAC id above |
| WASM hash (`pulserun_escrow`) | `ac2afe1fb2c4e037cb781b3827ea0adc11c1edd34578f7245205603c54e69417` |
| Upload tx | [`2e70d0c3…`](https://stellar.expert/explorer/testnet/tx/2e70d0c3b58ef9ced3d8ee39d1411bc538cc92b466618a4598fcf33eeee44c79) |
| Deploy tx | [`b25f75e1…`](https://stellar.expert/explorer/testnet/tx/b25f75e148580888819affd050a015db9fafdfcfa06a946a82db26419cc49c8d) |
| `init` tx | [`c20632f2…`](https://stellar.expert/explorer/testnet/tx/c20632f263f493b80db7de868342e02bb66d5f9fdd78a42cd6bff4b0a0e276f0) |

The machine-readable record is [`testnet.env`](testnet.env).

### Initialization

```bash
stellar contract invoke \
  --id CAPNQ4LWFOVVJG76UHGAOUT4RL4OH2LJKQOY36KYU5JQ7SEJ5F4DSI2D \
  --network testnet --source pulserun-deployer \
  -- init \
  --admin GAFRQAM4VZDKUY5TL4PJFFLCAN5E7PPDXP3QCELHHMBP4QZZY54Q7IQP \
  --dispute_window_secs 60
```

Read back: `admin` → the admin address above; `dispute_window` → `60`;
`get_config` → `[admin, 60, 0]`.

### Verified end-to-end settlement (v0.3.0)

1. `create_job` with `max_budget = 10_000_000` stroops (1 XLM),
   `rate_per_second = 100_000`, `max_duration_secs = 100`, and
   `payment_token` = the native XLM SAC → `job_id = 1`. The SAC moved
   `10_000_000` stroops from the requester into the escrow.
   tx: [`b86876c8…`](https://stellar.expert/explorer/testnet/tx/b86876c8e30c0916ff851908cdf57b942aa564bc25ffd0de731454b24e3c2010)
2. `submit_proof` by the runner with `duration_secs = 20`, `exit_code = 0`,
   `output_hash = 0x0707…07`. The job moved to `Completed` and the 60-second
   dispute window opened.
   tx: [`8b98b93a…`](https://stellar.expert/explorer/testnet/tx/8b98b93a8cb932d16b631dcc6933d419e7064941d485c516d0462e65bd94d5eb)
3. `claim_payout` **before** the window closed → rejected with `Error #11`
   (`DisputeWindowActive`). This is the expected behavior.
4. `claim_payout` **after** the window → returned `2_000_000` stroops
   (= `rate_per_second * duration_secs` = `100_000 * 20`). The escrow also
   refunded `8_000_000` stroops to the requester and the job became `Settled`.
   tx: [`7a51c15c…`](https://stellar.expert/explorer/testnet/tx/7a51c15c03d1dfcbf959baee7d570124da39c307d35a4b9378ec38e5d89820ef)

Final balances for the job:

| Party | Amount |
| --- | --- |
| Runner | `2_000_000` stroops earnings |
| Requester | `8_000_000` stroops refund |
| Escrow | `0` |

`2_000_000 + 8_000_000 = 10_000_000 = max_budget`, and the escrow holds
nothing — the core invariant, observed on the public network against the real
SAC.

## Redeploying

```bash
PAYMENT_ASSET=native DISPUTE_WINDOW_SECS=86400 ./scripts/deploy-testnet.sh
```

The script builds with `stellar contract build`, creates/funds the deployer
identity if needed, resolves the SAC for `PAYMENT_ASSET`, deploys
`pulserun_escrow`, calls `init`, and rewrites [`testnet.env`](testnet.env).
Use `PAYMENT_ASSET="CODE:ISSUER"` to settle in an issued Stellar asset instead
of XLM. After redeploying, update the ids and hashes above; do not mix records
from different builds.

## Historical v0.1.0 deployment

> The v0.1.0 instance below is kept as evidence of the original settlement flow.
> It settled in the retired test-only token that was removed from the tree, so
> it is not the current architecture. Use the v0.3.0 deployment above.

| Contract | Network | Contract ID | Stellar Expert |
| --- | --- | --- | --- |
| `PulseEscrow` (v0.1.0) | testnet | `CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB` | [view](https://stellar.expert/explorer/testnet/contract/CCEBIZGVFQMOA4T2UKI5WC7PCEYGT3IDCZC75VWMMH3CHRE55BGNGTYB) |
| Retired test-only token | testnet | `CCCNLWIQ2MZVXDNC2BTTHNGFRBP65EEJHEHCNFR5TUSLTLVDYO6HHVF6` | [view](https://stellar.expert/explorer/testnet/contract/CCCNLWIQ2MZVXDNC2BTTHNGFRBP65EEJHEHCNFR5TUSLTLVDYO6HHVF6) |

| Setting | Value |
| --- | --- |
| Admin | `GDBDQRK2AJM2CTJ3HPYWS33E3OET5J5ZTU5Q45V34VYHAWIURQBAKZEA` |
| Dispute window | `60` seconds |
| WASM hash (`pulserun_escrow`) | `79b505b1329f421f2355e0002d81dccc0a52a3b3cd68d9439876ce083f8db2dd` |
| WASM hash (retired token) | `6764571d73cabb024b39b6376201a68b44cea3176fab5c461385c9d4088d511c` |

### Verified end-to-end settlement (v0.1.0)

1. Minted `1000` units of the retired test-only token to the requester.
2. `create_job` with `max_budget = 1000`, `rate_per_second = 10`,
   `max_duration_secs = 100` → `job_id = 1`.
3. `submit_proof` with `duration_secs = 20`, `exit_code = 0`,
   `output_hash = 0x0707…07`.
   tx: [`0139ecfe…`](https://stellar.expert/explorer/testnet/tx/0139ecfe545a053e59904dfedd705599a80fbf8bef79c2ca867eb7c9187f15b8)
4. `claim_payout` before the window → `Error #11` (`DisputeWindowActive`).
5. `claim_payout` after the window → `200` stroops to the runner and `800` back
   to the requester.
   tx: [`a6c17ad8…`](https://stellar.expert/explorer/testnet/tx/a6c17ad89e7d6166ea82c438da974da7cef547e3a103915c394268156ac8b5a0)

## Mainnet

Deploy `pulserun_escrow` with `stellar contract build` and pass the **SAC of a
trusted asset** (for example native XLM or USDC on Stellar) as each job's
`payment_token`. Raise `dispute_window_secs` to a production value and use a
multisignature or governed admin account. There is no test token to retire: the
escrow only ever calls the SEP-41 interface of the SAC it is given.
