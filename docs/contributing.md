# Contributing

The canonical guide is [`CONTRIBUTING.md`](../CONTRIBUTING.md) at the repository
root. This page is the short version.

## The gate

Run these before opening a pull request. They are exactly what CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  cargo build --target wasm32v1-none --release -p pulserun-escrow -p mock-token
```

Clippy is a hard gate: `-D warnings` means one lint fails the build.

## Rules that matter here

- **No `std`.** Contracts are `#![no_std]`.
- **Errors, not panics.** Return `Result<_, Error>` and add a variant to
  `errors.rs`. Error codes are ABI: never renumber.
- **Storage via `storage.rs`.** Add a `DataKey` variant; do not touch
  `env.storage()` from `lib.rs`.
- **Authorization first.** Validate the caller against stored state, then call
  `require_auth()`.
- **Tests are mandatory.** New behavior needs a test covering the failure path
  and asserting the error code through a `try_*` client.

## Finding and scoping work

Open work is tracked in the [issue backlog](../../issues). Issues carry a size
label — `trivial`, `medium`, or `high` — so you can pick something that matches
the time you have. To pick up work, choose an issue, comment to be assigned, and
open a draft PR early. Push until the full gate is green and keep the diff
focused on the issue.

## Reporting security issues

Do not open a public issue. See [`SECURITY.md`](../SECURITY.md).
