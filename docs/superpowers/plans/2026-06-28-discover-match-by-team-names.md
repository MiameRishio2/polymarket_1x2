---
change: discover-match-by-team-names
design-doc: docs/superpowers/specs/2026-06-28-discover-match-by-team-names-design.md
base-ref: 6674ba79ee85e58eeb8ed6f9ac6952e428c82297
---

# Team-Name Market and Score Observation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Configure one South Africa–Canada team pair, automatically discover the matching
Polymarket and OddsPortal events, and emit four independent timestamped JSON observation streams
for odds and scores.

**Architecture:** Root configuration validates and injects one match pair into provider-local
runtimes. Polymarket runs independent CLOB and Sports WebSocket loops; OddsPortal discovers its
odds and score URLs once and runs non-overlapping one-second cycles containing two concurrent HTTP
requests. Providers serialize their own observations directly to stdout and keep diagnostics on
stderr; there is no cross-provider state.

**Tech Stack:** Rust 2021, Tokio, reqwest, tokio-tungstenite, serde/serde_json, chrono, anyhow,
existing OddsPortal decoder and provider-local JSONL loggers.

## Global Constraints

- Keep Polymarket-specific code under `src/polymarket/` and OddsPortal-specific code under
  `src/oddsportal/`.
- Keep `src/main.rs` as orchestration only; do not add provider parsing there.
- stdout contains complete observation JSON objects only; lifecycle and error diagnostics use
  stderr with stable provider prefixes.
- Polymarket console odds include only the classified home/draw/away `Yes` tokens.
- Each OddsPortal cycle starts one odds request and one score request concurrently, and cycles
  never overlap.
- Preserve existing provider JSONL record schemas and append behavior.
- Preserve all live-trading gates; observation code must not invoke authenticated order APIs.
- Do not add a root aggregator, shared observation state, arbitrage calculation, or provider
  comparison.
- Run `cargo test` after Rust changes.

---

### Task 1: Shared Match Configuration

**Files:**
- Modify: `src/config.rs`
- Modify: `src/polymarket/config.rs`
- Modify: `src/oddsportal/config.rs`
- Modify: `config.yaml`

**Interfaces:**
- Produces: `MatchSection { home_team: String, away_team: String }` owned by root configuration.
- Produces: provider-local `Config.home_team` and `Config.away_team` strings.
- Removes: configured `polymarket.url`, `oddsportal.home_team`, and `oddsportal.away_team`.
- Preserves: `build_runtime(RuntimeInput) -> Result<(polymarket::Config, Option<LiveConfig>)>`.

- [ ] **Step 1: Add failing root configuration tests**

Replace the target-specific fixtures in `src/config.rs` with a shared pair and add validation
tests:

```rust
const BASE: &str = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
match:
  home_team: Australia
  away_team: Egypt
polymarket:
  enabled: true
  log_path: logs/aus-egy-polymarket.log
oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  log_path: logs/aus-egy-oddsportal.log
  poll_interval_seconds: 1
trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
"#;

#[test]
fn injects_one_match_pair_into_both_providers() {
    let runtime = FileConfig::parse(BASE).unwrap().into_runtime().unwrap();
    let polymarket = runtime.polymarket.unwrap().config;
    let oddsportal = runtime.oddsportal.unwrap();

    assert_eq!((polymarket.home_team.as_str(), polymarket.away_team.as_str()),
               ("Australia", "Egypt"));
    assert_eq!((oddsportal.config.home_team.as_str(), oddsportal.config.away_team.as_str()),
               ("Australia", "Egypt"));
    assert_eq!(oddsportal.poll_interval, Duration::from_secs(1));
}

#[test]
fn rejects_blank_or_equal_match_names() {
    for yaml in [
        BASE.replace("home_team: Australia", "home_team: '  '"),
        BASE.replace("away_team: Egypt", "away_team: australia"),
    ] {
        assert!(FileConfig::parse(&yaml)
            .unwrap()
            .into_runtime()
            .unwrap_err()
            .to_string()
            .contains("match"));
    }
}

#[test]
fn committed_config_targets_south_africa_canada_read_only() {
    let runtime = FileConfig::parse(include_str!("../config.yaml"))
        .unwrap()
        .into_runtime()
        .unwrap();
    let polymarket = runtime.polymarket.unwrap();
    let oddsportal = runtime.oddsportal.unwrap();

    assert_eq!(polymarket.config.home_team, "South Africa");
    assert_eq!(polymarket.config.away_team, "Canada");
    assert_eq!(oddsportal.config.home_team, "South Africa");
    assert_eq!(oddsportal.config.away_team, "Canada");
    assert_eq!(oddsportal.poll_interval, Duration::from_secs(1));
    assert!(polymarket.live.is_none());
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
cargo test config::tests::injects_one_match_pair_into_both_providers -- --exact
```

Expected: compilation fails because `FileConfig` has no `match` field and Polymarket config has no
team fields.

- [ ] **Step 3: Implement shared target validation and provider injection**

Add these root types and helpers in `src/config.rs`:

```rust
#[derive(Clone, Deserialize)]
struct MatchSection {
    home_team: String,
    away_team: String,
}

impl MatchSection {
    fn validated(self) -> Result<Self> {
        let home_team = self.home_team.trim().to_string();
        let away_team = self.away_team.trim().to_string();
        if home_team.is_empty() || away_team.is_empty() {
            bail!("match.home_team and match.away_team must not be blank");
        }
        if normalized_team_name(&home_team) == normalized_team_name(&away_team) {
            bail!("match.home_team and match.away_team must identify different teams");
        }
        Ok(Self {
            home_team,
            away_team,
        })
    }
}

fn normalized_team_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}
```

