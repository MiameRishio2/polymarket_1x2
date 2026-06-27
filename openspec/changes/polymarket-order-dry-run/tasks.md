## 1. Dry-run contract

- [x] 1.1 Add failing tests for decimal limit-intent validation at the required `0.01` price and `5` size and for invalid price/size inputs.
- [x] 1.2 Add provider-local order intent, side, lifecycle result, and error models under `src/polymarket/order.rs`.

## 2. Quote integration

- [x] 2.1 Add failing tests for immutable latest-quote lookup after CLOB/WebSocket-style state updates and for missing quote state.
- [x] 2.2 Add the minimal read-only `QuoteState` snapshot accessor used by the dry-run.

## 3. Lifecycle simulation

- [x] 3.1 Add failing tests for the New Zealand vs Belgium buy acceptance and `0.11` sell sequence.
- [x] 3.2 Add failing tests proving failure triggers at most one simulated cancellation and no placement retry.
- [x] 3.3 Implement the deterministic local simulator and fail-closed lifecycle orchestration without account or network access.

## 4. Architecture and verification

- [x] 4.1 Update `src/polymarket/mod.rs` and `ARCHITECTURE.md` with the new provider-local, simulation-only module and unchanged read-only boundary.
- [x] 4.2 Run focused tests and the full `cargo test` suite, then record the verification result.
