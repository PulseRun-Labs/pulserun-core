---
name: "Task · Medium (150 pts)"
about: Focused feature task — a new entrypoint, a storage/type change, or a test suite.
title: "[medium] "
labels: ["backlog", "medium", "150pts"]
assignees: ""
---

<!--
Medium · 150 points
Scope: a new entrypoint, a storage/type change, or a multi-case test suite.
Expected to touch a small set of files and require new tests.
-->

## Summary

<!-- One paragraph describing the task and the outcome. -->

## Motivation

<!-- The problem this solves. Reference the settlement flow it affects. -->

## Acceptance criteria

<!--
Concrete, verifiable outcomes. Include error codes, state transitions, and the
tests that must pass. Example:
- [ ] `foo(...) -> Result<T, Error>` added and exported from `lib.rs`
- [ ] New `Error` variant (appended, never renumbered) documented in `errors.rs`
- [ ] Happy path + failure path covered in `test.rs`
- [ ] `cargo test --all` and clippy are clean
-->

- [ ]
- [ ]
- [ ]
- [ ]

## Interface sketch

<!--
If this changes the contract API, show the call shape and note the backwards
compatibility impact on existing clients.
-->

```rust
// fn new_entrypoint(env: Env, ...) -> Result<..., Error>
```

## Edge cases to cover

- [ ]
- [ ]

## Scope guardrails

- No changes to unrelated modules or existing error code numbering.
- Reuse `storage.rs` accessors; do not reach into `env.storage()` directly.

## Points

**150 pts** · Effort: medium

## Useful context

- [`README.md`](../../README.md) — contract API table and security model
- [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — conventions and the local gate
