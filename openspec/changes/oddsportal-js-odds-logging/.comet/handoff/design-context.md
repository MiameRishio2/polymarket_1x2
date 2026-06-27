# Comet Design Handoff

- Change: oddsportal-js-odds-logging
- Phase: design
- Mode: compact
- Context hash: 4a4e122629e391efb0156f30d20cbbc1548bb8981b1291b941d8038651584b65

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/oddsportal-js-odds-logging/proposal.md

- Source: openspec/changes/oddsportal-js-odds-logging/proposal.md
- Lines: 1-28
- SHA256: 2db3a0b60f1f89191ce37241bc2fcb04812a242122f78c905e4b88b16abed16e

```md
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
```

## openspec/changes/oddsportal-js-odds-logging/design.md

- Source: openspec/changes/oddsportal-js-odds-logging/design.md
- Lines: 1-103
- SHA256: 889a99185baddb50dfe04627334217a7e8e0767352c2eb34ca3a274c6d42e8b6

[TRUNCATED]

```md
## Context

The binary currently discovers one configured Polymarket event, loads initial CLOB quotes, subscribes to market WebSocket updates, and appends quote records to `logs/polymarket_quotes.log`. `src/oddsportal/` exists only as a provider boundary.

For the reference OddsPortal page, the tournament HTML exposes Norway - France as an event row URL containing `#bsJSJ30L`. The H2H page for France/Norway embeds JavaScript component data with `requestPreMatch.url` shaped like `/match-event/1-1-bsJSJ30L-1-2-<bookmaker-hash>.dat?_=`, plus base paths for match event and odds history requests. The `.dat` response is not JSON; it is base64 data decoded by `JXG.decompress` from `/js/lscompressor.min.js`, which inflates zipped data and URL-decodes the result. The Vue event bundle then reads `d.oddsdata`, provider metadata, betting type, and scope fields.

## Goals / Non-Goals

**Goals:**

- Collect OddsPortal 1X2 bookmaker odds without scraping rendered DOM table text.
- Derive the target event and internal odds request from page-embedded JavaScript/HTML state.
- Decode the internal `.dat` response and normalize bookmaker odds for outcomes `1`, `X`, and `2`.
- Log OddsPortal odds records in append-only JSONL format while keeping Polymarket logging unchanged.
- Keep all OddsPortal-specific code under `src/oddsportal/`.

**Non-Goals:**

- Do not place bets, authenticate, manage user accounts, or bypass logged-in-only data.
- Do not implement every OddsPortal market type; this change targets pre-match football 1X2 odds.
- Do not create a shared provider abstraction unless implementation proves both providers need it.
- Do not replace the existing Polymarket WebSocket collector.

## Decisions

### Approach Options

1. Parse rendered match table HTML.
   - Pros: quick if the server returns visible odds rows.
   - Cons: brittle, tightly coupled to CSS/DOM rendering, and does not satisfy the requirement to avoid direct webpage scraping.

2. Launch a browser and read hydrated JavaScript state.
   - Pros: follows the same path as the website and can handle dynamic behavior.
   - Cons: heavy dependency for a CLI collector, harder to test, and unnecessary because the needed request metadata is present in HTML.

3. Fetch HTML as text, extract embedded JavaScript state, call the internal `.dat` endpoint, and decode the response in Rust.
   - Pros: read-only, lightweight, testable with fixtures, and aligned with the user's requested data source.
   - Cons: depends on OddsPortal's private response shape and compression scheme.

Use option 3.

### Provider Structure

`src/oddsportal/` will own configuration, page discovery, internal request construction, `.dat` decoding, odds parsing, and JSONL logging. `src/main.rs` remains the orchestration layer and should only call provider-level entry points.

### Data Flow

```text
OddsPortal config
    |
    v
Tournament page fetch
    |
    v
Embedded state extraction
    |
    v
Target match event URL/hash discovery
    |
    v
H2H page fetch
    |
    v
requestPreMatch URL extraction
    |
    v
Compressed .dat fetch and JXG-compatible decode
    |
    v
1X2 odds normalization
    |
    v
append JSONL records
```

### Logging

OddsPortal should write structured records that identify the provider, event, bookmaker, outcome, decimal odds, and source request. Polymarket logging should keep its existing records and path unless implementation exposes a clear need to add a separate OddsPortal log path. If both providers write to one file, records must be distinguishable by provider.

### Decoding
```

Full source: openspec/changes/oddsportal-js-odds-logging/design.md

## openspec/changes/oddsportal-js-odds-logging/tasks.md

- Source: openspec/changes/oddsportal-js-odds-logging/tasks.md
- Lines: 1-24
- SHA256: 12193809f95cf110e79679e27b545bb44b3a98d799586c376755b39a29abb2e0

```md
## 1. OddsPortal Fixtures And Decoding

