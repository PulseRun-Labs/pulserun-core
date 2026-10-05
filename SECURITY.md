# Security Policy

PulseRun Core holds the on-chain escrow and settlement logic for PulseRun's
pay-per-run compute protocol on Stellar. The contracts custody requester funds
between job creation and settlement, so a bug here is a fund-safety bug.

This code is **unaudited**. It is a reference implementation; commission an
independent review before it touches mainnet value.

## Supported versions

Security fixes land on `main` and are released through reviewed pull requests.
Use the latest `main` commit or the most recent tagged release when deploying.
There is no long-term-support branch — only the tip of `main` is maintained.

## Scope

In scope:

- All Soroban contracts under [`contracts/escrow/`](contracts/escrow/) —
  `PulseEscrow` and its storage, types, and error surface.
- The test token under [`contracts/mock_token/`](contracts/mock_token/), where a
  finding could mask a real escrow bug in tests.
- Settlement arithmetic, authorization checks, state transitions, TTL/storage
  handling, and the claim/dispute/cancel flows.

Out of scope:

- Third-party wallets, RPC providers, and Stellar network infrastructure outside
  this repository.
- `mock_token`'s unauthenticated `mint` — it is test-only and documented as such.
- Issues in forked or unpublished deployments that diverge from `main`.
- Social engineering and denial-of-service against endpoints we do not run.

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately through one of these channels:

1. **GitHub private vulnerability reporting (preferred)**
   [Open a private security advisory](https://github.com/PulseRun-Labs/pulserun-core/security/advisories/new).
2. **Email** `security@pulserun.com` with a description and reproduction.

Include:

- The affected function and contract path.
- Steps to reproduce, including network (testnet/mainnet) and contract IDs if
  relevant.
- Impact assessment (fund loss, authorization bypass, locked funds, etc.).
- A proof of concept where possible, preferably against testnet.

## What to expect

- Acknowledgement within **72 hours**.
- Status updates as the report is triaged and remediated.
- Coordinated disclosure timing so users can patch before details are public.

## Safe harbour

We support good-faith research on **testnet** and local environments. Do not test
against mainnet funds you do not own, do not exfiltrate user data, and do not
degrade services you do not operate.

## Related documentation

- [Security model](README.md#security-model)
- [Protocol mechanics](docs/protocol-mechanics.md)
- [Contributing](CONTRIBUTING.md)
