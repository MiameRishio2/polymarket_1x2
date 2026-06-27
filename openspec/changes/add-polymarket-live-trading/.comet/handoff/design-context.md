# Comet Design Handoff

- Change: add-polymarket-live-trading
- Phase: design
- Mode: compact
- Context hash: 9646bea89f0854cbf0625d07d5766b5654998bce40ed011786672f738b5ccf2d

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/add-polymarket-live-trading/proposal.md

- Source: openspec/changes/add-polymarket-live-trading/proposal.md
- Lines: 1-30
- SHA256: 33550f1986837b644591e289b93c3fc8719a96a3139c8bd38dd0d216fa6d8501

```md
## Why

The repository currently models the fixed Polymarket buy/sell lifecycle only through a mock executor, so it cannot exercise the same flow against the live CLOB. A deliberately gated test path is needed to place the existing `0.01 × 5` buy followed by the `0.11 × 5` sell with a configured test account.

## What Changes

- Load Polymarket account, signature, funder, endpoint, proxy, and trade-mode settings from the tracked `config.yaml`.
- Replace the current third-party CLOB crate with Polymarket's official `polymarket_client_sdk_v2`, then add a live `OrderExecutor` with wallet authentication, limit-order posting, strict response validation, and single-order cancellation.
- Start the fixed live flow only when `trader_mode`, `account_mode`, and `market_mode` are all `real`.
- Select the configured account whose `type` is `long` and the first token returned by event discovery.
- Preserve the existing simulation semantics: post the sell immediately after the buy is accepted; if the sell fails, attempt one cancellation of the accepted buy.
- **BREAKING**: remove `trade.order_mode`; the three remaining mode fields are the complete live-trading gate.
- Keep secrets out of logs and error output. Tests use invalid placeholder credentials and mocked responses; they never submit network orders.

## Capabilities

### New Capabilities

- `polymarket-live-trading`: Configuration gating, authenticated CLOB execution, fixed-flow startup, response handling, and fail-closed behavior for the live test path.

### Modified Capabilities

None. The existing dry-run capability remains valid and provides the executor-independent lifecycle reused by the live backend.

## Impact

- Affected source: `src/main.rs` and Polymarket modules for configuration, CLOB client construction, and order execution.
- Affected configuration: `config.yaml` trade and account fields.
- Dependencies change from the third-party `rs-clob-client-v2` crate to Polymarket's official Rust SDK, plus YAML deserialization support.
- Runtime impact is limited to configurations where all three live mode gates are `real`; other configurations must not place or cancel orders.
```

## openspec/changes/add-polymarket-live-trading/design.md

- Source: openspec/changes/add-polymarket-live-trading/design.md
- Lines: 1-78
- SHA256: 1d14f0495688fbad5dc1bd86158454a422bf267ddfb41c52fecc7be0a03a347f

```md
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
```

## openspec/changes/add-polymarket-live-trading/tasks.md

- Source: openspec/changes/add-polymarket-live-trading/tasks.md
- Lines: 1-22
- SHA256: e8b268f7455d7f5b321f366a26acff6ff9827a3558ba60b4836ee6db233f9e0c

```md
## 1. Configuration and gating

- [ ] 1.1 Add typed `config.yaml` models for root transport settings, accounts, and the three retained trade modes, with focused deserialization tests.
- [ ] 1.2 Remove `trade.order_mode` from `config.yaml` and implement the all-three-`real` gate so disabled modes do not parse credentials or create write-side clients.
- [ ] 1.3 Implement exact `type: long` account selection, signature-type validation/defaulting, and sanitized configuration errors with missing/duplicate account tests.

## 2. Official SDK and authenticated CLOB execution

- [ ] 2.1 Replace the third-party CLOB dependency and public order-book adapter with Polymarket's official `polymarket_client_sdk_v2`, retaining focused read-path tests.
- [ ] 2.2 Implement official-SDK signer and authentication-builder initialization without logging credential material.
- [ ] 2.3 Implement `LiveOrderExecutor` typed-decimal limit mapping, GTC signing/posting, strict success/order-ID validation, and confirmed single-order cancellation behind a mockable adapter.
- [ ] 2.4 Add focused tests for side/decimal mapping, failed or malformed placement responses, buy failure, sell failure, confirmed cancellation, and rejected cancellation without live network writes.

## 3. One-shot runtime integration

- [ ] 3.1 Wire the fixed lifecycle once after initial order books, using the first discovered token and the selected long account only when the three-mode gate is enabled.
- [ ] 3.2 Add tests proving non-live modes issue zero authenticated/write calls, empty-token and missing-quote paths fail closed, and WebSocket reconnect logic cannot repeat the live flow.

## 4. Documentation and verification

- [ ] 4.1 Update `ARCHITECTURE.md` to document typed configuration, authenticated executor ownership, one-shot activation, and preserved OddsPortal boundaries.
- [ ] 4.2 Run formatting, focused tests, the full `cargo test` suite, Clippy, OpenSpec strict validation, and a credential/output scan.
```

