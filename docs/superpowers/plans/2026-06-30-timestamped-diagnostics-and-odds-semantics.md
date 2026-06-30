# Timestamped Diagnostics and Odds Semantics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every persisted runtime output line an explicit UTC timestamp while preserving JSONL parsing, and document that the collected OddsPortal prices are pre-match odds.

**Architecture:** Add a small shared `src/diagnostics.rs` output boundary used by the binary and both provider modules. Human-readable stderr messages receive an RFC 3339 millisecond prefix there; structured stdout and quote-file records retain their existing `received_at` and `ts` fields without textual prefixes.

**Tech Stack:** Rust, `chrono`, standard stderr I/O, JSONL, built-in Rust tests.

## Global Constraints

- Do not add in-play OddsPortal collection.
- Do not change odds values, polling behavior, provider discovery, or output schemas.
- Do not introduce a logging framework solely for this change.
- Keep provider prefixes unchanged after the timestamp.
- Preserve unrelated changes already present in the worktree.
- Run `cargo test` after Rust changes.

---

### Task 1: Shared Timestamped Diagnostic Boundary

**Files:**
- Create: `src/diagnostics.rs`
- Modify: `src/main.rs`
- Test: `src/diagnostics.rs`
- Test: `src/main.rs`

**Interfaces:**
- Consumes: `chrono::{DateTime, SecondsFormat, Utc}` and `std::fmt::Arguments`.
- Produces: `diagnostics::write(Arguments<'_>)`, which writes one timestamped line to stderr; private deterministic formatting used by unit tests.

- [ ] **Step 1: Write failing formatter and subprocess assertions**

Add a unit test in `src/diagnostics.rs` that formats a fixed UTC time and requires:

```rust
assert_eq!(
    format_line(fixed, format_args!("[oddsportal] starting collection pass")),
    "2026-06-30T12:34:56.789Z [oddsportal] starting collection pass"
);
```

In `src/main.rs`, add a test helper that splits each non-empty stderr line once at the first space, parses the first field with `DateTime::parse_from_rfc3339`, and asserts that the remaining text begins with `[polymarket]`, `[oddsportal]`, `[trade]`, or `[runtime]`. Apply it to the existing subprocess diagnostic tests.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test diagnostics -- --nocapture
```

Expected: compilation fails because `format_line`/the diagnostics module does not yet exist, or the existing unprefixed diagnostic lines fail the timestamp assertion.

- [ ] **Step 3: Implement the shared diagnostic writer**

Create `src/diagnostics.rs` with a deterministic formatter and production writer:

```rust
use std::fmt::Arguments;

use chrono::{DateTime, SecondsFormat, Utc};

fn format_line(at: DateTime<Utc>, message: Arguments<'_>) -> String {
    format!(
        "{} {message}",
        at.to_rfc3339_opts(SecondsFormat::Millis, true)
    )
}

pub(crate) fn write(message: Arguments<'_>) {
    eprintln!("{}", format_line(Utc::now(), message));
}
```

Declare `mod diagnostics;` in `src/main.rs`. Replace the binary's direct `eprintln!` calls with:

```rust
diagnostics::write(format_args!("{error}"));
```

and equivalent `format_args!` invocations for multiline messages. Use `[runtime]` for diagnostics that currently have no known provider prefix; preserve existing provider prefixes unchanged.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
cargo test diagnostics -- --nocapture
cargo test observation_helper_keeps_stdout_json_and_diagnostics_on_stderr -- --nocapture
```

Expected: both commands pass; stdout remains parseable JSON and every captured diagnostic line begins with an RFC 3339 timestamp.

- [ ] **Step 5: Commit the shared boundary**

```bash
git add src/diagnostics.rs src/main.rs
git commit -m "feat: timestamp runtime diagnostics"
```

### Task 2: Route Provider Diagnostics Through the Shared Boundary

**Files:**
- Modify: `src/polymarket/clob.rs`
- Modify: `src/polymarket/discovery.rs`
- Modify: `src/polymarket/sports.rs`
- Modify: `src/polymarket/ws.rs`
- Modify: `src/oddsportal/mod.rs`
- Test: `src/main.rs`
- Test: `src/oddsportal/mod.rs`

**Interfaces:**
- Consumes: `crate::diagnostics::write(Arguments<'_>)` from Task 1.
- Produces: timestamped Polymarket, OddsPortal, and trade diagnostics with unchanged provider prefixes and messages.

- [ ] **Step 1: Strengthen failing coverage for every provider prefix**

Update the existing subprocess helpers so they emit at least one diagnostic for `[polymarket]`, `[oddsportal]`, and `[trade]`. Assert, for every non-empty captured stderr line:

```rust
let (timestamp, diagnostic) = line
    .split_once(' ')
    .expect("diagnostic line must contain timestamp and message");
DateTime::parse_from_rfc3339(timestamp)
    .expect("diagnostic timestamp must be RFC 3339");
assert!(
    ["[polymarket]", "[oddsportal]", "[trade]", "[runtime]"]
        .iter()
        .any(|prefix| diagnostic.starts_with(prefix))
);
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test observation_helper_keeps_stdout_json_and_diagnostics_on_stderr -- --nocapture
cargo test polling_output_helper -- --nocapture
```

Expected: failure identifies provider diagnostics that still begin directly with `[provider]`.

- [ ] **Step 3: Replace provider `eprintln!` calls**

For each provider diagnostic, change:

```rust
eprintln!("{LOG_PREFIX} connecting market websocket");
```

