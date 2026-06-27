---
change: add-polymarket-live-trading
design-doc: docs/superpowers/specs/2026-06-27-polymarket-live-trading-design.md
base-ref: 670ca87f5a40ab2f4d495c844539fc1d3a151450
archived-with: 2026-06-27-add-polymarket-live-trading
---

# Polymarket Fixed-Flow Live Trading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Execute the existing fixed Polymarket buy/sell/cancel lifecycle once through an authenticated, proxy-routed CLOB client when three YAML modes explicitly enable live trading.

**Architecture:** Parse `config.yaml` into provider-local typed configuration, select the unique `type: long` account, and pass pre-existing L2 credentials plus its signer settings to `rs-clob-client-v2`. A mockable live adapter implements the existing `OrderExecutor`; startup invokes it once after initial quotes and before the WebSocket reconnect loop.

**Tech Stack:** Rust 2021, Tokio, Serde YAML, `rust_decimal`, Alloy local signer, `rs-clob-client-v2 0.2.2`

## Global Constraints

- Keep all authenticated trading code under `src/polymarket/`.
- Do not call `create_api_key`, `derive_api_key`, or `create_or_derive_api_key`.
- Never log or include private keys, API keys, API secrets, passphrases, signed payloads, or authentication headers in errors.
- Preserve the user's existing `config.yaml` values; make only the requested structural edits and never print its secret-bearing diff.
- Automated tests must not send live create-order or cancel-order requests.
- The live flow runs only when `trader_mode`, `account_mode`, and `market_mode` all equal `real`.
- Prices and sizes remain `Decimal` internally and are converted to `f64` only at the existing SDK boundary.

---

### Task 1: Typed configuration, redaction, and live gate

**Files:**
- Modify: `Cargo.toml`
- Modify: `config.yaml`
- Modify: `src/polymarket/config.rs`
- Test: `src/polymarket/config.rs`

**Interfaces:**
- Produces: `FileConfig::load`, `FileConfig::into_runtime`, `LiveConfig`, `SecretString`, and the exact three-mode gate.
- Consumes: Existing provider `Config` defaults for the event URL, WebSocket URL, and log path.

- [ ] **Step 1: Add failing YAML and redaction tests**

Add tests that parse an in-memory fixture containing one `long` account, assert all three `real` values enable `Some(LiveConfig)`, and assert `Debug` output contains `<redacted>` but none of the fixture secrets.

```rust
const LIVE_YAML: &str = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
accounts:
  - name: long-test
    type: long
    signature_type: null
    private_key: test-private
    api_key: test-key
    api_secret: test-secret
    api_passphrase: test-passphrase
    host: https://clob.polymarket.com
    chain_id: 137
    funder: null
trade:
  trader_mode: real
  account_mode: real
  market_mode: real
"#;

#[test]
fn all_real_modes_select_unique_long_account_without_exposing_secrets() {
    let file: FileConfig = serde_yaml::from_str(LIVE_YAML).unwrap();
    let (_, live) = file.into_runtime().unwrap();
    let live = live.unwrap();
    let debug = format!("{live:?}");

    assert_eq!(live.account_name, "long-test");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("test-private"));
    assert!(!debug.contains("test-secret"));
}
```

Add table-driven cases where each mode is changed to `mock`; every case must return `None` without requiring an account. Add missing and duplicate `long` account cases that fail only when all three modes are `real`.

- [ ] **Step 2: Run focused tests and confirm RED**

Run:

```bash
cargo test polymarket::config::tests -- --nocapture
```

Expected: compilation fails because the YAML models and `into_runtime` do not exist.

- [ ] **Step 3: Add dependencies**

Add direct dependencies:

```toml
alloy-signer-local = "1"
serde_yaml = "0.9"
```

Retain `rs-clob-client-v2 = "0.2.2"`.

- [ ] **Step 4: Implement secret-safe configuration models**

Add:

