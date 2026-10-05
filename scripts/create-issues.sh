#!/usr/bin/env bash
#
# Create the PulseRun Core backlog on GitHub in one run: labels first, then
# every planned issue with a Summary / Acceptance Criteria / Tech Stack body.
#
# Idempotent: issues whose exact title already exists (open or closed) are
# skipped, so re-running after adding an issue to this file only creates the new
# ones. Labels are created with --force so colors update in place.
#
# Requires: gh CLI authenticated with push access.
# Usage:    ./scripts/create-issues.sh [owner/repo]
set -euo pipefail

REPO="${1:-PulseRun-Labs/pulserun-core}"
GH=(gh -R "${REPO}")

echo "==> Creating/updating labels on ${REPO}"
create_label() { "${GH[@]}" label create "$1" --color "$2" --description "$3" --force >/dev/null; }
create_label "wave"          "7D00FF" "Drips Wave program issue"
create_label "trivial"       "C5DEF5" "Wave effort: trivial"
create_label "medium"        "BFD4F2" "Wave effort: medium"
create_label "high"          "D4C5F9" "Wave effort: high"
create_label "100pts"        "0E8A16" "Wave points: 100"
create_label "150pts"        "1D76DB" "Wave points: 150"
create_label "200pts"        "B60205" "Wave points: 200"
create_label "smart-contract" "5319E7" "Soroban contract work"
create_label "testing"       "FBCA04" "Test coverage"
create_label "security"      "D93F0B" "Security-relevant"
create_label "documentation" "0075CA" "Docs and guides"
create_label "ci"            "006B75" "Build and CI"

echo "==> Reading existing issues"
EXISTING="$("${GH[@]}" issue list --state all --limit 1000 --json title --jq '.[].title')"

create_issue() {
  local title="$1" labels="$2" body="$3"
  if printf '%s\n' "${EXISTING}" | grep -Fxq "${title}"; then
    echo "    skip (exists): ${title}"
    return 0
  fi
  "${GH[@]}" issue create --title "${title}" --label "${labels}" --body "${body}" >/dev/null
  echo "    created: ${title}"
}

echo "==> Creating issues"

create_issue \
  "feat(escrow): emit contract events on lifecycle transitions" \
  "wave,medium,150pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

The contract emits no events, so indexers must poll views to reconstruct job
state. Emit `contractevent`s on every lifecycle transition so the app layer can
follow jobs without polling.

## Acceptance Criteria

- [ ] Events emitted from `create_job`, `submit_proof`, `claim_payout`, `dispute_job`, and `cancel_unclaimed_job`
- [ ] Each event carries `job_id`; settlement carries earnings and refund
- [ ] Event topics/schema documented in `docs/contract-reference.md`
- [ ] Tests assert events via `env.events().all()` for each transition, including the failure paths (no event on a rejected call)
- [ ] `cargo test --all` and clippy clean

## Tech Stack

Rust, Soroban SDK v28, `#[contractevent]`. See `contracts/escrow/src/lib.rs`.
EOF
)"

create_issue \
  "feat(escrow): add admin dispute resolution to split or refund escrowed funds" \
  "wave,high,200pts,smart-contract,security" \
  "$(cat <<'EOF'
## Summary

`dispute_job` freezes payout but nothing decides the outcome — funds sit in
escrow forever. Add an admin-gated resolution that either refunds the requester
or pays the runner (optionally a split), and documents the trust assumption.

## Acceptance Criteria

