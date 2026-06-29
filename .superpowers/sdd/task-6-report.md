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

## Follow-up quality review

Three blocking test gaps were closed in commit follow-up work without changing the Task 6 runtime
contract.

### Strengthened non-overlap RED/GREEN

The paused-time test now holds both cycle-1 futures behind a semaphore, advances five one-second
ticks, and proves neither collector starts again while the first pair remains active. After
release, exactly one later cycle runs and `max_active` remains two.

RED:

```text
cargo test oddsportal::tests::cycle_starts_odds_and_score_together_without_overlap -- --exact
```

Exit 101: the new gated test referenced the not-yet-created `gated_odds` and `gated_scores`
helpers.

GREEN: the same command exited 0 with 1 passed and 0 failed.

### Actual sink handling RED/GREEN

Production cycle handling is now injectable at the append-only odds sink, grouped odds output
sink, and score output sink. The collectors remain pure request futures.

RED:

```text
cargo test oddsportal::tests::odds_failure_still_emits_score -- --exact
```

Exit 101: `handle_cycle_with` did not yet exist.

GREEN:

```text
cargo test oddsportal::tests::odds_failure_still_emits_score -- --exact
cargo test oddsportal::tests::score_failure_still_appends_and_emits_odds -- --exact
```

Both commands exited 0 with 1 passed and 0 failed. The first proves a score reaches its output
sink when odds fail; the second proves odds reach both the append and grouped-output sinks when
score fails.

### HTTP policy boundary RED/GREEN

Deterministic loopback HTTP tests now cover:

- missing score URL returning unavailable without a request;
- score HTTP 404 returning unavailable after one request;
- score HTTP error receiving no retry;
- one primary odds attempt followed by one fallback attempt after failure;
- one primary odds attempt followed by one fallback attempt after an empty normalized batch;
- successful primary odds preventing any fallback request.

RED:

```text
cargo test oddsportal::tests::score_404_returns_unavailable_after_one_attempt -- --exact
```

Exit 101: `TestHttpServer` and `encoded_odds_payload` did not yet exist. The first loopback run
also exposed ambient proxy inheritance (502), so the deterministic test client was explicitly
configured with `no_proxy`.

GREEN: all five exact request-policy commands exited 0; the complete OddsPortal polling module
then passed 18 tests.

### Follow-up verification

Focused commands:

```text
cargo test oddsportal::tests
cargo test oddsportal::odds::tests
cargo test oddsportal::decoder::tests
cargo test oddsportal::logging::tests
cargo test oddsportal::output::tests
cargo test oddsportal::score::tests
```

All exited 0; respectively 18, 2, 2, 1, 3, and 2 tests passed.

Full suite:

```text
cargo test
```

Exit 0: 99 passed, 0 failed, 0 ignored.

Follow-up concerns: none. The loopback server is test-only, binds an ephemeral localhost port,
and bypasses ambient proxy variables for deterministic request counts.

## Sink-status regression follow-up

### Root cause

The injectable sink handler logged append/output errors but returned `()`. The polling loop then
used the presence of collected odds or score (`Option::is_some`) as its success signal. That
discarded the distinction between collection success and required sink success. The handler also
called grouped odds output after append failure, violating the required per-outcome JSONL-first
ordering.

### RED

Command:

```text
cargo test oddsportal::tests::odds_append_failure_short_circuits_grouped_output_and_marks_side_failed -- --exact
```

Result: exit 101. Compilation failed with `E0609` because `handle_cycle_with` returned `()` and
therefore had no `odds_succeeded` or `score_succeeded` status fields.

### GREEN

Commands:

```text
cargo test oddsportal::tests::odds_append_failure_short_circuits_grouped_output_and_marks_side_failed -- --exact
cargo test oddsportal::tests::stdout_sink_failures_mark_each_side_failed -- --exact
```

Both exited 0 with 1 passed and 0 failed.

The handler now returns independent per-side status. Odds append failure logs the failure,
short-circuits grouped odds output, and leaves score handling independent. Grouped odds or score
stdout failure marks only that side failed. Polling success diagnostics are emitted only from
these post-sink statuses, and the loop continues to its next interval after all sink failures.

### Verification

Focused commands:

```text
cargo test oddsportal::tests
cargo test oddsportal::logging::tests
cargo test oddsportal::output::tests
cargo test oddsportal::score::tests
```

All exited 0; respectively 20, 1, 3, and 2 tests passed.

Full suite:

```text
cargo test
```

Exit 0: 101 passed, 0 failed, 0 ignored.

Concerns: none.