## openspec/changes/add-polymarket-live-trading/specs/polymarket-live-trading/spec.md

- Source: openspec/changes/add-polymarket-live-trading/specs/polymarket-live-trading/spec.md
- Lines: 1-101
- SHA256: fb47dadb619d45109c5a8c85392d6ba9b314bf106730e53d3a35594720f00dde

[TRUNCATED]

```md
## ADDED Requirements

### Requirement: Three-mode live-trading gate
The system SHALL enable authenticated order placement only when `trade.trader_mode`, `trade.account_mode`, and `trade.market_mode` in `config.yaml` are all exactly `real`. The `trade.order_mode` field SHALL be removed from the supported configuration.

#### Scenario: All modes enable live trading
- **WHEN** all three retained trade modes equal `real`
- **THEN** the system may initialize the authenticated client and run the fixed live flow

#### Scenario: Any non-real mode prevents writes
- **WHEN** at least one retained trade mode is absent or differs from `real`
- **THEN** the system does not parse a signer, derive API credentials, place an order, or cancel an order

### Requirement: Long-account selection
The system SHALL select exactly one configured account whose `type` is `long` and SHALL fail before authentication when no such account or more than one such account exists.

#### Scenario: One long account is selected
- **WHEN** configuration contains exactly one account with `type: long`
- **THEN** that account supplies the private key, signature type, optional funder, CLOB host, and chain ID

#### Scenario: Long account selection is ambiguous
- **WHEN** configuration contains zero or multiple accounts with `type: long`
- **THEN** startup fails without constructing an authenticated client or issuing a write request

### Requirement: Official SDK wallet authentication
The system SHALL use Polymarket's official `polymarket_client_sdk_v2` for public CLOB access and authenticated trading, construct the signer from the selected account's configured private key, map a null signature type to EOA type `0`, accept configured signature types `0` through `3`, apply the optional funder, and authenticate through the SDK before trading.

#### Scenario: Null signature type uses EOA
- **WHEN** the selected long account has `signature_type: null`
- **THEN** the authenticated client is constructed with signature type `0`

#### Scenario: Explicit signature type and funder are forwarded
- **WHEN** the selected long account provides a supported signature type and funder address
- **THEN** both values are passed to the CLOB client used for signing and order creation

#### Scenario: Authentication fails safely
- **WHEN** the private key is invalid or API credential derivation fails
- **THEN** startup fails without placing an order and without including the private key in output

### Requirement: First-token fixed live flow
The system SHALL use the first token returned by event discovery and the initial quote snapshot to run one `0.01 × 5` limit buy followed immediately after accepted placement by one `0.11 × 5` limit sell.

#### Scenario: Fixed flow succeeds
- **WHEN** live trading is enabled, the event has at least one token with an initial quote, and both placements return valid order IDs
- **THEN** the system posts the buy and sell once each and returns both live order IDs

#### Scenario: Event has no token
- **WHEN** live trading is enabled but event discovery returns no token
- **THEN** startup fails before authentication or order placement

#### Scenario: First token has no quote
- **WHEN** live trading is enabled but no initial quote exists for the first token
- **THEN** the lifecycle fails before any order placement

### Requirement: Live limit-order mapping
The live executor SHALL map each validated `LimitOrderIntent` to an official-SDK signed GTC limit order with the same asset ID, side, decimal price, and decimal size. Placement SHALL succeed only when the response reports success and includes a non-empty order ID.

#### Scenario: Buy intent is posted as GTC
- **WHEN** the lifecycle submits the fixed buy intent
- **THEN** the live executor signs and posts one GTC buy with asset ID from the first token, price `0.01`, and size `5`

#### Scenario: Placement response lacks an order ID
- **WHEN** the CLOB accepts the HTTP request but the response has no usable order ID
- **THEN** the placement is treated as failed and no synthetic order ID is generated

#### Scenario: Placement response reports failure
- **WHEN** the CLOB response reports `success: false` even if the HTTP request completed
- **THEN** the placement is treated as failed and the response is not converted into an accepted order ID

### Requirement: Sell failure cancellation
The live executor SHALL preserve the existing fail-closed lifecycle: when the accepted buy has an order ID and the immediate sell fails, it attempts cancellation of that buy exactly once and performs no placement retry.

#### Scenario: Sell fails after accepted buy
- **WHEN** buy placement returns an order ID and sell placement fails
- **THEN** the system calls live cancellation once for the buy order ID and terminates the lifecycle with the sell and cancellation results

#### Scenario: Cancellation response does not confirm the order ID
- **WHEN** cancellation returns successfully at the HTTP layer but the requested buy order ID is absent from `canceled` or present in `not_canceled`
- **THEN** the lifecycle records cancellation as failed

```

Full source: openspec/changes/add-polymarket-live-trading/specs/polymarket-live-trading/spec.md

