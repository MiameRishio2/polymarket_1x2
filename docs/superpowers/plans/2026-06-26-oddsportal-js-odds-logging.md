---
change: oddsportal-js-odds-logging
design-doc: docs/superpowers/specs/2026-06-26-oddsportal-js-odds-logging-design.md
base-ref: 1e4f7c7fd0720d6aa7af9b4a09dd504f6926e20d
---

# OddsPortal JS Odds Logging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Do not create git commits unless the user explicitly asks; use status checkpoints instead.

**Goal:** Add a read-only OddsPortal 1X2 odds collector that derives internal `.dat` requests from embedded page JavaScript state and logs normalized bookmaker odds alongside the existing Polymarket collector.

**Architecture:** Keep provider-specific behavior in `src/oddsportal/`. Add focused modules for config, models, discovery, decoder, odds parsing, and logging, then keep `src/main.rs` as orchestration only. Use fixture-driven tests for private OddsPortal shapes so network volatility does not break unit tests.

**Tech Stack:** Rust 2021, `reqwest`, `serde_json`, `scraper`, `html-escape`, `base64`, `flate2`, `percent-encoding`, existing `tokio` runtime.

---

## File Structure

- Modify: `Cargo.toml`
  Add small parsing/decoding dependencies.
- Modify: `src/oddsportal/mod.rs`
  Provider facade and module declarations.
- Create: `src/oddsportal/config.rs`
  Defaults for tournament URL, target teams, base URL, user agent, log path, and proxy option.
- Create: `src/oddsportal/models.rs`
  `TargetMatch`, `DiscoveredMatch`, `RequestMetadata`, `OddsPortalRecord`, and outcome types.
- Create: `src/oddsportal/discovery.rs`
  HTML embedded state extraction for tournament and H2H pages.
- Create: `src/oddsportal/decoder.rs`
  JXG-compatible `.dat` decode pipeline.
- Create: `src/oddsportal/odds.rs`
  JSON odds parser and 1X2 normalization.
- Create: `src/oddsportal/logging.rs`
  Append-only JSONL logger for OddsPortal records.
- Modify: `src/main.rs`
  Start OddsPortal collection/logging without changing Polymarket provider internals.
- Modify: `ARCHITECTURE.md`
  Update OddsPortal component and data flow after runtime behavior exists.

### Task 1: Dependencies And Models

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/oddsportal/mod.rs`
- Create: `src/oddsportal/config.rs`
- Create: `src/oddsportal/models.rs`

- [ ] **Step 1: Add dependencies**

Add:

```toml
base64 = "0.22"
flate2 = "1"
html-escape = "0.2"
percent-encoding = "2"
scraper = "0.20"
```

Run: `cargo check`
Expected: dependencies resolve, existing code still builds or only fails on missing new modules once imports are added.

- [ ] **Step 2: Define OddsPortal config**

Create `src/oddsportal/config.rs`:

```rust
use std::path::PathBuf;

pub const DEFAULT_BASE_URL: &str = "https://www.oddsportal.com";
pub const DEFAULT_TOURNAMENT_URL: &str =
    "https://www.oddsportal.com/football/world/world-championship-2026/";
