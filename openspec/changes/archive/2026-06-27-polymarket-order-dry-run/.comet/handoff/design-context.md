# Comet Design Handoff

- Change: polymarket-order-dry-run
- Phase: design
- Mode: compact
- Context hash: 0aca54902b24c97e896199b53e76d0b0bb75702bab705d9a9a1e353e8dacb0e6

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/polymarket-order-dry-run/proposal.md

- Source: openspec/changes/polymarket-order-dry-run/proposal.md
- Lines: 1-31
- SHA256: 84d95f26746fa844573761661e5c582c2127924e423de81b7035733b02b41acd

```md
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
```

## openspec/changes/polymarket-order-dry-run/design.md

- Source: openspec/changes/polymarket-order-dry-run/design.md
- Lines: 1-85
- SHA256: aad677a0edc4df86e6f0bcb9807ddd394c56379a2c27181485cfc3d6b0235953

[TRUNCATED]

```md
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

```

Full source: openspec/changes/polymarket-order-dry-run/design.md

## openspec/changes/polymarket-order-dry-run/tasks.md

- Source: openspec/changes/polymarket-order-dry-run/tasks.md
- Lines: 1-20
- SHA256: 1f7f6b12c15a8d7fc31f2eeddf19942bc475db7a94df4c4bad55dc1ad29f7795

```md
## 1. Dry-run contract

- [ ] 1.1 Add failing tests for decimal limit-intent validation at the required `0.01` price and `5` size and for invalid price/size inputs.
- [ ] 1.2 Add provider-local order intent, side, lifecycle result, and error models under `src/polymarket/order.rs`.

## 2. Quote integration

- [ ] 2.1 Add failing tests for immutable latest-quote lookup after CLOB/WebSocket-style state updates and for missing quote state.
- [ ] 2.2 Add the minimal read-only `QuoteState` snapshot accessor used by the dry-run.

## 3. Lifecycle simulation

- [ ] 3.1 Add failing tests for the New Zealand vs Belgium buy acceptance and `0.11` sell sequence.
- [ ] 3.2 Add failing tests proving failure triggers at most one simulated cancellation and no placement retry.
- [ ] 3.3 Implement the deterministic local simulator and fail-closed lifecycle orchestration without account or network access.

## 4. Architecture and verification

- [ ] 4.1 Update `src/polymarket/mod.rs` and `ARCHITECTURE.md` with the new provider-local, simulation-only module and unchanged read-only boundary.
- [ ] 4.2 Run focused tests and the full `cargo test` suite, then record the verification result.
```

## openspec/changes/polymarket-order-dry-run/specs/polymarket-order-dry-run/spec.md

- Source: openspec/changes/polymarket-order-dry-run/specs/polymarket-order-dry-run/spec.md
- Lines: 1-63
- SHA256: 89bde8a9ebfc4e7d4315960250bc3020a13663bb505feaa13f7a1dc86a882f4b

```md
## ADDED Requirements

### Requirement: Validated dry-run limit intents
The system SHALL represent dry-run limit-order intents with a Polymarket asset ID, side, decimal price strictly between zero and one, and positive decimal size.

#### Scenario: Valid initial buy
- **WHEN** the dry-run builds a buy intent at price `0.01` and size `5`
- **THEN** it accepts the intent without floating-point conversion

#### Scenario: Invalid intent
- **WHEN** the dry-run receives a non-positive size or a price outside the open interval from zero to one
- **THEN** it rejects the intent before generating a simulated order ID

### Requirement: Deterministic New Zealand versus Belgium sequence
The system SHALL model the New Zealand vs Belgium dry-run as a buy at price `0.01` and size `5`, followed only after simulated buy acceptance by a sell for the same asset and size at price `0.11`.

#### Scenario: Accepted buy advances to sell
- **WHEN** the simulator accepts the initial buy and returns a simulated order ID
- **THEN** the workflow records that order ID and submits exactly one simulated sell intent at price `0.11` and size `5`

#### Scenario: Buy is not accepted
- **WHEN** the simulator rejects the initial buy without returning an order ID
- **THEN** the workflow terminates without creating a sell intent or retrying the buy

### Requirement: Fail-closed cancellation
The system SHALL attempt simulated cancellation at most once after a lifecycle failure when an accepted simulated order ID is available, and SHALL never retry placement.

#### Scenario: Sell fails after buy acceptance
- **WHEN** the initial buy has a simulated order ID and the sell step fails
- **THEN** the workflow attempts one simulated cancellation for the accepted buy order ID and terminates

#### Scenario: Cancellation fails
- **WHEN** the single simulated cancellation attempt fails
- **THEN** the workflow reports both failures and performs no additional placement or cancellation attempt

### Requirement: No live trading capability
The dry-run SHALL operate without loading accounts, credentials, private keys, signatures, balances, allowances, or write-side exchange clients.

#### Scenario: Running the dry-run
- **WHEN** the dry-run scenario is executed in tests
- **THEN** it uses only local quote data and simulated responses and sends no create-order or cancel-order network request

### Requirement: Abstract order executor
The lifecycle orchestration SHALL depend on an asynchronous order executor interface for limit placement and cancellation, and the change SHALL provide only a configurable mock implementation.

#### Scenario: Successful mock placement
- **WHEN** the mock executor is configured to accept a limit intent
- **THEN** it returns a synthetic order ID through the same executor interface used by lifecycle orchestration

#### Scenario: Executor call audit
- **WHEN** the mock workflow completes or fails
- **THEN** tests can inspect the exact placement and cancellation call order without any network activity

### Requirement: Quote-gated simulation
The system SHALL require a latest quote snapshot for the selected asset before starting the simulated lifecycle.

#### Scenario: Quote is available
- **WHEN** the existing CLOB snapshot or market WebSocket pipeline has populated a quote for the selected asset
- **THEN** the dry-run may validate and execute the local simulation

#### Scenario: Quote is unavailable
- **WHEN** no quote snapshot exists for the selected asset
- **THEN** the dry-run fails before generating a simulated order ID
```

## openspec/changes/polymarket-order-dry-run/specs/polymarket-ws-quotes/spec.md

- Source: openspec/changes/polymarket-order-dry-run/specs/polymarket-ws-quotes/spec.md
- Lines: 1-12
- SHA256: 61be870138398f31b3e626f5346d350316d3818e1ae093756dac27708f61f711

```md
## ADDED Requirements

### Requirement: Read-only latest quote access
The system SHALL expose an immutable latest quote snapshot for a known Polymarket asset so provider-local dry-run logic can consume the same state populated by initial CLOB order books and market WebSocket updates.

#### Scenario: Known asset has quote state
- **WHEN** an initial order book or WebSocket update has populated the selected asset
- **THEN** the caller receives a cloned latest quote record without mutating subscription or quote state

#### Scenario: Asset has no quote state
- **WHEN** the selected asset has not received an initial order book or WebSocket update
- **THEN** the caller receives no quote snapshot
```

