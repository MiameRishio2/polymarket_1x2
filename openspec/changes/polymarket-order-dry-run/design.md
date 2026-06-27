## Context

The binary is an unauthenticated, read-only collector. It already normalizes initial CLOB snapshots and market WebSocket messages into an in-memory `QuoteState`, but that state has no read-only snapshot API and there is no model for validating an order sequence.

The requested live account selection and exchange writes conflict with the repository's security boundary. This design therefore treats the named match and prices as a deterministic simulation fixture. It must remain impossible for the workflow to load credentials, sign requests, or reach order placement and cancellation endpoints.

## Goals / Non-Goals

**Goals:**

- Represent validated limit-order intents using decimal prices and sizes.
- Drive a deterministic two-step dry-run from a quote snapshot for New Zealand vs Belgium.
- Simulate buy acceptance with a local order ID, followed by a sell at `0.01 + 0.10 = 0.11`.
- Simulate at most one cancellation after failure and never retry placement.
- Reuse the quote state populated by the existing CLOB snapshot and WebSocket code.
- Keep normal collector behavior unchanged.

**Non-Goals:**

- Reading `config.yaml` or selecting an account.
- Private-key, API-key, signature, allowance, balance, or nonce handling.
- Calling live create-order, post-order, cancel-order, or user WebSocket APIs.
- Guaranteeing that the named match exists or that a real order would fill.

## Decisions

### Keep the module provider-local

Add `src/polymarket/order.rs`, exported from `src/polymarket/mod.rs`. Order concepts here are Polymarket-specific, so a top-level `src/order/` module would violate the documented provider boundary.

Alternative considered: add `src/order/`. Rejected because no second provider shares an order abstraction and OddsPortal is explicitly read-only.

### Separate pure planning from an executor interface

Use typed `LimitOrderIntent`, `OrderSide`, and lifecycle result models. A pure planner validates price and size and creates the fixed buy/sell sequence. Lifecycle orchestration depends on an object-safe asynchronous `OrderExecutor` interface whose methods return boxed futures. `MockOrderExecutor` returns deterministic synthetic order IDs and records placement and cancellation attempts.

Alternatives considered:

- A synchronous trait is smaller but would require an interface change before supporting network I/O.
- Native `async fn` in a trait avoids boxed futures but is not object-safe for runtime backend substitution.
- `async-trait` is ergonomic but adds a dependency for only two methods.

No live executor is included. Adding one would expand the project into credentialed order placement and requires a separate policy and security change.

### Consume immutable quote snapshots

Add a narrowly scoped `QuoteState` accessor that clones the latest `QuoteRecord` for an asset. The dry-run consumes this immutable record and does not own WebSocket transport or mutate the quote book.

Alternative considered: subscribe independently from the dry-run module. Rejected because it would duplicate the existing WebSocket and order-book pipeline.

### Encode fail-closed lifecycle sequencing

The sell step is created only after the simulator returns an accepted buy order ID. Any failure transitions directly to one cancellation attempt when an order ID exists, then terminates. There is no retry loop.

The sell price is computed with `rust_decimal` as `0.01 + 0.10`, then normalized as `0.11`; floating-point arithmetic is not used.

### Keep runtime activation explicit

Normal binary startup must not run the dry-run automatically. Tests exercise the scenario through module APIs. Any future CLI exposure requires a separate approved change and must remain simulation-only.

### Preserve test coverage

Rust changes follow repository validation rules. Focused tests cover the interface contract, call ordering, decimal values, quote gating, single cancellation, and absence of retries; the full `cargo test` suite remains required.

## Risks / Trade-offs

- [Synthetic acceptance can diverge from exchange behavior] → Name all outputs as simulated and keep network calls outside the module.
- [Quote snapshots can be absent or stale] → Return a typed failure before creating any simulated order.
- [Price or size precision can be invalid] → Parse and validate with `rust_decimal`, including the `(0, 1)` price range and positive size.
- [A future edit could accidentally attach live APIs] → Keep credential and write-side types absent; tests assert that the workflow depends only on local quote records.
- [The fixture match name can drift from current markets] → Treat it as test data, not live discovery evidence.

## Migration Plan

1. Add failing unit tests for intent validation and lifecycle state transitions.
2. Add the provider-local dry-run models and simulator.
3. Add the immutable quote snapshot accessor and integration-focused test.
4. Document the new module and read-only boundary in `ARCHITECTURE.md`.
5. Run `cargo test`.

Rollback is deletion of the new module/accessor/tests and the corresponding documentation entries; no persisted or remote state is created.

## Open Questions

None for the simulation-only scope. Live trading would require a separate repository policy decision and a new security design.
