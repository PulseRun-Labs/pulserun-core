# Protocol mechanics

This page describes the core object — the compute job — its full lifecycle, and
the arithmetic that settles it.

## The compute job

A job is an escrow record with a spend ceiling and a linear price:

| Field | Type | Meaning |
| --- | --- | --- |
| `job_id` | `u64` | Monotonic id, allocated by `create_job` and starting at 1. |
| `requester` | `Address` | The party paying for the compute. |
| `runner` | `Address` | The party expected to execute the job. |
| `payment_token` | `Address` | Token used to denominate the budget and payouts. |
| `max_budget` | `i128` | Collateral locked at creation; the absolute spend ceiling. |
| `rate_per_second` | `i128` | Linear price, in token base units, per executed second. |
| `max_duration_secs` | `u64` | Upper bound on billable seconds; also the unclaimed-job timeout. |
| `status` | `JobStatus` | Current lifecycle state. |
| `created_at` | `u64` | Ledger timestamp the escrow opened. |
| `completed_at` | `u64` | Ledger timestamp the proof was accepted (`0` while queued). |
| `output_hash` | `BytesN<32>` | Commitment to the run's output; zeroed until a proof lands. |

The runner's proof is a separate record:

| Field | Type | Meaning |
| --- | --- | --- |
| `job_id` | `u64` | The job this proof settles. |
| `duration_secs` | `u64` | Measured execution time; validated to `1..=max_duration_secs`. |
| `exit_code` | `i32` | Process exit code reported by the runner. |
| `output_hash` | `BytesN<32>` | Commitment to the run's output. |

## State machine

```text
            create_job            submit_proof
 (none) ──────────────► Queued ──────────────► Completed
                           │                        │
                           │ cancel_unclaimed_job   ├── claim_payout ──► Settled
                           ▼ (after max_duration)   │   (after window)
                        Refunded                    └── dispute_job ──► Disputed
                                                            (in window)
```

- `Queued` → `Completed` via `submit_proof`, by the designated `runner`.
- `Queued` → `Refunded` via `cancel_unclaimed_job`, by the `requester`, once
  `max_duration_secs` has elapsed since `created_at`.
- `Completed` → `Settled` via `claim_payout`, by anyone, once
  `completed_at + dispute_window_secs` has passed.
- `Completed` → `Disputed` via `dispute_job`, by the `requester`, only while the
  dispute window is open.

`Settled` and `Refunded` are terminal. `Disputed` stops automatic payout
entirely; funds stay escrowed until an off-chain resolution decides where they
go.

## Settlement math

```text
metered   = rate_per_second * proof.duration_secs     (checked_mul; overflow → error)
earnings  = min(metered, max_budget)                  (runner's payout)
refund    = max_budget - earnings                     (returned to requester)
```

Three invariants hold on every settlement:

1. `earnings + refund == max_budget` — the escrow drains exactly.
2. `earnings <= max_budget` — a runner can never be paid more than was locked.
3. `duration_secs <= max_duration_secs` — enforced before the proof is stored.

## Worked examples

Assume a token with `dispute_window_secs = 3600`.

**Under budget.** `max_budget = 100`, `rate_per_second = 2`, `max_duration = 60`.
The runner proves `duration_secs = 30`.

```text
metered  = 2 * 30 = 60
earnings = min(60, 100) = 60   → runner
refund   = 100 - 60 = 40       → requester
```

**At the ceiling.** `max_budget = 100`, `rate_per_second = 5`, `max_duration = 60`.
The runner proves the full `60` seconds.

```text
metered  = 5 * 60 = 300
earnings = min(300, 100) = 100 → runner
refund   = 100 - 100 = 0       → requester
```

The requester's liability is bounded by `max_budget` no matter what the runner
does; the `rate_per_second` is a claim on the budget, not an uncapped meter.

**Timeout refund.** The runner never submits a proof. `max_duration_secs = 60`
after `created_at`, the requester calls `cancel_unclaimed_job` and the entire
`max_budget` returns.

## Dispute window

The window exists so a requester can stop payment on a proof they believe is
wrong — for example, a `duration_secs` that does not match observed execution.
It is a single `dispute_window_secs` value set at `init`, applied to every job.

- `claim_payout` requires `ledger.timestamp >= completed_at + dispute_window_secs`.
- `dispute_job` requires `ledger.timestamp < completed_at + dispute_window_secs`.

The window is intentionally narrow-scope: this contract does not adjudicate
disputes. It halts payout and holds funds. Resolution is an off-chain process
(and a planned on-chain admin resolution path, tracked as an issue).