Add the YAML-renamed field to `FileConfig`, validate it before provider conversion, and pass
cloned names into both provider inputs:

```rust
#[serde(rename = "match")]
match_target: MatchSection,
```

Change `RuntimeInput` and `polymarket::Config`:

```rust
pub struct RuntimeInput {
    pub proxy_url: String,
    pub gamma_host: String,
    pub clob_host: String,
    pub chain_id: u64,
    pub accounts: Vec<AccountConfig>,
    pub trade: TradeConfig,
    pub home_team: String,
    pub away_team: String,
    pub log_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub home_team: String,
    pub away_team: String,
    pub proxy_url: String,
    pub clob_api_url: String,
    pub gamma_api_url: String,
    pub gamma_event_base: String,
    pub market_ws_url: String,
    pub sports_ws_url: String,
    pub log_path: PathBuf,
}
```

Use `wss://sports-api.polymarket.com/ws` as `DEFAULT_SPORTS_WS_URL`. Remove
`polymarket_url` from runtime construction. Change `oddsportal::config::FileConfig::into_runtime`
to accept `home_team` and `away_team` arguments and remove its deserialized team fields:

```rust
pub fn into_runtime(
    self,
    proxy_url: Option<String>,
    home_team: String,
    away_team: String,
) -> Result<(bool, Config, Duration)> {
    if self.enabled && self.poll_interval_seconds == 0 {
        bail!("oddsportal.poll_interval_seconds must be greater than zero");
    }
    let defaults = Config::default();
    Ok((
        self.enabled,
        Config {
            tournament_url: self.tournament_url,
            home_team,
            away_team,
            proxy_url,
            log_path: self.log_path,
            ..defaults
        },
        Duration::from_secs(self.poll_interval_seconds),
    ))
}
```

Update `config.yaml` to the exact target:

```yaml
match:
  home_team: South Africa
  away_team: Canada
polymarket:
  enabled: true
  log_path: logs/polymarket_quotes.log
oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  log_path: logs/oddsportal_odds.log
  poll_interval_seconds: 1
```

- [ ] **Step 4: Run configuration tests and verify GREEN**

Run:

```bash
cargo test config::tests
cargo test polymarket::config::tests
cargo test oddsportal::config::tests
```

Expected: all configuration tests pass and no test expects a configured Polymarket URL.

- [ ] **Step 5: Check off OpenSpec tasks 1.1 and 1.2 and commit**

Update `openspec/changes/discover-match-by-team-names/tasks.md`, then run:

```bash
git add config.yaml src/config.rs src/polymarket/config.rs src/oddsportal/config.rs \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "feat: configure one shared match target"
```

---

### Task 2: Polymarket Name-Based Event Discovery

**Files:**
- Modify: `src/polymarket/models.rs`
- Modify: `src/polymarket/discovery.rs`

**Interfaces:**
- Produces: `MatchResult::{Home, Draw, Away}`.
- Produces: `TokenMeta.result: Option<MatchResult>` for classified Yes tokens.
- Preserves: `parse_event_response(body: &str) -> Result<DiscoveredEvent>`.
- Produces: `discover_event(config: &Config) -> Result<DiscoveredEvent>` using Gamma pagination.

- [ ] **Step 1: Add failing discovery fixture tests**

Add `MatchResult` fixtures and tests to `src/polymarket/discovery.rs`:

```rust
#[test]
fn finds_reversed_team_title_on_later_page_and_classifies_yes_tokens() {
    let page = r#"[{
      "slug":"fifwc-rsa-can-2026-06-28",
      "title":"Canada vs. South Africa",
      "active":true,
      "closed":false,
      "markets":[
        {"slug":"rsa","question":"Will South Africa win on 2026-06-28?",
         "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"11\",\"12\"]"},
        {"slug":"draw","question":"Will South Africa vs. Canada end in a draw?",
         "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"21\",\"22\"]"},
        {"slug":"can","question":"Will Canada win on 2026-06-28?",
         "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"31\",\"32\"]"}
      ]
    }]"#;

    let matches = parse_event_page(page, "South Africa", "Canada").unwrap();
    assert_eq!(matches.len(), 1);
    let yes_results: Vec<_> = matches[0].tokens.iter()
        .filter(|token| token.outcome == "Yes")
        .filter_map(|token| token.result)
        .collect();
    assert_eq!(yes_results,
               vec![MatchResult::Home, MatchResult::Draw, MatchResult::Away]);
}

#[test]
fn rejects_incomplete_and_ambiguous_1x2_candidates() {
    let incomplete = event_fixture("a", "South Africa vs Canada", false);
    assert!(parse_event_page(&incomplete, "South Africa", "Canada")
        .unwrap()
        .is_empty());

    let ambiguous = format!("[{},{}]",
        event_object_fixture("a", "South Africa vs Canada"),
        event_object_fixture("b", "Canada vs. South Africa"));
    let error = select_unique_event(
        parse_event_page(&ambiguous, "South Africa", "Canada").unwrap(),
        "South Africa",
        "Canada",
    ).unwrap_err();
    assert!(error.to_string().contains("ambiguous"));
    assert!(error.to_string().contains("a"));
    assert!(error.to_string().contains("b"));
}
```

Fixture helper strings must contain complete home/draw/away market objects, not network calls.

- [ ] **Step 2: Run the focused discovery tests and verify RED**

Run:

```bash
cargo test polymarket::discovery::tests::finds_reversed_team_title_on_later_page_and_classifies_yes_tokens -- --exact
```

Expected: compilation fails because `MatchResult`, `parse_event_page`, and token classification do
not exist.

- [ ] **Step 3: Add result classification models**

