# Comet Design Handoff

- Change: switch-target-to-jordan-argentina
- Phase: design
- Mode: compact
- Context hash: be54eead6fa0481833173119338fb53a4c057d5dcfa5b1889e116b06736ee5a3

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/switch-target-to-jordan-argentina/proposal.md

- Source: openspec/changes/switch-target-to-jordan-argentina/proposal.md
- Lines: 1-29
- SHA256: ebf72a50b4e2eaa24f4f2458cac01113af322fc60a4a379f05b2ce38d0d6d701

```md
## Why

The deployed collectors are configured for Australia–Egypt, but the requested operational target
is now Jordan–Argentina. The runtime configuration and focused target checks must use the same
match so startup diagnostics and collected records are attributable to the intended event.

## What Changes

- Change the configured Polymarket event to
  `https://polymarket.com/ja/sports/world-cup/fifwc-jor-arg-2026-06-27`.
- Change the configured OddsPortal target to Jordan as home team and Argentina as away team.
- Update focused configuration and localized-slug tests for the new target.
- Keep `trade.enabled: false`, provider log paths, polling cadence, and proxy configuration
  unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change selects different values for existing provider-target configuration.

## Impact

The change is limited to `config.yaml` and focused Rust tests. It does not alter provider
interfaces, parsing, concurrency, logging, trading gates, or JSONL formats.
```

## openspec/changes/switch-target-to-jordan-argentina/design.md

- Source: openspec/changes/switch-target-to-jordan-argentina/design.md
- Lines: 1-21
- SHA256: 52c8895cab34c1a395259947bee3c902ca2e6eec1aff3bfd03161e1e0cb66808

```md
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

## Risks

- A wrong team order would select another event or fail discovery. The configuration test asserts
  Jordan as home team and Argentina as away team.
- A wrong Polymarket date or abbreviation would fail Gamma discovery. The slug test asserts
  `fifwc-jor-arg-2026-06-27`.
```

## openspec/changes/switch-target-to-jordan-argentina/tasks.md

- Source: openspec/changes/switch-target-to-jordan-argentina/tasks.md
- Lines: 1-8
- SHA256: 238f49e5a7c73af47fb276e4df28a12052c56551413f61444eb45b8a966bd415

```md
## 1. Target Configuration

- [x] 1.1 Add focused failing checks for the committed Jordan–Argentina provider targets.
- [x] 1.2 Update `config.yaml` and localized slug fixtures, then run Rust validation.

## 2. Runtime Verification

- [x] 2.1 Build and run a bounded trading-disabled smoke test with a reachable temporary proxy, then confirm both provider logs and JSONL output.
```
