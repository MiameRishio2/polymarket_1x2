## Context

Both providers already load their targets from `config.yaml`. The target values and exact
localized Polymarket URL check currently name Australia–Egypt.

## Implementation

Replace only the provider target values with Jordan–Argentina. Add a root-config regression test
that reads the committed YAML and proves both runtime providers resolve to the new match while
live trading remains disabled. Update the localized slug fixture to the verified Jordan–Argentina
URL.

The committed proxy placeholder is intentionally unchanged; operators must provide a reachable
proxy in their deployment configuration.

The delta specs update only target-specific acceptance examples in the existing Polymarket and
OddsPortal capabilities; no provider behavior or interface changes.

## Risks

- A wrong team order would select another event or fail discovery. The configuration test asserts
  Jordan as home team and Argentina as away team.
- A wrong Polymarket date or abbreviation would fail Gamma discovery. The slug test asserts
  `fifwc-jor-arg-2026-06-27`.
