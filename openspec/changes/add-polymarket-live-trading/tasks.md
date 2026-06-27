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
