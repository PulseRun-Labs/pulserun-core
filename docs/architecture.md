# Contract architecture

This page is the design spec for the contract layer: what exists, how it depends
on what, where state lives, and which address authorizes each action. Read it
alongside the [contract reference](contract-reference.md).

## Contracts and responsibilities

| Contract | Path | Single responsibility |
| --- | --- | --- |
| `PulseEscrow` | [`contracts/escrow`](../contracts/escrow) | Custody requester collateral and settle jobs: create, prove, claim, dispute, refund. |
| `MockToken` | [`contracts/mock_token`](../contracts/mock_token) | A test-only SEP-41-style token to exercise settlement. Never deployed as money. |

Each contract owns exactly one concern. `PulseEscrow` is the only contract that
holds value; `MockToken` exists purely so the escrow's token transfers run in
tests, and on a real network it is replaced by a SEP-41 token or the Stellar
Asset Contract of an issued asset.

## Dependency graph

```text
             calls transfer/balance
┌────────────┐  ───────────────────────►  ┌──────────────────┐
│ PulseEscrow │                            │ payment_token    │  (SEP-41 / SAC,
│  (custody)  │  ◄───────────────────────  │ / MockToken      │   test-only)
└────────────┘                            └──────────────────┘
      ▲
      │ invoked by requester / runner / anyone
┌───────────────────────────────────────────────────────────┐
│ Stellar accounts & contract accounts (require_auth)        │
└───────────────────────────────────────────────────────────┘
```

**Build and deploy order (dependencies first):**

1. Deploy the settlement token (testnet: `mock_token`; mainnet: the SEP-41/SAC
   token address, which already exists).
2. Deploy `pulserun_escrow`. It has no deploy-time dependency on the token — the
   token address is supplied per job to `create_job`.
3. `init(admin, dispute_window_secs)` on the escrow.

There is no inter-contract call between `PulseEscrow` and any other contract
except the token transfer that settles a job, so the escrow can be deployed and
upgraded independently of the token.

## Storage layout

Keys are defined once in [`storage.rs`](../contracts/escrow/src/storage.rs) as
`DataKey`. The split is deliberate: contract-wide config is cheap to read and
belongs in instance storage; per-job records go to persistent storage so a large
backlog does not bloat the instance footprint.

| Key | Storage | Type | Written by |
| --- | --- | --- | --- |
| `Admin` | instance | `Address` | `init` |
| `DisputeWindow` | instance | `u64` | `init` |
| `JobCount` | instance | `u64` | `create_job` (monotonic id allocation) |
| `Job(u64)` | persistent | `ComputeJob` | `create_job`, `submit_proof`, `claim_payout`, `dispute_job`, `cancel_unclaimed_job` |
| `Proof(u64)` | persistent | `ExecutionProof` | `submit_proof` |

### TTL policy

Persistent entries are extended on every write so an active job never expires
mid-flight:

- `TTL_THRESHOLD = 17_280` ledgers (~1 day at 5s/ledger) — bump threshold.
- `TTL_EXTEND_TO = 518_400` ledgers (~30 days at 5s/ledger) — new TTL.

Instance (config) storage lives with the contract instance and is bumped by the
host. A `DataKey` or struct change on a live deployment needs an explicit
migration note — see the planned threat-model/migration issue in
[`SUBMISSION.md`](../SUBMISSION.md).

## Authorization per entrypoint

The rule everywhere is: **validate the caller against stored state first, then
call `require_auth`** on that checked address, so an impostor cannot spend
someone else's signature.

| Entrypoint | Authorized address | Checked against |
| --- | --- | --- |
| `init` | `admin` (arg) | — (one-time; `AlreadyInitialized` otherwise) |
| `create_job` | `requester` (arg) | — (new record) |
| `submit_proof` | `runner` (arg) | `job.runner` |
| `claim_payout` | none | payout destinations are fixed by the stored job |
| `dispute_job` | `requester` (arg) | `job.requester` |
| `cancel_unclaimed_job` | `requester` (arg) | `job.requester` |

## Effects before interactions

`claim_payout` writes `Settled` and `cancel_unclaimed_job` writes `Refunded`
*before* any token transfer. A re-entrant token therefore cannot observe a job as
still claimable and double-settle it.

## Events

The contracts currently emit **no** `contractevent`s. Indexers reconcile state by
polling views (`get_job`, `job_count`). Emitting events on each lifecycle
transition is planned work, tracked as an issue in
[`SUBMISSION.md`](../SUBMISSION.md).

## Arithmetic and invariants

- Amounts are `i128` base units; no floats anywhere.
- `metered = rate_per_second * duration_secs` uses `checked_mul` → `MathOverflow`.
- `earnings = min(metered, max_budget)`, `refund = max_budget - earnings`.
- Timeouts/windows use `checked_add` on `created_at`/`completed_at`.

Invariants held at every settlement:

1. `earnings + refund == max_budget` (the escrow drains exactly).
2. `earnings <= max_budget`.
3. `duration_secs <= max_duration_secs`, enforced before the proof is stored.
