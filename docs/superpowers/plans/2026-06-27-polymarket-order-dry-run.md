---
change: polymarket-order-dry-run
design-doc: docs/superpowers/specs/2026-06-27-polymarket-order-dry-run-design.md
base-ref: 678e33583a5d7bfb72be9e13b030b5df7308b231
archived-with: 2026-06-27-polymarket-order-dry-run
---

# Polymarket Order Dry-Run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a provider-local, quote-gated simulation of a `0.01 × 5` limit buy followed by a `0.11 × 5` limit sell through an abstract asynchronous executor and configurable mock.

**Architecture:** `src/polymarket/order.rs` owns validated decimal intents, the object-safe asynchronous `OrderExecutor`, `MockOrderExecutor`, and fail-closed lifecycle orchestration. `QuoteState` gains only an immutable snapshot accessor; the existing CLOB and market WebSocket ingestion remains unchanged.

**Tech Stack:** Rust 2021, `tokio`, `rust_decimal`, existing Polymarket models and quote state, standard-library `Future`/`Pin`/`VecDeque`.

## Global Constraints

- Keep all order simulation code under `src/polymarket/`; do not add a top-level `src/order/`.
- Do not read `config.yaml`, credentials, private keys, signatures, balances, or allowances.
- Do not add a live executor or call create-order/cancel-order network endpoints.
- Keep `src/main.rs` unchanged; normal collection must never start the simulation.
- Use `rust_decimal::Decimal`; do not use floating-point prices or sizes.
- Placement and cancellation must never retry.
- Run `cargo test` after Rust changes.
- Do not create a git commit unless the user explicitly authorizes it.

---

### Task 1: Validated order intent contract

**Files:**

- Create: `src/polymarket/order.rs`
- Modify: `src/polymarket/mod.rs:1-7`

**Interfaces:**

- Consumes: `rust_decimal::Decimal`.
- Produces:
  - `pub const SCENARIO_NAME: &str`
  - `pub enum OrderSide { Buy, Sell }`
  - `pub struct LimitOrderIntent`
  - `pub enum OrderValidationError`
  - `LimitOrderIntent::new(asset_id, side, price, size) -> Result<Self, OrderValidationError>`

- [ ] **Step 1: Export the module and add failing validation tests**

Add `pub mod order;` in alphabetical position in `src/polymarket/mod.rs`. Create `src/polymarket/order.rs` with the imports, public types, and tests below. Leave `LimitOrderIntent::new` returning `unimplemented!()` for the red test.

```rust
use std::fmt;

use rust_decimal::Decimal;

pub const SCENARIO_NAME: &str = "New Zealand vs Belgium";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitOrderIntent {
    pub asset_id: String,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderValidationError {
    EmptyAssetId,
    PriceOutOfRange,
    NonPositiveSize,
}

impl fmt::Display for OrderValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAssetId => formatter.write_str("asset id must not be empty"),
            Self::PriceOutOfRange => {
                formatter.write_str("limit price must be strictly between zero and one")
            }
            Self::NonPositiveSize => formatter.write_str("order size must be positive"),
        }
    }
}

impl std::error::Error for OrderValidationError {}

impl LimitOrderIntent {
    pub fn new(
        asset_id: impl Into<String>,
        side: OrderSide,
        price: Decimal,
        size: Decimal,
    ) -> Result<Self, OrderValidationError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn accepts_required_buy_intent_without_floating_point() {
        let intent = LimitOrderIntent::new(
            "asset-101",
            OrderSide::Buy,
            Decimal::new(1, 2),
            Decimal::new(5, 0),
        )
        .unwrap();

        assert_eq!(intent.price, Decimal::new(1, 2));
        assert_eq!(intent.size, Decimal::new(5, 0));
        assert_eq!(intent.side, OrderSide::Buy);
    }

    #[test]
    fn rejects_empty_asset_invalid_price_and_non_positive_size() {
        assert_eq!(
            LimitOrderIntent::new(
                "",
                OrderSide::Buy,
                Decimal::new(1, 2),
                Decimal::new(5, 0),
            ),
            Err(OrderValidationError::EmptyAssetId)
        );
        assert_eq!(
            LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::ZERO,
                Decimal::new(5, 0),
            ),
            Err(OrderValidationError::PriceOutOfRange)
        );
        assert_eq!(
            LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::ONE,
                Decimal::new(5, 0),
            ),
            Err(OrderValidationError::PriceOutOfRange)
        );
        assert_eq!(
            LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::new(1, 2),
                Decimal::ZERO,
            ),
            Err(OrderValidationError::NonPositiveSize)
        );
    }
}
```

