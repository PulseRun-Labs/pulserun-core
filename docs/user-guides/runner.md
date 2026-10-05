# For runners

You are executing compute jobs and getting paid by the second. This page is the
plain-language version of how you get paid.

## Before you start

- You need a Stellar account.
- A requester must have named your address as the `runner` on a job. Only that
  address can submit the proof for that job.

## What to check before doing work

Read the job with `get_job(job_id)` and confirm:

- `status` is `Queued` — the job is open.
- `max_budget` is the collateral already sitting in the contract. You are
  guaranteed this upper bound; it cannot be withdrawn while your job is live.
- `rate_per_second` and `max_duration_secs` are the deal. Your payout is
  `rate_per_second * seconds`, capped at `max_budget`.

If those numbers do not cover your cost of running the job, do not run it.

## After the job runs

Call `submit_proof` with an `ExecutionProof`:

- `job_id` — the job you ran.
- `duration_secs` — the seconds you actually executed. It must be **at least 1**
  and **no more than `max_duration_secs`**. A proof outside that range is
  rejected, so do not round up past the ceiling; it will fail rather than pay.
- `exit_code` — your process exit code.
- `output_hash` — a 32-byte commitment to the output. Compute it over the logs,
  artifacts, or build cache you produced. This is what the requester checks.

Once the proof lands, the job is `Completed` and the dispute window opens.

## Getting paid

After `completed_at + dispute_window_secs`, call `claim_payout(job_id)`. Anyone
can call it, but the money is fixed: your earnings go to your address and the
unspent remainder goes to the requester. You do not need to trust the caller.

Your earnings are `min(rate_per_second * duration_secs, max_budget)`. If your
metered total exceeds the budget, the budget is your cap — the `max_budget` you
saw before starting is a hard ceiling on what you can be paid.

Billing the full `max_duration_secs` is allowed and is the maximum you can
charge. There is no bonus for finishing early.

## If the requester disputes

The requester can call `dispute_job` inside the window. If they do, payment is
frozen and `claim_payout` will refuse the job. Funds stay in escrow until the
dispute is resolved. You cannot force a payout through a dispute — that is the
point of the window, and it is why you should confirm the job terms before you
run.

## Honest metering is enforced by bounds, not by the contract

The contract enforces that you cannot bill past `max_duration_secs` or
`max_budget`. It cannot verify that `duration_secs` matches your real runtime.
That check is the requester's, via the output hash and the dispute window. Report
accurate seconds; a disputed proof delays your money indefinitely.
