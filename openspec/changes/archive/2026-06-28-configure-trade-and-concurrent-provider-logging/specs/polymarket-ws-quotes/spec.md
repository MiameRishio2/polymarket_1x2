## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket
subscription, quote normalization, and append-only logging under `src/polymarket/`. The root
orchestration layer SHALL pass the configured Polymarket event URL and quote-log path into this
provider-local workflow and SHALL start it concurrently with enabled OddsPortal collection.

#### Scenario: Running the executable with both providers configured
- **WHEN** the executable starts with both providers enabled and the Australia–Egypt Polymarket event URL configured
- **THEN** it uses modules under `src/polymarket/` to discover that event, load and log initial quotes, and subscribe to its tokens without waiting for OddsPortal collection to finish

## ADDED Requirements

### Requirement: Configurable Polymarket collection target
The system SHALL load the Polymarket enabled flag, website event URL, and quote JSONL path from
the `polymarket` section of `config.yaml`, with existing provider defaults when the section is
absent.

#### Scenario: Localized event URL is configured
- **WHEN** `polymarket.url` is `https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03`
- **THEN** discovery extracts `fifwc-aus-egy-2026-07-03` and requests that Gamma event

#### Scenario: Quote log path is configured
- **WHEN** `polymarket.log_path` names a writable local path
- **THEN** initial and WebSocket quote records are appended to that path

### Requirement: Visible Polymarket lifecycle
The system SHALL emit `[polymarket]`-prefixed process output for startup, event discovery,
initial quote records, WebSocket subscription, quote updates, disconnections, and terminal
errors.

#### Scenario: No WebSocket update has arrived
- **WHEN** event discovery and initial CLOB snapshots succeed
- **THEN** the process log already contains Polymarket-attributed lifecycle and initial quote output

#### Scenario: WebSocket reconnects
- **WHEN** a market WebSocket connection fails or closes
- **THEN** the process log identifies the Polymarket connection failure or reconnect attempt
