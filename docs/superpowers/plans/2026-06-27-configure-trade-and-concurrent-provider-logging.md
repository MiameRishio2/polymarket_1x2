---
change: configure-trade-and-concurrent-provider-logging
design-doc: docs/superpowers/specs/2026-06-27-configure-trade-and-concurrent-provider-logging-design.md
base-ref: cf601acd3a9d7b6a1e9d87dba2af6fc6507b08f8
archived-with: 2026-06-28-configure-trade-and-concurrent-provider-logging
---

# Configurable Trade and Concurrent Provider Logging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load independently enabled Polymarket, OddsPortal, and trade settings from `config.yaml`, run both collectors concurrently, and expose provider-attributed process output.

**Architecture:** A new root `config` module deserializes the shared YAML and delegates provider-specific conversion to each provider module. `main` supervises independently spawned provider tasks with `JoinSet`, while OddsPortal owns its repeated polling loop and Polymarket retains its one-shot trade flow inside the existing WebSocket runtime.

**Tech Stack:** Rust 2021, Tokio, Serde/serde_yaml, anyhow, existing reqwest and WebSocket clients.

## Global Constraints

- Keep Polymarket-specific transport, credentials, and trade code under `src/polymarket/`.
- Keep OddsPortal-specific collection and polling under `src/oddsportal/`.
- Keep top-level provider task orchestration in `src/main.rs`.
- Do not add a logging dependency; use one prefixed stdout/stderr call per line.
- `trade.enabled` defaults to `false` and does not replace the existing three-mode `real` gate.
- Automated tests must not issue authenticated create-order or cancel-order requests.
- Preserve both provider JSONL schemas.
- Run `cargo test` after Rust changes.

archived-with: 2026-06-28-configure-trade-and-concurrent-provider-logging
---

### Task 1: Root Configuration and Independent Trade Gate

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs:1-3`
- Modify: `src/polymarket/config.rs:43-187`
- Modify: `src/polymarket/live.rs:217-430`
- Modify: `openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md`

**Interfaces:**
- Produces: `config::FileConfig::load(path) -> anyhow::Result<FileConfig>`
- Produces: `FileConfig::into_runtime() -> anyhow::Result<RuntimeConfig>`
- Produces: `RuntimeConfig { polymarket: Option<PolymarketRuntime>, oddsportal: Option<OddsPortalRuntime> }`
- Produces: `polymarket::config::build_runtime(input: RuntimeInput) -> anyhow::Result<(Config, Option<LiveConfig>)>`
- Consumes later: Task 2 uses `OddsPortalRuntime { config, poll_interval }`; Task 3 uses both optional runtime fields.

- [ ] **Step 1: Add compiling root configuration tests and API skeletons**

Create `src/config.rs` with the public runtime shapes and `FileConfig::parse` /
`FileConfig::into_runtime` signatures returning `unimplemented!()`, then add these expectations:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
polymarket:
  enabled: true
  url: https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03
  log_path: logs/aus-egy-polymarket.log
oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  home_team: Australia
  away_team: Egypt
  log_path: logs/aus-egy-oddsportal.log
  poll_interval_seconds: 30
trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
"#;

    #[test]
    fn builds_both_provider_runtimes_for_australia_egypt() {
        let runtime = FileConfig::parse(BASE).unwrap().into_runtime().unwrap();
        let polymarket = runtime.polymarket.unwrap();
        let oddsportal = runtime.oddsportal.unwrap();

        assert!(polymarket.live.is_none());
        assert_eq!(
            polymarket.config.polymarket_url,
            "https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03"
        );
        assert_eq!(oddsportal.config.home_team, "Australia");
        assert_eq!(oddsportal.config.away_team, "Egypt");
        assert_eq!(oddsportal.poll_interval.as_secs(), 30);
    }

    #[test]
    fn rejects_runtime_when_both_collectors_are_disabled() {
        let yaml = format!("{BASE}\n")
            .replace("polymarket:\n  enabled: true", "polymarket:\n  enabled: false")
            .replace("oddsportal:\n  enabled: true", "oddsportal:\n  enabled: false");
        assert_eq!(
            FileConfig::parse(&yaml)
                .unwrap()
                .into_runtime()
                .unwrap_err()
                .to_string(),
            "at least one provider collector must be enabled"
        );
    }
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test config::tests -- --nocapture`

