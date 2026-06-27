---
comet_change: reorganize-polymarket-layout
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-26-reorganize-polymarket-layout
status: final
---

# Reorganize Polymarket Layout Design

## Context

The repository is a single Rust binary that collects Polymarket 1X2 quote data. Current provider code is flat under `src/`, which is workable for one provider but will blur ownership once OddsPortal code is added.

## Design

Keep the project as one crate and introduce provider-specific module roots:

- `src/polymarket/` owns Polymarket Gamma discovery, CLOB client setup, WebSocket parsing, quote state, quote logging, provider config, and provider models.
- `src/oddsportal/` is reserved for future OddsPortal collection, parsing, and normalization code.
- `src/main.rs` remains the binary entry point and only orchestrates provider-level calls.

No shared provider abstraction is introduced in this change. A shared layer would be premature because only Polymarket behavior exists today.

## Module Layout

```text
src/
├── main.rs
├── polymarket/
│   ├── mod.rs
│   ├── clob.rs
│   ├── config.rs
│   ├── discovery.rs
│   ├── logging.rs
│   ├── models.rs
│   ├── quotes.rs
│   └── ws.rs
└── oddsportal/
    └── mod.rs
```

`src/polymarket/mod.rs` exposes the provider modules. Existing internal references move from `crate::<module>` to `crate::polymarket::<module>` where needed.

## Documentation

`ARCHITECTURE.md` follows the architecture.md-style intent: project identity, structure, components, data flow, external integrations, development workflow, and provider extension rules. It explicitly states that Polymarket and OddsPortal must live in different `src` subdirectories.

`AGENTS.md` gives coding agents concise repo-specific rules: preserve provider boundaries, validate with Cargo, and avoid commits unless requested.

`AGENTS.md` depends on `ARCHITECTURE.md` as the canonical source for project structure and module ownership. Agent guidance can summarize practical rules, but source layout and provider-boundary changes should be made in `ARCHITECTURE.md` first and reflected back into `AGENTS.md`.

## Testing

Run `cargo test` after the refactor. The expected verification surface is compile correctness and existing unit tests for URL parsing, Gamma response parsing, quote state updates, WebSocket message parsing, CLOB order book adaptation, logger behavior, and crypto provider installation.
