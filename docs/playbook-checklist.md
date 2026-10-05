# Playbook checklist

Status of each phase of the Stellar Wave builder playbook for PulseRun, mapped
to the artifacts that satisfy it. `DONE` = artifact exists and is verified;
`BLOCKED` = cannot be completed from this environment; `PENDING` = owned by a
later step or the other repository.

| Phase | Status | Evidence / notes |
| --- | --- | --- |
| 1 · Ecosystem reconnaissance | DONE | Live search: 825 approved repos, escrow well-represented; SDF mandate (Growth / Product & Innovation); Wave 4 per-repo budgets. Findings reflected in positioning. |
| 2 · Idea generation | N/A | Product already defined (PulseRun); no competing directions to generate. |
| 3 · Critical review | N/A | Verdict already taken; the load-bearing Stellar dependency is argued in [`README.md`](../README.md#built-on-stellar--soroban). |
| 4 · Naming, scoping, repo structure | DONE | Two-repo split: `pulserun-core` (pure Rust workspace) and `pulserun-client` (app monorepo). |
| 5 · Contract architecture | DONE | [`architecture.md`](architecture.md) (dependency graph, storage layout, auth, effects-before-interactions) and [`contract-reference.md`](contract-reference.md). |
| 6 · Contract system prompt | DONE | [`contract-system-prompt.md`](contract-system-prompt.md), standalone. |
| 7 · App system prompt | DONE | [`app-system-prompt.md`](app-system-prompt.md), standalone; targets the empty `pulserun-client` repo. |
| 8 · Local environment & deployment | DONE | [`../scripts/deploy-testnet.sh`](../scripts/deploy-testnet.sh); deployment run and verified — see [`deployments/testnet.md`](deployments/testnet.md). |
| 9 · Hosting & service topology | PENDING | Topology (web / indexer / Postgres) specified in the app prompt; no services provisioned yet. |
| 10 · Repo hygiene | PARTIAL | Done: [`SECURITY.md`](../SECURITY.md), [`CONTRIBUTING.md`](../CONTRIBUTING.md), README, 16 labeled backlog issues, release `v0.1.0`. **BLOCKED**: GitHub topics + branch protection need an admin-scoped PAT (`../scripts/setup-repo.sh`). |
| 11 · Documentation site | DONE (content) | Full [`docs/`](.) site with `.gitbook.yaml`; not yet published to a docs host. |
| 12 · Submission | PARTIAL | [`SUBMISSION.md`](../SUBMISSION.md) assembled (description, links, repo relationship, planned issues). **PENDING**: live app, demo video. |
| 13 · Post-approval iteration | N/A | Applies after approval; scoping rules noted in `CONTRIBUTING.md`. |

## Open items, in dependency order

1. **Build `pulserun-client`** from [`app-system-prompt.md`](app-system-prompt.md).
   The repo currently holds only a placeholder README and LICENSE.
2. **Apply admin-scoped repo settings** (topics + branch protection) on both
   repos: `./scripts/setup-repo.sh`.
3. **Deploy the app** and wire `PULSEESCROW_ID` / `MOCKTOKEN_ID` into its env
   (local `.env.local` and the hosting platform UI).
4. **Publish the docs site** (GitBook from `docs/`).
5. **Record the demo video** — create job → submit proof → window-blocked claim →
   paid claim, using the deployed testnet contracts.
6. **Fill the maintainer Telegram handle** in `README.md`.

## Verified facts (not claims)

- Contracts deployed on Stellar testnet and exercised end-to-end over the public
  RPC — IDs, tx hashes, and balances in [`deployments/testnet.md`](deployments/testnet.md).
- `cargo test --all` → 27 passing; CI green (`fmt`, `clippy -D warnings`, test,
  `wasm32v1-none` build).
- Contracts emit no events yet; indexers must poll views (tracked as an issue).
