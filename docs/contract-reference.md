# Smart contract reference

`PulseEscrow` lives at
[`contracts/escrow/`](https://github.com/PulseRun-Labs/pulserun-core/tree/main/contracts/escrow).
All amounts
are in the `payment_token`'s base units. Every mutating entrypoint returns a
typed [`Error`](#error-codes) rather than panicking.

## Configuration

### `init(env, admin, dispute_window_secs) -> Result<(), Error>`

One-time setup. Stores the administrator and the dispute window.

| Parameter | Type | Notes |
| --- | --- | --- |
| `admin` | `Address` | Must authorize the call; stored for future admin actions. |
| `dispute_window_secs` | `u64` | Seconds a requester has to challenge a proof. |

Fails with `AlreadyInitialized` if called twice, and with
`InvalidDisputeWindow` if `dispute_window_secs` is `0`.

### `update_dispute_window(env, admin, dispute_window_secs) -> Result<(), Error>`

Changes the dispute window captured by jobs created after this call. Existing
jobs keep the window that was live when `create_job` ran. Only the stored admin
may call it, and the new window must be non-zero.

| Parameter | Type | Constraint |
| --- | --- | --- |
| `admin` | `Address` | Must equal the stored admin; must authorize. |
| `dispute_window_secs` | `u64` | Must be `> 0`. |

Errors: `NotInitialized`, `Unauthorized`, `InvalidDisputeWindow`. Emits
`DisputeWindowUpdated`.

## Job lifecycle

### `create_job(env, requester, runner, payment_token, max_budget, rate_per_second, max_duration_secs) -> Result<u64, Error>`

Opens an escrow and locks collateral. Transfers `max_budget` of `payment_token`
from `requester` into the contract, then records a `Queued` job. Returns the new
`job_id`.

| Parameter | Type | Constraint |
| --- | --- | --- |
| `requester` | `Address` | Must authorize the call. Also the source of funds. |
| `runner` | `Address` | The only address allowed to submit the proof. |
| `payment_token` | `Address` | Token contract used for the transfer and payouts. |
| `max_budget` | `i128` | Must be `> 0`. |
| `rate_per_second` | `i128` | Must be `> 0`. |
| `max_duration_secs` | `u64` | Must be `> 0`. |

The job also captures the current `dispute_window_secs`; later admin changes do
not rewrite it. Errors: `NotInitialized`, `InvalidBudget`, `InvalidRate`,
`InvalidDuration`, plus any token-transfer failure (which reverts the whole
call). Emits `JobCreated`.

### `submit_proof(env, runner, proof) -> Result<(), Error>`

Records the runner's execution proof, locks the output hash, and starts the
dispute clock. The job must be `Queued`.

| Parameter | Type | Constraint |
| --- | --- | --- |
| `runner` | `Address` | Must equal the stored job's `runner`; must authorize. |
| `proof.job_id` | `u64` | Must reference an existing job. |
| `proof.duration_secs` | `u64` | Must be `1..=max_duration_secs`. |
| `proof.exit_code` | `i32` | Recorded as reported. |
| `proof.output_hash` | `BytesN<32>` | Stored on the job and in the proof. |

Errors: `JobNotFound`, `InvalidStatus`, `Unauthorized`, `InvalidDuration`,
`DurationExceedsMax`.

### `claim_payout(env, job_id) -> Result<i128, Error>`

Settles a `Completed` job after the dispute window. Pays the runner their linear
earnings, refunds the unspent remainder to the requester, marks the job
`Settled`, and returns the payout. Callable by anyone — settlement is
deterministic and the funds have fixed destinations.

Errors: `JobNotFound`, `InvalidStatus`, `ProofNotFound`, `DisputeWindowActive`,
`MathOverflow`.

### `dispute_job(env, requester, job_id) -> Result<(), Error>`

Challenges a `Completed` proof inside the window, moving the job to `Disputed`
and halting automatic payout. Funds stay escrowed until the admin resolves the
dispute.

| Parameter | Type | Constraint |
| --- | --- | --- |
| `requester` | `Address` | Must equal the stored job's `requester`; must authorize. |
| `job_id` | `u64` | Must be a `Completed` job whose window is still open. |

Errors: `JobNotFound`, `InvalidStatus`, `Unauthorized`, `DisputeWindowElapsed`,
`MathOverflow`. Emits `JobDisputed`.

### `resolve_dispute(env, admin, job_id, runner_award) -> Result<(), Error>`

Resolves a `Disputed` job by splitting the escrow. The runner receives
`runner_award`; the requester receives `max_budget - runner_award`. The job moves
to the terminal `Resolved` state, so it can never be claimed or resolved again.

| Parameter | Type | Constraint |
| --- | --- | --- |
| `admin` | `Address` | Must equal the stored admin; must authorize. |
| `job_id` | `u64` | Must be `Disputed`. |
| `runner_award` | `i128` | Must be `0..=max_budget`. |

Errors: `JobNotFound`, `InvalidStatus`, `Unauthorized`, `InvalidShare`. Emits
`DisputeResolved`.

This is the only admin power over funds. The split is an explicit trust
decision; see the [threat model](threat-model.md#admin-dispute-resolution).

### `cancel_unclaimed_job(env, requester, job_id) -> Result<(), Error>`

Refunds a still-`Queued` job in full once `max_duration_secs` has elapsed since
`created_at`, marking it `Refunded`.

| Parameter | Type | Constraint |
| --- | --- | --- |
| `requester` | `Address` | Must equal the stored job's `requester`; must authorize. |
| `job_id` | `u64` | Must be `Queued` and past expiry. |

Errors: `JobNotFound`, `InvalidStatus`, `Unauthorized`, `JobNotExpired`,
`MathOverflow`. Emits `JobRefunded`.

## Views

| Function | Returns | Notes |
| --- | --- | --- |
| `get_job(job_id)` | `Result<ComputeJob, Error>` | `JobNotFound` if absent. |
| `get_proof(job_id)` | `Result<ExecutionProof, Error>` | `ProofNotFound` if absent. |
| `job_count()` | `u64` | Number of jobs ever created. |
| `dispute_window()` | `u64` | Configured window, in seconds. |
| `admin()` | `Address` | Configured administrator. |
| `get_config()` | `Result<(Address, u64, u64), Error>` | Admin, dispute window, and job count in one call. |

## Events

Every state transition emits a typed `#[contractevent]` that is included in the
contract spec:

| Event | Emitted by | Fields |
| --- | --- | --- |
| `EscrowInitialized` | `init` | `admin`, `dispute_window_secs` |
| `JobCreated` | `create_job` | `job_id` (topic), requester, runner, token, budget, rate, duration, dispute window |
| `ProofSubmitted` | `submit_proof` | `job_id` (topic), runner, duration, exit code, output hash |
| `PayoutClaimed` | `claim_payout` | `job_id` (topic), requester, runner, earnings, refund |
| `JobDisputed` | `dispute_job` | `job_id` (topic), requester |
| `JobRefunded` | `cancel_unclaimed_job` | `job_id` (topic), requester, amount |
| `DisputeResolved` | `resolve_dispute` | `job_id` (topic), admin, runner award, requester refund |
| `DisputeWindowUpdated` | `update_dispute_window` | admin, new window |

No event is emitted when a call fails; state changes and events are atomic.

## Error codes

Error codes are part of the public ABI. Branch on the numeric value, not the
variant name. Codes are never renumbered.

| Code | Variant | Meaning |
| --- | --- | --- |
| 1 | `AlreadyInitialized` | `init` was called on an initialized escrow. |
| 2 | `NotInitialized` | A mutating call ran before `init`. |
| 3 | `JobNotFound` | No job stored under the supplied id. |
| 4 | `ProofNotFound` | No proof for a job that should have one. |
| 5 | `InvalidStatus` | The job is not in the state this operation needs. |
| 6 | `InvalidBudget` | `max_budget` must be strictly positive. |
| 7 | `InvalidRate` | `rate_per_second` must be strictly positive. |
| 8 | `InvalidDuration` | A duration (job or proof) must be strictly positive. |
| 9 | `DurationExceedsMax` | The proof claims more seconds than allowed. |
| 10 | `Unauthorized` | The caller is not the address this operation is restricted to. |
| 11 | `DisputeWindowActive` | Payout attempted while the dispute window is open. |
| 12 | `DisputeWindowElapsed` | The challenge came after the window closed. |
| 13 | `JobNotExpired` | The queued job has not passed its expiry yet. |
| 14 | `MathOverflow` | A settlement computation overflowed `i128`. |
| 15 | `InvalidShare` | A dispute resolution award is outside `0..=max_budget`. |
| 16 | `InvalidDisputeWindow` | The dispute window must be strictly positive. |

## Mock token

[`contracts/mock_token/`](https://github.com/PulseRun-Labs/pulserun-core/tree/main/contracts/mock_token)
implements just enough of the
token surface for tests: `mint`, `burn`, `balance`, and `transfer` (with
`from.require_auth()`). Amounts use checked arithmetic, and balance entries are
TTL-extended on write. Its `mint` is unauthenticated and **test-only** — never
deploy it as real money. In production, pass a SEP-41 token or its Stellar Asset
Contract address as `payment_token`.