In `src/polymarket/models.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchResult {
    Home,
    Draw,
    Away,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenMeta {
    pub market_slug: String,
    pub question: String,
    pub outcome: String,
    pub asset_id: String,
    pub result: Option<MatchResult>,
}
```

Update every existing `TokenMeta` test fixture with `result: None`.

- [ ] **Step 4: Implement normalized page parsing and unique selection**

In `src/polymarket/discovery.rs`, deserialize a Gamma list and classify complete candidates:

```rust
pub fn parse_event_page(
    body: &str,
    home_team: &str,
    away_team: &str,
) -> Result<Vec<DiscoveredEvent>> {
    let events: Vec<GammaEvent> =
        serde_json::from_str(body).context("failed to parse Gamma event page")?;
    events.into_iter()
        .filter_map(|event| classify_event(event, home_team, away_team).transpose())
        .collect()
}

fn normalize_words(value: &str) -> String {
    value.to_lowercase()
        .replace("vs.", "vs")
        .replace(['-', '–', '—'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn select_unique_event(
    matches: Vec<DiscoveredEvent>,
    home_team: &str,
    away_team: &str,
) -> Result<DiscoveredEvent> {
    match matches.as_slice() {
        [event] => Ok(event.clone()),
        [] => bail!("no active Polymarket football 1X2 event found for {home_team} vs {away_team}"),
        _ => bail!(
            "ambiguous Polymarket events for {home_team} vs {away_team}: {}",
            matches.iter().map(|event| event.slug.as_str()).collect::<Vec<_>>().join(", ")
        ),
    }
}
```

`classify_event` must require both names in the normalized title and find exactly one question for
each result. It assigns `Some(result)` only to that market's case-insensitive `Yes` token; all
other tokens use `None`.

- [ ] **Step 5: Implement paginated Gamma HTTP discovery**

Build one proxied client and iterate `limit=100`, `offset=0,100,...`. Accumulate candidates and stop
when a page has fewer than 100 events:

```rust
pub async fn discover_event(config: &Config) -> Result<DiscoveredEvent> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&config.proxy_url)?)
        .build()?;
    let mut offset = 0_u32;
    let mut matches = Vec::new();
    loop {
        let response = client.get(format!("{}/events", config.gamma_api_url.trim_end_matches('/')))
            .query(&[
                ("active", "true"),
                ("closed", "false"),
                ("limit", "100"),
                ("offset", &offset.to_string()),
            ])
            .send().await?.error_for_status()?.text().await?;
        let page_count = serde_json::from_str::<Vec<serde_json::Value>>(&response)?.len();
        matches.extend(parse_event_page(
            &response,
            &config.home_team,
            &config.away_team,
        )?);
        if page_count < 100 {
            break;
        }
        offset += 100;
    }
    select_unique_event(matches, &config.home_team, &config.away_team)
}
```

Avoid borrowing a temporary `offset.to_string()` in the final implementation: bind it before the
`.query` call.

- [ ] **Step 6: Run discovery tests and verify GREEN**

Run:

```bash
cargo test polymarket::discovery::tests
cargo test polymarket::quotes::tests
cargo test polymarket::ws::tests
```

Expected: all tests pass after existing token fixtures gain `result: None`.

- [ ] **Step 7: Check off OpenSpec tasks 2.1 and 2.2 and commit**

```bash
git add src/polymarket/models.rs src/polymarket/discovery.rs \
  src/polymarket/quotes.rs src/polymarket/ws.rs \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "feat: discover Polymarket event from team names"
```

---

### Task 3: Structured Polymarket Odds Output

**Files:**
- Create: `src/polymarket/output.rs`
- Modify: `src/polymarket/mod.rs`
- Modify: `src/polymarket/ws.rs`
- Modify: `src/polymarket/discovery.rs`

**Interfaces:**
- Produces: `PolymarketOddsObservation::from_quote(...) -> Option<Self>`.
- Produces: `write_observation<T: Serialize>(observation: &T) -> Result<()>`.
- Consumes: `QuoteRecord`, classified `TokenMeta`, configured team names.

- [ ] **Step 1: Add failing observation serialization tests**

Create `src/polymarket/output.rs` with tests first:

```rust
#[test]
fn serializes_only_classified_yes_quote() {
    let token = TokenMeta {
        market_slug: "rsa".into(),
        question: "Will South Africa win?".into(),
        outcome: "Yes".into(),
        asset_id: "11".into(),
        result: Some(MatchResult::Home),
    };
    let quote = QuoteRecord {
        ts: "2026-06-28T12:00:00Z".into(),
        event_slug: "fifwc-rsa-can-2026-06-28".into(),
        market_slug: "rsa".into(),
        question: token.question.clone(),
        outcome: "Yes".into(),
        asset_id: "11".into(),
        bid_price: Some("0.16".into()),
        bid_size: Some("100".into()),
        ask_price: Some("0.17".into()),
        ask_size: Some("80".into()),
        source: "book".into(),
    };
    let observation = PolymarketOddsObservation::from_quote(
        &quote, &token, "South Africa", "Canada",
    ).unwrap();
    let json = serde_json::to_value(observation).unwrap();
    assert_eq!(json["provider"], "polymarket");
    assert_eq!(json["type"], "polymarket_odds");
    assert_eq!(json["result"], "home");
}

#[test]
fn ignores_no_token_or_unclassified_market() {
    let mut token = token_fixture();
    token.outcome = "No".into();
    assert!(PolymarketOddsObservation::from_quote(
        &quote_fixture(), &token, "South Africa", "Canada"
    ).is_none());
}
```

- [ ] **Step 2: Run the output test and verify RED**

Run:

```bash
cargo test polymarket::output::tests -- --nocapture
```

Expected: compilation fails because the output module and observation type do not exist.