Expected: tests compile and fail at runtime at the explicit `unimplemented!()` configuration
conversion boundary. A missing-symbol or syntax error is not an acceptable RED result.

- [ ] **Step 3: Implement the root config types and provider delegation**

Add these concrete public runtime shapes in `src/config.rs`:

```rust
pub struct RuntimeConfig {
    pub polymarket: Option<PolymarketRuntime>,
    pub oddsportal: Option<OddsPortalRuntime>,
}

pub struct PolymarketRuntime {
    pub config: crate::polymarket::config::Config,
    pub live: Option<crate::polymarket::config::LiveConfig>,
}

pub struct OddsPortalRuntime {
    pub config: crate::oddsportal::config::Config,
    pub poll_interval: std::time::Duration,
}
```

Define private Serde sections with `default_true()` for provider `enabled`, optional override
fields, and `trade.enabled` defaulting through `bool::default`. Move only root file reading and
deserialization from `polymarket::config::FileConfig`; keep `SecretString`, `AccountConfig`,
`TradeConfig`, `LiveConfig`, and account validation in the provider module. Add
`mod config;` to `main.rs`.

The Polymarket conversion input must carry:

```rust
pub struct RuntimeInput {
    pub proxy_url: String,
    pub gamma_host: String,
    pub clob_host: String,
    pub chain_id: u64,
    pub accounts: Vec<AccountConfig>,
    pub trade: TradeConfig,
    pub polymarket_url: String,
    pub log_path: PathBuf,
}
```

Implement `build_runtime(RuntimeInput)` by adapting the current `FileConfig::into_runtime`.

- [ ] **Step 4: Add and run trade-gate regression tests**

Update the existing live YAML fixture to include `enabled: true`. Add:

```rust
#[test]
fn missing_or_false_enabled_disables_live_account_validation() {
    for enabled_line in ["", "  enabled: false\n"] {
        let yaml = LIVE_YAML
            .replace("  enabled: true\n", enabled_line)
            .replace("    type: long", "    type: short");
        let input = runtime_input_from_yaml(&yaml);
        let (_, live) = build_runtime(input).unwrap();
        assert!(live.is_none());
    }
}
```

Run: `cargo test config::tests -- --nocapture`

Expected: all focused configuration and credential-redaction tests pass.

- [ ] **Step 5: Update dependent test fixtures and commit**

Replace `polymarket::config::FileConfig` construction in `src/polymarket/live.rs` tests with the
new root parser or `RuntimeInput` helper. Run:

```bash
cargo fmt --check
cargo test config::tests
cargo test polymarket::live::tests
```

Expected: all selected tests pass.

Commit:

```bash
git add src/config.rs src/main.rs src/polymarket/config.rs src/polymarket/live.rs \
  openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md
git commit -m "feat: configure provider runtimes and trade gate"
```

Mark OpenSpec tasks 1.1, 1.2, and 1.3 complete in the same commit.

archived-with: 2026-06-28-configure-trade-and-concurrent-provider-logging
---

### Task 2: OddsPortal Runtime Configuration and Polling Loop

**Files:**
- Modify: `src/oddsportal/config.rs:1-64`
- Modify: `src/oddsportal/mod.rs:8-178`
- Modify: `src/config.rs`
- Modify: `openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md`

**Interfaces:**
- Consumes: Task 1 `OddsPortalRuntime`.
- Produces: `oddsportal::run_poll_loop(config: Config, interval: Duration) -> anyhow::Result<()>`
- Produces privately: `run_poll_loop_with(config, interval, max_iterations, collect)`.
- Consumes later: Task 3 spawns `run_poll_loop`.

- [ ] **Step 1: Add compiling OddsPortal file-setting tests and API skeleton**

