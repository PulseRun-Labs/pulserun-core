# For requesters

You are paying for compute. This page is the plain-language version of what
happens to your money.

## Before you start

- You need a Stellar account with a balance of the token you want to pay in.
- You need to know the runner's Stellar address.
- You decide three numbers: the **most you are willing to pay** (`max_budget`),
  the **price per second** you are offering (`rate_per_second`), and the **most
  seconds** the job may run (`max_duration_secs`).

Your worst-case spend is `max_budget`. It cannot go higher. Ever.

## Opening a job

Call `create_job` with the runner, the token, and your three numbers. The full
`max_budget` leaves your account immediately and is held by the contract. This
is the whole point: the runner can see the money is there before they start.

You get back a `job_id`. Keep it — it identifies the job for every later step.

## While the job runs

Nothing happens on-chain until the runner submits a proof. If the runner never
submits one, your money is not stuck: once `max_duration_secs` has passed since
you created the job, call `cancel_unclaimed_job` and get all of it back.

## When the runner submits a proof

The runner calls `submit_proof` with the seconds they actually ran. A clock
starts — the **dispute window** — before any payment can move.

- **If you agree** with the proof: do nothing. Once the window closes, anyone
  (often the runner) calls `claim_payout`. The runner is paid
  `rate_per_second * seconds`, capped at `max_budget`, and anything left over
  comes straight back to you.
- **If you disagree** — the seconds look inflated, or the output is wrong —
  call `dispute_job` before the window closes. This **freezes the payment**.
  Nothing is paid to anyone while the job is disputed. The configured admin then
  calls `resolve_dispute` to award the runner between `0` and `max_budget` and
  refund you the rest; the job becomes `Resolved`.

You must call `dispute_job` before `completed_at + dispute_window_secs`. The
window is captured when you create the job, so later admin changes cannot
rewrite your terms. After the window closes, payment is automatic and cannot be
stopped by you.

## What you are guaranteed

1. You never pay more than `max_budget`.
2. A runner cannot bill more seconds than `max_duration_secs`.
3. A job that never runs is refunded in full.
4. You can freeze any payment you believe is wrong, inside the window.
5. A frozen payment has an on-chain resolution path; a disputed job cannot stay
   locked with no outcome.

## What you are not guaranteed

The contract does not inspect your logs or judge whether the work was correct.
`dispute_job` stops the money; the configured admin decides the split with
`resolve_dispute`, subject to the hard `max_budget` bound. That means the admin
is a trust assumption: before using a deployment, check who controls the admin
key (a multisignature or governed account is appropriate for real value). Treat
the dispute window as a brake, not an automatic court.