- [ ] **Step 2: Run the focused tests and verify the red state**

Run:

```bash
cargo test polymarket::order::tests:: -- --nocapture
```

Expected: FAIL because `LimitOrderIntent::new` reaches `not implemented`.

- [ ] **Step 3: Implement minimal validation**

Replace `LimitOrderIntent::new` with:

```rust
pub fn new(
    asset_id: impl Into<String>,
    side: OrderSide,
    price: Decimal,
    size: Decimal,
) -> Result<Self, OrderValidationError> {
    let asset_id = asset_id.into();
    if asset_id.trim().is_empty() {
        return Err(OrderValidationError::EmptyAssetId);
    }
    if price <= Decimal::ZERO || price >= Decimal::ONE {
        return Err(OrderValidationError::PriceOutOfRange);
    }
    if size <= Decimal::ZERO {
        return Err(OrderValidationError::NonPositiveSize);
    }

    Ok(Self {
        asset_id,
        side,
        price,
        size,
    })
}
```

- [ ] **Step 4: Run the focused tests and verify the green state**

Run:

```bash
cargo test polymarket::order::tests:: -- --nocapture
```

Expected: PASS for both validation tests.

- [ ] **Step 5: Record the task checkpoint**

Mark OpenSpec tasks `1.1` and `1.2` complete. Do not commit without explicit user authorization. If authorization is later provided, the intended commit is:

```bash
git add src/polymarket/mod.rs src/polymarket/order.rs openspec/changes/polymarket-order-dry-run/tasks.md
git commit -m "feat: add validated Polymarket order intents"
```

### Task 2: Immutable latest quote access

**Files:**

- Modify: `src/polymarket/quotes.rs:23-126`

**Interfaces:**

- Consumes: existing `QuoteState::record`.
- Produces: `pub fn latest_quote(&self, asset_id: &str) -> Option<QuoteRecord>`.

- [ ] **Step 1: Add failing snapshot tests**

Append these tests inside the existing `quotes.rs` test module:

```rust
#[test]
fn latest_quote_returns_a_clone_without_mutating_state() {
    let mut state = QuoteState::new("event", vec![token()]);
    state.apply_book(
        "101",
        vec![PriceLevel {
            price: "0.61".to_string(),
            size: "10".to_string(),
        }],
        vec![PriceLevel {
            price: "0.64".to_string(),
            size: "40".to_string(),
        }],
        "book",
    );

    let first = state.latest_quote("101").unwrap();
    let second = state.latest_quote("101").unwrap();

    assert_eq!(first.asset_id, "101");
    assert_eq!(first.bid_price.as_deref(), Some("0.61"));
    assert_eq!(second.ask_price.as_deref(), Some("0.64"));
}

#[test]
fn latest_quote_is_none_before_an_asset_update() {
    let state = QuoteState::new("event", vec![token()]);

    assert!(state.latest_quote("101").is_none());
    assert!(state.latest_quote("unknown").is_none());
}
```

- [ ] **Step 2: Run the quote tests and verify the red state**

Run:

```bash
cargo test polymarket::quotes::tests::latest_quote -- --nocapture
```

Expected: compilation FAIL with no method named `latest_quote`.

- [ ] **Step 3: Implement the immutable accessor**

Add this method inside `impl QuoteState`, immediately before the private `record` method:

```rust
pub fn latest_quote(&self, asset_id: &str) -> Option<QuoteRecord> {
    self.record(asset_id, "snapshot")
}
```

This returns owned strings through the existing `QuoteRecord` construction and does not expose mutable quote internals.

- [ ] **Step 4: Run the quote tests and verify the green state**

Run:

```bash
cargo test polymarket::quotes::tests::latest_quote -- --nocapture
```

Expected: PASS for both snapshot tests.

- [ ] **Step 5: Record the task checkpoint**

Mark OpenSpec tasks `2.1` and `2.2` complete. Do not commit without explicit user authorization. If authorization is later provided, the intended commit is:

```bash
git add src/polymarket/quotes.rs openspec/changes/polymarket-order-dry-run/tasks.md
git commit -m "feat: expose immutable Polymarket quote snapshots"
```

### Task 3: Abstract executor and fail-closed mock lifecycle

**Files:**

