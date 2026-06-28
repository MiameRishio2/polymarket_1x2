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

- `polymarket-ws-quotes`: Update the configured localized-event acceptance scenario to the
  Jordan–Argentina URL and slug.
- `oddsportal-js-odds`: Update the configured match acceptance scenario to Jordan–Argentina.

## Impact

The change is limited to `config.yaml` and focused Rust tests. It does not alter provider
interfaces, parsing, concurrency, logging, trading gates, or JSONL formats.
