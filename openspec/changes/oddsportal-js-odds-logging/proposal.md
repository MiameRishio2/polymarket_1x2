## Why

OddsPortal odds are currently not collected by the binary, and reading rendered table cells from the page would be brittle. The World Championship 2026 page and the Norway - France H2H page expose the match event ID and internal `.dat` odds request through embedded JavaScript state, which gives us a more direct source for bookmaker odds.

## What Changes

- Add an OddsPortal provider implementation under `src/oddsportal/`.
- Discover a configured football match from the tournament page by reading embedded JavaScript/HTML state, using Norway - France on the World Championship 2026 page as the reference case.
- Fetch OddsPortal pre-match 1X2 odds through the internal `/match-event/...dat` request derived from page state, then decode the compressed response.
- Normalize bookmaker 1X2 odds into append-only JSON log records.
- Update binary orchestration so Polymarket quote logging and OddsPortal odds logging both run and write records.
- Preserve read-only behavior; do not add credentials, private-key handling, betting, or order placement.

## Capabilities

### New Capabilities

- `oddsportal-js-odds`: Collects OddsPortal football 1X2 odds from embedded JavaScript state and internal odds data responses.

### Modified Capabilities

- `polymarket-ws-quotes`: The binary orchestration changes from Polymarket-only logging to running Polymarket and OddsPortal logging in the same process while preserving existing Polymarket behavior.

## Impact

- Affected files: `src/main.rs`, new OddsPortal modules under `src/oddsportal/`, `Cargo.toml`, provider tests, and architecture documentation if the runtime data flow changes.
- Adds HTTP fetching, embedded state parsing, compressed `.dat` response decoding, odds normalization, and JSONL logging for OddsPortal.
- No authentication, write-side exchange behavior, order placement, or credential storage.