- Modify: `src/polymarket/order.rs`
- Consume: `src/polymarket/quotes.rs`

**Interfaces:**

- Consumes:
  - `QuoteState::latest_quote(&self, asset_id: &str) -> Option<QuoteRecord>`
  - `LimitOrderIntent::new(...)`
- Produces:
  - `pub type ExecutorFuture<'a, T>`
  - `pub trait OrderExecutor`
  - `pub struct MockOrderExecutor`
  - `pub enum ExecutorCall`
  - `pub struct OrderFlowReceipt`
  - `pub enum OrderFlowError`
  - `pub async fn run_new_zealand_belgium_flow(...)`

- [ ] **Step 1: Add executor and lifecycle type declarations**

Add these imports and declarations above the existing test module in `order.rs`:

```rust
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;

use crate::polymarket::quotes::QuoteState;

pub type ExecutorFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ExecutorError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutorError(pub String);

impl fmt::Display for ExecutorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ExecutorError {}

pub trait OrderExecutor {
    fn place_limit<'a>(
        &'a mut self,
        intent: &'a LimitOrderIntent,
    ) -> ExecutorFuture<'a, String>;

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> ExecutorFuture<'a, ()>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutorCall {
    Place(LimitOrderIntent),
    Cancel(String),
}

pub struct MockOrderExecutor {
    placements: VecDeque<Result<String, ExecutorError>>,
    cancellations: VecDeque<Result<(), ExecutorError>>,
    calls: Vec<ExecutorCall>,
}

impl MockOrderExecutor {
    pub fn scripted(
        placements: impl IntoIterator<Item = Result<String, ExecutorError>>,
        cancellations: impl IntoIterator<Item = Result<(), ExecutorError>>,
    ) -> Self {
        Self {
            placements: placements.into_iter().collect(),
            cancellations: cancellations.into_iter().collect(),
            calls: Vec::new(),
        }
    }

    pub fn calls(&self) -> &[ExecutorCall] {
        &self.calls
    }
}

impl OrderExecutor for MockOrderExecutor {
    fn place_limit<'a>(
        &'a mut self,
        intent: &'a LimitOrderIntent,
    ) -> ExecutorFuture<'a, String> {
        self.calls.push(ExecutorCall::Place(intent.clone()));
        let response = self
            .placements
            .pop_front()
            .unwrap_or_else(|| Err(ExecutorError("unscripted placement".to_string())));
        Box::pin(async move { response })
    }

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> ExecutorFuture<'a, ()> {
        self.calls
            .push(ExecutorCall::Cancel(order_id.to_string()));
        let response = self
            .cancellations
            .pop_front()
            .unwrap_or_else(|| Err(ExecutorError("unscripted cancellation".to_string())));
        Box::pin(async move { response })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderFlowReceipt {
    pub buy_order_id: String,
    pub sell_order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderFlowError {
    MissingQuote(String),
    InvalidIntent(OrderValidationError),
    BuyFailed(ExecutorError),
    SellFailed {
        buy_order_id: String,
        sell_error: ExecutorError,
        cancellation: Result<(), ExecutorError>,
    },
}
```

- [ ] **Step 2: Add failing success and failure tests**

Append the following helpers and async tests to the existing `order.rs` test module:

