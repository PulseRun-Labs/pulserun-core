# PulseRun Core

PulseRun is a pay-per-run compute and CI runner protocol on Stellar. This
repository holds the on-chain escrow and settlement contracts that make it work.

## The problem

Compute and CI work is metered and bursty, but it is almost always paid for
retroactively — monthly invoices, prepaid credits, or a platform account that
gets billed after the fact. That mismatch has a measurable cost:

- **27% of cloud spend is wasted**, mostly on idle or over-provisioned
  resources (Flexera, *State of the Cloud 2025*).
- An estimated **21% of enterprise cloud infrastructure spend — $44.5B in 2025 —
  goes to underutilized resources** (Harness, *FinOps in Focus*).
- **83% of container cost is associated with idle resources** (Datadog, *State
  of Cloud Costs*).

The waste is structural: capacity is provisioned ahead of demand and billed
whether or not it does useful work. Pay-per-run inverts that. The money is
committed *before* the job runs and settled for exactly the seconds that
executed, so the payer never funds idle capacity and the runner is always
funded before starting.

## Stellar-native by design

The protocol does not merely happen to run on Stellar — it depends on it. The
escrow is a **Soroban smart contract** that custodies funds and enforces
settlement; jobs settle in any **SEP-41 token or its Stellar Asset Contract
(SAC)**, so requesters pay in the assets Stellar already moves, with no wrapping;
and per-run settlement is only economically sensible because Stellar settles
cheaply and quickly. Authorization comes from **`Address::require_auth`** on the
requester and runner, and the dispute window and job timeout are measured against
the **ledger timestamp**. Remove any of those and the design does not exist. More
detail in the [repository README](../README.md#built-on-stellar--soroban).

## How it works

1. A **requester** calls `create_job`, which moves the full `max_budget` into
   the escrow contract. The job is now `Queued`, and the runner is guaranteed to
   be paid for whatever they bill.
2. The **runner** executes the job and calls `submit_proof` with the measured
   `duration_secs` and a `BytesN<32>` commitment to the output. The job becomes
   `Completed` and a dispute window opens.
3. After the dispute window, anyone calls `claim_payout`. The contract computes
   `rate_per_second * duration_secs`, caps it at `max_budget`, pays the runner,
   and refunds the unspent remainder to the requester.
4. If the requester challenges inside the window, `dispute_job` freezes the
   payout pending resolution. If the runner never submits a proof,
   `cancel_unclaimed_job` refunds the requester in full after `max_duration_secs`.

Settlement only ever moves money already held by the contract, so it cannot fail
for lack of funds.

## Where to next

- [Protocol mechanics](protocol-mechanics.md) — lifecycle, state machine, and
  settlement math with worked numbers.
- [Smart contract reference](contract-reference.md) — every function, parameter,
  and error code.
- [Protocol mechanics](protocol-mechanics.md) and the per-persona guides:
  [for requesters](user-guides/requester.md) and
  [for runners](user-guides/runner.md).
- [Developer guide](developer-guide.md) — local setup, build, test, deploy.

Repository: [PulseRun-Labs/pulserun-core](https://github.com/PulseRun-Labs/pulserun-core).
Application layer: [PulseRun-Labs/pulserun-client](https://github.com/PulseRun-Labs/pulserun-client).
