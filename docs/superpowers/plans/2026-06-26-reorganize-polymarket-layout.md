---
change: reorganize-polymarket-layout
design-doc: docs/superpowers/specs/2026-06-26-reorganize-polymarket-layout-design.md
archived-with: 2026-06-26-reorganize-polymarket-layout
---

# Reorganize Polymarket Layout Plan

## Implementation

- Add top-level `AGENTS.md` with repository-specific coding-agent instructions.
- Add top-level `ARCHITECTURE.md` documenting source layout, components, data flow, external integrations, and provider boundaries.
- Move flat Polymarket modules into `src/polymarket/`.
- Add `src/polymarket/mod.rs` and update all module paths.
- Add `src/oddsportal/mod.rs` as a documented future provider boundary.
- Run `cargo fmt --check` and `cargo test`.

## Verification

- `cargo fmt --check`
- `cargo test`