- [ ] New entrypoint `resolve_dispute(admin, job_id, runner_share: i128)` (share in base units, `0..=max_budget`)
- [ ] Only the configured `admin` may call; `require_auth` on admin, checked against stored admin
- [ ] Job must be `Disputed`; moves to `Settled` (or a new terminal state) and drains the escrow exactly
- [ ] New `Error` variant(s) appended, never renumbered
- [ ] Tests: admin-only, wrong-state rejection, share bounds, exact drain (`runner_share + requester_refund == max_budget`)
- [ ] `docs/protocol-mechanics.md` updated with the resolution path and trust model

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `types.rs`, `errors.rs`, `storage.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): add runner accept/decline so runners are not silently assigned" \
  "wave,medium,150pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

`create_job` names a runner who never consents. If the address is wrong or the
runner is uninterested, the collateral is locked until timeout. Add an explicit
acceptance step.

## Acceptance Criteria

- [ ] New `accept_job(runner, job_id)` that only the named runner may call, moving `Queued` → a distinct accepted substate (or set an `accepted_at` field)
- [ ] `submit_proof` requires acceptance
- [ ] `cancel_unclaimed_job` still works if the runner never accepts
- [ ] New `Error` variant(s) appended; existing codes unchanged
- [ ] Tests: accept by non-runner rejected, accept after timeout rejected, proof before accept rejected, happy path accepted then proven
- [ ] Storage/type changes documented with a migration note

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `types.rs`, `storage.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): add paginated listing of jobs by requester and runner" \
  "wave,medium,150pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

There is no way to enumerate a party's jobs on-chain; clients would have to
scan every `job_id`. Add paginated index views keyed by requester and by runner.

## Acceptance Criteria

- [ ] `get_jobs_by_requester(requester, start, limit) -> Vec<u64>`
- [ ] `get_jobs_by_runner(runner, start, limit) -> Vec<u64>`
- [ ] `limit` bounded to a sane max to protect the ledger footprint
- [ ] Index entries written in `create_job` and TTL-extended alongside job records
- [ ] Tests: ordering, pagination boundaries, empty results, limit clamp
- [ ] `docs/contract-reference.md` updated

## Tech Stack

Rust, Soroban SDK v28. Touches `storage.rs` (new `DataKey` variants), `lib.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): emit a protocol fee to the admin on settlement" \
  "wave,high,200pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

The protocol captures no revenue. Add an optional, admin-configured fee taken
from the runner's earnings at settlement, expressed in basis points.

## Acceptance Criteria

- [ ] Fee in basis points (`0..=10_000`) set at `init` or via admin update, stored in instance storage
- [ ] Settlement computes `fee = earnings * bps / 10_000` with integer math only (no floats, checked)
- [ ] Fee paid to the admin; runner receives `earnings - fee`; requester refund unchanged
- [ ] Invariant preserved: `runner_net + fee + refund == max_budget`
- [ ] Tests: zero fee, max fee, rounding behavior, fee + cap interaction, exact drain
- [ ] Fee model documented with worked numbers in `docs/protocol-mechanics.md`

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `errors.rs`, `storage.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): allow anyone to trigger timeout refund after a grace period" \
  "wave,medium,150pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

Only the requester can cancel an unclaimed job. If they lose their keys or go
quiet, the runner cannot settle and the funds stay locked. Allow a permissionless
refund after `max_duration_secs + grace`, always sending funds to the requester.

## Acceptance Criteria

