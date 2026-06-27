# Comet Design Handoff

- Change: reorganize-polymarket-layout
- Phase: design
- Mode: compact
- Context hash: 8951ff54555c3c06d32018cc3be49184102e1da7629f24b2f7f7626a0590465c

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/reorganize-polymarket-layout/proposal.md

- Source: openspec/changes/reorganize-polymarket-layout/proposal.md
- Lines: 1-25
- SHA256: 016605c34795620c08139c4e5eb23bdf5e9703fcd857e575ec7c9b8c201c7967

```md
## Why

The current Rust code keeps all Polymarket modules flat under `src/`, which makes it harder to add an OddsPortal integration without mixing provider-specific code. The repository also lacks top-level agent and architecture guidance for future contributors.

## What Changes

- Add `AGENTS.md` with project working rules for coding agents.
- Add `ARCHITECTURE.md` using the architecture.md reference style: project identity, structure, components, data flow, external integrations, development workflow, and extension points.
- Move existing Polymarket-specific code into `src/polymarket/`.
- Add a placeholder `src/oddsportal/` module boundary for future OddsPortal-specific code.
- Keep runtime behavior, dependencies, and public command behavior unchanged.

## Capabilities

### New Capabilities
- `provider-source-layout`: Documents and enforces separate source-tree boundaries for market data providers.

### Modified Capabilities
- `polymarket-ws-quotes`: Implementation moves under `src/polymarket/` without changing quote discovery, subscription, or logging requirements.

## Impact

- Affected files: `src/main.rs`, existing Polymarket modules, new provider module directories, `AGENTS.md`, `ARCHITECTURE.md`, and OpenSpec change artifacts.
- No new runtime dependencies.
- No expected behavior changes for quote discovery, CLOB order book loading, WebSocket parsing, or append-only logging.
```

## openspec/changes/reorganize-polymarket-layout/design.md

- Source: openspec/changes/reorganize-polymarket-layout/design.md
- Lines: 1-35
- SHA256: 974780bbab69127e90651035d15c0ebb355928085d3beacf7dd5518228fa6273

```md
## Overview

The change introduces source-level provider boundaries while preserving the existing single-binary crate. Polymarket code becomes a cohesive module tree under `src/polymarket/`, and OddsPortal receives a separate `src/oddsportal/` placeholder so future collection code has an explicit home.

## Architecture Decisions

- Keep one Rust crate and one binary target. This is a layout refactor, not a workspace split.
- Use `src/polymarket/mod.rs` as the provider facade. Root `main.rs` should call provider-level functions and avoid importing provider internals directly.
- Keep shared bootstrapping in root-level code only when it is not provider-specific. Today that is limited to installing the Rustls crypto provider.
- Add `src/oddsportal/mod.rs` with module documentation only. It marks the intended boundary but does not add dead runtime behavior.
- Document expected source ownership in `ARCHITECTURE.md`:
  - Polymarket code belongs under `src/polymarket/`.
  - OddsPortal code belongs under `src/oddsportal/`.
  - Shared abstractions may be introduced later only when both providers need them.

## Data Flow

Runtime data flow remains unchanged:

1. Read default configuration.
2. Extract the Polymarket event slug.
3. Fetch Gamma event metadata through the configured proxy.
4. Load initial CLOB order books through `rs-clob-client-v2`.
5. Subscribe to market WebSocket token updates.
6. Normalize updates into `QuoteRecord` values and append JSON lines to `logs/polymarket_quotes.log`.

## Risks

- Moving modules can break `crate::` paths or unit test imports. This is mitigated with `cargo test`.
- Adding an OddsPortal placeholder could imply implemented functionality. The module and architecture docs must be explicit that it is a future boundary only.

## Alternatives Considered

- Split into multiple crates now. Rejected because the current codebase is small and there is no shared provider abstraction yet.
- Keep flat files and rely only on docs. Rejected because the code layout would still encourage mixing provider responsibilities.
```

## openspec/changes/reorganize-polymarket-layout/tasks.md

- Source: openspec/changes/reorganize-polymarket-layout/tasks.md
- Lines: 1-7
- SHA256: 7b5a6bcfdde00a310f339d8429eeaece658255826c66b1ef86abb9542aeae885

```md
## Tasks

- [ ] Create repository guidance documents: `AGENTS.md` and `ARCHITECTURE.md`.
- [ ] Move Polymarket implementation modules under `src/polymarket/`.
- [ ] Add `src/oddsportal/` provider boundary placeholder.
- [ ] Update module paths and tests after the layout refactor.
- [ ] Run Rust verification.
```

## openspec/changes/reorganize-polymarket-layout/specs/polymarket-ws-quotes/spec.md

- Source: openspec/changes/reorganize-polymarket-layout/specs/polymarket-ws-quotes/spec.md
- Lines: 1-8
- SHA256: 3281b3dd0ed83677e0dd55972d9b0457d8600da24ad90aab244d61067c6c437e

```md
## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket subscription, quote normalization, and append-only logging under `src/polymarket/` while preserving existing runtime behavior.

#### Scenario: Running the executable after layout refactor
- **WHEN** the executable starts with the default Polymarket event URL
- **THEN** it uses the Polymarket modules under `src/polymarket/` to perform the same discovery, subscription, and logging workflow as before
```

## openspec/changes/reorganize-polymarket-layout/specs/provider-source-layout/spec.md

- Source: openspec/changes/reorganize-polymarket-layout/specs/provider-source-layout/spec.md
- Lines: 1-26
- SHA256: fb512ee2206cbf99661762e2f89db79ffa3529686283b10b2286102b5106fbd9

```md
## ADDED Requirements

### Requirement: Provider source boundaries
The system SHALL keep provider-specific source code under provider-specific directories below `src/`.

#### Scenario: Polymarket code location
- **WHEN** code implements Polymarket Gamma, CLOB, WebSocket, quote state, or quote logging behavior
- **THEN** it resides under `src/polymarket/`

#### Scenario: OddsPortal code location
- **WHEN** code implements OddsPortal scraping, API collection, parsing, or odds normalization behavior
- **THEN** it resides under `src/oddsportal/`

### Requirement: Architecture documentation
The repository SHALL include top-level architecture documentation that identifies the project purpose, module layout, provider boundaries, data flow, external integrations, and development workflow.

#### Scenario: Reading architecture guidance
- **WHEN** a contributor opens `ARCHITECTURE.md`
- **THEN** they can identify where Polymarket code belongs and where OddsPortal code belongs before adding provider code

### Requirement: Agent guidance
The repository SHALL include top-level coding-agent guidance.

#### Scenario: Agent starts work
- **WHEN** a coding agent starts a repository task
- **THEN** `AGENTS.md` provides concise instructions for validation, source layout, and change discipline
```