```rust
use std::fmt;
use std::fs;
use std::path::Path;
use serde::Deserialize;

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    fn require_non_empty(&self, field: &str) -> anyhow::Result<()> {
        if self.0.trim().is_empty() {
            anyhow::bail!("{field} must not be empty");
        }
        Ok(())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Deserialize)]
pub struct FileConfig {
    pub proxy: String,
    pub gamma_host: String,
    pub host: String,
    pub chain_id: u64,
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub trade: TradeConfig,
}

#[derive(Deserialize)]
pub struct AccountConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub signature_type: Option<u8>,
    pub private_key: SecretString,
    pub api_key: Option<SecretString>,
    pub api_secret: Option<SecretString>,
    pub api_passphrase: Option<SecretString>,
    pub host: String,
    pub chain_id: u64,
    pub funder: Option<String>,
}

#[derive(Default, Deserialize)]
pub struct TradeConfig {
    pub trader_mode: Option<String>,
    pub account_mode: Option<String>,
    pub market_mode: Option<String>,
}
```

Implement `FileConfig::load(path)` with sanitized `with_context(|| format!("failed to read {}", path.display()))` and `serde_yaml::from_str` context that never includes YAML content.

Implement `TradeConfig::is_live()` as an exact equality check against `"real"`. `into_runtime` must return the existing market `Config` plus `None` before selecting or validating an account when the gate is disabled.

When enabled:

- require exactly one `account_type == "long"`;
- validate `signature_type.unwrap_or(0) <= 3`;
- require non-empty private key and all three L2 fields;
- copy secrets into a `LiveConfig` whose `Debug` implementation remains redacted;
- override the existing market config's proxy, CLOB host, Gamma host, and Gamma event base.

- [ ] **Step 5: Update config structure without exposing values**

Remove only:

```yaml
order_mode: real
```

Under the `type: long` account, add fields if absent:

```yaml
api_key: "YOUR_API_KEY"
api_secret: "YOUR_API_SECRET"
api_passphrase: "YOUR_API_PASSPHRASE"
```

Do not print `config.yaml`, its diff, or its staged contents.

- [ ] **Step 6: Run focused tests and confirm GREEN**

Run:

```bash
cargo test polymarket::config::tests -- --nocapture
```

Expected: all configuration tests pass and captured output contains none of the fixture secrets.

- [ ] **Step 7: Commit Task 1**

```bash
git add Cargo.toml Cargo.lock config.yaml src/polymarket/config.rs \
  openspec/changes/add-polymarket-live-trading/tasks.md
git commit -m "feat: add gated live trading configuration"
```

Before committing, mark OpenSpec tasks `1.1`, `1.2`, and `1.3` complete.

---

### Task 2: Authenticated client and live executor

**Files:**
- Create: `src/polymarket/live.rs`
- Modify: `src/polymarket/mod.rs`
- Test: `src/polymarket/live.rs`

**Interfaces:**
- Consumes: `LiveConfig`, `LimitOrderIntent`, `OrderSide`, and `OrderExecutor`.
- Produces: `create_live_executor`, `LiveOrderExecutor<A>`, pure response validators, and a mockable `TradingApi`.

- [ ] **Step 1: Add failing response and executor tests**

Define fake `TradingApi` responses and tests for:

- `success: true` plus non-empty `orderID` returns that ID;
- `success: false` fails even with an ID;
- empty/missing `orderID` fails;
- `canceled` containing the requested ID succeeds;
- `not_canceled` containing the requested ID fails;
- buy maps to `Side::Buy`, `0.01`, and `5`;
- sell maps to `Side::Sell`, `0.11`, and `5`.

Use only local `serde_json::json!` fixtures and a fake adapter.

- [ ] **Step 2: Run focused tests and confirm RED**

Run:

```bash
cargo test polymarket::live::tests -- --nocapture
```

Expected: compilation fails because `polymarket::live` does not exist.

- [ ] **Step 3: Implement the mockable adapter boundary**

Create provider-local DTO and trait types:

```rust
#[derive(Clone, Debug, PartialEq)]
struct LiveLimitOrder {
    token_id: String,
    price: f64,
    size: f64,
    side: rs_clob_client_v2::types::Side,
}

type TradingFuture<'a> = Pin<
    Box<dyn Future<Output = Result<serde_json::Value, ExecutorError>> + Send + 'a>,
>;

trait TradingApi {
    fn place<'a>(&'a self, order: LiveLimitOrder) -> TradingFuture<'a>;
    fn cancel<'a>(&'a self, order_id: &'a str) -> TradingFuture<'a>;
}
```

`ClobTradingApi` owns `ClobClient`. Its `place` implementation constructs:

```rust
UserLimitOrder {
    token_id: order.token_id,
    price: order.price,
    size: order.size,
    side: order.side,
    expiration: None,
    timestamp: None,
    metadata: None,
    builder: None,
}
```