- [ ] **Step 3: Implement the pure observation model**

Use owned strings so serialization does not hold provider-state borrows:

```rust
#[derive(Debug, Serialize)]
pub struct PolymarketOddsObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_slug: String,
    pub home_team: String,
    pub away_team: String,
    pub result: MatchResult,
    pub market_slug: String,
    pub asset_id: String,
    pub bid_price: Option<String>,
    pub bid_size: Option<String>,
    pub ask_price: Option<String>,
    pub ask_size: Option<String>,
    pub source: String,
}

impl PolymarketOddsObservation {
    pub fn from_quote(
        quote: &QuoteRecord,
        token: &TokenMeta,
        home_team: &str,
        away_team: &str,
    ) -> Option<Self> {
        let result = token.result?;
        (token.outcome.eq_ignore_ascii_case("yes") && token.asset_id == quote.asset_id).then(|| Self {
            provider: "polymarket",
            record_type: "polymarket_odds",
            received_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            source_updated_at: None,
            event_slug: quote.event_slug.clone(),
            home_team: home_team.to_string(),
            away_team: away_team.to_string(),
            result,
            market_slug: quote.market_slug.clone(),
            asset_id: quote.asset_id.clone(),
            bid_price: quote.bid_price.clone(),
            bid_size: quote.bid_size.clone(),
            ask_price: quote.ask_price.clone(),
            ask_size: quote.ask_size.clone(),
            source: quote.source.clone(),
        })
    }
}

pub fn write_observation<T: Serialize>(observation: &T) -> Result<()> {
    let line = serde_json::to_string(observation)?;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{line}")?;
    Ok(())
}
```

- [ ] **Step 4: Wire initial and WebSocket quotes to JSON stdout**

Build an asset-to-token lookup once in `run_market_stream`. For every initial or changed
`QuoteRecord`, preserve `logger.append(&record)` and call the pure converter:

```rust
if let Some(token) = tokens_by_asset.get(&record.asset_id) {
    if let Some(observation) = PolymarketOddsObservation::from_quote(
        &record,
        token,
        &config.home_team,
        &config.away_team,
    ) {
        output::write_observation(&observation)?;
    }
}
```

Change all CLOB discovery/start/subscription/reconnect messages from `println!` to `eprintln!`.

- [ ] **Step 5: Run output and WebSocket tests and verify GREEN**

```bash
cargo test polymarket::output::tests
cargo test polymarket::ws::tests
```

Expected: classified Yes quotes serialize, No/unclassified tokens do not, and existing quote
parsing tests pass.

- [ ] **Step 6: Check off OpenSpec tasks 3.1 and 3.2 and commit**

```bash
git add src/polymarket/output.rs src/polymarket/mod.rs \
  src/polymarket/ws.rs src/polymarket/discovery.rs \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "feat: emit Polymarket Yes-token odds JSON"
```

---

### Task 4: Polymarket Sports Score WebSocket

**Files:**
- Create: `src/polymarket/sports.rs`
- Modify: `src/polymarket/output.rs`
- Modify: `src/polymarket/ws.rs`
- Modify: `src/polymarket/mod.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `parse_sports_message(text, event, config) -> Result<SportsAction>`.
- Produces: `SportsAction::{Pong, Observation(PolymarketScoreObservation), Ignore}`.
- Produces: `polymarket::run(config, live) -> Result<()>` that discovers once and runs both streams.
- Consumes: reusable `ws::connect_ws_via_proxy`.

- [ ] **Step 1: Add failing sports protocol tests**

In `src/polymarket/sports.rs`:

```rust
#[test]
fn returns_pong_for_heartbeat_and_ignores_other_slugs() {
    assert!(matches!(
        parse_sports_message("ping", &event_fixture(), &config_fixture()).unwrap(),
        SportsAction::Pong
    ));
    let other = r#"{"slug":"fifwc-other","score":"0-0"}"#;
    assert!(matches!(
        parse_sports_message(other, &event_fixture(), &config_fixture()).unwrap(),
        SportsAction::Ignore
    ));
}

#[test]
fn serializes_matching_score_with_source_and_receipt_times() {
    let text = r#"{
      "slug":"fifwc-rsa-can-2026-06-28",
      "score":"1-0",
      "status":"InProgress",
      "period":"1H",
      "elapsed":"32:15",
      "live":true,
      "ended":false,
      "last_update":"2026-06-28T12:00:01.050Z"
    }"#;
    let SportsAction::Observation(record) =
        parse_sports_message(text, &event_fixture(), &config_fixture()).unwrap()
    else { panic!("expected observation") };
    assert_eq!(record.score.as_deref(), Some("1-0"));
    assert_eq!(record.source_updated_at.as_deref(), Some("2026-06-28T12:00:01.050Z"));
}
```

- [ ] **Step 2: Run sports tests and verify RED**

```bash
cargo test polymarket::sports::tests -- --nocapture
```

Expected: compilation fails because `sports.rs`, `SportsAction`, and score observation do not
exist.

- [ ] **Step 3: Implement score message parsing**

Add this output type in `output.rs`:

```rust
#[derive(Debug, Serialize)]
pub struct PolymarketScoreObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_slug: String,
    pub home_team: String,
    pub away_team: String,
    pub score: Option<String>,
    pub status: Option<String>,
    pub period: Option<String>,
    pub elapsed: Option<String>,
    pub live: Option<bool>,
    pub ended: Option<bool>,
}
```

Implement a private deserialization struct and pure action parser:

```rust
#[derive(Deserialize)]
struct SportsMessage {
    slug: String,
    score: Option<String>,
    status: Option<String>,
    period: Option<String>,
    elapsed: Option<String>,
    live: Option<bool>,
    ended: Option<bool>,
    #[serde(default, alias = "last_update")]
    source_updated_at: Option<String>,
}