- [ ] 1.1 Add captured fixture coverage for the Norway - France tournament/H2H embedded state and compressed `.dat` response shape.
- [ ] 1.2 Implement a JXG-compatible `.dat` decoder with unit tests for base64, inflate, URL-decode, and JSON parse behavior.

## 2. OddsPortal Provider

- [ ] 2.1 Add OddsPortal config and data models under `src/oddsportal/`.
- [ ] 2.2 Implement tournament page match discovery from embedded state.
- [ ] 2.3 Implement H2H page request metadata extraction and internal pre-match `.dat` URL construction.
- [ ] 2.4 Implement 1X2 odds extraction and normalization from decoded OddsPortal data.
- [ ] 2.5 Implement append-only OddsPortal JSONL logging.

## 3. Binary Orchestration

- [ ] 3.1 Wire OddsPortal collection into `src/main.rs` while preserving existing Polymarket behavior.
- [ ] 3.2 Ensure provider failures include provider-specific context in errors or logs.
- [ ] 3.3 Update `ARCHITECTURE.md` if the runtime data flow or provider responsibilities change.

## 4. Verification

- [ ] 4.1 Run focused OddsPortal parser/decoder tests.
- [ ] 4.2 Run `cargo test`.
- [ ] 4.3 Perform a network smoke test when OddsPortal is reachable.
```

## openspec/changes/oddsportal-js-odds-logging/specs/oddsportal-js-odds/spec.md

- Source: openspec/changes/oddsportal-js-odds-logging/specs/oddsportal-js-odds/spec.md
- Lines: 1-36
- SHA256: f9fcc048631c30a646a468a890652c8d42966b4bbb67667b4522a6567282c529

```md
## ADDED Requirements

### Requirement: OddsPortal match discovery from embedded state
The system SHALL discover a configured OddsPortal football match from page-embedded JavaScript or HTML state rather than from rendered DOM table text.

#### Scenario: Discover Norway France event hash
- **WHEN** the configured tournament URL is `https://www.oddsportal.com/football/world/world-championship-2026/` and the configured teams are Norway and France
- **THEN** the system discovers the OddsPortal H2H URL and encoded event ID for the Norway - France match from embedded page state

### Requirement: Internal pre-match odds request
The system SHALL derive the OddsPortal pre-match 1X2 odds request from the H2H page's embedded request metadata.

#### Scenario: Build request from H2H page metadata
- **WHEN** the H2H page exposes a `requestPreMatch.url` value for the target event
- **THEN** the system fetches that internal `.dat` URL for odds data instead of scraping visible odds rows

### Requirement: Compressed odds response decoding
The system SHALL decode OddsPortal compressed `.dat` responses into structured data before odds normalization.

#### Scenario: Decode compressed match event response
- **WHEN** the internal `.dat` response body is base64 encoded, compressed, and URL encoded according to OddsPortal's JavaScript decoder
- **THEN** the system produces parseable JSON odds data or returns a contextual decoding error

### Requirement: 1X2 bookmaker odds normalization
The system SHALL normalize OddsPortal football 1X2 odds into records that include event identity, bookmaker identity, outcome, decimal odds, and source metadata.

#### Scenario: Normalize bookmaker odds
- **WHEN** decoded OddsPortal data contains bookmaker prices for outcomes `1`, `X`, and `2`
- **THEN** the system emits one normalized record per bookmaker outcome price

### Requirement: OddsPortal append-only logging
The system SHALL append normalized OddsPortal odds records to a local log without requiring credentials or write-side betting permissions.

#### Scenario: Log OddsPortal records
- **WHEN** OddsPortal odds records are normalized
- **THEN** the system writes them as JSON lines that identify `oddsportal` as the provider
```

## openspec/changes/oddsportal-js-odds-logging/specs/polymarket-ws-quotes/spec.md

- Source: openspec/changes/oddsportal-js-odds-logging/specs/polymarket-ws-quotes/spec.md
- Lines: 1-8
- SHA256: c1f94d0b9b9ee36adefcab2b61ffdae2d241ef047077cc273065c2d9cdef34e3

```md
## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket subscription, quote normalization, and append-only logging under `src/polymarket/` while preserving existing Polymarket runtime behavior when OddsPortal logging is added to binary orchestration.

#### Scenario: Running the executable with both providers configured
- **WHEN** the executable starts with the default Polymarket event URL and default OddsPortal match configuration
- **THEN** it uses the Polymarket modules under `src/polymarket/` to perform the same discovery, subscription, and logging workflow as before while also starting OddsPortal odds logging
```
