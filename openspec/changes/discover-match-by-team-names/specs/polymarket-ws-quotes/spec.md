## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket football event discovery, initial order book loading, CLOB
WebSocket subscription, Sports WebSocket subscription, quote and score normalization, and
append-only quote logging under `src/polymarket/`. The root orchestration layer SHALL pass the
shared configured team pair and quote-log path into this provider-local workflow and SHALL start
it concurrently with enabled OddsPortal collection.

#### Scenario: Running the executable with both providers configured
- **WHEN** both providers are enabled and South Africa versus Canada is configured
- **THEN** modules under `src/polymarket/` discover that event, load and log initial quotes, and
  stream its CLOB quotes and score without waiting for OddsPortal collection to finish

### Requirement: Configurable Polymarket collection target
The system SHALL load the Polymarket enabled flag and quote JSONL path from the `polymarket`
section of `config.yaml`, SHALL receive the shared configured team pair from root configuration,
and SHALL retain existing provider defaults when optional provider settings are absent.

#### Scenario: Team pair is configured
- **WHEN** the shared target names South Africa and Canada
- **THEN** Polymarket discovery searches for the corresponding active football event without
  requiring an exact website event URL

#### Scenario: Quote log path is configured
- **WHEN** `polymarket.log_path` names a writable local path
- **THEN** initial and WebSocket quote records are appended to that path

### Requirement: Visible Polymarket lifecycle
The system SHALL emit `[polymarket]`-prefixed diagnostics to stderr for startup, event discovery,
CLOB and Sports WebSocket lifecycle, reconnects, and terminal errors. It SHALL emit
machine-readable Polymarket odds and score observations as complete JSON lines to stdout.

#### Scenario: Initial quote is loaded
- **WHEN** event discovery and an initial CLOB snapshot succeed
- **THEN** stdout contains a `polymarket_odds` JSON record and stderr contains
  Polymarket-attributed lifecycle diagnostics

#### Scenario: A Polymarket WebSocket reconnects
- **WHEN** either the CLOB or Sports WebSocket connection fails or closes
- **THEN** stderr identifies the Polymarket connection and its reconnect attempt without writing
  diagnostic text to stdout

## ADDED Requirements

### Requirement: Polymarket event discovery by team names
The system SHALL normalize the configured team names, inspect paginated active non-closed Gamma
events, and select the unique football 1X2 event containing those two teams in either displayed
order.

#### Scenario: One matching football event exists
- **WHEN** Gamma contains one active South Africa versus Canada football event with home-win,
  draw, and away-win markets
- **THEN** discovery returns that event, classifies its three 1X2 markets, and returns their CLOB
  token metadata

#### Scenario: No matching event exists
- **WHEN** no active football 1X2 event contains both configured teams
- **THEN** discovery fails with a contextual error naming the target pair

#### Scenario: Matching event is ambiguous
- **WHEN** more than one active football 1X2 event satisfies the normalized pair
- **THEN** discovery fails and identifies the candidates instead of choosing one implicitly

### Requirement: Structured Polymarket Yes-token odds output
The system SHALL write one `polymarket_odds` JSON object to stdout for each initial or changed Yes
token quote in the classified home-win, draw, or away-win market.

#### Scenario: Home-win Yes quote changes
- **WHEN** a CLOB message changes the Yes token for the classified home-win market
- **THEN** stdout receives one JSON record identifying `home` and containing event identity,
  configured teams, bid/ask prices and sizes, source, and local receipt time

#### Scenario: No token quote changes
- **WHEN** a CLOB message does not produce a normalized Yes-token quote change
- **THEN** no `polymarket_odds` JSON record is emitted for that message

### Requirement: Polymarket live score stream
The system SHALL connect to the unauthenticated Polymarket Sports WebSocket, respond to its
heartbeat, filter updates by the discovered event slug, and write each matching score observation
as one `polymarket_score` JSON object to stdout.

#### Scenario: Matching score update arrives
- **WHEN** the Sports WebSocket sends a score update whose slug equals the discovered event slug
- **THEN** stdout receives a JSON record containing score, period, elapsed time, status, source
  update time when present, and local receipt time

#### Scenario: Unrelated sports update arrives
- **WHEN** the Sports WebSocket sends an update for any other event slug
- **THEN** no score record is emitted for that update

#### Scenario: Sports heartbeat arrives
- **WHEN** the Sports WebSocket sends `ping`
- **THEN** the collector replies `pong` within the connection deadline
