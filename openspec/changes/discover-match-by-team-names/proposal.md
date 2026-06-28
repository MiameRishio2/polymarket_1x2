## Why

The two collectors currently use separate target configuration: OddsPortal accepts team names,
while Polymarket requires an exact event URL. Operators should be able to name one football match
once and have both providers discover and monitor the corresponding markets without manually
finding provider-specific URLs.

## What Changes

- Add one shared configured home/away team pair for the monitored football match.
- Discover the matching Polymarket event automatically from the configured team names instead of
  requiring an exact Polymarket event URL.
- Use the same team pair for OddsPortal match discovery and request its 1X2 odds and score
  independently once per second.
- Keep Polymarket on its existing CLOB WebSocket subscription and add its public Sports WebSocket
  for score updates.
- Print four independent JSON record types for Polymarket odds, Polymarket scores, OddsPortal odds,
  and OddsPortal scores without cross-provider aggregation.
- Configure the committed target as South Africa versus Canada with trading still disabled.
- Preserve append-only provider JSONL logging and all live-trading safety gates.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `polymarket-ws-quotes`: Discover the configured football event from team names, emit structured
  Yes-token quote records, and stream the corresponding Polymarket score.
- `oddsportal-js-odds`: Consume the shared team pair and independently collect and emit the
  corresponding 1X2 bookmaker prices and score at a one-second interval.
- `provider-runtime-orchestration`: Supply one validated team pair to both provider collectors
  while preserving provider boundaries and failure isolation.

## Impact

The change affects root configuration conversion and orchestration, Polymarket event discovery,
OddsPortal request discovery, command-line output, focused provider/configuration tests,
`config.yaml`, and synchronized architecture/user documentation. It does not place orders, expose
credentials, correlate provider records, change provider JSONL record formats, or replace a
Polymarket WebSocket with HTTP polling.
