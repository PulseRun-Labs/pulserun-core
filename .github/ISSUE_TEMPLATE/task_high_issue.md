---
name: "Task · High (200 pts)"
about: Cross-cutting design task — new settlement flows, security hardening, or ABI evolution.
title: "[high] "
labels: ["backlog", "high", "200pts"]
assignees: ""
---

<!--
High · 200 points
Scope: cross-cutting design work — new settlement flows, security hardening,
gas/resource optimization, or ABI evolution. Expect a design note in the PR.
-->

## Summary

<!-- One paragraph describing the objective and the end state. -->

## Problem statement

<!--
What is wrong or missing today? Quantify where possible (gas, correctness,
liveness, capital efficiency).
-->

## Proposed approach

<!--
Outline the design. Call out the invariants it must preserve and the state
transitions it introduces.
-->

## Acceptance criteria

<!--
Verifiable outcomes, including invariants and tests. Example:
- [ ] Invariant: contract token balance == sum of open escrows (asserted in tests)
- [ ] Property covered by table-driven tests
- [ ] Wasm size within 5% of baseline
- [ ] Full CI gate green
-->

- [ ]
- [ ]
- [ ]
- [ ]

## Invariants & risk

<!--
What must always hold? What could break? What is the failure/recovery path if
it goes wrong on-chain?
-->

- Invariant:
- Risk:
- Recovery:

## Design note required

The PR must include a short design note covering: threat model, state changes,
ABI impact, migration path for live escrows, and why alternatives were rejected.

## Scope guardrails

- May touch multiple modules, but must not change existing error codes.
- Any storage-layout change needs an explicit migration note.

## Points

**200 pts** · Effort: high

## Useful context

- [`README.md`](../../README.md) — security model and settlement math
- [`contracts/escrow/src/lib.rs`](../../contracts/escrow/src/lib.rs)
- [`contracts/escrow/src/storage.rs`](../../contracts/escrow/src/storage.rs)