```rust
use crate::polymarket::models::{PriceLevel, TokenMeta};
use crate::polymarket::quotes::QuoteState;

fn quote_state() -> QuoteState {
    let mut state = QuoteState::new(
        "new-zealand-vs-belgium",
        vec![TokenMeta {
            market_slug: "new-zealand-vs-belgium".to_string(),
            question: "New Zealand vs Belgium".to_string(),
            outcome: "New Zealand".to_string(),
            asset_id: "asset-101".to_string(),
        }],
    );
    state.apply_book(
        "asset-101",
        vec![PriceLevel {
            price: "0.01".to_string(),
            size: "5".to_string(),
        }],
        vec![PriceLevel {
            price: "0.02".to_string(),
            size: "5".to_string(),
        }],
        "book",
    );
    state
}

#[tokio::test]
async fn accepted_buy_is_followed_by_decimal_sell() {
    let state = quote_state();
    let mut executor = MockOrderExecutor::scripted(
        [
            Ok("sim-buy-1".to_string()),
            Ok("sim-sell-1".to_string()),
        ],
        [],
    );

    let receipt =
        run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap();

    assert_eq!(receipt.buy_order_id, "sim-buy-1");
    assert_eq!(receipt.sell_order_id, "sim-sell-1");
    assert_eq!(executor.calls().len(), 2);
    let ExecutorCall::Place(sell) = &executor.calls()[1] else {
        panic!("second call must place the sell");
    };
    assert_eq!(sell.side, OrderSide::Sell);
    assert_eq!(sell.price, Decimal::new(11, 2));
    assert_eq!(sell.size, Decimal::new(5, 0));
}

#[tokio::test]
async fn sell_failure_cancels_buy_once_without_retry() {
    let state = quote_state();
    let mut executor = MockOrderExecutor::scripted(
        [
            Ok("sim-buy-1".to_string()),
            Err(ExecutorError("sell rejected".to_string())),
        ],
        [Ok(())],
    );

    let error =
        run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

    assert_eq!(
        error,
        OrderFlowError::SellFailed {
            buy_order_id: "sim-buy-1".to_string(),
            sell_error: ExecutorError("sell rejected".to_string()),
            cancellation: Ok(()),
        }
    );
    assert_eq!(
        executor.calls(),
        &[
            ExecutorCall::Place(LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::new(1, 2),
                Decimal::new(5, 0),
            ).unwrap()),
            ExecutorCall::Place(LimitOrderIntent::new(
                "asset-101",
                OrderSide::Sell,
                Decimal::new(11, 2),
                Decimal::new(5, 0),
            ).unwrap()),
            ExecutorCall::Cancel("sim-buy-1".to_string()),
        ]
    );
}

#[tokio::test]
async fn buy_failure_stops_after_one_placement() {
    let state = quote_state();
    let mut executor = MockOrderExecutor::scripted(
        [Err(ExecutorError("buy rejected".to_string()))],
        [],
    );

    let error =
        run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

    assert_eq!(
        error,
        OrderFlowError::BuyFailed(ExecutorError("buy rejected".to_string()))
    );
    assert_eq!(executor.calls().len(), 1);
}

#[tokio::test]
async fn cancellation_failure_is_reported_without_another_call() {
    let state = quote_state();
    let mut executor = MockOrderExecutor::scripted(
        [
            Ok("sim-buy-1".to_string()),
            Err(ExecutorError("sell rejected".to_string())),
        ],
        [Err(ExecutorError("cancel rejected".to_string()))],
    );

    let error =
        run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

    assert_eq!(
        error,
        OrderFlowError::SellFailed {
            buy_order_id: "sim-buy-1".to_string(),
            sell_error: ExecutorError("sell rejected".to_string()),
            cancellation: Err(ExecutorError("cancel rejected".to_string())),
        }
    );
    assert_eq!(executor.calls().len(), 3);
}

#[tokio::test]
async fn missing_quote_stops_before_executor_calls() {
    let state = QuoteState::new("new-zealand-vs-belgium", Vec::new());
    let mut executor = MockOrderExecutor::scripted([], []);

    let error =
        run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

    assert_eq!(
        error,
        OrderFlowError::MissingQuote("asset-101".to_string())
    );
    assert!(executor.calls().is_empty());
}
```

- [ ] **Step 3: Run lifecycle tests and verify the red state**

Run:

```bash
cargo test polymarket::order::tests:: -- --nocapture
```

Expected: compilation FAIL because `run_new_zealand_belgium_flow` is not defined.

- [ ] **Step 4: Implement the lifecycle once, without retry loops**

Add this function before the test module:

```rust
pub async fn run_new_zealand_belgium_flow(
    quotes: &QuoteState,
    asset_id: &str,
    executor: &mut dyn OrderExecutor,
) -> Result<OrderFlowReceipt, OrderFlowError> {
    quotes
        .latest_quote(asset_id)
        .ok_or_else(|| OrderFlowError::MissingQuote(asset_id.to_string()))?;

    let size = Decimal::new(5, 0);
    let buy_price = Decimal::new(1, 2);
    let sell_price = buy_price + Decimal::new(1, 1);
    let buy = LimitOrderIntent::new(asset_id, OrderSide::Buy, buy_price, size)
        .map_err(OrderFlowError::InvalidIntent)?;
    let buy_order_id = executor
        .place_limit(&buy)
        .await
        .map_err(OrderFlowError::BuyFailed)?;

    let sell = LimitOrderIntent::new(asset_id, OrderSide::Sell, sell_price, size)
        .map_err(OrderFlowError::InvalidIntent)?;
    match executor.place_limit(&sell).await {
        Ok(sell_order_id) => Ok(OrderFlowReceipt {
            buy_order_id,
            sell_order_id,
        }),
        Err(sell_error) => {
            let cancellation = executor.cancel(&buy_order_id).await;
            Err(OrderFlowError::SellFailed {
                buy_order_id,
                sell_error,
                cancellation,
            })
        }
    }
}
```

