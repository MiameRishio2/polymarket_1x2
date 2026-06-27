---
comet_change: remove-live-trading-prohibition
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-27-remove-live-trading-prohibition
status: final
---

# Live-Trading Policy Design

## Goal

Remove repository-level rules that categorically prohibit Polymarket live-trading work, without claiming that the current binary already supports authenticated execution.

## Documentation Boundaries

`AGENTS.md` permits credentials, private-key handling, signing, and order placement only when an explicit task requires them. It also requires secrets to remain outside source control, logs, fixtures, and test output, and keeps authenticated trading concerns under `src/polymarket/`.

`ARCHITECTURE.md` continues to describe the current collector and mock order executor as unauthenticated and read-only. It separately states that future Polymarket work may add authenticated execution paths with explicit credential, signing, placement, cancellation, validation, and provider boundaries.

The OddsPortal provider remains read-only and unauthenticated.

## Alternatives Rejected

- Removing all read-only and simulation descriptions would make the architecture document inaccurate.
- Allowing live trading without secret-handling and validation constraints would remove necessary safety boundaries.
- Implementing live trading in this change would mix a documentation-policy update with a new runtime capability; that work belongs in a separate OpenSpec change.

## Verification

- Confirm `AGENTS.md` and `ARCHITECTURE.md` consistently permit explicitly requested Polymarket live-trading work.
- Confirm descriptions of current runtime behavior remain accurate.
- Confirm no Rust source, API, dependency, or runtime behavior changes.
- Run whitespace checks and the project build/test commands required by the Comet guards.