- [ ] New permissionless `sweep_expired_job(job_id)` callable by any address
- [ ] Requires `Queued` status and `created_at + max_duration_secs + grace <= now`
- [ ] Refund always goes to the stored requester, never the caller
- [ ] `grace` configured at `init` or a documented constant
- [ ] Tests: too-early rejection, funds to requester not caller, existing `cancel_unclaimed_job` unaffected
- [ ] `docs/protocol-mechanics.md` updated

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `storage.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): add update_dispute_window admin entrypoint and get_config view" \
  "wave,trivial,100pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

`dispute_window_secs` is fixed at `init`. Admins cannot tune it as requesters and
runners learn what window works, and clients must make two calls to read config.

## Acceptance Criteria

- [ ] `update_dispute_window(admin, secs)` gated on the stored admin with `require_auth`
- [ ] `get_config()` returning `(admin, dispute_window_secs, job_count)` in one call
- [ ] Non-admin callers rejected with `Unauthorized`
- [ ] Tests: admin-only update, effect on `claim_payout` timing, config view values
- [ ] `docs/contract-reference.md` updated

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `storage.rs`, `test.rs`.
EOF
)"

create_issue \
  "test(escrow): add invariant test that contract balance equals sum of open escrows" \
  "wave,high,200pts,testing" \
  "$(cat <<'EOF'
## Summary

The core safety property of the escrow is that the contract's token balance
always equals the sum of open (non-terminal) escrows. Prove it as a property over
long random action sequences, not just a couple of hand-written cases.

## Acceptance Criteria

- [ ] Property/fuzz harness driving create, proof, claim, dispute, cancel in random order
- [ ] Asserts token balance of the contract == sum of `max_budget` over non-terminal jobs after every action
- [ ] Asserts no terminal job can be re-settled
- [ ] Uses `soroban_sdk::Env` testutils; deterministic seeds logged on failure
- [ ] Runs in CI within a bounded time budget

## Tech Stack

Rust, Soroban SDK v28 testutils. Lives in `contracts/escrow/src/test.rs`.
EOF
)"

create_issue \
  "test(escrow): add table-driven cases for settlement cap and duration bounds" \
  "wave,medium,150pts,testing" \
  "$(cat <<'EOF'
## Summary

Settlement cap, duration bounds, and refund arithmetic are covered by a few
examples. Add table-driven cases spanning the boundaries to lock the behavior in.

## Acceptance Criteria

- [ ] Table covers `duration_secs` of `0, 1, max-1, max, max+1`
- [ ] Table covers metered totals below, equal to, and above `max_budget`
- [ ] Table covers very large `rate_per_second` that overflows `i128` -> `MathOverflow`
- [ ] Each case asserts payout, runner balance, requester refund, and contract balance
- [ ] `cargo test --all` clean

## Tech Stack

Rust, Soroban SDK v28 testutils. Lives in `contracts/escrow/src/test.rs`.
EOF
)"

create_issue \
  "test(mock_token): cover burn insufficiencies and self-transfer edges" \
  "wave,trivial,100pts,testing" \
  "$(cat <<'EOF'
## Summary

The mock token is what the escrow tests trust. Its own math should be fully
covered so a token bug cannot masquerade as an escrow bug.

## Acceptance Criteria

- [ ] `burn` over balance returns `InsufficientBalance`
- [ ] Self-transfer behavior is defined and asserted
- [ ] Zero/negative `burn` amount rejected
- [ ] Balance defaults to zero for unknown addresses
- [ ] `cargo test --all` clean

## Tech Stack

Rust, Soroban SDK v28 testutils. Lives in `contracts/mock_token/src/test.rs`.
EOF
)"

create_issue \
  "feat(escrow): validate payment_token is a contract before locking collateral" \
  "wave,medium,150pts,smart-contract,security" \
  "$(cat <<'EOF'
## Summary

`create_job` accepts any `Address` as `payment_token` and only fails when the
transfer call reverts. Validate the token interface explicitly so a typo surfaces
as a clear error rather than a confusing cross-contract failure.

## Acceptance Criteria

- [ ] `payment_token` must be a contract address, validated before the transfer
- [ ] A token without the expected `transfer` symbol is rejected with a typed error
- [ ] New `Error` variant appended
- [ ] Tests: non-contract address rejected, non-token contract rejected, valid token accepted
- [ ] `docs/contract-reference.md` updated

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `errors.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): add emergency pause for create_job and claim_payout" \
  "wave,high,200pts,smart-contract,security" \
  "$(cat <<'EOF'
## Summary

There is no way to stop new escrows or settlement if a flaw is found. Add an
admin emergency pause that halts state-changing entrypoints without touching
already-locked funds.

## Acceptance Criteria

- [ ] `set_paused(admin, bool)` gated on the stored admin
- [ ] When paused, `create_job`, `submit_proof`, `claim_payout`, `dispute_job`, and `cancel_unclaimed_job` return a typed `Paused` error
- [ ] Pausing never moves or locks existing funds; unpausing restores normal flow
- [ ] New `Error` variant appended; existing codes unchanged
- [ ] Tests: each entrypoint blocked while paused, resumes after unpause, funds intact throughout
- [ ] Trust assumption documented in `SECURITY.md` and the threat model

## Tech Stack

Rust, Soroban SDK v28. Touches `lib.rs`, `storage.rs`, `errors.rs`, `test.rs`.
EOF
)"