pub enum SportsAction {
    Pong,
    Observation(PolymarketScoreObservation),
    Ignore,
}

pub fn parse_sports_message(
    text: &str,
    event: &DiscoveredEvent,
    config: &Config,
) -> Result<SportsAction> {
    if text == "ping" {
        return Ok(SportsAction::Pong);
    }
    let message: SportsMessage = serde_json::from_str(text)?;
    if message.slug != event.slug {
        return Ok(SportsAction::Ignore);
    }
    Ok(SportsAction::Observation(PolymarketScoreObservation {
        provider: "polymarket",
        record_type: "polymarket_score",
        received_at: now_rfc3339(),
        source_updated_at: message.source_updated_at,
        event_slug: event.slug.clone(),
        home_team: config.home_team.clone(),
        away_team: config.away_team.clone(),
        score: message.score,
        status: message.status,
        period: message.period,
        elapsed: message.elapsed,
        live: message.live,
        ended: message.ended,
    }))
}
```

- [ ] **Step 4: Implement the reconnecting Sports WebSocket loop**

Expose the existing proxy connector as `pub(crate)` and use it for `sports_ws_url`. The loop must
handle individual connection failures without returning:

```rust
pub async fn run_score_stream(config: Config, event: DiscoveredEvent) -> Result<()> {
    loop {
        eprintln!("{LOG_PREFIX} connecting sports websocket");
        match connect_ws_via_proxy(&config.sports_ws_url, &config.proxy_url).await {
            Ok((ws, _)) => {
                let (mut write, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(Message::Text(text)) => match parse_sports_message(&text, &event, &config) {
                            Ok(SportsAction::Pong) => {
                                write.send(Message::Text("pong".into())).await?;
                            }
                            Ok(SportsAction::Observation(record)) => {
                                write_observation(&record)?;
                            }
                            Ok(SportsAction::Ignore) => {}
                            Err(error) => eprintln!("{LOG_PREFIX} invalid sports update: {error:#}"),
                        },
                        Ok(Message::Close(_)) => break,
                        Ok(_) => {}
                        Err(error) => {
                            eprintln!("{LOG_PREFIX} sports websocket read failed: {error:#}");
                            break;
                        }
                    }
                }
            }
            Err(error) => eprintln!("{LOG_PREFIX} sports websocket failed: {error:#}"),
        }
        sleep(Duration::from_secs(3)).await;
    }
}
```

Do not use `?` for per-connection send/read failures in the final loop; log them, break the inner
loop, and reconnect so the other Polymarket stream remains alive.

- [ ] **Step 5: Add a provider-local dual-stream entry point**

In `src/polymarket/mod.rs`:

```rust
pub async fn run(config: config::Config, live: Option<config::LiveConfig>) -> anyhow::Result<()> {
    let event = discovery::discover_event(&config).await?;
    let clob = ws::run_market_stream(config.clone(), live, event.clone());
    let scores = sports::run_score_stream(config, event);
    tokio::try_join!(clob, scores)?;
    Ok(())
}
```

Change `main.rs` to call `polymarket::run(runtime.config, runtime.live)` and remove provider parsing
from root.

- [ ] **Step 6: Run sports and orchestration tests and verify GREEN**

```bash
cargo test polymarket::sports::tests
cargo test polymarket::ws::tests
cargo test tests::supervisor_waits_for_remaining_provider_after_one_fails -- --exact
```

Expected: protocol tests pass, and root provider failure isolation remains unchanged.

- [ ] **Step 7: Check off OpenSpec tasks 3.3 and 3.4 and commit**

```bash
git add src/polymarket/sports.rs src/polymarket/output.rs \
  src/polymarket/ws.rs src/polymarket/mod.rs src/main.rs \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "feat: stream Polymarket score observations"
```

---

### Task 5: OddsPortal Score Discovery and Models

**Files:**
- Modify: `src/oddsportal/models.rs`
- Modify: `src/oddsportal/discovery.rs`
- Create: `src/oddsportal/score.rs`
- Create: `src/oddsportal/output.rs`
- Modify: `src/oddsportal/mod.rs`

**Interfaces:**
- Extends: `RequestMetadata.score_url: Option<String>`.
- Produces: `parse_score_payload(...) -> Result<OddsPortalScoreObservation>`.
- Produces: `unavailable_score(...) -> OddsPortalScoreObservation`.
- Produces: `OddsPortalOddsObservation::from_records(...) -> Result<Self>`.

- [ ] **Step 1: Add failing H2H score URL tests**

Extend the existing H2H fixture test:

```rust
#[test]
fn extracts_odds_and_score_request_urls_independently() {
    let html = r#"<Event :data="{&quot;requestPreMatch&quot;:{
      &quot;url&quot;:&quot;\/match-event\/1-1-EZmXxG15-1-2-yj93f.dat?_=&quot;},
      &quot;updateScoreRequest&quot;:{
      &quot;url&quot;:&quot;\/feed\/postmatch-score\/1-EZmXxG15-yj93f.dat?_=&quot;}}">
      </Event>"#;
    let metadata = parse_h2h_request_metadata(html).unwrap();
    assert_eq!(metadata.score_url.as_deref(), Some(
        "https://www.oddsportal.com/feed/postmatch-score/1-EZmXxG15-yj93f.dat?_="
    ));
}

