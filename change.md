# PulseRun Core — Soroban rebase changelog

**Prepared:** 2026-11-22
**Version:** `0.3.0` (was `0.2.0`)
**Repository:** [PulseRun-Labs/pulserun-core](https://github.com/PulseRun-Labs/pulserun-core)

This document records the development work completed since the repository was
declined for the Drips Stellar Wave program. It is written to be attached to the
appeal: every claim below is tied to a code change and a command that can be
re-run.

---

## 1. Context and scope of the rebase

PulseRun Core was **already a Rust / `soroban-sdk` project** before this change:
the escrow is a `#[contract]`, authorization uses `Address::require_auth`, time
uses `env.ledger().timestamp()`, and the README already described Stellar and
Soroban as core dependencies. This was not a migration from another chain.

What was *not* fully Soroban-native was the surrounding implementation:

- Settlement was tested against a **hand-written `MockToken` contract** that lived
  in the repository, not against Stellar's actual asset implementation.
- The testnet deployment script **deployed that toy token** and used it as the
  payment rail.
- The wasm build used plain `cargo build` plus the
  `SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1` environment variable —
  a workaround for a build step that the Stellar CLI is supposed to perform.
- The workspace pinned `soroban-sdk 28.0.0` while protocol 29 was current.

The goal of this rebase was therefore to remove the remaining lookalike pieces
and make every path — tests, build, deploy, docs — run on the canonical Stellar
stack. The contract's job did not change; how it is exercised and shipped did.

### Before / after

| Area | Before | After |
| --- | --- | --- |
| Soroban SDK | `28.0.0` | `29.0.1` (protocol 29, current stable) |
| Settlement asset | Hand-written `MockToken` contract in this repo | Stellar Asset Contract (SAC) — native or issued asset |
| Contracts in workspace | 2 (`pulserun-escrow`, `mock-token`) | 1 (`pulserun-escrow`) |
| Wasm build | `cargo build` + spec-shaking env-var workaround | `stellar contract build` (Stellar CLI 30.0.1) |
| Deploy | Deploy `MockToken`, then deploy escrow | Resolve the asset's SAC id, then deploy escrow |
| CI wasm job | `cargo build` + workaround | official `stellar/stellar-cli` action |
| Tests | 48 (39 escrow + 9 token) | 41 (all escrow, against the real SAC) |
| Version | `0.2.0` | `0.3.0` |

---

## 2. What changed

### 2.1 The custom token is gone; settlement runs on the Stellar Asset Contract

The `contracts/mock_token` crate was deleted, along with its `MockToken`,
`TokenError`, balance storage, and nine token tests. The workspace now contains
one contract: `PulseEscrow`.

The escrow already called the SEP-41 `transfer`/`balance` interface through
`soroban_sdk::token::TokenClient`. What changed is what sits behind that
interface in tests and deployment:

- Tests now register a **real SAC** with
  `Env::register_stellar_asset_contract_v2(admin)`.
- Test balances are minted through the SAC's `StellarAssetClient`.
- Every balance assertion reads the SAC through `TokenClient`.

```rust
let token_id = env
    .register_stellar_asset_contract_v2(admin.clone())
    .address();
StellarAssetClient::new(&env, &token_id).mint(&requester, &FUNDING);
```

This matters for the appeal: the suite no longer proves that the escrow works
with a token we wrote to make it work. It proves that the escrow settles against
the contract Stellar itself uses to move XLM and issued assets.

Because the SAC emits its own `transfer` events, the event assertion helper was
scoped to events published by the escrow:

```rust
let emitted = w.env.events().all().filter_by_contract(&w.escrow_id);
assert_eq!(emitted.events().len(), 1);
```

### 2.2 Two new tests cover the SAC boundary

Removing the toy token removed nine tests that only tested the toy token. Two
new escrow-level tests replace the parts that matter:

- `create_job_rejects_requester_without_token_balance` — the SAC refuses the
  transfer, so no unfunded job is recorded (`job_count` stays `0`).
- `create_job_rejects_payment_token_that_is_not_a_token` — pointing
  `payment_token` at a non-token contract reverts the entire `create_job` call.

These test real failure behavior at the token boundary, which the mock token
could only simulate.

### 2.3 Upgraded to `soroban-sdk 29.0.1`

`Cargo.toml` and `Cargo.lock` now use the protocol-29 SDK line (`soroban-sdk`,
`soroban-env-host`, and the spec/macro crates all at `29.0.1`). The two most
recent SDK majors receive security fixes; this moves the project onto the
current supported major instead of trailing it.

### 2.4 Canonical build path: `stellar contract build`

From `soroban-sdk` 28 onward, a plain `cargo build --target wasm32v1-none
--release` does not produce a deployable contract: the build must shake the
contract spec and optimize the wasm, which `stellar contract build` performs.
The repository previously worked around this with an environment variable.

That workaround is removed from the README, developer guide, contributing guide,
PR template, and CI. The build is now:

```bash
stellar contract build
```

CI installs the official CLI through `stellar/stellar-cli@v30.0.1` and runs the
same command, so the artifact CI uploads is the canonical Soroban artifact.

### 2.5 Testnet deploy now targets a Stellar Asset Contract

`scripts/deploy-testnet.sh` no longer deploys a token. It:

1. Builds with `stellar contract build`.
2. Creates/funds the testnet identity.
3. Resolves the SAC address for `PAYMENT_ASSET`
   (default `native` = XLM) with `stellar contract id asset`.
4. Deploys `pulserun_escrow`, calls `init`, and writes `PAYMENT_ASSET`,
   `PAYMENT_TOKEN_ID`, and `PULSEESCROW_ID` to
   `docs/deployments/testnet.env`.

Setting `PAYMENT_ASSET="CODE:ISSUER"` settles in any issued Stellar asset. No
wrapper, bridge, or custom token is involved.

### 2.6 Documentation and repository metadata updated

The architecture, contract reference, developer guide, contributing guide,
security scope, README, and testnet deployment pages now describe the SAC-based
design. The dependency graph in `docs/architecture.md` shows the escrow calling
the network's Stellar Asset Contract rather than a repository-owned token.

The previously deployed v0.1.0 instance is preserved as **historical evidence**
and clearly marked as legacy in `docs/deployments/testnet.md` and
`docs/deployments/testnet.env`; it settled in the retired toy token. The current
v0.3.0 build requires a fresh testnet deploy (see §4).

### 2.7 Version bump and lockfile

The workspace version is now `0.3.0`. `Cargo.lock` was regenerated: the
`mock-token` package is gone, and the Soroban dependency graph is at `29.0.1`.

---

## 3. Verification evidence

All commands were run from a clean checkout of this change set.

### Test suite

```text
$ cargo test --all --locked
...
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The 41 tests cover initialization, job creation and collateral locking, proof
submission and duration bounds, payout caps, dispute windows, timeout refunds,
admin resolution, TTL extension, event emission, the contract-balance invariant,
and the two new SAC-boundary failures. Settlement paths assert real SAC balances.

### Lint gate

```text
$ cargo fmt --all -- --check
$ cargo clippy --locked --all-targets -- -D warnings
```

Both completed with no changes required and no warnings.

### Canonical wasm build

```text
$ stellar contract build
    Wasm File: target/wasm32v1-none/release/pulserun_escrow.wasm
    Wasm Size: 20637 bytes optimized (original size was 23109 bytes)
    Wasm Hash: ac2afe1fb2c4e037cb781b3827ea0adc11c1edd34578f7245205603c54e69417
    Exported Functions: 14 found
✅ Build Complete
```

The 14 exports are the complete public ABI: `init`, `create_job`,
`submit_proof`, `claim_payout`, `dispute_job`, `resolve_dispute`,
`cancel_unclaimed_job`, `update_dispute_window`, and the six views.

### Toolchain versions used

- `rustc 1.100.0` / `cargo 1.100.0`
- `soroban-sdk 29.0.1`
- `stellar 30.0.1`

---

## 4. Remaining step: fresh testnet deployment

A current v0.3.0 instance has **not** been deployed by this change. That step
requires a funded identity controlled by the maintainer, which this code change
does not create. The path is ready:

```bash
PAYMENT_ASSET=native DISPUTE_WINDOW_SECS=86400 ./scripts/deploy-testnet.sh
```

The script prints the new escrow id, the resolved SAC id, and the admin address,
and writes them to `docs/deployments/testnet.env`. The historical v0.1.0
deployment and its transaction hashes remain valid on-chain evidence of the
original settlement flow, but the appeal should not claim that the v0.3.0 build
is deployed until this command has been run.

---

## 5. Files changed

| File | Change |
| --- | --- |
| `Cargo.toml` | Remove `mock_token` member; SDK `28.0.0` → `29.0.1`; version `0.2.0` → `0.3.0` |
| `contracts/escrow/Cargo.toml` | Remove `mock-token` dev-dependency |
| `contracts/escrow/src/lib.rs` | Document that `payment_token` is normally a SAC |
| `contracts/escrow/src/types.rs` | Document the SAC-backed `payment_token` field |
| `contracts/escrow/src/test.rs` | Use `register_stellar_asset_contract_v2` and `StellarAssetClient`; scope event assertions to the escrow; add two SAC-boundary tests |
| `contracts/mock_token/` | **Deleted** (contract, tests, manifest) |
| `.github/workflows/ci.yml` | Install `stellar/stellar-cli`; build with `stellar contract build`; remove env-var workaround |
| `.github/pull_request_template.md` | Verification step now `stellar contract build` |
| `scripts/deploy-testnet.sh` | Resolve a SAC instead of deploying a token; write `PAYMENT_ASSET`/`PAYMENT_TOKEN_ID` |
| `README.md` | SAC-based quick start, deploy, and layout |
| `CONTRIBUTING.md`, `docs/contributing.md` | Canonical build and SAC test conventions |
| `docs/architecture.md` | One contract; SAC dependency graph and deploy order |
| `docs/contract-reference.md` | Replace the mock-token section with the settlement-asset/SAC section |
| `docs/developer-guide.md` | Stellar CLI prerequisite, canonical build, SAC fixtures |
| `docs/protocol-mechanics.md` | `payment_token` documented as normally a SAC |
| `docs/deployments/testnet.md` | Historical deployment marked legacy; current SAC deploy flow |
| `docs/deployments/testnet.env` | Legacy record with the new SAC key names documented |
| `SECURITY.md` | Report scope now covers the escrow and its SAC test harness; SAC itself out of scope |
| `Cargo.lock` | Regenerated without `mock-token`, with the `29.0.1` graph |

Net diff: 22 tracked files changed, 295 insertions, 447 deletions (the deletion
count is mostly the removed toy-token crate), plus this changelog.

---

## 6. Why this strengthens the Stellar/Soroban relevance

- **There is nothing chain-agnostic left to point at.** The only contract is the
  escrow; money moves through Stellar's own SAC implementation.
- **Settlement works in the assets Stellar already moves.** Native XLM or any
  issued asset can be passed as the SAC address — no wrapping, no bridged
  counterparty, no bespoke token.
- **The project tracks the current protocol.** SDK 29 and the Stellar CLI 30
  build path are what a Soroban project in this wave should use.
- **The build is reproducible and canonical.** `stellar contract build` produces
  a spec-shaken, optimized wasm; the removed env-var workaround was a sign the
  project was not using the intended toolchain.
- **The test suite is stricter about Stellar behavior.** It exercises SAC
  transfers and their events, and covers the token-boundary failures that a
  handwritten mock could hide.

---

## 7. Suggested appeal summary

> Since the rejection, PulseRun Core has been rebased fully onto the canonical
> Soroban stack. The custom mock token and its deployment path were removed:
> settlement is now tested and deployed against Stellar's built-in Stellar Asset
> Contract, so jobs can settle in native XLM or any issued Stellar asset without
> a wrapper. The workspace moved to `soroban-sdk 29.0.1` (protocol 29), the wasm
> build and CI now use `stellar contract build` and the official Stellar CLI
> instead of a cargo workaround, and the deploy script resolves the asset's SAC
> id instead of deploying a toy token. The suite is 41 passing tests on the real
> SAC, clippy and fmt are clean, and `stellar contract build` produces a
> 20,637-byte artifact with a 14-function ABI. Architecture, reference,
> developer, contributing, security, and deployment docs were updated to match.
> The remaining step is a fresh testnet deployment of v0.3.0 with the
> maintainer's funded identity; the deploy script and docs are ready, and the
> prior v0.1.0 on-chain settlement evidence is preserved.

---

## 8. Reproduce locally

```bash
git clone https://github.com/PulseRun-Labs/pulserun-core
cd pulserun-core

rustup target add wasm32v1-none
rustup component add rustfmt clippy
curl -fsSL github.com/stellar/stellar-cli/raw/main/install.sh | sh

cargo test --all --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
stellar contract build
```

## 9. References

- [Stellar Asset Contract](https://developers.stellar.org/docs/build/smart-contracts/example-contracts/stellar-asset-contract)
- [SEP-41 token interface](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)
- [`soroban-sdk` 29.0.1](https://docs.rs/soroban-sdk/29.0.1/soroban_sdk/)
- [`stellar contract build`](https://developers.stellar.org/docs/tools/cli/stellar-cli)
