<p align="center">
  <img src="docs/assets/banner.svg" alt="PulseRun Core" width="100%">
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://stellar.org/soroban"><img src="https://img.shields.io/badge/Stellar-Soroban-7D00FF.svg" alt="Built on Stellar"></a>
  <a href="../../actions/workflows/ci.yml"><img src="https://github.com/PulseRun-Labs/pulserun-core/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://www.drips.network/wave/stellar"><img src="https://img.shields.io/badge/Drips-Stellar%20Wave-000000.svg" alt="Drips Stellar Wave"></a>
</p>

<p align="center">
  <a href="https://github.com/PulseRun-Labs/pulserun-core">Repository</a> ·
  <a href="docs/README.md">Docs</a> ·
  <a href="SUBMISSION.md">Submission</a> ·
  <a href="https://github.com/PulseRun-Labs/pulserun-client">Client app</a>
</p>

# PulseRun Core

On-chain **escrow and settlement** for [PulseRun](https://pulserun.com), a
pay-per-run compute and CI runner protocol on [Stellar](https://stellar.org).
A requester locks a budget up front; a runner executes a job and submits a
metered proof; the contract pays for the seconds actually executed and returns
the unspent remainder. No invoices, no trust, no custodians.

Settlement runs on [Soroban](https://stellar.org/soroban) and settles in any
SEP-41 token (or its Stellar Asset Contract). This repository is the contracts
half of the project; the application layer lives in
[`pulserun-client`](https://github.com/PulseRun-Labs/pulserun-client).

---

## Maintainers

<table align="center">
  <tr>
    <td align="center">
      <strong>Adesh</strong>
      <br />
      <a href="https://github.com/Adesh-tech09">@Adesh-tech09</a>
      <br />
      <a href="https://t.me/PLACEHOLDER_TELEGRAM">Telegram</a>
    </td>
  </tr>
</table>

<!--
TODO before submitting to Drips Wave:
  1. Replace PLACEHOLDER_TELEGRAM with the maintainer's real Telegram handle.
  2. Add teammates as extra <td> cells if there are more maintainers.
-->

Community: [GitHub Discussions](https://github.com/PulseRun-Labs/pulserun-core/discussions)
· [Drips Stellar Wave](https://www.drips.network/wave/stellar)

---

## Why it exists

CI and compute work is metered and bursty, but payment is usually batched and
retroactive. PulseRun inverts that: **the money is already on the table before
the job starts**. The escrow contract enforces three properties that make
pay-per-run safe for both sides:

| Property | Mechanism |
| --- | --- |
| A runner is always funded | `create_job` transfers the full `max_budget` into the contract before the job is written. |
| A requester can never be overbilled | Proof durations are bounded by `max_duration_secs` and payout is hard-capped at `max_budget`. |
| Either side can escalate | Payout is frozen for `dispute_window_secs`; `dispute_job` halts it indefinitely pending resolution. |

The scale of the problem is real: 27% of cloud spend is wasted on idle or
over-provisioned resources (Flexera, *State of the Cloud 2025*), and 83% of
container cost is associated with idle resources (Datadog). Pay-per-run removes
the funding of unused capacity from the model.

---

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
    Settled --> [*]
    Refunded --> [*]
    Disputed --> [*]
```

### Settlement math

```text
metered      = proof.duration_secs            (validated: 1 ..= max_duration_secs)
earnings     = min(rate_per_second * metered, max_budget)   (runner)
refund       = max_budget - earnings                        (requester)
```

Because `create_job` escrows the whole `max_budget`, settlement is fully
self-contained: it only ever moves money already in the contract, so it cannot
fail for lack of funds. Full mechanics: [`docs/protocol-mechanics.md`](docs/protocol-mechanics.md).

---

## Repository layout

```text
pulserun-core/
├── contracts/
│   ├── escrow/          # PulseEscrow: escrow + settlement
│   │   └── src/
│   │       ├── lib.rs       # entrypoints
│   │       ├── types.rs     # ComputeJob, ExecutionProof, JobStatus
│   │       ├── storage.rs   # keys, TTL policy, accessors
│   │       ├── errors.rs    # stable error codes
│   │       └── test.rs      # integration-style unit tests
│   └── mock_token/      # minimal SEP-41-style token for tests
├── docs/                # GitBook documentation site
├── scripts/             # deploy + issue-generation tooling
├── .github/workflows/   # CI: fmt, clippy, test, wasm build
└── Cargo.toml           # workspace
```

---

## Contract API

`PulseEscrow` (`contracts/escrow`):

| Function | Access | Description |
| --- | --- | --- |
| `init(admin, dispute_window_secs)` | admin | One-time setup of the administrator and dispute window. |
| `create_job(requester, runner, token, max_budget, rate_per_second, max_duration_secs) -> u64` | requester | Locks collateral, returns the new `job_id`. |
| `submit_proof(runner, proof)` | runner | Records the proof, locks the output hash, opens the dispute window. |
| `claim_payout(job_id) -> i128` | anyone | After the window: pays the runner, refunds dust to the requester. |
| `dispute_job(requester, job_id)` | requester | Within the window: halts automatic payout. |
| `cancel_unclaimed_job(requester, job_id)` | requester | After `max_duration_secs`: refunds a still-`Queued` job in full. |
| `get_job` / `get_proof` / `job_count` / `dispute_window` / `admin` | view | Read-only accessors. |

Full reference including error codes: [`docs/contract-reference.md`](docs/contract-reference.md).
Error codes are enumerated in [`contracts/escrow/src/errors.rs`](contracts/escrow/src/errors.rs)
and are part of the public ABI — branch on the numeric value, not the name.

---

## Zero-friction local build & test

Everything runs with a stock Rust toolchain — no Stellar CLI, no Docker, no
network services.

```bash
# 1. One-time toolchain setup (Rust 1.84+ for the wasm32v1-none target).
rustup target add wasm32v1-none
rustup component add rustfmt clippy

# 2. Run the full test suite (27 tests, including token settlement).
cargo test --all

# 3. Match CI exactly.
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

# 4. Compile the deployable contracts.
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  cargo build --target wasm32v1-none --release -p pulserun-escrow -p mock-token
```

Artifacts land in `target/wasm32v1-none/release/{pulserun_escrow,mock_token}.wasm`.

> **Why the env var?** `soroban-sdk` v28 refuses to emit a contract wasm unless
> the build system advertises spec-shaking support; the Stellar CLI sets that
> flag itself. Setting it manually lets plain `cargo` compile wasm for CI and
> local checks. **For real deployments, prefer `stellar contract build`**, which
> produces a properly spec-shaken, canonical artifact.

### Deploying (testnet)

```bash
./scripts/deploy-testnet.sh        # builds, deploys in order, prints contract ids
```

The script does the network/identity setup, deploys `mock_token` then the escrow
(the escrow depends on a token), calls `init`, and prints copy-pasteable contract
ids. See [`docs/deployments/testnet.md`](docs/deployments/testnet.md) to record
them.

---

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

This code is **unaudited**. Treat it as a reference implementation and get an
independent review before mainnet use. See [`SECURITY.md`](SECURITY.md) to
report a vulnerability privately.

---

## Contributing

We participate in **Drips Wave**. Issues are labeled by effort — `100pts`
(trivial), `150pts` (medium), `200pts` (high) — and every change runs the
standard CI gate. Start with [`CONTRIBUTING.md`](CONTRIBUTING.md); planned work
is listed in [`SUBMISSION.md`](SUBMISSION.md#planned-issues).

## Contributors

Thanks to everyone who has contributed to PulseRun Core.

<a href="https://github.com/PulseRun-Labs/pulserun-core/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=PulseRun-Labs/pulserun-core" alt="Contributors" />
</a>

## License

[MIT](LICENSE) © 2026 PulseRun Labs