pub const DEFAULT_HOME_TEAM: &str = "Norway";
pub const DEFAULT_AWAY_TEAM: &str = "France";
pub const DEFAULT_LOG_PATH: &str = "logs/oddsportal_odds.log";
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126 Safari/537.36";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub base_url: String,
    pub tournament_url: String,
    pub home_team: String,
    pub away_team: String,
    pub user_agent: String,
    pub proxy_url: Option<String>,
    pub log_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            tournament_url: DEFAULT_TOURNAMENT_URL.to_string(),
            home_team: DEFAULT_HOME_TEAM.to_string(),
            away_team: DEFAULT_AWAY_TEAM.to_string(),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            proxy_url: None,
            log_path: PathBuf::from(DEFAULT_LOG_PATH),
        }
    }
}
```

- [ ] **Step 3: Define data models**

Create `src/oddsportal/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetMatch {
    pub home_team: String,
    pub away_team: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredMatch {
    pub event_name: String,
    pub h2h_url: String,
    pub encoded_event_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestMetadata {
    pub pre_match_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OddsPortalRecord {
    pub ts: String,
    pub provider: String,
    pub event_id: String,
    pub event_name: String,
    pub bookmaker_id: String,
    pub bookmaker_name: String,
    pub outcome: String,
    pub decimal_odds: String,
    pub source_url: String,
}
```

- [ ] **Step 4: Expose modules**

Update `src/oddsportal/mod.rs`:

```rust
pub mod config;
pub mod decoder;
pub mod discovery;
pub mod logging;
pub mod models;
pub mod odds;
```

Run: `cargo test`
Expected: existing tests pass or fail only because later empty modules are not created yet.

### Task 2: Discovery From Embedded State

**Files:**
- Create: `src/oddsportal/discovery.rs`

- [ ] **Step 1: Write parser tests**

Add tests using small inline fixtures:

```rust
#[test]
fn discovers_match_from_tournament_embedded_state() {
    let html = r#"<div id="react-leagues-events" data='{"rows":[{"url":"\/football\/h2h\/france-QkGeVG1n\/norway-8rP6JO0H\/#bsJSJ30L","event":"Norway - France"}]}'></div>"#;
    let found = parse_tournament_match(html, "Norway", "France").unwrap();
    assert_eq!(found.event_name, "Norway - France");
    assert_eq!(found.encoded_event_id, "bsJSJ30L");
    assert_eq!(found.h2h_url, "https://www.oddsportal.com/football/h2h/france-QkGeVG1n/norway-8rP6JO0H/#bsJSJ30L");
}

#[test]
fn extracts_request_pre_match_url_from_h2h_state() {
    let html = r#"<event :data="{&quot;requestPreMatch&quot;:{&quot;url&quot;:&quot;\/match-event\/1-1-bsJSJ30L-1-2-yj159.dat?_= &quot;}}"></event>"#;
    let metadata = parse_h2h_request_metadata(html).unwrap();
    assert!(metadata.pre_match_url.starts_with("https://www.oddsportal.com/match-event/1-1-bsJSJ30L-1-2-yj159.dat?_="));
}
```

Run: `cargo test oddsportal::discovery -- --nocapture`
Expected: fail because functions do not exist.

- [ ] **Step 2: Implement parser helpers**

Implement:

```rust
pub fn parse_tournament_match(html: &str, home: &str, away: &str) -> Result<DiscoveredMatch>
pub fn parse_h2h_request_metadata(html: &str) -> Result<RequestMetadata>
fn absolute_url(base_url: &str, maybe_relative: &str) -> Result<String>
fn decode_attr(value: &str) -> String
```

Use `scraper::Html` to find `#react-leagues-events[data]` first, then parse the decoded `data` attribute with `serde_json`. Fallback to scanning decoded HTML for `"event":"Norway - France"` and the closest `url` if needed. For H2H, decode HTML entities and find `"requestPreMatch":{"url":"..."}` in component props.

Run: `cargo test oddsportal::discovery`
Expected: tests pass.

### Task 3: `.dat` Decoder

**Files:**
- Create: `src/oddsportal/decoder.rs`

- [ ] **Step 1: Write decoder tests**

Add tests for each phase with a generated fixture:

```rust
#[test]
fn decodes_base64_zlib_urlencoded_json() {
    let json = r#"{"d":{"oddsdata":{"back":{}}}}"#;
    let encoded = encode_test_payload(json);
    let decoded = decode_dat_payload(&encoded).unwrap();
    assert_eq!(decoded["d"]["oddsdata"]["back"], serde_json::json!({}));
}
```

Use a test-only `encode_test_payload` that zlib-compresses `percent_encoding::utf8_percent_encode(json, NON_ALPHANUMERIC)` and base64-encodes the bytes.

Run: `cargo test oddsportal::decoder -- --nocapture`
Expected: fail because decoder is missing.

- [ ] **Step 2: Implement decoder**

Implement:

```rust
pub fn decode_dat_payload(body: &str) -> Result<serde_json::Value>
```

Pipeline:

1. Trim body.
2. Base64 decode with `base64::engine::general_purpose::STANDARD`.
3. Try `flate2::read::ZlibDecoder`.
4. If zlib fails, try `flate2::read::GzDecoder`.
5. Convert bytes to UTF-8.
6. Percent-decode with `percent_encoding::percent_decode_str`.
7. Parse `serde_json::Value`.

Run: `cargo test oddsportal::decoder`
Expected: tests pass.

### Task 4: Odds Normalization

**Files:**
- Create: `src/oddsportal/odds.rs`

- [ ] **Step 1: Write odds parser tests**

Add a compact decoded fixture that mirrors the front-end shape:

```rust
#[test]
fn normalizes_one_x_two_bookmaker_odds() {
    let decoded = serde_json::json!({
        "d": {
            "encodeventId": "bsJSJ30L",
            "oddsdata": {
                "back": {
                    "0": {
                        "odds": {
                            "16": {"0": "4.20", "1": "3.70", "2": "1.85"}
                        },
                        "act": {"16": true}
                    }
                }
            },
            "providersNames": {"16": "bet365"}
        }
    });
    let records = normalize_1x2_odds(&decoded, "Norway - France", "https://www.oddsportal.com/match-event/test.dat").unwrap();
    assert_eq!(records.len(), 3);
    assert!(records.iter().any(|record| record.bookmaker_name == "bet365" && record.outcome == "X" && record.decimal_odds == "3.70"));
}
```

Run: `cargo test oddsportal::odds -- --nocapture`
Expected: fail because parser is missing.

- [ ] **Step 2: Implement 1X2 normalization**

Implement:

```rust
pub fn normalize_1x2_odds(
    decoded: &serde_json::Value,
    event_name: &str,
    source_url: &str,
) -> Result<Vec<OddsPortalRecord>>
```

Map column indexes to outcomes:

```rust
const OUTCOMES: [(&str, &str); 3] = [("0", "1"), ("1", "X"), ("2", "2")];
```

Read `d.encodeventId`, `d.providersNames`, and `d.oddsdata.back[*].odds[bookmaker_id][column]`. Skip inactive bookmakers when `act[bookmaker_id] == false`. Emit records with `provider = "oddsportal"` and `chrono::Utc::now().to_rfc3339()`.

Run: `cargo test oddsportal::odds`
Expected: tests pass.

### Task 5: Logging And Provider Facade

**Files:**
- Create: `src/oddsportal/logging.rs`
- Modify: `src/oddsportal/mod.rs`

- [ ] **Step 1: Write logger test**

Mirror the Polymarket logger test:

```rust
#[test]
fn creates_parent_directory_and_appends_oddsportal_json_line() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("logs/oddsportal_odds.log");
    let mut logger = OddsPortalLogger::new(&path).unwrap();
    logger.append(&sample_record()).unwrap();
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains("\"provider\":\"oddsportal\""));
    assert!(contents.ends_with('\n'));
}
```

Run: `cargo test oddsportal::logging -- --nocapture`
Expected: fail because logger is missing.

- [ ] **Step 2: Implement logger**

Use the same append-only pattern as `src/polymarket/logging.rs`, but accept `OddsPortalRecord`.

Run: `cargo test oddsportal::logging`
Expected: tests pass.

- [ ] **Step 3: Implement provider facade**

Add to `src/oddsportal/mod.rs`:

```rust
pub async fn collect_once(config: config::Config) -> anyhow::Result<Vec<models::OddsPortalRecord>> {
    let client = reqwest::Client::builder()
        .user_agent(config.user_agent.clone())
        .build()?;
    let tournament_html = client.get(&config.tournament_url).send().await?.error_for_status()?.text().await?;
    let discovered = discovery::parse_tournament_match(&tournament_html, &config.home_team, &config.away_team)?;
    let h2h_html = client.get(&discovered.h2h_url).send().await?.error_for_status()?.text().await?;
    let request = discovery::parse_h2h_request_metadata(&h2h_html)?;
    let dat_body = client.get(&request.pre_match_url).send().await?.error_for_status()?.text().await?;
    let decoded = decoder::decode_dat_payload(&dat_body)?;
    let records = odds::normalize_1x2_odds(&decoded, &discovered.event_name, &request.pre_match_url)?;
    let mut logger = logging::OddsPortalLogger::new(&config.log_path)?;
    for record in &records {
        println!(
            "oddsportal {} {} {} {}",
            record.event_name, record.bookmaker_name, record.outcome, record.decimal_odds
        );
        logger.append(record)?;
    }
    Ok(records)
}
```

If proxy support is needed in this task, add `reqwest::Proxy::all(proxy_url)` when `config.proxy_url` is `Some`.

Run: `cargo test oddsportal`
Expected: all OddsPortal unit tests pass.

### Task 6: Main Orchestration And Docs

**Files:**
- Modify: `src/main.rs`
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Wire OddsPortal startup**

Because Polymarket WebSocket streaming loops forever, run OddsPortal once before starting the stream:

```rust
let oddsportal_config = oddsportal::config::Config::default();
if let Err(error) = oddsportal::collect_once(oddsportal_config).await {
    eprintln!("OddsPortal collection failed: {error:#}");
}
let config = polymarket::config::Config::default();
let event = polymarket::discovery::discover_event(&config).await?;
polymarket::ws::run_market_stream(config, event).await
```

Run: `cargo test main -- --nocapture`
Expected: main module tests still pass.

- [ ] **Step 2: Update architecture docs**

Update `ARCHITECTURE.md`:

- Project identity includes collecting Polymarket quotes and OddsPortal 1X2 bookmaker odds.
- `src/oddsportal/` lists the implemented files.
- Components describe OddsPortal discovery, decoder, odds parser, and logger.
- Data flow includes the new OddsPortal path.
- External integrations include OddsPortal tournament/H2H pages and internal `/match-event/...dat` endpoint.

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Optional network smoke test**

Run:

```bash
cargo run
```

Expected before Polymarket stream takes over: stdout prints one or more `oddsportal ...` rows and `logs/oddsportal_odds.log` contains JSONL records with `"provider":"oddsportal"`. Stop manually after confirming startup logs.

## Self-Review

- Spec coverage: Tasks 2-5 cover OddsPortal embedded discovery, internal request use, compressed response decoding, 1X2 normalization, and logging. Task 6 covers preserving Polymarket orchestration while adding OddsPortal.
- Placeholder scan: No task relies on TBD/TODO placeholders; implementation signatures and test fixtures are concrete.
- Type consistency: `Config`, `DiscoveredMatch`, `RequestMetadata`, and `OddsPortalRecord` names are used consistently across tasks.