#[test]
fn missing_score_url_does_not_hide_odds_url() {
    let metadata = parse_h2h_request_metadata(pre_match_only_fixture()).unwrap();
    assert!(metadata.pre_match_url.contains("/match-event/"));
    assert_eq!(metadata.score_url, None);
}
```

- [ ] **Step 2: Run discovery test and verify RED**

```bash
cargo test oddsportal::discovery::tests::extracts_odds_and_score_request_urls_independently -- --exact
```

Expected: compilation fails because `RequestMetadata` has no `score_url`.

- [ ] **Step 3: Implement independent score URL extraction**

Add the model field:

```rust
pub struct RequestMetadata {
    pub pre_match_url: String,
    pub fallback_pre_match_url: Option<String>,
    pub score_url: Option<String>,
}
```

In `parse_h2h_request_metadata`, find the score marker independently:

```rust
let score_url = find_json_string(
    &decoded,
    r#""updateScoreRequest":{"url":""#,
)
.map(|raw| raw.replace("\\/", "/"))
.map(|raw| absolute_url(DEFAULT_BASE_URL, &raw))
.transpose()?;
```

Update every existing `RequestMetadata` test expectation with the optional score URL.

- [ ] **Step 4: Add failing score and grouped-odds model tests**

Create pure tests in `score.rs` and `output.rs`:

```rust
#[test]
fn parses_available_score_fields() {
    let decoded = serde_json::json!({
        "d": {
            "score": "1-0",
            "status": "live",
            "period": "1H",
            "elapsed": "32:00",
            "lastUpdate": "2026-06-28T12:00:01Z"
        }
    });
    let record = parse_score_payload(
        &decoded, &match_fixture(), "South Africa", "Canada"
    ).unwrap();
    assert!(record.available);
    assert_eq!(record.score.as_deref(), Some("1-0"));
}

#[test]
fn groups_all_bookmakers_into_one_odds_record() {
    let records = vec![
        odds_record("16", "bet365", "1", "5.50"),
        odds_record("16", "bet365", "X", "3.80"),
        odds_record("16", "bet365", "2", "1.62"),
        odds_record("18", "Pinnacle", "1", "5.60"),
    ];
    let output = OddsPortalOddsObservation::from_records(
        &records, "South Africa", "Canada"
    ).unwrap();
    assert_eq!(output.bookmakers.len(), 2);
    assert_eq!(output.bookmakers[0].outcomes["X"], "3.80");
}
```

- [ ] **Step 5: Implement provider-local output models**

In `output.rs`, define:

```rust
#[derive(Debug, Serialize)]
pub struct BookmakerOdds {
    pub bookmaker_id: String,
    pub bookmaker_name: String,
    pub outcomes: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct OddsPortalOddsObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_id: String,
    pub event_name: String,
    pub home_team: String,
    pub away_team: String,
    pub bookmakers: Vec<BookmakerOdds>,
}

#[derive(Debug, Serialize)]
pub struct OddsPortalScoreObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_id: String,
    pub event_name: String,
    pub home_team: String,
    pub away_team: String,
    pub available: bool,
    pub score: Option<String>,
    pub status: Option<String>,
    pub period: Option<String>,
    pub elapsed: Option<String>,
}
```

Group normalized records with a `BTreeMap<(String, String), BTreeMap<String, String>>` so output is
deterministic. Reject an empty input batch with a contextual error.

Add a provider-local writer (do not import Polymarket output code across the provider boundary):

```rust
pub fn write_observation<T: Serialize>(observation: &T) -> Result<()> {
    let line = serde_json::to_string(observation)?;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{line}")?;
    Ok(())
}
```

- [ ] **Step 6: Implement score normalization**

In `score.rs`, decode the `d` object and accept string or numeric fields:

```rust
pub fn parse_score_payload(
    decoded: &Value,
    event: &DiscoveredMatch,
    home_team: &str,
    away_team: &str,
) -> Result<OddsPortalScoreObservation> {
    let data = decoded.get("d").unwrap_or(decoded);
    Ok(OddsPortalScoreObservation {
        provider: "oddsportal",
        record_type: "oddsportal_score",
        received_at: now_rfc3339(),
        source_updated_at: field(data, &["lastUpdate", "updatedAt", "ts"]),
        event_id: event.encoded_event_id.clone(),
        event_name: event.event_name.clone(),
        home_team: home_team.to_string(),
        away_team: away_team.to_string(),
        available: true,
        score: field(data, &["score", "result"]),
        status: field(data, &["status", "state"]),
        period: field(data, &["period", "stage"]),
        elapsed: field(data, &["elapsed", "time"]),
    })
}
```

`field` converts `String` and `Number` values. Add an `unavailable_score` constructor setting
`available: false` and all score-state fields to `None`.

- [ ] **Step 7: Run pure OddsPortal tests and verify GREEN**

```bash
cargo test oddsportal::discovery::tests
cargo test oddsportal::score::tests
cargo test oddsportal::output::tests
```

Expected: both URLs parse, absent score metadata is allowed, score fixtures normalize, and
bookmakers group deterministically.

- [ ] **Step 8: Check off OpenSpec tasks 4.1 and 4.2 and commit**

```bash
git add src/oddsportal/models.rs src/oddsportal/discovery.rs \
  src/oddsportal/score.rs src/oddsportal/output.rs src/oddsportal/mod.rs \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "feat: model OddsPortal score observations"
