# Developer guide

Tests run with a stock Rust toolchain. There is no database and no network
service. Building the deployable wasm and deploying need the
[Stellar CLI](https://github.com/stellar/stellar-cli).

## Prerequisites

- Rust **1.84+** (`rustup`). The `wasm32v1-none` target needs a recent stable.
  [`rust-toolchain.toml`](../rust-toolchain.toml) pins the channel and installs
  the required components and target automatically.
- Components: `rustfmt`, `clippy`.
- The [Stellar CLI](https://github.com/stellar/stellar-cli) for contract builds
  and testnet deployment.

```bash
rustup target add wasm32v1-none
rustup component add rustfmt clippy
curl -fsSL github.com/stellar/stellar-cli/raw/main/install.sh | sh
```

## Local build and test

```bash
# Full test suite (41 tests).
cargo test --all --locked

# Match the lint gate.
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings

# Compile the deployable contract.
stellar contract build
```

The artifact lands at
`target/wasm32v1-none/release/pulserun_escrow.wasm`.

> **Why the CLI?** From `soroban-sdk` 28 onward, `stellar contract build` is the
> supported build path: it shakes the contract spec and optimizes the wasm, steps
> the Soroban runtime requires. Plain `cargo build` does not produce a deployable
> contract.

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
│   │       └── test.rs      # integration tests against the SAC
└── Cargo.toml           # workspace
```

## Settlement asset in tests

The suite does not define a token. It registers a real **Stellar Asset Contract**
for each test and mints balances through `StellarAssetClient`, then asserts
settlement with `TokenClient`:

```rust
let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
StellarAssetClient::new(&env, &token_id).mint(&requester, &1_000);
```

The escrow only ever talks to the token through the SEP-41 interface, so this is
the same code path a native or issued asset takes in production.

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

Read functions (`get_job`, `get_proof`, `job_count`, `dispute_window`, `admin`,
`get_config`) are invoked the same way and simulated without signing. Lifecycle
events can be read from transaction metadata or an indexer; the typed event
structs live in `contracts/escrow/src/events.rs`.

## Deploying to testnet

See [`deployments/testnet.md`](deployments/testnet.md) and
[`scripts/deploy-testnet.sh`](https://github.com/PulseRun-Labs/pulserun-core/blob/main/scripts/deploy-testnet.sh).

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

Full conventions are in
[`CONTRIBUTING.md`](https://github.com/PulseRun-Labs/pulserun-core/blob/main/CONTRIBUTING.md).
