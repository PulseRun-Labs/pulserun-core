# Contract architecture

This page is the design spec for the contract layer: what exists, how it depends
on what, where state lives, and which address authorizes each action. Read it
alongside the [contract reference](contract-reference.md).

## Contracts and responsibilities

| Contract | Path | Single responsibility |
| --- | --- | --- |
| `PulseEscrow` | [`contracts/escrow`](https://github.com/PulseRun-Labs/pulserun-core/tree/main/contracts/escrow) | Custody requester collateral and settle jobs: create, prove, claim, dispute, refund. |

`PulseEscrow` is the only contract in this repository and the only one that
holds value. The settlement rail is a **Stellar Asset Contract (SAC)** — the
built-in Soroban contract that backs every Stellar asset, issued or native. The
escrow never deploys or owns a token; it calls the SEP-41 interface exposed by
the SAC at the address passed to `create_job`.

Tests register a real SAC with `Env::register_stellar_asset_contract_v2`, so the
suite exercises the same contract that moves funds on-chain instead of a
hand-written stand-in.

## Dependency graph

```text
             calls transfer/balance (SEP-41)
┌────────────┐  ───────────────────────►  ┌────────────────────────┐
│ PulseEscrow │                            │ Stellar Asset Contract │  (built into
│  (custody)  │  ◄───────────────────────  │  (native or issued)    │   the network)
└────────────┘                            └────────────────────────┘
      ▲
      │ invoked by requester / runner / anyone
┌───────────────────────────────────────────────────────────┐
│ Stellar accounts & contract accounts (require_auth)        │
└───────────────────────────────────────────────────────────┘
```

**Build and deploy order (dependencies first):**

1. Resolve the SAC address for the asset to settle in. Native XLM's SAC exists on
   every network; an issued asset's SAC is derived from its `CODE:ISSUER` and
   exists without a deployment step.
2. Deploy `pulserun_escrow`. It has no deploy-time dependency on the asset — the
   SAC address is supplied per job to `create_job`.
3. `init(admin, dispute_window_secs)` on the escrow.

There is no inter-contract call between `PulseEscrow` and any contract other
than the SAC transfer that moves funds, so the escrow can be deployed and
upgraded independently of the asset.

## Storage layout

Keys are defined once in
[`storage.rs`](https://github.com/PulseRun-Labs/pulserun-core/blob/main/contracts/escrow/src/storage.rs)
as
`DataKey`. The split is deliberate: contract-wide config is cheap to read and
belongs in instance storage; per-job records go to persistent storage so a large
backlog does not bloat the instance footprint.

| Key | Storage | Type | Written by |
| --- | --- | --- | --- |
| `Admin` | instance | `Address` | `init` |
| `DisputeWindow` | instance | `u64` | `init` |
| `JobCount` | instance | `u64` | `create_job` (monotonic id allocation) |
| `Job(u64)` | persistent | `ComputeJob` | `create_job`, `submit_proof`, `claim_payout`, `dispute_job`, `resolve_dispute`, `cancel_unclaimed_job` |
| `Proof(u64)` | persistent | `ExecutionProof` | `submit_proof` |

### TTL policy

Persistent entries are extended on every write so an active job never expires
mid-flight:

- `TTL_THRESHOLD = 17_280` ledgers (~1 day at 5s/ledger) — bump threshold.
- `TTL_EXTEND_TO = 518_400` ledgers (~30 days at 5s/ledger) — new TTL.

Each job snapshots `DisputeWindow` into its own `dispute_window_secs` field at
`create_job`, so an admin update applies only to future jobs and cannot rewrite
the terms of a job already in flight.

Instance (config) storage is **not** extended by the host automatically. Every
state-changing entrypoint calls `storage::extend_instance_ttl`, which applies the
same threshold/extend-to policy to the instance entry. Without that call the
admin, dispute window, and job counter would expire after the TTL and the
contract would become unusable. A `DataKey` or struct change on a live
deployment needs an explicit migration note — see
[`threat-model.md`](threat-model.md#storage-and-ttl).

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
| `resolve_dispute` | `admin` (arg) | stored `Admin` |
| `cancel_unclaimed_job` | `requester` (arg) | `job.requester` |
| `update_dispute_window` | `admin` (arg) | stored `Admin` |

## Effects before interactions

`claim_payout` writes `Settled`, `cancel_unclaimed_job` writes `Refunded`, and
`resolve_dispute` writes `Resolved` *before* any token transfer. A re-entrant
token therefore cannot observe a job as still claimable and double-settle it.

## Events

Every lifecycle transition publishes a typed `#[contractevent]` defined in
the [`events`](https://github.com/PulseRun-Labs/pulserun-core/blob/main/contracts/escrow/src/events.rs)
module:

| Event | Published by |
| --- | --- |
| `EscrowInitialized` | `init` |
| `JobCreated` | `create_job` |
| `ProofSubmitted` | `submit_proof` |
| `PayoutClaimed` | `claim_payout` |
| `JobDisputed` | `dispute_job` |
| `JobRefunded` | `cancel_unclaimed_job` |
| `DisputeResolved` | `resolve_dispute` |
| `DisputeWindowUpdated` | `update_dispute_window` |

The event structs are part of the contract spec, so generated clients can decode
them without a hand-maintained schema.

## Arithmetic and invariants

- Amounts are `i128` base units; no floats anywhere.
- `metered = rate_per_second * duration_secs` uses `checked_mul` → `MathOverflow`.
- `earnings = min(metered, max_budget)`, `refund = max_budget - earnings`.
- `runner_award` on dispute resolution is bounded by `0..=max_budget`.
- Job id allocation uses `checked_add` → `MathOverflow`.
- Timeouts/windows use `checked_add` on `created_at`/`completed_at`.

Invariants held at every settlement:

1. `earnings + refund == max_budget` (the escrow drains exactly).
2. `earnings <= max_budget`.
3. `duration_secs <= max_duration_secs`, enforced before the proof is stored.
4. `runner_award + requester_refund == max_budget` on dispute resolution, with
   `runner_award` in `0..=max_budget`.