```

---

### Task 6: OddsPortal Concurrent Dual Polling

**Files:**
- Modify: `src/oddsportal/mod.rs`
- Modify: `src/oddsportal/output.rs`
- Modify: `src/oddsportal/score.rs`

**Interfaces:**
- Produces: one-time `discover_requests(&Config) -> Result<(DiscoveredMatch, RequestMetadata)>`.
- Produces: `collect_odds(...)` and `collect_score(...)` independent futures.
- Preserves: odds normalization and append-only `OddsPortalLogger`.

- [ ] **Step 1: Add failing paused-time polling tests**

Refactor the test seam to inject separate odds and score futures, then add:

```rust
#[tokio::test(start_paused = true)]
async fn cycle_starts_odds_and_score_together_without_overlap() {
    let odds_calls = Arc::new(AtomicUsize::new(0));
    let score_calls = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    let task = tokio::spawn(run_poll_loop_with(
        config_fixture(),
        Duration::from_secs(1),
        Some(2),
        test_discovery(),
        counting_odds(odds_calls.clone(), active.clone(), max_active.clone()),
        counting_score(score_calls.clone(), active.clone(), max_active.clone()),
    ));
    tokio::task::yield_now().await;
    assert_eq!(odds_calls.load(Ordering::SeqCst), 1);
    assert_eq!(score_calls.load(Ordering::SeqCst), 1);
    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    task.await.unwrap().unwrap();
    assert_eq!(max_active.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn emits_successful_side_when_peer_request_fails() {
    let result = run_one_cycle_with(
        test_discovery(),
        async { Err(anyhow!("odds failed")) },
        async { Ok(score_fixture()) },
    ).await.unwrap();
    assert!(result.odds.is_none());
    assert!(result.score.is_some());
}
```

The concurrency assertion is two in-flight requests inside one cycle, never four from overlapping
cycles.

- [ ] **Step 2: Run polling tests and verify RED**

```bash
cargo test oddsportal::tests::cycle_starts_odds_and_score_together_without_overlap -- --exact
```

Expected: compilation fails because the current loop accepts only one collector future.

- [ ] **Step 3: Separate one-time discovery from recurring requests**

Build one configured HTTP client, fetch tournament and H2H pages once, and return:

```rust
async fn discover_requests(
    client: &reqwest::Client,
    config: &Config,
) -> Result<(DiscoveredMatch, RequestMetadata)> {
    let target = config.target_match();
    let tournament_html =
        get_text_with_retries(client, &config.tournament_url, "OddsPortal tournament").await?;
    let event = discovery::parse_tournament_match(
        &tournament_html,
        &target.home_team,
        &target.away_team,
    )?;
    let h2h_html =
        get_text_with_retries(client, &http_request_url(&event.h2h_url), "OddsPortal H2H").await?;
    let requests = discovery::parse_h2h_request_metadata(&h2h_html)?;
    Ok((event, requests))
}
```

Recurring odds collection receives the already discovered event/request and no longer downloads
the tournament and H2H pages every second.

- [ ] **Step 4: Implement independent one-attempt odds and score requests**

Add:

```rust
async fn collect_score(
    client: &reqwest::Client,
    score_url: Option<&str>,
    event: &DiscoveredMatch,
    config: &Config,
) -> Result<OddsPortalScoreObservation> {
    let Some(url) = score_url else {
        return Ok(unavailable_score(event, &config.home_team, &config.away_team));
    };
    let response = client.get(cache_busted_url(url)).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(unavailable_score(event, &config.home_team, &config.away_team));
    }
    let body = response.error_for_status()?.text().await?;
    let decoded = decoder::decode_dat_payload(&body)?;
    score::parse_score_payload(&decoded, event, &config.home_team, &config.away_team)
}
```

Recurring odds uses the same one-attempt policy while retaining frontend-xhash fallback as a
second URL only within the same odds future.

- [ ] **Step 5: Implement non-overlapping interval cycles**

Use an immediate Tokio interval and skip missed ticks:

```rust
let mut ticker = tokio::time::interval(interval);
ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
loop {
    ticker.tick().await;
    let odds_future = collect_odds(&client, &requests, &event);
    let score_future =
        collect_score(&client, requests.score_url.as_deref(), &event, &config);
    let (odds_result, score_result) = tokio::join!(odds_future, score_future);

    match odds_result {
        Ok(records) => {
            append_odds_records(&config.log_path, &records)?;
            let output = OddsPortalOddsObservation::from_records(
                &records, &config.home_team, &config.away_team,
            )?;
            write_observation(&output)?;
        }
        Err(error) => eprintln!("{LOG_PREFIX} odds collection failed: {error:#}"),
    }
    match score_result {
        Ok(record) => write_observation(&record)?,
        Err(error) => eprintln!("{LOG_PREFIX} score collection failed: {error:#}"),
    }
}
```

Construct the two futures before awaiting either. Do not spawn detached poll tasks.

- [ ] **Step 6: Run OddsPortal polling and regression tests**

```bash
cargo test oddsportal::tests
cargo test oddsportal::odds::tests
cargo test oddsportal::decoder::tests
cargo test oddsportal::logging::tests
```

Expected: dual polling tests and all existing decode/normalize/logging tests pass.

- [ ] **Step 7: Check off OpenSpec tasks 4.3 and 4.4 and commit**

```bash
git add src/oddsportal/mod.rs src/oddsportal/output.rs src/oddsportal/score.rs \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "feat: poll OddsPortal odds and score each second"
```

---

### Task 7: Data-Only stdout and Provider Diagnostics

**Files:**
- Modify: `src/main.rs`
- Modify: `src/polymarket/discovery.rs`
- Modify: `src/polymarket/ws.rs`
- Modify: `src/polymarket/sports.rs`
- Modify: `src/oddsportal/mod.rs`

**Interfaces:**
- stdout: complete JSON observation lines only.
- stderr: stable `[polymarket]`, `[oddsportal]`, and `[trade]` diagnostic prefixes.

- [ ] **Step 1: Add failing subprocess stream-separation tests**

Use the existing helper-test subprocess pattern in `src/main.rs`:

```rust
#[test]
fn observation_helper_keeps_stdout_json_and_diagnostics_on_stderr() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tests::observation_output_helper", "--nocapture"])
        .env("OBSERVATION_OUTPUT_HELPER", "1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    for line in stdout.lines().filter(|line| line.starts_with('{')) {
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("[polymarket]"));
    assert!(!stdout.contains("[polymarket]"));
}

