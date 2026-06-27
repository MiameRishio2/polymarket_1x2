---
comet_change: add-polymarket-live-trading
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-27-add-polymarket-live-trading
status: final
---

# Polymarket Fixed-Flow Live Trading Design

## Objective

Connect the existing executor-independent `0.01 × 5` buy followed by `0.11 × 5` sell lifecycle to Polymarket's live CLOB for a test account. Live execution is explicitly enabled only by three YAML mode gates and runs once per process start.

OpenSpec remains the normative source for behavior and scenarios:

- `openspec/changes/add-polymarket-live-trading/specs/polymarket-live-trading/spec.md`

## Confirmed Product Constraints

- Read the tracked `config.yaml` directly.
- Enable live trading only when `trader_mode`, `account_mode`, and `market_mode` all equal `real`.
- Remove `trade.order_mode`.
- Select exactly one account with `type: long`.
- Use the first token returned by event discovery.
- Treat accepted buy placement as sufficient to attempt the sell immediately; do not wait for a fill.
- If the sell fails, attempt cancellation of the accepted buy once.
- The tracked test-account key risk is explicitly accepted, but application code must not print credential material.

## SDK Decision

Retain `rs-clob-client-v2 0.2.2` for both public and authenticated CLOB access because its constructor supports the required `config.yaml.proxy`.

The selected account also supplies pre-existing L2 credentials:

- `api_key`;
- `api_secret`;
- `api_passphrase`.

Alternatives rejected:

- The official SDK has stronger typed-decimal and authentication-state APIs but does not expose programmatic HTTP proxy injection.
- Forking the official SDK to add proxy injection creates an unnecessary maintenance surface for this fixed test flow.
- Raw REST duplicates EIP-712 signing, L2 authentication, response parsing, and protocol behavior.
- Calling `rs-clob-client-v2::create_or_derive_api_key` is prohibited because its credential-create fallback prints complete L1 authentication headers.

Primary references:

- https://docs.polymarket.com/api-reference/authentication
- https://docs.polymarket.com/trading/orders/create
- https://docs.polymarket.com/trading/orders/cancel
- https://docs.rs/rs-clob-client-v2/0.2.2

## Components

### Typed configuration

Provider-local Serde models load only the fields needed by Polymarket while tolerating unrelated strategy and web sections. The selected long account supplies:

- private key;
- API key, API secret, and API passphrase;
- signature type (`null` defaults to EOA `0`);
- optional funder;
- CLOB host;
- chain ID.

Root fields supply the Gamma host and HTTP proxy. Account selection and mode evaluation are pure functions with table-driven tests.

The gate is evaluated before signer parsing:

```text
trader_mode == real
AND account_mode == real
AND market_mode == real
```

If false, the runtime follows the current read-only collection path and never touches credential fields.

### Client factory

The factory retains the current unauthenticated client for public order-book access. For the live path it:

1. validates non-empty configured L2 API credentials;
2. parses the selected private key into an Alloy local signer;
3. maps chain ID to Polygon or Amoy;
4. passes signature type `0..=3`, optional funder, host, root proxy, wallet, and `ApiKeyCreds` to `ClobClient::new`;
5. returns the authenticated client without calling credential creation/derivation or logging configuration, signer, or headers.

Error context identifies only the stage, such as `invalid long-account signer` or `CLOB authentication failed`.

### Live executor adapter

`LiveOrderExecutor` implements the existing `OrderExecutor` trait. A smaller internal adapter trait isolates the CLOB client for deterministic tests.

Placement:

1. map `OrderSide` to the SDK side;
2. convert only the validated fixed `rust_decimal::Decimal` values (`0.01`, `0.11`, and `5`) to the SDK's `f64` inputs and reject failed conversion;
3. call `create_and_post_limit_order` with GTC;
4. require the response to report success;
5. require a non-empty order ID.

Cancellation:

1. call the single-order cancellation endpoint;
2. require the requested ID in `canceled`;
3. treat absence from `canceled` or presence in `not_canceled` as failure.

No executor method retries.

### One-shot orchestration

`run_market_stream` retains ownership of initial order-book loading and the mutable quote state. Immediately after initial snapshots:

1. if live gating is disabled, continue directly to the WebSocket loop;
2. if enabled, require the first discovered token;
3. create the authenticated executor from the long account;
4. invoke the existing fixed lifecycle once;
5. log only market slug, outcome, asset ID, order IDs, and stage result;
6. propagate lifecycle failure and stop startup.

The invocation is outside the reconnect loop, so disconnects cannot duplicate orders.

## Data Flow

```text
config.yaml
    |
    v
three-mode gate ---- disabled ----> read-only collection
    |
  enabled
    v
unique type: long account
    |
    v
signer + signature type + funder
    |
    v
proxied authenticated CLOB client
    |
    v
first discovered token + initial quote
    |
    v
GTC buy 0.01 × 5
    |
    +---- failure --------------------> stop
    |
 accepted order ID
    v
GTC sell 0.11 × 5
    |
    +---- failure ----> cancel buy once ----> stop
    |
 accepted order ID
    v
continue market WebSocket collection
```

## Safety and Failure Semantics

- Missing or duplicate long accounts fail before signer construction.
- Missing tokens fail before authentication.
- Missing initial quote fails before order placement.
- HTTP success is insufficient: placement requires response success plus order ID.
- HTTP success is insufficient for cancellation: the requested ID must be confirmed in `canceled`.
- No private key, API secret, passphrase, signed order body, or authentication header is logged.
- Automated tests never construct a real network-writing client.
- Repeating the process can repeat orders; this accepted test limitation is documented at startup.

## Test Strategy

1. Configuration unit tests cover YAML parsing, removal of `order_mode`, every gate combination, long-account ambiguity, signature mapping, and private-key redaction.
2. Client-factory tests cover pure mapping and invalid local configuration without network access.
3. Adapter tests use fake placement/cancellation responses to cover success flags, empty IDs, `canceled`, and `not_canceled`.
4. Existing lifecycle tests remain the contract for buy/sell/cancel call order.
5. Orchestration tests inject a fake executor factory and prove zero calls when disabled and one call when enabled.
6. Full verification runs formatting, focused tests, `cargo test`, Clippy, strict OpenSpec validation, and scans diffs/output for credential material.

## Rollback

Operational rollback is immediate: set any of the three mode fields away from `real`. Code rollback removes the one-shot invocation and live adapter while the existing proxied SDK continues serving public order-book reads.
