<p align="center">
  <img src="docs/assets/banner.svg" alt="PulseRun Core" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://stellar.org/soroban"><img src="https://img.shields.io/badge/Stellar-Soroban-7D00FF.svg" alt="Stellar Soroban"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-2021-000000.svg" alt="Rust 2021"></a>
  <a href="../../actions/workflows/ci.yml"><img src="https://github.com/PulseRun-Labs/pulserun-core/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://pulserun-labs.github.io/pulserun-core/"><img src="https://img.shields.io/badge/Docs-GitHub%20Pages-0969da.svg" alt="Docs"></a>
</p>

# PulseRun Core

On-chain **escrow and settlement** for [PulseRun](https://pulserun.com), a
pay-per-run compute and CI runner protocol. A requester locks a budget on-chain
before a job starts; a runner executes the job and submits a metered proof; a
Soroban smart contract pays for the seconds actually executed and returns the
unspent remainder. No invoices, no trust, no custodians.

This repository is the **contracts** half of PulseRun. The application layer
lives in [`pulserun-client`](https://github.com/PulseRun-Labs/pulserun-client).

📖 **[Documentation](https://pulserun-labs.github.io/pulserun-core/)** — the
full protocol mechanics, contract reference, and per-persona guides are
published on GitHub Pages.

---

## Contents

- [Why PulseRun](#why-pulserun)
- [Built on Stellar & Soroban](#built-on-stellar--soroban)
- [Maintainers](#maintainers)
- [Architecture](#architecture)
- [Job lifecycle](#job-lifecycle)
- [Settlement math](#settlement-math)
- [Repository layout](#repository-layout)
- [Contract API](#contract-api)
- [Quick start](#quick-start)
- [Live testnet deployment](#live-testnet-deployment)
- [Security model](#security-model)
- [Contributing](#contributing)
- [License](#license)

---

## Why PulseRun

Compute and CI work is metered and bursty, but it is almost always paid for
retroactively. That mismatch has a measurable cost:

- **27% of cloud spend is wasted**, mostly on idle or over-provisioned resources
  ([Flexera, *State of the Cloud 2025*](https://www.flexera.com/)).
- An estimated **21% of enterprise cloud infrastructure spend — $44.5B in 2025 —
  goes to underutilized resources**
  ([Harness, *FinOps in Focus*](https://www.prnewswire.com/news-releases/44-5-billion-in-infrastructure-cloud-waste-projected-for-2025-due-to-finops-and-developer-disconnect-finds-finops-in-focus-report-from-harness-302385580.html)).
- **83% of container cost is associated with idle resources**
  ([Datadog](https://www.datadoghq.com/state-of-cloud-costs/)).

Pay-per-run inverts the payment model so a payer never funds idle capacity and a
runner is funded before starting. The escrow enforces the terms on both sides:

| Property | Mechanism |
| --- | --- |
| A runner is always funded | `create_job` transfers the full `max_budget` into the contract before the job is written. |
| A requester can never be overbilled | Proof durations are bounded by `max_duration_secs` and payout is hard-capped at `max_budget`. |
| Either side can escalate | Payout is frozen for `dispute_window_secs`; `dispute_job` halts it pending resolution. |

## Built on Stellar & Soroban

Stellar and Soroban are not decorative here — the protocol is only possible
because of what the network provides:

- **Soroban smart contracts are the escrow.** The budget is custodied by a
  `#[contract]` deployed to Soroban. Without an on-chain contract VM that can
  hold and transfer value under program logic, there is no trustless escrow to
  build.
- **Settlement in Stellar assets via the Stellar Asset Contract (SAC).** Jobs
  settle in any SEP-41 token, including the SAC of a Stellar-issued asset such
  as a stablecoin — so requesters pay in the asset they already hold, with no
  wrapping or bridged counterparty.
- **Metering is economically viable because settlement is cheap and fast.**
  Pay-per-run only works if settling a single run is cheap relative to the run.
  Stellar's low fees and roughly five-second ledger close make per-job
  settlement practical where a per-transaction fee of dollars would make it
  absurd.
- **Authorization and account model.** `Address::require_auth` on requesters and
  runners gives the contract cryptographic proof of who agreed to the terms,
  including for smart-wallet (contract) accounts.
- **Ledger time drives the protocol.** The dispute window and the unclaimed-job
  timeout are both measured against `env.ledger().timestamp()`, so the clock is
  the network's, not an operator's.
- **Deterministic WASM.** Contracts compile to `wasm32v1-none` and deploy
  through the Stellar toolchain, so the settlement logic that runs on-chain is
  the logic in this repo, verifiable byte for byte.

If you lifted this design onto a chain without a general-purpose contract VM, or
without an on-chain asset rail to settle in, it would not exist. That is the
point.

## Maintainers

<table align="center">
  <tr>
    <td align="center">
      <strong>Adesh</strong>
      <br />
      <a href="https://github.com/Adesh-tech09">@Adesh-tech09</a>
      <br />
    </td>
  </tr>
</table>

## Architecture

```text
        create_job (locks max_budget)          submit_proof
┌───────────┐  ───────────────────────►  ┌──────────────┐  ───────────►  ┌────────────┐
│ Requester │                            │    Escrow    │                │   Runner   │
└───────────┘  ◄───────────────────────  │  (this repo) │  ◄───────────  └────────────┘
     │            claim_payout (dust)    └──────────────┘    earnings
     │                 │                       │  ▲
     │                 │                       ▼  │
     │         ┌───────────────┐        ┌───────────────┐
     └────────►│ dispute_job   │        │ payment_token │  (SEP-41 / SAC)
               │ (halts payout)│        └───────────────┘
               └───────────────┘
```

### Job lifecycle

```mermaid
stateDiagram-v2
    [*] --> Queued: create_job
    Queued --> Completed: submit_proof
    Queued --> Refunded: cancel_unclaimed_job\n(after max_duration_secs)
    Completed --> Settled: claim_payout\n(after dispute window)
    Completed --> Disputed: dispute_job\n(within dispute window)
    Disputed --> Resolved: resolve_dispute\n(admin)
    Settled --> [*]
    Refunded --> [*]
    Resolved --> [*]
```

### Settlement math

```text
duration     = proof.duration_secs            (validated: 1 ..= max_duration_secs)
earnings     = min(rate_per_second * duration, max_budget)   (runner)
refund       = max_budget - earnings                         (requester)
```

Because `create_job` escrows the whole `max_budget`, settlement is fully
self-contained: it only ever moves money already in the contract, so it cannot
fail for lack of funds. Full mechanics:
[`docs/protocol-mechanics.md`](docs/protocol-mechanics.md).

### Dispute resolution

`dispute_job` freezes a payout, but a freeze is not an outcome. The configured
admin resolves a `Disputed` job with `resolve_dispute(admin, job_id, runner_award)`,
where `runner_award` is bounded by `0..=max_budget`; the runner receives that
amount and the requester receives the remainder. The job becomes `Resolved` and
cannot be claimed or resolved again. This is a deliberate, documented trust
assumption: the admin decides the split, so deployments should use a
multisignature or governed admin account. See
[`docs/threat-model.md`](docs/threat-model.md).

### Events

Every lifecycle transition emits a typed `contractevent`, so an indexer can
follow jobs from the ledger without polling views:

| Event | Emitted by | Key fields |
| --- | --- | --- |
| `EscrowInitialized` | `init` | admin, dispute window |
| `JobCreated` | `create_job` | job id, requester, runner, token, budget, rate, duration, dispute window |
| `ProofSubmitted` | `submit_proof` | job id, runner, duration, exit code, output hash |
| `PayoutClaimed` | `claim_payout` | job id, requester, runner, earnings, refund |
| `JobDisputed` | `dispute_job` | job id, requester |
| `JobRefunded` | `cancel_unclaimed_job` | job id, requester, amount |
| `DisputeResolved` | `resolve_dispute` | job id, admin, runner award, requester refund |
| `DisputeWindowUpdated` | `update_dispute_window` | admin, new window |

## Repository layout

```text
pulserun-core/
├── contracts/
│   ├── escrow/          # PulseEscrow: escrow + settlement
│   │   └── src/
│   │       ├── lib.rs       # entrypoints
│   │       ├── types.rs     # ComputeJob, ExecutionProof, JobStatus
│   │       ├── events.rs    # typed lifecycle events
│   │       ├── storage.rs   # keys, TTL policy, accessors
│   │       ├── errors.rs    # stable error codes
│   │       └── test.rs      # integration tests against the Stellar Asset Contract
├── docs/                # documentation source (mdBook → GitHub Pages)
├── scripts/             # deploy + issue-generation tooling
├── .github/workflows/   # CI: fmt, clippy, test, wasm build
└── Cargo.toml           # workspace
```

## Contract API

`PulseEscrow` (`contracts/escrow`):

| Function | Access | Description |
| --- | --- | --- |
| `init(admin, dispute_window_secs)` | admin | One-time setup of the administrator and dispute window. |
| `create_job(requester, runner, token, max_budget, rate_per_second, max_duration_secs) -> u64` | requester | Locks collateral, returns the new `job_id`. |
| `submit_proof(runner, proof)` | runner | Records the proof, locks the output hash, opens the dispute window. |
| `claim_payout(job_id) -> i128` | anyone | After the window: pays the runner, refunds dust to the requester. |
| `dispute_job(requester, job_id)` | requester | Within the window: halts automatic payout. |
| `resolve_dispute(admin, job_id, runner_award)` | admin | Splits a `Disputed` escrow and marks it `Resolved`. |
| `cancel_unclaimed_job(requester, job_id)` | requester | After `max_duration_secs`: refunds a still-`Queued` job in full. |
| `update_dispute_window(admin, dispute_window_secs)` | admin | Changes the window for jobs created after the call. |
| `get_job` / `get_proof` / `job_count` / `dispute_window` / `admin` / `get_config` | view | Read-only accessors. |

Full reference including error codes:
[`docs/contract-reference.md`](docs/contract-reference.md).

## Quick start

Tests run with a stock Rust toolchain. Building a deployable contract and
deploying need the [Stellar CLI](https://github.com/stellar/stellar-cli); there
is no Docker or network service involved in either.

```bash
# 1. One-time toolchain setup (Rust 1.84+ for the wasm32v1-none target).
rustup target add wasm32v1-none
rustup component add rustfmt clippy
curl -fsSL github.com/stellar/stellar-cli/raw/main/install.sh | sh

# 2. Run the full test suite (41 tests).
cargo test --all --locked

# 3. Match the lint gate.
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings

# 4. Compile the deployable contract.
stellar contract build
```

The artifact lands at `target/wasm32v1-none/release/pulserun_escrow.wasm`.

> **Why the CLI?** From `soroban-sdk` 28 onward only `stellar contract build`
> applies the spec-shaking and optimization steps the Soroban runtime requires;
> plain `cargo build` is not a supported way to produce a deployable contract.

The tests exercise the real **Stellar Asset Contract** (SAC) that backs every
Stellar asset — the same contract that settles on-chain — rather than a
hand-written stand-in token.

### Deploying (testnet)

```bash
./scripts/deploy-testnet.sh        # builds, deploys, initializes, prints contract ids
```

The script builds with `stellar contract build`, creates and funds a testnet
identity, resolves the **native XLM Stellar Asset Contract**, deploys the escrow,
and calls `init`. Set `PAYMENT_ASSET="CODE:ISSUER"` to settle in an issued asset
instead of XLM. See
[`docs/deployments/testnet.md`](docs/deployments/testnet.md) to record the result.

## Live testnet deployment

The current **v0.3.0** build is deployed and verified on Stellar testnet. It
settles in the native XLM Stellar Asset Contract; no custom token contract is
deployed.

| Item | Value |
| --- | --- |
| `PulseEscrow` | [`CAPNQ4LWFOVVJG76UHGAOUT4RL4OH2LJKQOY36KYU5JQ7SEJ5F4DSI2D`](https://stellar.expert/explorer/testnet/contract/CAPNQ4LWFOVVJG76UHGAOUT4RL4OH2LJKQOY36KYU5JQ7SEJ5F4DSI2D) |
| Payment asset | Native XLM SAC [`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`](https://stellar.expert/explorer/testnet/contract/CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC) |
| Admin | `GAFRQAM4VZDKUY5TL4PJFFLCAN5E7PPDXP3QCELHHMBP4QZZY54Q7IQP` |
| Dispute window | `60` seconds (shortened for the demo; production default is `86400`) |
| WASM hash | `ac2afe1fb2c4e037cb781b3827ea0adc11c1edd34578f7245205603c54e69417` |

A full lifecycle ran against the public network: `create_job` locked
`10,000,000` stroops (1 XLM), `submit_proof` recorded a 20-second run, an early
`claim_payout` was rejected with `Error #11` (`DisputeWindowActive`), and the
post-window claim paid the runner `2,000,000` stroops, refunded the requester
`8,000,000`, and drained the escrow to `0`. Job `1` ended `Settled`.

- `init`: [tx](https://stellar.expert/explorer/testnet/tx/c20632f263f493b80db7de868342e02bb66d5f9fdd78a42cd6bff4b0a0e276f0)
- `create_job`: [tx](https://stellar.expert/explorer/testnet/tx/b86876c8e30c0916ff851908cdf57b942aa564bc25ffd0de731454b24e3c2010)
- `submit_proof`: [tx](https://stellar.expert/explorer/testnet/tx/8b98b93a8cb932d16b631dcc6933d419e7064941d485c516d0462e65bd94d5eb)
- `claim_payout`: [tx](https://stellar.expert/explorer/testnet/tx/7a51c15c03d1dfcbf959baee7d570124da39c307d35a4b9378ec38e5d89820ef)

The admin is a testnet-only key for this demo; use a multisignature or governed
account for real value. The earlier v0.1.0 deployment is retained as historical
evidence in [`docs/deployments/testnet.md`](docs/deployments/testnet.md); it used
the retired test-only token.

## Security model

- **Authorization** — every mutating entrypoint calls `Address::require_auth`
  on the exact party it constrains (`requester`, `runner`, or `admin`). The
  supplied address is validated against the stored job record *before*
  authorization, so an impostor cannot spend someone else's signature.
- **Checks-effects-interactions** — `claim_payout` marks the job `Settled`
  before any token transfer, so a re-entrant token cannot double-settle.
- **Checked arithmetic** — settlement math uses `checked_mul`/`checked_add` and
  the release profile keeps `overflow-checks = true`.
- **Bounded liability** — payout can never exceed the locked `max_budget`, and
  billable time can never exceed `max_duration_secs`.
- **Funds cannot be frozen forever** — a disputed job has an on-chain resolution
  path via the configured admin, with the award bounded by the escrow balance.
- **Configuration cannot silently expire** — every state-changing entrypoint
  extends the contract instance TTL, so `Admin`, `DisputeWindow`, and `JobCount`
  stay live while the escrow is in use.

This code is **unaudited**. Treat it as a reference implementation and get an
independent review before mainnet use. See [`SECURITY.md`](SECURITY.md) to
report a vulnerability privately.

## Contributing

Contributions are welcome through pull requests. Every change runs the CI gate
(`fmt`, `clippy -D warnings`, `test`, wasm build). Start with
[`CONTRIBUTING.md`](CONTRIBUTING.md); open work is listed in the
[issue backlog](https://github.com/PulseRun-Labs/pulserun-core/issues).

<a href="https://github.com/PulseRun-Labs/pulserun-core/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=PulseRun-Labs/pulserun-core" alt="Contributors" />
</a>

## License

[MIT](LICENSE) © 2026 PulseRun Labs
