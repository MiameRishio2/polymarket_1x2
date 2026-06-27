## Why

The collector already receives Polymarket order books but has no safe way to validate an order lifecycle against those quotes. A deterministic dry-run can exercise order construction, acceptance, follow-up selling, and cancellation behavior without introducing credentials, signatures, or write-side exchange access.

## What Changes

- Add a Polymarket-specific dry-run order workflow under `src/polymarket/`.
- Reuse the existing market WebSocket quote state as the source of order-book data.
- Model the New Zealand vs Belgium scenario with an initial limit buy at price `0.01` and size `5`.
- After a simulated successful acceptance returns an order ID, model a limit sell at price `0.11` and size `5`.
- On a simulated failure, issue at most one simulated cancellation and never retry order placement.
- Add focused unit tests for validation, successful sequencing, and failure cancellation.
- Update `ARCHITECTURE.md` to document the dry-run module and preserve the read-only boundary.
- Do not read `config.yaml` accounts, handle credentials or private keys, sign orders, or call placement/cancellation endpoints.

## Capabilities

### New Capabilities

- `polymarket-order-dry-run`: Deterministic, read-only simulation of a two-step Polymarket limit-order lifecycle driven by existing quote state.

### Modified Capabilities

- `polymarket-ws-quotes`: Expose the existing in-memory quote state to the dry-run orchestration without changing WebSocket transport behavior.

## Impact

- Affected code: `src/polymarket/`, its focused tests, and binary orchestration only if an explicit dry-run entry point is required.
- Documentation: `ARCHITECTURE.md`.
- Dependencies: reuse the existing Rust, decimal, CLOB, and WebSocket dependencies; no authentication or signing dependency is added.
- Runtime: normal collection remains unchanged and read-only; dry-run produces only local simulated results.