and calls `create_and_post_limit_order(..., None, OrderType::Gtc)`. `cancel` calls `cancel_order`.

- [ ] **Step 4: Implement strict response validators**

Add pure functions equivalent to:

```rust
fn accepted_order_id(value: &Value) -> Result<String, ExecutorError> {
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(ExecutorError("CLOB placement rejected".into()));
    }
    value
        .get("orderID")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ExecutorError("CLOB placement response missing order ID".into()))
}

fn cancellation_confirmed(value: &Value, order_id: &str) -> Result<(), ExecutorError> {
    let canceled = value
        .get("canceled")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(order_id)));
    let rejected = value
        .get("not_canceled")
        .and_then(Value::as_object)
        .is_some_and(|items| items.contains_key(order_id));
    if canceled && !rejected {
        Ok(())
    } else {
        Err(ExecutorError("CLOB cancellation not confirmed".into()))
    }
}
```

Do not include remote `errorMsg` or `not_canceled` reason text in the returned error because upstream text may contain request details.

- [ ] **Step 5: Implement `LiveOrderExecutor`**

Convert `Decimal` using `ToPrimitive::to_f64()` and fail with a fixed sanitized message on `None`. Map order side and call the adapter. Keep the existing lifecycle responsible for retry/cancel ordering.

```rust
impl<A: TradingApi> OrderExecutor for LiveOrderExecutor<A> {
    fn place_limit<'a>(
        &'a mut self,
        intent: &'a LimitOrderIntent,
    ) -> ExecutorFuture<'a, String> {
        Box::pin(async move {
            let response = self.api.place(map_intent(intent)?).await?;
            accepted_order_id(&response)
        })
    }

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> ExecutorFuture<'a, ()> {
        Box::pin(async move {
            let response = self.api.cancel(order_id).await?;
            cancellation_confirmed(&response, order_id)
        })
    }
}
```

- [ ] **Step 6: Implement secret-safe authenticated client creation**

Map chain IDs explicitly:

```rust
let chain = match config.chain_id {
    137 => Chain::Polygon,
    80002 => Chain::Amoy,
    _ => anyhow::bail!("unsupported long-account chain id"),
};
```

Parse the signer with:

```rust
let signer: alloy_signer_local::PrivateKeySigner = config
    .private_key
    .expose()
    .parse()
    .map_err(|_| anyhow::anyhow!("invalid long-account private key"))?;
```

Construct `ApiKeyCreds` from the three secret fields, then call `ClobClient::new` with:

- account host;
- root Gamma host;
- mapped chain;
- `Some(signer)`;
- `Some(creds)`;
- configured/default signature type;
- optional funder;
- no geoblock token;
- `use_server_time = false`;
- no builder config;
- `Some(root proxy)`.

Never call any API-key management method.

- [ ] **Step 7: Run focused and lifecycle tests**

Run:

```bash
cargo test polymarket::live::tests -- --nocapture
cargo test polymarket::order::tests -- --nocapture
```

Expected: all tests pass with no network access and no credential text in output.

- [ ] **Step 8: Commit Task 2**

Mark OpenSpec tasks `2.1` through `2.4` complete, then:

```bash
git add src/polymarket/live.rs src/polymarket/mod.rs \
  openspec/changes/add-polymarket-live-trading/tasks.md
git commit -m "feat: add proxied live order executor"
```

---

### Task 3: One-shot startup orchestration

**Files:**
- Modify: `src/main.rs`
- Modify: `src/polymarket/ws.rs`
- Modify: `src/polymarket/live.rs`
- Test: `src/polymarket/live.rs`
- Test: `src/polymarket/ws.rs`

**Interfaces:**
- Consumes: `Option<LiveConfig>`, discovered event tokens, and populated `QuoteState`.
- Produces: `maybe_run_fixed_live_flow` and the updated `run_market_stream(config, live, event)` entry point.

- [ ] **Step 1: Add failing one-shot gate tests**

Add tests around an injected executor factory proving:

- `None` live config returns before invoking the factory;
- an empty event returns before invoking the factory;
- a first token without a quote returns before invoking the factory;
- enabled configuration invokes the factory once and the lifecycle once;
- no call site exists inside the WebSocket reconnect loop.

