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
