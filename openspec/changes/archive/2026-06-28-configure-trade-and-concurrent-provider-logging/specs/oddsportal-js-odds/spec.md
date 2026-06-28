## ADDED Requirements

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