The factory returns `MockOrderExecutor`; no test may call `create_live_executor`.

- [ ] **Step 2: Run focused tests and confirm RED**

Run:

```bash
cargo test polymarket::live::tests::disabled_mode_does_not_create_executor -- --exact
```

Expected: compilation fails because the orchestration helper does not exist.

- [ ] **Step 3: Implement the injectable orchestration helper**

The helper must perform checks in this order:

1. return `Ok(None)` when live configuration is absent;
2. obtain the first token or fail;
3. require `state.latest_quote(first.asset_id)` before creating an executor;
4. invoke a supplied `FnOnce(&LiveConfig) -> anyhow::Result<Box<dyn OrderExecutor>>`;
5. call `run_new_zealand_belgium_flow`;
6. return the receipt or sanitized lifecycle error.

The production wrapper supplies `create_live_executor`.

- [ ] **Step 4: Wire startup outside the reconnect loop**

Change:

```rust
pub async fn run_market_stream(
    config: Config,
    live: Option<LiveConfig>,
    event: DiscoveredEvent,
) -> Result<()>
```

Call the production one-shot helper immediately after `load_initial_orderbooks` populates `QuoteState` and before `let payload = ...` and `loop { ... }`.

Log only scenario name, market slug, outcome, asset ID, and accepted order IDs. Do not print `LiveConfig`, client errors with request bodies, or any secrets.

- [ ] **Step 5: Load file configuration in `main`**

Replace `Config::default()` construction with:

```rust
let file = polymarket::config::FileConfig::load("config.yaml")?;
let (config, live) = file.into_runtime()?;
let event = polymarket::discovery::discover_event(&config).await?;
polymarket::ws::run_market_stream(config, live, event).await
```

Keep OddsPortal orchestration unchanged.

- [ ] **Step 6: Run focused and full tests**

Run:

```bash
cargo test polymarket::live::tests -- --nocapture
cargo test polymarket::ws::tests -- --nocapture
cargo test
```

Expected: all tests pass; no test performs an authenticated network request.

- [ ] **Step 7: Commit Task 3**

Mark OpenSpec tasks `3.1` and `3.2` complete, then:

```bash
git add src/main.rs src/polymarket/live.rs src/polymarket/ws.rs \
  openspec/changes/add-polymarket-live-trading/tasks.md
git commit -m "feat: run fixed live flow once at startup"
```

---

### Task 4: Architecture, safety verification, and change closure

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `openspec/changes/add-polymarket-live-trading/tasks.md`

**Interfaces:**
- Consumes: Completed runtime behavior and final file layout.
- Produces: Synchronized architecture documentation and verification evidence.

- [ ] **Step 1: Update architecture documentation**

Document:

- typed `config.yaml` ownership under `src/polymarket/config.rs`;
- `src/polymarket/live.rs` ownership of signer/client construction and live executor;
- three-mode gate and unique long-account selection;
- first-token, immediate-sell, single-cancel, one-shot data flow;
- retained proxy-routed `rs-clob-client-v2`;
- no change to OddsPortal's read-only boundary.

- [ ] **Step 2: Run formatting and all verification commands**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
openspec validate add-polymarket-live-trading --strict
git diff --check
```

If Clippy reports warnings present at `base-ref`, compare against the base and document only proven baseline exclusions; do not suppress new warnings.

- [ ] **Step 3: Scan source and output for credential exposure**

Without printing `config.yaml`, scan changed Rust and documentation files for:

```bash
rg -n 'println!|eprintln!|dbg!|Debug' src/polymarket src/main.rs
rg -n 'create_api_key|derive_api_key|create_or_derive_api_key' src
```

Review every match. The second command must return no live-path calls. Run test output through placeholder canaries and confirm no configured or fixture credential appears.

- [ ] **Step 4: Mark documentation and verification tasks complete**

Mark OpenSpec tasks `4.1` and `4.2` complete.

- [ ] **Step 5: Commit Task 4**

```bash
git add ARCHITECTURE.md openspec/changes/add-polymarket-live-trading/tasks.md
git commit -m "docs: document guarded live trading flow"
```

- [ ] **Step 6: Run Comet build guard**

Run:

```bash
"$COMET_BASH" "$COMET_GUARD" add-polymarket-live-trading build --apply
```

Expected: all build checks pass and `.comet.yaml` transitions to `phase: verify`.
