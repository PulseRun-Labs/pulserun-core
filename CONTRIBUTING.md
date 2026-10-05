# Contributing to PulseRun Core

Thanks for helping build the settlement layer for pay-per-run compute on
Stellar. This guide gets you from zero to a merged pull request.

## Getting set up

You only need a stock Rust toolchain — no Stellar CLI, database, or services.

```bash
git clone https://github.com/PulseRun-Labs/pulserun-core
cd pulserun-core

rustup target add wasm32v1-none     # needed for the wasm build
rustup component add rustfmt clippy # needed for the lint gate

cargo test --all                    # should be green before you start
```

## The local gate

CI runs exactly these commands, in this order. Run them before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  cargo build --target wasm32v1-none --release -p pulserun-escrow -p mock-token
```

`cargo fmt --all` rewrites files; the `--check` form only verifies. Clippy is a
**hard gate**: `-D warnings` means a single lint fails the build. Fix warnings
rather than silencing them — if a suppression is genuinely warranted, explain
why in the PR description.

## Repository conventions

- **No `std`.** Contracts are `#![no_std]`; use `soroban_sdk` collections.
- **Errors, not panics.** Return `Result<_, Error>` from entrypoints and add a
  variant to `errors.rs` for each new failure mode. Error codes are ABI: never
  renumber an existing variant.
- **Storage goes through `storage.rs`.** Add a `DataKey` variant and typed
  accessors; don't reach into `env.storage()` from `lib.rs`.
- **Types live in `types.rs`** and derive `Clone, Debug, Eq, PartialEq`.
- **Authorization first.** Any entrypoint that acts on behalf of an address must
  validate the caller against stored state and call `require_auth`.
- **Tests are mandatory.** New behavior needs a test in
  `contracts/escrow/src/test.rs`, including the failure path and the error code.

## Test conventions

The suite drives the real contract through generated clients on
`soroban_sdk::Env`:

```rust
let w = setup();                       // env + funded mock token + initialized escrow
let escrow = w.escrow();
let token = w.token();

let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
w.env.ledger().set_timestamp(1_020);
escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 20, 0));
```

- Control the clock with `env.ledger().set_timestamp(..)`; never sleep.
- Assert contract errors by numeric code via the `try_*` clients:
  `assert_eq!(escrow.try_claim_payout(&job_id), Err(Ok(Error::DisputeWindowActive)))`.
- Keep each test to one behavior, and name it as a sentence about that behavior.

## Finding and scoping work

Open work is tracked in the [issue backlog](https://github.com/PulseRun-Labs/pulserun-core/issues).
Issues carry a size label so you can pick something that matches the time you
have:

| Label | Scope |
| --- | --- |
| `trivial` | Docs, tiny fixes, isolated tests, one-file changes. |
| `medium` | A new entrypoint, a storage change, or a multi-case test suite. |
| `high` | Cross-cutting design work: new settlement flows, gas/perf, security hardening. |

**How to pick up work**

1. Find an open issue you want and comment to be assigned it.
2. Open a **draft PR** early and link the issue (`Closes #123`).
3. Push until the full gate is green; keep the diff focused on the issue.
4. Request review once CI passes. Keep the PR description's checklist honest.

**Definition of done:** CI green, the behavior is covered by a test that fails
without your change, and the PR has no unrelated edits.

## Pull requests

Use the [pull request template](.github/pull_request_template.md). A good PR:

- Links its issue and states the scope it targets.
- Explains *why*, not just *what*.
- Calls out anything a reviewer should scrutinize (state transitions, arithmetic,
  authorization, TTL, ABI changes).
- Updates `README.md` when the public API changes.

## Reporting security issues

Do **not** open a public issue for vulnerabilities. Email
`security@pulserun.com` with a description and reproduction; we'll acknowledge
within 72 hours.

## License

By contributing you agree that your work is licensed under the
[MIT License](LICENSE).
