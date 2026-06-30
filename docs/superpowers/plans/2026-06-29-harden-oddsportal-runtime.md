---
change: harden-oddsportal-runtime
design-doc: docs/superpowers/specs/2026-06-29-harden-oddsportal-runtime-design.md
base-ref: eb22ffa68175ad3d5d41ee0f9c41c782b53ef3b0
archived-with: 2026-06-30-harden-oddsportal-runtime
---

# Harden OddsPortal Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the proxied OddsPortal client request identity-encoded HTTP responses without
changing its one-second polling interval.

**Architecture:** Extend the existing shared OddsPortal `reqwest::Client` default headers. Verify
the resulting wire request through the local HTTP test server, then validate the release binary
against the live proxied endpoint.

**Tech Stack:** Rust, Tokio, reqwest, Cargo tests, Bash deployment scripts.

## Global Constraints

- Keep `oddsportal.poll_interval_seconds: 1`.
- Keep all OddsPortal-specific code under `src/oddsportal/`.
- Keep the root proxy route; do not add direct fallback.
- Preserve unrelated user changes in `config.yaml`.

archived-with: 2026-06-30-harden-oddsportal-runtime
---

### Task 1: Identity-Encoding Transport Header

**Files:**
- Modify: `src/oddsportal/mod.rs`
- Test: `src/oddsportal/mod.rs`

**Interfaces:**
- Consumes: `build_client_with_timeouts(config, connect_timeout, request_timeout)`.
- Produces: an OddsPortal client whose requests contain `Accept-Encoding: identity`.

- [x] **Step 1: Add a failing wire-header test**

Build the real client, issue a request to `TestHttpServer`, and assert that the captured
case-insensitive request text contains `accept-encoding: identity\r\n`.

- [x] **Step 2: Verify the test fails**

Run: `cargo test oddsportal_client_requests_identity_encoding -- --nocapture`

Expected: FAIL because the header is absent.

- [x] **Step 3: Add the default header**

Import `reqwest::header::ACCEPT_ENCODING` and insert:

```rust
headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
```

- [x] **Step 4: Verify the focused tests pass**

Run: `cargo test oddsportal_client_requests_identity_encoding -- --nocapture`

Expected: one test passes.

- [x] **Step 5: Commit the transport fix**

Stage only `src/oddsportal/mod.rs` and the current change artifacts, excluding the unrelated
Brazil/Japan hunk in `config.yaml`.

### Task 2: Full and Live Verification

**Files:**
- Modify: `openspec/changes/harden-oddsportal-runtime/tasks.md`
- Create: `docs/superpowers/reports/2026-06-29-harden-oddsportal-runtime-verify.md`

**Interfaces:**
- Consumes: the release binary and configured HTTP proxy.
- Produces: repeatable test/build evidence and a bounded live-runtime result.

- [x] **Step 1: Format and run all tests**

Run: `cargo fmt -- --check` and `cargo test`.

Expected: formatting succeeds and all tests pass.

- [x] **Step 2: Build release**

Run: `./scripts/build.sh`.

Expected: `target/release/polymarket-1x2` builds successfully.

- [x] **Step 3: Run live verification**

Run the release binary under a bounded timeout with network access and inspect fresh output for
successful OddsPortal odds collection, while separately recording external proxy/upstream
transients.

- [x] **Step 4: Record verification and commit**

Write the verification report, complete the OpenSpec task checklist, archive the change, and
commit only task-owned files.
