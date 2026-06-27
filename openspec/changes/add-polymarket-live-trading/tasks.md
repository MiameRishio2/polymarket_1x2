## 1. Configuration and gating

- [x] 1.1 Add typed `config.yaml` models for root transport settings, accounts, and the three retained trade modes, with focused deserialization tests.
- [x] 1.2 Remove `trade.order_mode` from `config.yaml` and implement the all-three-`real` gate so disabled modes do not parse credentials or create write-side clients.
- [x] 1.3 Implement exact `type: long` account selection, signature-type validation/defaulting, and sanitized configuration errors with missing/duplicate account tests.

## 2. Proxied authenticated CLOB execution

- [x] 2.1 Add direct YAML and signer dependencies while retaining the existing proxied `rs-clob-client-v2` public order-book adapter.
- [x] 2.2 Implement client initialization from configured private key and L2 API credentials, signature type, funder, chain, host, and proxy without calling credential create/derive or logging secrets.
- [x] 2.3 Implement `LiveOrderExecutor` checked fixed-decimal boundary mapping, GTC signing/posting, strict success/order-ID validation, and confirmed single-order cancellation behind a mockable adapter.
- [x] 2.4 Add focused tests for side/decimal mapping, failed or malformed placement responses, buy failure, sell failure, confirmed cancellation, and rejected cancellation without live network writes.

## 3. One-shot runtime integration

- [ ] 3.1 Wire the fixed lifecycle once after initial order books, using the first discovered token and the selected long account only when the three-mode gate is enabled.
- [ ] 3.2 Add tests proving non-live modes issue zero authenticated/write calls, empty-token and missing-quote paths fail closed, and WebSocket reconnect logic cannot repeat the live flow.

## 4. Documentation and verification

- [ ] 4.1 Update `ARCHITECTURE.md` to document typed configuration, authenticated executor ownership, one-shot activation, and preserved OddsPortal boundaries.
- [ ] 4.2 Run formatting, focused tests, the full `cargo test` suite, Clippy, OpenSpec strict validation, and a credential/output scan.