to:

```rust
crate::diagnostics::write(format_args!("{LOG_PREFIX} connecting market websocket"));
```

Apply the same transformation to all production and subprocess-helper diagnostics in the listed files. Do not change the message body or provider prefix.

- [ ] **Step 4: Prove no direct production diagnostics remain**

Run:

```bash
rg -n 'eprintln!' src --glob '*.rs'
```

Expected: only the single low-level stderr write in `src/diagnostics.rs` remains.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run:

```bash
cargo test observation_helper_keeps_stdout_json_and_diagnostics_on_stderr -- --nocapture
cargo test terminal_sink_error_output -- --nocapture
```

Expected: all selected tests pass, every stderr line is timestamped, and all stdout lines remain JSON objects.

- [ ] **Step 6: Commit provider routing**

```bash
git add src/polymarket/clob.rs src/polymarket/discovery.rs src/polymarket/sports.rs src/polymarket/ws.rs src/oddsportal/mod.rs src/main.rs
git commit -m "refactor: route provider diagnostics through timestamped sink"
```

### Task 3: Preserve and Prove Structured Timestamp Contracts

**Files:**
- Modify: `src/main.rs`
- Modify: `src/oddsportal/models.rs`
- Test: `src/main.rs`
- Test: `src/oddsportal/models.rs`
- Test: `src/oddsportal/discovery.rs`

**Interfaces:**
- Consumes: existing stdout observation objects and `OddsPortalRecord`.
- Produces: explicit regression coverage that observations use `received_at`, detailed odds use `ts`, and request discovery selects `requestPreMatch.url`.

- [ ] **Step 1: Add structured-output timestamp assertions**

In the stdout subprocess test, require every parsed observation to contain a non-empty RFC 3339 `received_at`:

```rust
let received_at = observation["received_at"]
    .as_str()
    .expect("observation must contain received_at");
DateTime::parse_from_rfc3339(received_at)
    .expect("received_at must be RFC 3339");
```

In the `OddsPortalRecord` serialization test, parse the serialized record and assert:

```rust
assert_eq!(json["ts"], "2026-06-26T12:00:00Z");
DateTime::parse_from_rfc3339(json["ts"].as_str().unwrap()).unwrap();
```

Keep the discovery test that passes `requestPreMatch.url` and asserts the resulting field is `pre_match_url`.

- [ ] **Step 2: Run structured-output tests**

Run:

```bash
cargo test observation_helper_keeps_stdout_json_and_diagnostics_on_stderr -- --nocapture
cargo test oddsportal_record_serializes_provider_tag -- --nocapture
cargo test extracts_request_pre_match_url_from_h2h_state -- --nocapture
```

Expected: all tests pass without production schema changes.

- [ ] **Step 3: Commit regression coverage**

```bash
git add src/main.rs src/oddsportal/models.rs src/oddsportal/discovery.rs
git commit -m "test: lock output timestamps and pre-match odds source"
```

### Task 4: Synchronize Runtime Documentation

**Files:**
- Modify: `README.md`
- Modify: `ARCHITECTURE.md`
- Modify: `DEPLOYMENT.md`

**Interfaces:**
- Consumes: the timestamp and pre-match contracts implemented by Tasks 1–3.
- Produces: operator-facing documentation for diagnostic format, JSONL timestamp fields, and OddsPortal odds semantics.

- [ ] **Step 1: Update documentation**

Document these exact facts:

```text
Human-readable stderr diagnostics begin with an RFC 3339 UTC millisecond
timestamp followed by the stable provider prefix.

Stdout observations remain pure JSONL and carry `received_at`; detailed quote
JSONL remains pure JSONL and carries `ts`.

OddsPortal odds come from the discovered `requestPreMatch` `/match-event/...dat`
resource. They are pre-match odds, not in-play odds.
```

Update sample diagnostic lines to include a timestamp. Do not describe any live OddsPortal feed.

- [ ] **Step 2: Check documentation consistency**

Run:

```bash
rg -n 'pre-match|in-play|received_at|RFC 3339|diagnostic' README.md ARCHITECTURE.md DEPLOYMENT.md
git diff --check
```

Expected: all three documents state consistent timestamp and pre-match semantics; `git diff --check` is silent.

- [ ] **Step 3: Commit documentation**

```bash
git add README.md ARCHITECTURE.md DEPLOYMENT.md
git commit -m "docs: clarify timestamps and pre-match odds"
```

### Task 5: Full Verification and Main-Branch Delivery

**Files:**
- Verify: all files changed by Tasks 1–4.

**Interfaces:**
- Consumes: completed implementation and documentation.
- Produces: passing repository validation and pushed `origin/main`.

- [ ] **Step 1: Run formatting verification**

Run:

```bash
cargo fmt --check
```

Expected: exit status 0.

- [ ] **Step 2: Run full validation**

Run:

```bash
cargo test
```

Expected: exit status 0 with no failing tests.

- [ ] **Step 3: Audit scope and completion**

Run:

```bash
git status --short
git diff HEAD~3..HEAD --stat
rg -n 'eprintln!' src --glob '*.rs'
```

Expected: unrelated pre-existing worktree changes remain unstaged; the task commits contain only diagnostics, tests, and synchronized docs; direct `eprintln!` exists only in `src/diagnostics.rs`.

- [ ] **Step 4: Push main**

Run:

```bash
git push origin main
```

Expected: `origin/main` advances to the verified local `main` commit without force-push.
