## Purpose

Define configurable OddsPortal football match discovery, frontend feed decoding, 1X2 odds normalization, repeated polling, and provider-attributed append-only logging.

## Requirements

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

#### Scenario: Prefer frontend xhash request
- **WHEN** the H2H page exposes frontend `eventData.xhash` and `requestPreMatch.url` values
- **THEN** the system first requests the `.dat` URL using `eventData.xhash` and retains `requestPreMatch.url` as a fallback

### Requirement: JavaScript odds response decoding
The system SHALL decode OddsPortal `.dat` responses using the current frontend JavaScript envelope before odds normalization.

#### Scenario: Decode compressed match event response
- **WHEN** the internal `.dat` response body is base64 encoded, compressed, and URL encoded according to OddsPortal's JavaScript decoder
- **THEN** the system produces parseable JSON odds data or returns a contextual decoding error

#### Scenario: Decode encrypted match event response
- **WHEN** the internal `.dat` response body is a base64 envelope containing AES-CBC ciphertext and IV data according to OddsPortal's JavaScript decoder
- **THEN** the system decrypts and optionally decompresses the response into parseable JSON odds data or returns a contextual decoding error

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

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, home team, away team, JSONL
path, and positive polling interval from the `oddsportal` section of `config.yaml`, while using
the root proxy setting for HTTP requests.

#### Scenario: Australia Egypt target is configured
- **WHEN** the configured home team is Australia and away team is Egypt
- **THEN** each collection pass searches the configured tournament state for Australia - Egypt

#### Scenario: Polling interval is invalid
- **WHEN** `oddsportal.poll_interval_seconds` is zero
- **THEN** configuration validation fails before an OddsPortal task is spawned

### Requirement: Repeated OddsPortal collection
The system SHALL run OddsPortal collection repeatedly at the configured interval while its
provider task remains enabled and SHALL append every successful pass to the provider-local JSONL
log.

#### Scenario: Collection pass succeeds
- **WHEN** a pass normalizes and logs one or more 1X2 records
- **THEN** the task reports the completed pass, waits for the configured interval, and starts another pass

#### Scenario: Collection pass fails
- **WHEN** discovery, request, decoding, normalization, or logging fails
- **THEN** the task reports the contextual error, waits for the configured interval, and starts another pass

### Requirement: Visible OddsPortal lifecycle
The system SHALL emit `[oddsportal]`-prefixed process output for startup, pass completion, odds
records, retries, and failures.

#### Scenario: Polymarket is also running
- **WHEN** an OddsPortal polling pass executes while Polymarket waits for WebSocket messages
- **THEN** the shared process output unambiguously attributes OddsPortal progress and records to OddsPortal