create_issue \
  "feat(escrow): add per-job metadata hash for off-chain job specs" \
  "wave,medium,150pts,smart-contract" \
  "$(cat <<'EOF'
## Summary

Jobs settle against an `output_hash` but carry no commitment to the *input* —
the commands, image, or repo the runner is expected to execute. Add a
`metadata_hash` so the job terms are pinned on-chain.

## Acceptance Criteria

- [ ] `create_job` accepts `metadata_hash: BytesN<32>` and stores it on the job
- [ ] `ComputeJob` gains the field; documented with a storage migration note
- [ ] Proof submission does not overwrite it
- [ ] Tests: value round-trips through `get_job`; old zeroed default for existing jobs
- [ ] `docs/protocol-mechanics.md` and `docs/contract-reference.md` updated

## Tech Stack

Rust, Soroban SDK v28. Touches `types.rs`, `lib.rs`, `test.rs`.
EOF
)"

create_issue \
  "ci: add cargo-deny dependency and license audit job" \
  "wave,trivial,100pts,ci" \
  "$(cat <<'EOF'
## Summary

CI does not audit dependencies for advisories, banned licenses, or duplicate
crates. Add a `cargo-deny` job to the workflow.

## Acceptance Criteria

- [ ] `deny.toml` with advisories, licenses, bans, and sources configured
- [ ] New job in `.github/workflows/ci.yml` runs `cargo deny check`
- [ ] Job name documented in `scripts/setup-branch-protection.sh` if made required
- [ ] Fails the build on a new advisory, matching the clippy hard-gate policy
- [ ] Run locally with `cargo deny check`; document the command in `CONTRIBUTING.md`

## Tech Stack

GitHub Actions, `cargo-deny`. Touches `.github/workflows/ci.yml`, adds `deny.toml`.
EOF
)"

create_issue \
  "ci(wasm): assert wasm size budget and publish artifacts per release" \
  "wave,trivial,100pts,ci" \
  "$(cat <<'EOF'
## Summary

The wasm job uploads artifacts but does not guard against unchecked binary
growth, and does not attach wasm to tagged releases.

## Acceptance Criteria

- [ ] CI fails if `pulserun_escrow.wasm` exceeds a documented size budget
- [ ] Size budget and rationale recorded in `docs/developer-guide.md`
- [ ] Tagged releases attach both wasm artifacts
- [ ] `docs/deployments/testnet.md` notes the size at the last deploy

## Tech Stack

GitHub Actions, Bash. Touches `.github/workflows/ci.yml`.
EOF
)"

create_issue \
  "docs: write a threat model and storage/TTL migration note" \
  "wave,medium,150pts,documentation" \
  "$(cat <<'EOF'
## Summary

`SECURITY.md` points at a security model in the README, but there is no
standalone threat model and no note on how storage or TTL changes are migrated
for live escrows. Both are expected of a program-ready repo.

## Acceptance Criteria

- [ ] `docs/threat-model.md` covering trust boundaries, actors, and the exact mechanism of each threat
- [ ] Storage layout documented (instance vs persistent keys) with the TTL policy
- [ ] Migration procedure for a `DataKey` or struct change on live escrows
- [ ] Linked from `SECURITY.md`, `docs/SUMMARY.md`, and `docs/contributing.md`
- [ ] No vague claims — each threat names the function and failure mode

## Tech Stack

Markdown. Adds `docs/threat-model.md`.
EOF
)"

echo "==> Done. View the backlog: gh issue list -R ${REPO} --label wave"
