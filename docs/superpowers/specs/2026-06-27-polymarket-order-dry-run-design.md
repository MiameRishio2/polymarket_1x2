---
comet_change: polymarket-order-dry-run
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-27-polymarket-order-dry-run
status: final
---

# Polymarket Order Dry-Run Technical Design

## Scope

Add a provider-local, read-only order lifecycle simulation that consumes the same latest quote state populated by the existing initial CLOB snapshot and market WebSocket pipeline.

The workflow models New Zealand vs Belgium with:

1. A simulated limit buy at price `0.01` and size `5`.
2. A synthetic accepted order ID.
3. A simulated limit sell for the same asset and size at price `0.11`.
4. At most one simulated cancellation of the accepted buy after a later failure.

The implementation does not read `config.yaml`, load an account, handle credentials, sign payloads, or call exchange write endpoints.

## Architecture

The implementation remains under the Polymarket provider boundary:

```text
src/polymarket/
├── order.rs       # intents, executor interface, mock, lifecycle
├── quotes.rs      # existing state plus immutable snapshot accessor
└── ws.rs          # unchanged order-book/WebSocket ingestion
```

No top-level `src/order/` module is added because the abstraction is not shared by a second provider.

### Components

`LimitOrderIntent` contains the asset ID, side, price, and size. Construction validates that the asset ID is non-empty, price is strictly between zero and one, and size is positive. Values use `rust_decimal::Decimal`; the workflow never uses floating-point arithmetic.

`OrderExecutor` is an object-safe asynchronous interface. Its placement and cancellation methods return boxed, sendable futures. Lifecycle code accepts `&mut dyn OrderExecutor`, allowing deterministic backend substitution without adding `async-trait`.

`MockOrderExecutor` owns configured responses and an ordered call log. It returns synthetic order IDs and never owns an HTTP, WebSocket, credential, or signing client.

`OrderFlowResult` records accepted IDs, attempted intents, cancellation outcome, and the terminal error when present. This makes failure behavior observable without logs or remote state.

`QuoteState::latest_quote` returns a cloned `QuoteRecord` for a known asset after initial CLOB or market WebSocket state has populated it. It does not expose mutable book internals.

## Data Flow

```text
initial CLOB / market WSS update
              |
              v
         QuoteState
              |
       latest_quote(asset)
              |
              v
       run_order_flow
              |
       BUY 0.01 × 5
              |
      synthetic order_id
              |
       SELL 0.11 × 5
          |       |
       success  failure
                  |
        cancel buy once
                  |
               return
```

The sell intent is not constructed or submitted until buy placement returns an order ID. A failed buy without an order ID terminates immediately. No branch contains a retry loop.

## Interface Choice

The recommended interface is an object-safe trait whose methods return `Pin<Box<dyn Future<...> + Send + '_>>`.

- A synchronous trait would be simpler but would force a breaking interface change for any future network-backed executor.
- Native `async fn` in traits is concise but does not support the desired trait-object substitution.
- `async-trait` is unnecessary dependency overhead for two methods.

This change intentionally provides no live executor. A live backend would require a separate approved change to repository policy, credential boundaries, signing, network safety, and integration testing.

## Failure Semantics

- Missing quote: return before placement.
- Invalid decimal intent: return before placement.
- Buy placement failure without ID: return; do not sell or cancel.
- Sell placement failure after accepted buy: call cancellation exactly once for the buy ID, then return.
- Cancellation failure: retain both the sell and cancellation errors; do not retry either operation.

All synthetic IDs and results must be labeled as simulated.

## Testing

Focused unit tests cover:

- Valid `0.01 × 5` construction and invalid price/size rejection.
- Latest-quote lookup after book and price updates.
- Missing quote rejection before executor calls.
- Exact buy then sell ordering.
- Sell price computed as decimal `0.01 + 0.10 = 0.11`.
- Sell failure followed by exactly one buy cancellation.
- Cancellation failure without further calls.
- No placement retries.

After focused tests, run the full `cargo test` suite as required by `AGENTS.md`.

## Runtime and Documentation

`src/polymarket/mod.rs` exports the module. `main.rs` remains unchanged, so normal collection never starts the simulation. `ARCHITECTURE.md` documents the new module while retaining the unauthenticated, read-only project identity.
