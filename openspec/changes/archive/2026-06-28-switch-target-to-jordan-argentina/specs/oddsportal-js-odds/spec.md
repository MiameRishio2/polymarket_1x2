## MODIFIED Requirements

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, home team, away team, JSONL
path, and positive polling interval from the `oddsportal` section of `config.yaml`, while using
the root proxy setting for HTTP requests.

#### Scenario: Jordan Argentina target is configured
- **WHEN** the configured home team is Jordan and away team is Argentina
- **THEN** each collection pass searches the configured tournament state for Jordan - Argentina

#### Scenario: Polling interval is invalid
- **WHEN** `oddsportal.poll_interval_seconds` is zero
- **THEN** configuration validation fails before an OddsPortal task is spawned
