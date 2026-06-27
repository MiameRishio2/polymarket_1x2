## Context

The binary currently constructs an unauthenticated third-party `rs-clob-client-v2` client from hard-coded Polymarket defaults, loads initial books, and then enters the market WebSocket loop. `src/polymarket/order.rs` already expresses the required `0.01 × 5` buy, immediate `0.11 × 5` sell, and single-cancellation failure path through an executor trait, but only `MockOrderExecutor` implements that trait.

The tracked `config.yaml` already contains root endpoint/proxy settings, multiple account entries, and trade-mode fields. The live test path must use that file directly, choose the `type: long` account, and never expose its private key in logs or errors.

## Goals / Non-Goals

**Goals:**

- Parse the existing YAML configuration into typed Polymarket runtime settings.
- Require `trader_mode`, `account_mode`, and `market_mode` to all equal `real` before any authenticated client or write request is created.
- Migrate public CLOB access and authenticated signing to Polymarket's official `polymarket_client_sdk_v2`, using the selected long account's private key, signature type, funder, host, and chain.
- Reuse the executor-independent fixed flow with the first discovered token and the initial quote snapshot.
- Run the live test flow exactly once per process start.
- Keep credentials and signed payload details out of output.

**Non-Goals:**

- A generic trading CLI, configurable price/size, fill polling, retries, portfolio management, allowance management, or multi-account execution.
- Waiting for the buy to fill before posting the sell.
- Changing the OddsPortal provider.
- Automatically provisioning balances, token approvals, or proxy wallets.

## Decisions

### Typed YAML configuration with a strict write gate

Add Serde YAML models under `src/polymarket/`. The runtime reads `config.yaml`, removes `trade.order_mode`, and considers live trading enabled only when the three retained mode strings are exactly `real`. Configuration is evaluated before wallet parsing so non-live modes cannot accidentally initialize signing or write-side clients.

Alternative rejected: an environment-variable switch would conflict with the requested YAML-driven behavior. Treating any single `real` value as sufficient would make mixed test configurations unsafe.

### Exact long-account selection

Select exactly one account whose `type` equals `long`. Missing or duplicate long accounts fail before authentication. `signature_type: null` maps to EOA (`0`); explicit values `0` through `3` are passed to `rs-clob-client-v2`. The account-level private key, host, chain ID, and optional funder are authoritative; the root Gamma host and proxy remain shared transport settings.

Alternative rejected: selecting the first account makes list ordering a hidden safety control.

### Official SDK migration and authenticated client initialization

Replace the current third-party crate with Polymarket's official `polymarket_client_sdk_v2`. It provides typed decimal orders, compile-time authentication states, V1/V2 host detection, and the current signature/funder flow documented by Polymarket. Parse the configured private key into an Alloy local signer, configure the selected signature type and optional funder through the authentication builder, and authenticate before constructing the live executor. Initialization errors are wrapped with stage-only context and must not include the private key or full signed request.

Alternatives rejected: retaining the third-party crate keeps an `f64` conversion boundary for order price/size and an older manual credential lifecycle; raw REST would duplicate signing and authentication logic. Persisting API credentials in the repository is unnecessary because the official client authenticates from the configured signer.

### Live executor behind the existing trait

Add a `LiveOrderExecutor` that maps `LimitOrderIntent` to the official SDK's typed decimal limit-order builder, creates and signs a GTC order, posts it, and implements single-order cancellation. Placement succeeds only when the SDK response reports success and a non-empty order ID. Cancellation succeeds only when the response's `canceled` collection contains the requested ID; entries in `not_canceled` are failures. The fixed lifecycle remains in `run_new_zealand_belgium_flow`, preserving its current accepted-buy/immediate-sell semantics and at-most-one cancellation.

Alternative rejected: duplicating the lifecycle inside the client adapter would create divergent mock and live behavior.

### One-shot startup integration

After event discovery and initial order-book loading, select the event's first token and run the fixed lifecycle once if the three-mode gate is enabled. A live-flow error terminates startup rather than silently continuing. The WebSocket reconnect loop never retries the live flow.

Alternative rejected: running from the WebSocket loop risks duplicate orders after reconnects.

## Risks / Trade-offs

- **Accepted buy may remain unfilled, causing the immediate sell to fail** → Preserve the user-selected simulation semantics and attempt one cancellation of the buy.
- **CLOB response may report failure, omit an order ID, or reject cancellation** → Validate `success`, `orderID`, `canceled`, and `not_canceled`; never invent identifiers or treat HTTP success alone as trading success.
- **Configured account may lack balances or allowances** → Return a sanitized error; do not attempt automatic funding or approvals.
- **Tracked configuration may contain a test private key** → Never print or serialize the key from application code or tests; repository tracking is an explicitly accepted test-account risk.
- **Selecting the first discovered token can target an unintended outcome** → This is an explicitly accepted test-only constraint; log only the selected market/outcome/asset before execution.
- **Process restart can repeat the test orders** → The gate is explicit but not durable; operators must disable at least one mode before restarting if another test is not intended.

## Migration Plan

1. Remove `trade.order_mode` from `config.yaml`.
2. Add typed configuration loading while retaining unrelated YAML sections.
3. Add and test the live client adapter using mocks or local response parsing only.
4. Wire the gated one-shot flow after initial quotes.
5. Verify all non-`real` mode combinations produce zero authenticated/write calls.

Rollback consists of setting any one of the three trade modes away from `real` or reverting the startup wiring; quote collection remains available.

## Open Questions

None. The test-only first-token selection and immediate-sell semantics were explicitly confirmed.
