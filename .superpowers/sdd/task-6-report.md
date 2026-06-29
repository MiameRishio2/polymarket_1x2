# Task 6 Report: OddsPortal Concurrent Dual Polling

## Status

Complete.

## Files changed

- `src/oddsportal/mod.rs`
- `openspec/changes/discover-match-by-team-names/tasks.md`
- `.superpowers/sdd/task-6-report.md`

Task 5 already supplied the grouped odds and score observation models in
`src/oddsportal/output.rs` and score parsing/unavailable construction in
`src/oddsportal/score.rs`; Task 6 reused those interfaces without unnecessary edits.

## Implementation

- Builds one configured HTTP client and performs tournament/H2H discovery once at startup.
- Starts one odds future and one score future together with `tokio::join!` on every immediate
  interval tick.
- Uses `MissedTickBehavior::Skip` and awaits the joined cycle before requesting the next tick, so
  cycles cannot overlap.
- Retains frontend-xhash odds fallback inside the single odds future while removing recurring
  request retries.
- Treats a missing score URL and score HTTP 404 as `available: false`.
- Handles odds and score failures independently, preserving output from the successful side.
- Preserves per-outcome append-only odds JSONL and emits grouped odds plus score observations.

## TDD evidence

### RED

Command:

```text
cargo test oddsportal::tests::cycle_starts_odds_and_score_together_without_overlap -- --exact
```

Result: exit 101. Compilation failed because `run_poll_loop_with` still accepted four arguments
and only one collector; the new test supplied independent discovery, odds, and score inputs.
The compiler reported `E0061` plus the intentionally not-yet-created dual-polling helpers.

### GREEN

Command:

```text
cargo test oddsportal::tests::cycle_starts_odds_and_score_together_without_overlap -- --exact
```

Result: exit 0; 1 passed, 0 failed.

Independent-result command:

```text
cargo test oddsportal::tests::emits_successful_side_when_peer_request_fails -- --exact
```

Result: exit 0; 1 passed, 0 failed.

Focused regression commands:

```text
cargo test oddsportal::tests
cargo test oddsportal::odds::tests
cargo test oddsportal::decoder::tests
cargo test oddsportal::logging::tests
```

Results: exit 0 throughout; respectively 11, 2, 2, and 1 tests passed with zero failures.

### Full suite

Command:

```text
cargo test
```

Result: exit 0; 92 passed, 0 failed, 0 ignored.

Formatting/diff checks:

```text
cargo fmt
git diff --check
```

Result: formatting applied; diff check exited 0.

## Self-review

- Verified discovery retries remain startup-only.
- Verified recurring primary/fallback odds URLs each receive one attempt and score receives one
  attempt.
- Verified no detached tasks are spawned by production polling.
- Verified each odds record is still appended individually before grouped odds output.
- Verified the pre-existing `.comet.yaml` modification remains untouched and unstaged.

## Concerns

No known correctness concerns. HTTP status handling is implemented directly; existing focused
tests cover unavailable score serialization, while the paused-time seam covers scheduler
concurrency and independent side retention.