#[test]
fn observation_output_helper() {
    if std::env::var_os("OBSERVATION_OUTPUT_HELPER").is_none() {
        return;
    }
    eprintln!("[polymarket] helper diagnostic");
    polymarket::output::write_observation(&polymarket_odds_fixture()).unwrap();
}
```

- [ ] **Step 2: Run stream test and verify RED**

```bash
cargo test tests::observation_helper_keeps_stdout_json_and_diagnostics_on_stderr -- --exact
```

Expected: fails while any provider diagnostic still uses stdout or the test helper is incomplete.

- [ ] **Step 3: Route every provider diagnostic to stderr**

Audit with:

```bash
rg -n 'println!|eprintln!' src/main.rs src/polymarket src/oddsportal
```

Keep `println!` out of provider code except inside the single JSON writer if it uses `println!`.
Convert startup, discovery, pass status, subscription, reconnect, and error lines to `eprintln!`.
Do not change trade messages to JSON; keep their `[trade]` diagnostics on stderr.

- [ ] **Step 4: Run stream and supervisor tests and verify GREEN**

```bash
cargo test tests::observation_helper_keeps_stdout_json_and_diagnostics_on_stderr -- --exact
cargo test tests::provider_log_prefixes_are_stable -- --exact
cargo test tests::supervisor_waits_for_remaining_provider_after_one_fails -- --exact
cargo test tests::supervisor_attributes_panics_to_the_provider -- --exact
```

Expected: JSON parsing succeeds, diagnostics appear only on stderr, and supervision remains
failure-isolated.

- [ ] **Step 5: Check off OpenSpec task 5.1 and commit**

```bash
git add src/main.rs src/polymarket src/oddsportal \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "refactor: separate observation data from diagnostics"
```

---

### Task 8: Documentation and Full Verification

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `README.md`
- Review: `DEPLOYMENT.md`
- Modify: `openspec/changes/discover-match-by-team-names/tasks.md`
- Create: `docs/superpowers/reports/2026-06-28-discover-match-by-team-names-verify.md`

**Interfaces:**
- Documents: shared match configuration, four stdout schemas, two OddsPortal requests per second,
  Polymarket dual WebSockets, stderr diagnostics, and rate-limit caveat.

- [ ] **Step 1: Update architecture and user documentation**

Update the source tree in `ARCHITECTURE.md` with:

```text
src/polymarket/output.rs   # Polymarket stdout observation models
src/polymarket/sports.rs   # Public sports-score WebSocket
src/oddsportal/output.rs   # Grouped odds and score stdout models
src/oddsportal/score.rs    # Score payload normalization
```

Update its data flow to show the shared match pair feeding both providers, two Polymarket
WebSockets, and two concurrent OddsPortal HTTP resources. State explicitly that stdout is
observation JSONL and stderr is diagnostic text.

Expand `README.md` from its current title-only content with:

- build/run commands;
- the exact `match`, `polymarket`, and `oddsportal` configuration example;
- concise examples of all four observation types;
- the fact that OddsPortal performs two requests per second and advertises a slower source refresh;
- the fact that `trade.enabled` remains `false`.

Read `DEPLOYMENT.md` and change it only if its described stdout/stderr capture behavior is now
incorrect.

- [ ] **Step 2: Run formatting and focused static checks**

```bash
cargo fmt --all -- --check
git diff --check
rg -n 'polymarket\\.url|oddsportal:\\n.*home_team|cross-provider.*aggregat' \
  src config.yaml README.md ARCHITECTURE.md \
  openspec/changes/discover-match-by-team-names || true
```

Expected: formatting and whitespace checks pass; no stale configured Polymarket URL or
cross-provider aggregation requirement remains.

- [ ] **Step 3: Run the full Rust test suite**

```bash
cargo test
```

Expected: all unit, Tokio-time, subprocess, logger, decoder, configuration, and supervision tests
pass.

- [ ] **Step 4: Run strict OpenSpec validation**

```bash
openspec validate discover-match-by-team-names --strict
```

Expected: `Change 'discover-match-by-team-names' is valid`.

- [ ] **Step 5: Run a bounded read-only smoke test**

With a reachable non-secret proxy supplied through a temporary untracked configuration, run the
binary under a timeout:

```bash
timeout 20s cargo run
```

Expected:

- stderr shows both providers discovering South Africa–Canada;
- stdout lines parse with `jq -e .`;
- observed record types are among `polymarket_odds`, `polymarket_score`, `oddsportal_odds`, and
  `oddsportal_score`;
- no `[trade]` placement or cancellation message appears;
- no secret or credential value appears.

If the match is not live, `polymarket_score` may be absent and `oddsportal_score` may report
`available: false`; record this as expected rather than fabricating a score. If external access is
unavailable, record the network blocker and do not claim live smoke success.

- [ ] **Step 6: Write the verification report and finish the task checklist**

Record each command, exit status, and smoke-test observation in
`docs/superpowers/reports/2026-06-28-discover-match-by-team-names-verify.md`. Check off OpenSpec
tasks 5.2 and 5.3 only after the corresponding evidence exists.

- [ ] **Step 7: Commit documentation and verification evidence**

```bash
git add ARCHITECTURE.md README.md DEPLOYMENT.md \
  docs/superpowers/reports/2026-06-28-discover-match-by-team-names-verify.md \
  openspec/changes/discover-match-by-team-names/tasks.md
git commit -m "docs: verify team-name market observations"
```

Do not add runtime logs, temporary proxy configuration, credentials, or build artifacts.
