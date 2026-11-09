# Threat model

This page states what `PulseEscrow` protects, who can attack it, and which
mechanism stops each attack. It covers the contract as deployed; infrastructure,
wallets, and the client application are out of scope.

## Assets and invariants

The escrow custodies token balances between `create_job` and settlement. The
properties that must hold:

1. The contract's token balance equals the sum of `max_budget` over every
   non-terminal job.
2. Settlement never pays out more than the locked `max_budget`, and
   `runner_share + requester_refund == max_budget` on every settlement path.
3. Only the stored requester can dispute or cancel; only the stored runner can
   submit a proof; only the stored admin can resolve a dispute or change the
   dispute window.
4. A job reaches at most one terminal state, and no terminal job can move funds
   again.
5. Configuration (`Admin`, `DisputeWindow`, `JobCount`) stays live while the
   contract is in use.

## Actors and trust boundaries

| Actor | Trusted for | Not trusted for |
| --- | --- | --- |
| Requester | Funding the escrow, choosing the runner and terms, disputing in time | Anything after `create_job` other than its own calls |
| Runner | Executing the job honestly and reporting accurate `duration_secs` | The truth of the proof; that is checked by the requester and the admin |
| Admin | Resolving a disputed split within `0..=max_budget` | Unbounded payouts, moving funds from non-disputed jobs, rewriting history |
| Any caller | Triggering deterministic settlement after the window | Choosing payout destinations or changing amounts |
| Payment token | Implementing `transfer` and `balance` faithfully | Anything; a malicious token can revert or re-enter |
| Stellar ledger | Monotonic timestamps and finality | Wall-clock accuracy beyond ledger time |

The protocol trusts the admin to adjudicate disputes. That is the largest
centralization in the design and should be a multisignature or governed account
on any deployment holding real value. The contract bounds the damage: the award
is `0..=max_budget`, the remainder always returns to the requester, the state is
terminal, and the decision is emitted as `DisputeResolved`.

## Admin dispute resolution

The admin is the only address that can call `resolve_dispute` or
`update_dispute_window`, and it is checked against stored `Admin` state before
`require_auth`. Resolution is limited to jobs in `Disputed`, so the admin cannot
reach funds in `Queued` or `Completed` jobs. The award is bounded by
`max_budget`, the requester's remainder is computed from that same bound, and
the job becomes terminal before either transfer. A compromised admin can choose
the split of a disputed escrow; it cannot drain non-disputed escrows, mint
funds, or settle a job twice. Use a multisig admin for real value.

## Threats and mitigations

| Threat | Mechanism |
| --- | --- |
| An impostor submits a proof for someone else's job | `submit_proof` compares the supplied runner to `job.runner` and calls `require_auth` on that checked address. |
| A runner bills more than the agreed time or budget | `duration_secs` is rejected above `max_duration_secs`, `rate × duration` is checked, and earnings are capped at `max_budget`. |
| A payout is claimed before the challenge window closes | `claim_payout` requires `now >= completed_at + dispute_window_secs`. |
| A requester disputes after seeing settlement is safe | `dispute_job` requires `now < completed_at + dispute_window_secs`; late challenges fail with `DisputeWindowElapsed`. |
| A malicious token re-enters during payout and double-settles | The job is written to `Settled`, `Refunded`, or `Resolved` before any external token call (effects before interactions). |
| Funds are locked forever when a runner never proves | The requester can call `cancel_unclaimed_job` after `created_at + max_duration_secs` for a full refund. |
| A dispute freezes funds with no outcome | `resolve_dispute` provides a terminal, admin-gated split instead of an open-ended freeze. |
| A compromised admin drains an escrow | The award is bounded by `max_budget`; the requester receives the remainder; only `Disputed` jobs can be resolved. Use a multisig admin. |
| Arithmetic overflow wraps a payout | All settlement paths use `checked_mul`/`checked_add`, and the release profile keeps `overflow-checks = true`. |
| The contract instance expires and the admin/config disappear | Every state-changing entrypoint calls `extend_instance_ttl`, which applies the instance TTL policy explicitly. |
| A persistent job or proof expires mid-flight | `set_job` and `set_proof` extend the entry TTL on every write to ~30 days. |
| A job id collides or wraps | `JobCount` is monotonic and `next_job_id` uses `checked_add`. |
| An attacker chooses payout addresses or amounts by calling `claim_payout` | Settlement destinations are read from the stored job; the caller only triggers it. |
| A fake token is passed to `create_job` | The transfer must succeed for the job to exist. There is no token allowlist, so integrators must pass a trusted SEP-41 token or SAC. |
| A user is assigned as runner without consent | Collateral is recoverable after the timeout. There is no on-chain accept step; that remains a product-level residual risk. |
| A client misreads an event from another contract | Events are contract-scoped; indexers must filter on the escrow contract id. |

## Storage and TTL

| Entry | Storage | Why |
| --- | --- | --- |
| `Admin`, `DisputeWindow`, `JobCount` | instance | Small, contract-wide configuration |
| `Job(u64)` | persistent | One entry per job; keeps the instance footprint flat |
| `Proof(u64)` | persistent | One entry per completed job |

The TTL policy is `TTL_THRESHOLD = 17_280` ledgers (~1 day) and
`TTL_EXTEND_TO = 518_400` ledgers (~30 days). Persistent entries are extended
when written. The instance entry is *not* extended by the host automatically;
`extend_instance_ttl` is called from every mutating entrypoint. A contract that
sees no traffic for longer than the extend-to window can still expire, so
operators should monitor and bump it if the deployment must stay live while idle.

Changing a `DataKey` or a stored struct on a live contract requires a migration.
The contract has no upgrade entrypoint: ship a new deployment, pause new jobs
off-chain, let open jobs settle or resolve, and migrate references explicitly.
Never assume storage added by a new build exists in an old deployment.

## Residual risks

- **Admin adjudication is subjective.** The contract enforces bounds, not
  correctness of the split.
- **`duration_secs` is self-reported.** The requester's only recourse is the
  dispute window and the admin; the output hash is a commitment, not a proof of
  execution.
- **No runner acceptance.** A requester can lock funds against an address that
  never intended to run the job; the timeout refund returns the money.
- **Token risk.** Any SEP-41 contract can be passed as `payment_token`. A
  malicious or paused token can make transfers revert. Use a token you trust.
- **Ledger time.** Windows are measured in ledger timestamps; they are monotonic
  but not wall-clock precise.
- **Unaudited code.** This repository has not had an independent audit. Treat it
  as a reference implementation until it has.

## Reporting a vulnerability

Do not open a public issue. Follow [`SECURITY.md`](../SECURITY.md) to report
privately with a reproduction and impact assessment.
