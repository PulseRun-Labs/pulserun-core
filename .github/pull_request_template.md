<!--
Thanks for contributing to PulseRun Core! Keep this description accurate —
reviewers use it to decide what to scrutinize.
-->

## Summary

<!-- What does this PR change, and why? -->

## Related issue

<!-- e.g. Closes #123 -->

## Wave points

<!-- Delete what doesn't apply. -->
- [ ] `trivial` — 100 pts
- [ ] `medium` — 150 pts
- [ ] `high` — 200 pts
- [ ] Not a Wave issue

## Changes

<!-- Bullet the notable changes. Group by module if it helps. -->

-

## How it was tested

<!--
Describe the tests you added or ran. Name the behaviors they cover, including
the failure/error path where relevant.
-->

-

## Verification

<!--
Run the full gate locally. Check off only what actually passed. Paste the
`cargo test --all` summary line if it helps.
-->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --all`
- [ ] `SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 cargo build --target wasm32v1-none --release -p pulserun-escrow -p mock-token`

## Reviewer focus

<!--
What should a reviewer look at hardest? State transitions, arithmetic and
overflow behavior, authorization, TTL/storage layout, ABI/error-code changes,
or backwards compatibility.
-->

-

## Checklist

- [ ] No `std`; contracts remain `#![no_std]`.
- [ ] New failure modes return a typed `Error` (no panics on user input).
- [ ] No existing error code was renumbered.
- [ ] New behavior is covered by a test that fails without this change.
- [ ] `README.md` / docs updated if the public API changed.
- [ ] No unrelated edits are bundled into this PR.
