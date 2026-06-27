# Comet Design Handoff

- Change: add-polymarket-live-trading
- Phase: design
- Mode: compact
- Context hash: e4bbe82da9994eb6d313edaac08e58fc5f43ed6d467144c66e94ddc74366b7d9

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/add-polymarket-live-trading/proposal.md

- Source: openspec/changes/add-polymarket-live-trading/proposal.md
- Lines: 1-30
- SHA256: 199e476b925071e8801dbe47f39ef755cba775e86ad1bca647ee05c17f99667a

```md
## Why

The repository currently models the fixed Polymarket buy/sell lifecycle only through a mock executor, so it cannot exercise the same flow against the live CLOB. A deliberately gated test path is needed to place the existing `0.01 × 5` buy followed by the `0.11 × 5` sell with a configured test account.

## What Changes

- Load Polymarket account, signature, funder, endpoint, proxy, and trade-mode settings from the tracked `config.yaml`.
- Keep the existing `rs-clob-client-v2` integration so authenticated HTTP requests continue to use `config.yaml.proxy`, and add a live `OrderExecutor` with configured L2 credentials, wallet signing, strict response validation, and single-order cancellation.
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
- Dependencies add YAML deserialization and the signer type required to construct the existing proxied CLOB client directly.
- Runtime impact is limited to configurations where all three live mode gates are `real`; other configurations must not place or cancel orders.
```

## openspec/changes/add-polymarket-live-trading/design.md

- Source: openspec/changes/add-polymarket-live-trading/design.md
- Lines: 1-80
- SHA256: 5b00105b01ee83553a977c3a53e5f4fde182bbfb767dedfff6b0bc3110a28622

```md
## Context

The binary currently constructs an unauthenticated `rs-clob-client-v2` client from hard-coded Polymarket defaults, loads initial books, and then enters the market WebSocket loop. `src/polymarket/order.rs` already expresses the required `0.01 × 5` buy, immediate `0.11 × 5` sell, and single-cancellation failure path through an executor trait, but only `MockOrderExecutor` implements that trait.

The tracked `config.yaml` already contains root endpoint/proxy settings, multiple account entries, and trade-mode fields. The live test path must use that file directly, choose the `type: long` account, and never expose its private key in logs or errors.

## Goals / Non-Goals

**Goals:**

- Parse the existing YAML configuration into typed Polymarket runtime settings.
- Require `trader_mode`, `account_mode`, and `market_mode` to all equal `real` before any authenticated client or write request is created.
- Reuse `rs-clob-client-v2` for authenticated signing and proxy-routed trading, using the selected long account's private key, L2 API credentials, signature type, funder, host, and chain.
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

### Proxied authenticated client with configured L2 credentials

Retain `rs-clob-client-v2` because its constructor accepts the configured proxy URL used by this environment. Parse the configured private key into its Alloy signer, construct `ApiKeyCreds` from the selected account's `api_key`, `api_secret`, and `api_passphrase`, and pass wallet, credentials, signature type, funder, host, chain, and proxy directly to `ClobClient::new`.

Do not call `create_api_key` or `create_or_derive_api_key`: version `0.2.2` prints L1 authentication headers on the credential-creation path. Configuration and initialization errors are wrapped with stage-only context and must not include private keys, API credentials, or signed requests.

Alternatives rejected: the official SDK does not expose programmatic HTTP proxy injection; forking it expands scope. Raw REST would duplicate signing and authentication logic. Calling the existing SDK's credential-creation path violates the no-auth-header logging requirement.

### Live executor behind the existing trait

Add a `LiveOrderExecutor` that maps `LimitOrderIntent` to `rs-clob-client-v2::UserLimitOrder`, converts only the fixed validated decimal values at the SDK boundary, creates and signs a GTC order, posts it, and implements single-order cancellation. Placement succeeds only when the JSON response reports success and a non-empty `orderID`. Cancellation succeeds only when the response's `canceled` collection contains the requested ID; entries in `not_canceled` are failures. The fixed lifecycle remains in `run_new_zealand_belgium_flow`, preserving its current accepted-buy/immediate-sell semantics and at-most-one cancellation.

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
- SHA256: 6f7ab8f3d78b453f93d5e7c04bd430d1ec540dedb3a5c1baa6cfe7bddbfef62a

```md
## 1. Configuration and gating

- [ ] 1.1 Add typed `config.yaml` models for root transport settings, accounts, and the three retained trade modes, with focused deserialization tests.
- [ ] 1.2 Remove `trade.order_mode` from `config.yaml` and implement the all-three-`real` gate so disabled modes do not parse credentials or create write-side clients.
- [ ] 1.3 Implement exact `type: long` account selection, signature-type validation/defaulting, and sanitized configuration errors with missing/duplicate account tests.

## 2. Proxied authenticated CLOB execution

- [ ] 2.1 Add direct YAML and signer dependencies while retaining the existing proxied `rs-clob-client-v2` public order-book adapter.
- [ ] 2.2 Implement client initialization from configured private key and L2 API credentials, signature type, funder, chain, host, and proxy without calling credential create/derive or logging secrets.
- [ ] 2.3 Implement `LiveOrderExecutor` exact fixed-decimal boundary mapping, GTC signing/posting, strict success/order-ID validation, and confirmed single-order cancellation behind a mockable adapter.
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
- Lines: 1-109
- SHA256: 6cbbb3d11d54dbb843b7c004eb0bd19206f63db74c8f090bb284f88ec22c2126

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

### Requirement: Configured proxied wallet authentication
The system SHALL use `rs-clob-client-v2` with `config.yaml.proxy`, construct the signer from the selected account's configured private key, load `api_key`, `api_secret`, and `api_passphrase` from that account, map a null signature type to EOA type `0`, accept configured signature types `0` through `3`, and apply the optional funder. It MUST NOT call the SDK's API-key creation or derivation methods.

#### Scenario: Null signature type uses EOA
- **WHEN** the selected long account has `signature_type: null`
- **THEN** the authenticated client is constructed with signature type `0`

#### Scenario: Explicit signature type and funder are forwarded
- **WHEN** the selected long account provides a supported signature type and funder address
- **THEN** both values are passed to the CLOB client used for signing and order creation

#### Scenario: Configured L2 credentials are forwarded
- **WHEN** the selected long account provides non-empty API key, secret, and passphrase values
- **THEN** those values initialize the L2-authenticated client without invoking credential creation or derivation

#### Scenario: Authentication configuration fails safely
- **WHEN** the private key is invalid or any configured L2 credential is missing
- **THEN** startup fails without placing an order and without including the private key in output

#### Scenario: Configured proxy is used
- **WHEN** the authenticated CLOB client is constructed
- **THEN** its HTTP transport uses the root `config.yaml.proxy` value

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
The live executor SHALL map each validated `LimitOrderIntent` to an `rs-clob-client-v2` signed GTC limit order with the same asset ID and side and an exact boundary conversion of the validated fixed decimal price and size. Placement SHALL succeed only when the response reports success and includes a non-empty order ID.

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

```

Full source: openspec/changes/add-polymarket-live-trading/specs/polymarket-live-trading/spec.md