Add a provider-local deserializable `FileConfig` shape and an `into_runtime` signature returning
`unimplemented!()`, then add these expectations:

```rust
#[test]
fn file_config_builds_australia_egypt_runtime() {
    let file: FileConfig = serde_yaml::from_str(r#"
enabled: true
tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
home_team: Australia
away_team: Egypt
log_path: logs/oddsportal.log
poll_interval_seconds: 30
"#).unwrap();
    let (enabled, config, interval) = file.into_runtime(Some("http://proxy:7890".into())).unwrap();

    assert!(enabled);
    assert_eq!(config.home_team, "Australia");
    assert_eq!(config.away_team, "Egypt");
    assert_eq!(config.proxy_url.as_deref(), Some("http://proxy:7890"));
    assert_eq!(interval.as_secs(), 30);
}

#[test]
fn zero_poll_interval_is_rejected() {
    let file: FileConfig =
        serde_yaml::from_str("poll_interval_seconds: 0").unwrap();
    assert_eq!(
        file.into_runtime(None).unwrap_err().to_string(),
        "oddsportal.poll_interval_seconds must be greater than zero"
    );
}
```

- [ ] **Step 2: Run the tests and verify RED**

Run: `cargo test oddsportal::config::tests -- --nocapture`

Expected: tests compile and fail at the explicit `unimplemented!()` conversion boundary. Fix
all compiler errors before treating the run as RED.

- [ ] **Step 3: Implement provider-local file settings**

Add `FileConfig` with Serde defaults for current constants plus:

```rust
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 30;

pub fn into_runtime(
    self,
    proxy_url: Option<String>,
) -> anyhow::Result<(bool, Config, Duration)>
```

Use a `default_true()` function for `enabled`. Reject zero before returning `Duration`.
Update `src/config.rs` to embed this provider type and use its conversion.

- [ ] **Step 4: Add a compiling bounded polling-loop test**

Add the private `run_poll_loop_with` signature with an `unimplemented!()` body, then add an
internal loop test that returns errors without network access:

```rust
#[tokio::test]
async fn polling_continues_after_failed_pass() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&attempts);
    run_poll_loop_with(
        Config::default(),
        Duration::from_millis(1),
        Some(2),
        move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            async { Err(anyhow!("expected test failure")) }
        },
    )
    .await
    .unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}
```

The helper accepts `FnMut(Config) -> Future<Output = Result<Vec<OddsPortalRecord>>>`; production
passes `collect_once` and `max_iterations: None`.

- [ ] **Step 5: Run the polling test and verify RED**

Run: `cargo test oddsportal::tests::polling_continues_after_failed_pass -- --nocapture`

Expected: the test compiles and fails at `run_poll_loop_with`'s explicit `unimplemented!()`
body.

- [ ] **Step 6: Implement the polling loop and commit**

Implement immediate first collection, one success/error line, and one interval wait between
attempts. A pass error is consumed; configuration/setup errors have already been handled.

Run:

```bash
cargo fmt --check
cargo test oddsportal::config::tests
cargo test oddsportal::tests
```

Expected: all selected tests pass.

Commit:

```bash
git add src/config.rs src/oddsportal/config.rs src/oddsportal/mod.rs \
  openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md
git commit -m "feat: poll configured OddsPortal target"
```

Mark OpenSpec task 2.2 complete; task 1.1 may already cover the root OddsPortal settings.

archived-with: 2026-06-28-configure-trade-and-concurrent-provider-logging
---

### Task 3: Concurrent Provider Supervision

**Files:**
- Modify: `src/main.rs:1-35`
- Modify: `openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md`

**Interfaces:**
- Consumes: Task 1 `RuntimeConfig`, `PolymarketRuntime`, `OddsPortalRuntime`.
- Consumes: Task 2 `oddsportal::run_poll_loop`.
- Produces privately: `Provider` enum and `supervise(JoinSet<(Provider, Result<()>)>)`.

- [ ] **Step 1: Add compiling supervision tests and API skeleton**

Define `Provider` and the `supervise` signature with an `unimplemented!()` body, then add:

```rust
#[tokio::test]
async fn supervisor_waits_for_remaining_provider_after_one_fails() {
    let completed = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&completed);
    let mut tasks = JoinSet::new();
    tasks.spawn(async { (Provider::Polymarket, Err(anyhow!("expected failure"))) });
    tasks.spawn(async move {
        tokio::time::sleep(Duration::from_millis(1)).await;
        observed.store(true, Ordering::SeqCst);
        (Provider::OddsPortal, Ok(()))
    });

    assert!(supervise(tasks).await.is_err());
    assert!(completed.load(Ordering::SeqCst));
}
```

Also add a pure assertion that `RuntimeConfig::into_runtime` rejects zero enabled providers
before task creation.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test tests::supervisor_waits_for_remaining_provider_after_one_fails -- --nocapture`

Expected: the test compiles and fails at `supervise`'s explicit `unimplemented!()` body.

- [ ] **Step 3: Implement concurrent startup and supervision**

Add:

```rust
#[derive(Clone, Copy, Debug)]
enum Provider {
    Polymarket,
    OddsPortal,
}

async fn supervise(
    mut tasks: JoinSet<(Provider, anyhow::Result<()>)>,
) -> anyhow::Result<()> {
    let mut terminal_errors = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        // Record the provider result, report it, and continue joining.
    }
    Err(anyhow!("all provider tasks stopped: {}", terminal_errors.join("; ")))
}
```

In `main`, load `config::FileConfig`, spawn every configured provider before calling
`supervise`, and move Polymarket discovery into its spawned future:

```rust
let event = polymarket::discovery::discover_event(&runtime.config).await?;
polymarket::ws::run_market_stream(runtime.config, runtime.live, event).await
```

Do not restart the Polymarket future in the supervisor.

- [ ] **Step 4: Run orchestration tests and commit**

Run:

```bash
cargo fmt --check
cargo test tests:: -- --nocapture
```

Expected: crypto installation and supervision tests pass, including proof that the second task
finishes after the first error.

Commit:

```bash
git add src/main.rs openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md
git commit -m "feat: supervise provider collectors concurrently"
```

Mark OpenSpec tasks 2.1 and 2.3 complete.

archived-with: 2026-06-28-configure-trade-and-concurrent-provider-logging
---

### Task 4: Provider-Attributed Output and Operational Config

**Files:**
- Modify: `src/polymarket/discovery.rs`
- Modify: `src/polymarket/clob.rs`
- Modify: `src/polymarket/ws.rs:40-125`
- Modify: `src/polymarket/live.rs`
- Modify: `src/oddsportal/mod.rs`
- Modify: `config.yaml`
- Modify: `openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md`

**Interfaces:**
- Produces: stable `[polymarket]`, `[oddsportal]`, and `[trade]` process-output prefixes.
- Preserves: `QuoteLogger` and `OddsPortalLogger` serialized records.

- [ ] **Step 1: Add focused prefix and localized-slug tests with compiling prefix skeletons**

Add or update:

```rust
#[test]
fn extracts_supplied_localized_australia_egypt_slug() {
    assert_eq!(
        extract_slug(
            "https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03"
        )
        .unwrap(),
        "fifwc-aus-egy-2026-07-03"
    );
}
```

Define provider-local constants initially as empty strings so the test compiles and fails for
the expected value mismatch, then assert:

```rust
#[test]
fn provider_log_prefixes_are_stable() {
assert_eq!(polymarket::LOG_PREFIX, "[polymarket]");
assert_eq!(oddsportal::LOG_PREFIX, "[oddsportal]");
}
```

- [ ] **Step 2: Run focused tests and verify constants are missing**

Run:

```bash
cargo test extracts_supplied_localized_australia_egypt_slug
cargo test provider_log_prefixes_are_stable
```

Expected: slug behavior is verified independently; prefix test compiles and fails because the
skeleton constants are empty strings.

- [ ] **Step 3: Prefix affected lifecycle output**

Use the constants in every changed one-line output call. Initial snapshot and quote update lines
must begin `[polymarket]`; bookmaker record lines and poll summaries must begin `[oddsportal]`;
fixed-flow acceptance/failure output must begin `[trade]`. Do not interpolate a `SecretString`,
account, authenticated client, or signed order.

- [ ] **Step 4: Update committed runtime configuration**

Add:

```yaml
polymarket:
  enabled: true
  url: https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03
  log_path: logs/polymarket_quotes.log
oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  home_team: Australia
  away_team: Egypt
  log_path: logs/oddsportal_odds.log
  poll_interval_seconds: 30
trade:
  enabled: false
```

Preserve existing invalid credential placeholders and do not add real credentials.

- [ ] **Step 5: Run provider tests and commit**

Run:

```bash
cargo fmt --check
cargo test polymarket::
cargo test oddsportal::
```

Expected: provider parsing, logger, redaction, polling, and prefix tests pass.

Commit:

```bash
git add src/polymarket src/oddsportal config.yaml \
  openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md
git commit -m "feat: attribute concurrent provider output"
```

Mark OpenSpec tasks 3.1 through 3.4 and 4.1 complete.

archived-with: 2026-06-28-configure-trade-and-concurrent-provider-logging
---

### Task 5: Architecture, Deployment, and Full Verification

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `DEPLOYMENT.md`
- Modify: `openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md`
- Create: `docs/superpowers/verification/2026-06-27-configure-trade-and-concurrent-provider-logging.md`

**Interfaces:**
- Documents: root config ownership, concurrent data flow, trade migration, process-output prefixes.
- Verifies: deterministic suite plus bounded real-network smoke test.

- [ ] **Step 1: Update architecture and deployment documentation**

Add `src/config.rs` to the canonical tree, replace sequential provider flow with a forked
concurrent diagram, document provider-local polling and failure isolation, and state that
`trade.enabled: true` is required in addition to all three modes.

Document:

```bash
tail -f logs/polymarket-1x2.out.log
tail -f logs/polymarket_quotes.log
tail -f logs/oddsportal_odds.log
```

Explain the three prefixes and that a terminal error for one provider can coexist with the
other provider continuing.

- [ ] **Step 2: Run deterministic full verification**

Run:

```bash
cargo fmt --check
cargo test
./scripts/build.sh
```

Expected: formatting is clean, all tests pass, and `target/release/polymarket-1x2` is built.

- [ ] **Step 3: Run the bounded read-only smoke test**

First confirm `trade.enabled: false`. Ensure no managed instance is already running, then run:

```bash
timeout 90s target/release/polymarket-1x2 > /tmp/polymarket-1x2-smoke.log 2>&1
```

`timeout` exit 124 is expected for a healthy long-running collector. Any startup/configuration
exit before the timeout is a failure requiring investigation.

- [ ] **Step 4: Verify both provider outputs and Polymarket JSONL**

Run:

```bash
rg -n '^\\[polymarket\\]' /tmp/polymarket-1x2-smoke.log
rg -n '^\\[oddsportal\\]' /tmp/polymarket-1x2-smoke.log
test -s logs/polymarket_quotes.log
! rg -n '^\\[trade\\].*(placed|accepted|order_id)' /tmp/polymarket-1x2-smoke.log
```

Expected: both provider searches return at least one line, the Polymarket JSONL is non-empty,
and no placement output exists. If the environment proxy or upstream endpoint blocks a
provider, record the exact failure and do not mark live verification passed.

- [ ] **Step 5: Write verification evidence and commit**

Create the verification report with the commands, exit codes, test count, smoke duration,
provider-prefix evidence, JSONL evidence, and any external limitation. Do not paste credentials,
headers, signed payloads, or complete live records.

Mark OpenSpec tasks 4.2, 4.3, and 5.1 through 5.3 complete only when their evidence passes.

Commit:

```bash
git add ARCHITECTURE.md DEPLOYMENT.md docs/superpowers/verification \
  openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md
git commit -m "docs: verify concurrent provider runtime"
```

Expected: `git status --short` is clean after the commit.
