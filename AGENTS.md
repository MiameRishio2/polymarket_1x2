# Agent Instructions

## Project Scope

This repository is a Rust binary for collecting Polymarket 1X2 market quotes. Keep changes focused on the requested behavior and avoid unrelated refactors.

## Architecture Dependency

`AGENTS.md` depends on `ARCHITECTURE.md` for the canonical project structure and module ownership rules. Before changing source layout, provider boundaries, or agent guidance, read `ARCHITECTURE.md` first and keep this file consistent with it.

## Source Layout

- Polymarket-specific code belongs under `src/polymarket/`.
- OddsPortal-specific code belongs under `src/oddsportal/`.
- Add shared modules only when at least two provider implementations need the same abstraction.
- Keep `src/main.rs` as the binary orchestration layer.

## Validation

- Run `cargo test` after Rust code changes.
- Prefer focused tests near the code being changed.
- Do not commit changes unless the user explicitly asks.

## Change Discipline

- Preserve existing runtime behavior unless the task explicitly asks for a behavior change.
- Do not introduce credentials, private-key handling, or order-placement behavior.
- Keep documentation synchronized with source layout changes.
