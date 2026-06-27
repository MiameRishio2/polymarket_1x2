---
change: rs-clob-client-v2
design-doc: docs/superpowers/specs/2026-06-25-rs-clob-client-v2-design.md
base-ref: 27351ce7661f5513dab5b4bd045abf6ff49b313b
archived-with: 2026-06-25-rs-clob-client-v2
---

# rs-clob-client-v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI that discovers the Ecuador vs. Germany Polymarket event, subscribes to all CLOB outcome tokens over WebSocket through `http://10.32.110.233:7890`, and appends latest bid/ask prices and quantities to a log file.

**Architecture:** A single Rust binary with focused modules for config, discovery, typed models, WebSocket subscription, quote-state updates, and JSON-lines logging. Unit tests cover deterministic behavior; a bounded smoke run verifies live proxy/WebSocket behavior.

**Tech Stack:** Rust 2021, `tokio`, `reqwest`, `tokio-tungstenite`, `async-http-proxy`, `serde`, `serde_json`, `url`, `chrono`, `anyhow`, and `rs-clob-client-v2`.

## Global Constraints

- Default Polymarket URL: `https://polymarket.com/ja/sports/world-cup/fifwc-ecu-ger-2026-06-25`.
- Default proxy URL: `http://10.32.110.233:7890`.
- Default Gamma endpoint: `https://gamma-api.polymarket.com/events/slug/`.
- Default WebSocket endpoint: `wss://ws-subscriptions-clob.polymarket.com/ws/market`.
- Default log path: `logs/polymarket_quotes.log`.
- The client remains read-only and unauthenticated.
- Prices and sizes are logged as strings to avoid precision loss.

archived-with: 2026-06-25-rs-clob-client-v2
---

## File Structure

- Create `Cargo.toml`: binary crate metadata and dependencies.
- Create `src/main.rs`: runtime orchestration and shutdown.
- Create `src/config.rs`: constants and runtime config.
- Create `src/discovery.rs`: URL slug extraction, Gamma event request, child market parsing.
- Create `src/models.rs`: API and runtime structs.
- Create `src/quotes.rs`: best-level calculation and quote state.
- Create `src/logging.rs`: log directory creation and JSON-lines append.
- Create `src/ws.rs`: proxied WebSocket connection, subscription payload, event parsing, reconnect loop.
- Create tests in each module with `#[cfg(test)]`.

### Task 1: Project Skeleton and Config

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/config.rs`

**Interfaces:**
- Produces: `config::Config::default() -> Config`.
- Produces: constants for default URL, proxy, endpoints, and log path.

- [ ] **Step 1: Write failing config test**

Add a unit test in `src/config.rs` that asserts the default proxy and URLs match the global constraints.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::tests::default_config_matches_requested_values`
Expected: FAIL because the crate or config module does not exist yet.

- [ ] **Step 3: Implement minimal skeleton**

Create the Rust crate, config module, and `main` placeholder that prints startup config.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test config::tests::default_config_matches_requested_values`
Expected: PASS.

### Task 2: Event Discovery

**Files:**
- Create/modify: `src/discovery.rs`
- Modify: `src/models.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `config::Config`.
- Produces: `extract_slug(input: &str) -> anyhow::Result<String>`.
- Produces: `discover_event(config: &Config) -> anyhow::Result<DiscoveredEvent>`.
- Produces: `DiscoveredEvent { slug, title, tokens }` where each token includes market slug, question, outcome, and asset ID.

- [ ] **Step 1: Write failing slug and parser tests**

Test localized URL slug extraction, JSON-string encoded `clobTokenIds`, and outcome pairing.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test discovery`
Expected: FAIL because discovery code is missing.

- [ ] **Step 3: Implement discovery**

Implement slug extraction, Gamma GET through `reqwest::Proxy`, and typed parsing for the event markets.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test discovery`
Expected: PASS.

### Task 3: Quote State

**Files:**
- Create/modify: `src/quotes.rs`
- Modify: `src/models.rs`

**Interfaces:**
- Produces: `QuoteState::apply_book(asset_id, bids, asks) -> Option<QuoteRecord>`.
- Produces: `QuoteState::apply_best_bid_ask(asset_id, bid, ask) -> Option<QuoteRecord>`.
- Produces: best bid as max bid price and best ask as min ask price using decimal-string comparison.

- [ ] **Step 1: Write failing quote-state tests**

Test best bid/ask selection from unordered levels and preserving previous sizes when price-only updates arrive.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test quotes`
Expected: FAIL because quote state is missing.

- [ ] **Step 3: Implement quote state**

Implement level parsing, best-level calculation, metadata lookup, and quote records with optional sizes.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test quotes`
Expected: PASS.

### Task 4: Logging

**Files:**
- Create/modify: `src/logging.rs`
- Modify: `src/models.rs`

**Interfaces:**
- Produces: `QuoteLogger::new(path) -> anyhow::Result<QuoteLogger>`.
- Produces: `QuoteLogger::append(&mut self, record: &QuoteRecord) -> anyhow::Result<()>`.

- [ ] **Step 1: Write failing log test**

Use a temporary directory to assert that missing parent directories are created and one JSON line is appended.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test logging`
Expected: FAIL because logging code is missing.

- [ ] **Step 3: Implement logging**

Create parent directories and append serialized quote records with newline delimiters.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test logging`
Expected: PASS.

### Task 5: WebSocket Runtime

**Files:**
- Create/modify: `src/ws.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `DiscoveredEvent`, `QuoteState`, and `QuoteLogger`.
- Produces: subscription payload `{"assets_ids":[...],"type":"market"}`.
- Produces: event parser for `book`, `price_change`, and `best_bid_ask`.

- [ ] **Step 1: Write failing WebSocket parser tests**

Test subscription payload shape and parsing representative `book` and `best_bid_ask` messages into quote records.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ws`
Expected: FAIL because WebSocket parsing is missing.

- [ ] **Step 3: Implement WebSocket runtime**

Implement proxy-aware connection setup, subscription send, event parsing, ping handling, reconnect backoff, and stdout mirroring.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test ws`
Expected: PASS.

### Task 6: End-to-End Verification

**Files:**
- Modify: `openspec/changes/rs-clob-client-v2/tasks.md`
- Modify: `.gitignore` if needed for `target/` and runtime logs.

**Interfaces:**
- Produces: working `cargo run` default path.

- [ ] **Step 1: Format**

Run: `cargo fmt`
Expected: no formatting diff after command.

- [ ] **Step 2: Type-check and test**

Run: `cargo check && cargo test`
Expected: both PASS.

- [ ] **Step 3: Live smoke test**

Run a bounded command such as `timeout 30s cargo run`.
Expected: process discovers the event, subscribes to tokens, and writes at least one JSON line to `logs/polymarket_quotes.log`.

- [ ] **Step 4: Update OpenSpec task checklist**

Mark corresponding tasks in `openspec/changes/rs-clob-client-v2/tasks.md` complete after verification passes.
