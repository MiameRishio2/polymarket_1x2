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
