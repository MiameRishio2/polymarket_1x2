---
change: switch-target-to-jordan-argentina
design-doc: docs/superpowers/specs/2026-06-28-switch-target-to-jordan-argentina-design.md
base-ref: 18cfdf2c4a1db6c79c6d243170b605b7d8a8bc6d
archived-with: 2026-06-28-switch-target-to-jordan-argentina
---

# Switch Provider Target to Jordan–Argentina Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Configure both collectors for Jordan–Argentina while keeping trading disabled and
proving the selected live event produces both provider logs.

**Architecture:** Reuse the existing root configuration and provider runtimes. Change only target
values and focused regression fixtures; no provider interface, parser, supervisor, logger, or
trade behavior changes.

**Tech Stack:** Rust 2021, Serde YAML configuration, existing Tokio collectors and Cargo tests.

## Global Constraints

- Polymarket URL is exactly
  `https://polymarket.com/ja/sports/world-cup/fifwc-jor-arg-2026-06-27`.
- OddsPortal home team is `Jordan`; away team is `Argentina`.
- `trade.enabled` remains `false`.
- Provider log paths, polling cadence, and committed proxy placeholder remain unchanged.
- No credentials, authenticated writes, interfaces, dependencies, or JSONL schemas change.

archived-with: 2026-06-28-switch-target-to-jordan-argentina
---

### Task 1: Lock the Committed Provider Target

**Files:**
- Modify: `src/config.rs`
- Modify: `src/polymarket/discovery.rs`
- Modify: `config.yaml`
- Modify: `openspec/changes/switch-target-to-jordan-argentina/tasks.md`

**Interfaces:**
- Consumes: existing `FileConfig::parse` and `FileConfig::into_runtime`.
- Produces: no new runtime interface; only target values and regression coverage.

- [x] **Step 1: Add the failing committed-config test**

Add:

```rust
#[test]
fn committed_config_targets_jordan_argentina_with_trading_disabled() {
    let runtime = FileConfig::parse(include_str!("../config.yaml"))
        .unwrap()
        .into_runtime()
        .unwrap();
    let polymarket = runtime.polymarket.unwrap();
    let oddsportal = runtime.oddsportal.unwrap();

    assert_eq!(
        polymarket.config.polymarket_url,
        "https://polymarket.com/ja/sports/world-cup/fifwc-jor-arg-2026-06-27"
    );
    assert!(polymarket.live.is_none());
    assert_eq!(oddsportal.config.home_team, "Jordan");
    assert_eq!(oddsportal.config.away_team, "Argentina");
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test config::tests::committed_config_targets_jordan_argentina_with_trading_disabled -- --nocapture
```

Expected and observed: runtime assertion failure because the committed URL was still
`fifwc-aus-egy-2026-07-03`.

- [x] **Step 3: Change target values and slug fixture**

Set:

```yaml
polymarket:
  url: https://polymarket.com/ja/sports/world-cup/fifwc-jor-arg-2026-06-27
oddsportal:
  home_team: Jordan
  away_team: Argentina
```

Update the localized slug expectation to `fifwc-jor-arg-2026-06-27`.

- [x] **Step 4: Verify GREEN and commit**

Run:

```bash
cargo test config::tests::committed_config_targets_jordan_argentina_with_trading_disabled
cargo test polymarket::discovery::tests::extracts_supplied_localized_jordan_argentina_slug
cargo fmt --check
cargo test
```

Observed: both focused tests and all 70 tests passed.

Commit: `0dfab45 tweak: target Jordan Argentina providers`.

archived-with: 2026-06-28-switch-target-to-jordan-argentina
---

### Task 2: Verify the Live Read-Only Runtime

**Files:**
- Modify: `openspec/changes/switch-target-to-jordan-argentina/tasks.md`

**Interfaces:**
- Consumes: existing release binary and provider process-output/JSONL contracts.
- Produces: bounded smoke evidence; no production code.

- [x] **Step 1: Build and safety-check**

Run `./scripts/build.sh` and confirm `trade.enabled: false`.

- [x] **Step 2: Temporarily supply the documented proxy**

Change only the local `proxy` value to `http://10.32.110.233:7890`. Do not change credentials,
trading settings, targets, log paths, or polling.

- [x] **Step 3: Run the bounded smoke**

Run:

```bash
timeout 90s target/release/polymarket-1x2 > /tmp/jordan-argentina-smoke.log 2>&1
```

Expected and observed exit: `124`, proving the collector remained alive through the bound.

- [x] **Step 4: Verify both providers and restore configuration**

Observed:

- 438 Polymarket-prefixed lines;
- 85 OddsPortal-prefixed lines;
- event `fifwc-jor-arg-2026-06-27` discovered with 6 tokens;
- two successful OddsPortal passes with 39 records each;
- both provider JSONL files grew;
- zero trade-placement lines.

Restore `proxy: "YOUR_PROXY_URL"` and verify `git diff --exit-code HEAD -- config.yaml`.

- [x] **Step 5: Commit verification state**

Commit: `982fede tweak: verify Jordan Argentina collectors`.

archived-with: 2026-06-28-switch-target-to-jordan-argentina
---

### Task 3: Synchronize Target-Specific Acceptance Scenarios

**Files:**
- Create: `openspec/changes/switch-target-to-jordan-argentina/specs/polymarket-ws-quotes/spec.md`
- Create: `openspec/changes/switch-target-to-jordan-argentina/specs/oddsportal-js-odds/spec.md`
- Modify: `openspec/changes/switch-target-to-jordan-argentina/proposal.md`
- Modify: `openspec/changes/switch-target-to-jordan-argentina/design.md`
- Modify: `openspec/changes/switch-target-to-jordan-argentina/tasks.md`

**Interfaces:**
- Modifies no Rust interface.
- Produces archive-ready delta specs for existing provider-target requirements.

- [x] **Step 1: Copy complete affected requirement blocks**

Copy the full `Provider-local implementation` and `Configurable Polymarket collection target`
requirements, and the full `Configurable OddsPortal collection target` requirement, from the
main specs into delta specs under `## MODIFIED Requirements`.

- [x] **Step 2: Replace only target-specific scenario values**

Use the exact Jordan–Argentina URL, slug, home team, away team, and event display order. Preserve
all non-target requirement text and scenarios.

- [x] **Step 3: Run strict OpenSpec validation**

Run:

```bash
openspec validate switch-target-to-jordan-argentina --strict
```

Expected: PASS with both modified capability deltas parsed.
