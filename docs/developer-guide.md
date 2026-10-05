# Developer guide

Everything here runs with a stock Rust toolchain. There is no database, no
network service, and no required Stellar CLI for building or testing.

## Prerequisites

- Rust **1.84+** (`rustup`). The `wasm32v1-none` target needs a recent stable.
- Components: `rustfmt`, `clippy`.
- Optional: the [Stellar CLI](https://github.com/stellar/stellar-cli) for
  testnet deployment and canonical builds.

```bash
rustup target add wasm32v1-none
rustup component add rustfmt clippy
```

## Local build and test

```bash
# Full test suite (27 tests: 21 escrow + 6 token).
cargo test --all

# Match CI exactly.
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

# Compile deployable wasm.
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  cargo build --target wasm32v1-none --release -p pulserun-escrow -p mock-token
```

Artifacts land in
`target/wasm32v1-none/release/{pulserun_escrow,mock_token}.wasm`.

> **Why the env var?** `soroban-sdk` v28 will not emit a contract wasm unless the
> build system advertises spec-shaking support; the Stellar CLI sets that flag
> itself. Setting it lets plain `cargo` compile wasm in CI. For real deployments,
> prefer `stellar contract build`.

## Workspace layout

```text
pulserun-core/
├── contracts/
│   ├── escrow/          # PulseEscrow: escrow + settlement
│   │   └── src/
│   │       ├── lib.rs       # entrypoints
│   │       ├── types.rs     # ComputeJob, ExecutionProof, JobStatus
│   │       ├── storage.rs   # DataKey, TTL policy, accessors
│   │       ├── errors.rs    # stable error codes
│   │       └── test.rs      # integration-style unit tests
│   └── mock_token/      # minimal SEP-41-style token for tests
└── Cargo.toml           # workspace
```

## Calling the contract from TypeScript

The application layer uses `@stellar/stellar-sdk`. Build the contract client
from the deployed contract id and invoke by method name:

```ts
import {
  Contract,
  Networks,
  TransactionBuilder,
  BASE_FEE,
  nativeToScVal,
  Address,
} from "@stellar/stellar-sdk";

const contract = new Contract(ESCROW_CONTRACT_ID);

// create_job(requester, runner, payment_token, max_budget, rate_per_second, max_duration_secs)
const op = contract.call(
  "create_job",
  new Address(requester).toScVal(),
  new Address(runner).toScVal(),
  new Address(tokenId).toScVal(),
  nativeToScVal(1000n, { type: "i128" }),
  nativeToScVal(10n, { type: "i128" }),
  nativeToScVal(100, { type: "u64" }),
);

const tx = new TransactionBuilder(account, {
  fee: BASE_FEE,
  networkPassphrase: Networks.TESTNET,
})
  .addOperation(op)
  .setTimeout(30)
  .build();
```

Read functions (`get_job`, `get_proof`, `job_count`, `dispute_window`, `admin`)
are invoked the same way and simulated without signing.

## Deploying to testnet

See [`deployments/testnet.md`](deployments/testnet.md) and
[`scripts/deploy-testnet.sh`](../scripts/deploy-testnet.sh).

## Adding a new failure mode

1. Append a variant to `errors.rs`. Codes are ABI — never renumber an existing
   one.
2. Return it from the relevant entrypoint.
3. Add a test in `contracts/escrow/src/test.rs` asserting the numeric code via a
   `try_*` client.

## Conventions

- `#![no_std]` everywhere; use `soroban_sdk` collections.
- Errors, not panics, in entrypoints. No `unwrap()` outside tests.
- No floats. Amounts are `i128` base units; if you introduce a rate, express it
  in basis points as an integer.
- All storage access goes through `storage.rs`; add a `DataKey` variant rather
  than reaching into `env.storage()` from `lib.rs`.
- Types live in `types.rs` and derive `Clone, Debug, Eq, PartialEq`.
- Validate the caller against stored state, then call `require_auth()`.

Full conventions are in [`CONTRIBUTING.md`](../CONTRIBUTING.md).
