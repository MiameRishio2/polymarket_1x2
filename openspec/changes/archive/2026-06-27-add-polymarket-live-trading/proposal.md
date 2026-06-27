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