- [ ] **Step 5: Run focused lifecycle tests and Clippy**

Run:

```bash
cargo test polymarket::order::tests:: -- --nocapture
cargo clippy --all-targets -- -D warnings \
  -A dead_code \
  -A clippy::manual_is_multiple_of \
  -A clippy::filter_next \
  -A clippy::useless_conversion
```

Expected: all order tests PASS and Clippy exits successfully. `dead_code` is excluded because the approved module API is intentionally not called by `main.rs`; the other three excluded lints exist at `base-ref` in unrelated provider code.

- [ ] **Step 6: Record the task checkpoint**

Mark OpenSpec tasks `3.1`, `3.2`, and `3.3` complete. Do not commit without explicit user authorization. If authorization is later provided, the intended commit is:

```bash
git add src/polymarket/order.rs openspec/changes/polymarket-order-dry-run/tasks.md
git commit -m "feat: simulate fail-closed order lifecycle"
```

### Task 4: Module ownership, architecture, and full verification

**Files:**

- Modify: `ARCHITECTURE.md:9-116`
- Modify: `openspec/changes/polymarket-order-dry-run/tasks.md`
- Create: `docs/superpowers/reports/2026-06-27-polymarket-order-dry-run-verify.md`

**Interfaces:**

- Consumes: complete `src/polymarket/order.rs`.
- Produces: public provider-local module export and synchronized architecture documentation.

- [ ] **Step 1: Update architecture ownership and data flow**

In the `ARCHITECTURE.md` source tree, add:

```text
│   ├── order.rs             # Abstract executor and read-only order lifecycle simulation
```

Extend the Polymarket Provider component text to state:

```markdown
The provider also exposes a simulation-only order lifecycle through an abstract executor and local mock. It consumes immutable quote snapshots, is not started by `main.rs`, and has no credential, signing, placement, or cancellation network capability.
```

Add this data flow after the main Polymarket quote flow:

```text
Latest QuoteState snapshot
    |
    v
Validated decimal buy intent
    |
    v
Mock OrderExecutor -> synthetic order ID
    |
    v
Validated decimal sell intent or one simulated cancellation
```

Keep the final external integration statement unchanged:

```markdown
The collector is unauthenticated and read-only. It must not require private keys, API credentials, or order placement permissions.
```

- [ ] **Step 2: Format and run all verification commands**

Run:

```bash
cargo fmt -- --check
cargo test polymarket::order::tests:: -- --nocapture
cargo test polymarket::quotes::tests:: -- --nocapture
cargo test
cargo clippy --all-targets -- -D warnings \
  -A dead_code \
  -A clippy::manual_is_multiple_of \
  -A clippy::filter_next \
  -A clippy::useless_conversion
openspec validate polymarket-order-dry-run --strict
git diff --check
```

Expected: every command exits with status `0`; focused and full Rust tests report no failures; Clippy passes with the documented baseline exclusions; OpenSpec reports the change is valid; `git diff --check` prints nothing.

If `cargo fmt -- --check` reports formatting differences, run `cargo fmt`, then repeat the complete command list.

- [ ] **Step 3: Record verification evidence**

Create `docs/superpowers/reports/2026-06-27-polymarket-order-dry-run-verify.md`. Record the exact numeric counts printed by the focused and full test commands; do not estimate them. Include the change name, overall PASS result, focused order count, focused quote count, full test count, Clippy result and baseline exclusions, OpenSpec validation result, whitespace result, zero live orders, and zero credential reads.

- [ ] **Step 4: Complete the OpenSpec task checklist**

Mark tasks `4.1` and `4.2` complete only after all verification commands pass and the report contains actual counts.

- [ ] **Step 5: Record the final checkpoint**

Do not commit without explicit user authorization. If authorization is later provided, the intended commit is:

```bash
git add src/polymarket/mod.rs src/polymarket/order.rs src/polymarket/quotes.rs ARCHITECTURE.md openspec/changes/polymarket-order-dry-run docs/superpowers
git commit -m "feat: add Polymarket order dry-run"
```
