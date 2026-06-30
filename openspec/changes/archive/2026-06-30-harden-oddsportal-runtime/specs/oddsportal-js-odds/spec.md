## MODIFIED Requirements

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, JSONL path, and positive polling
interval from configuration, SHALL use the shared root match home-team and away-team pair from root
configuration, SHALL use the root proxy setting for HTTP requests, and SHALL request identity
content encoding for those HTTP responses.

#### Scenario: Proxy-routed identity request
- **WHEN** the enabled OddsPortal collector issues an HTTP request
- **THEN** the request uses the configured root proxy and sends `Accept-Encoding: identity`

#### Scenario: Disabled OddsPortal collector
- **WHEN** `oddsportal.enabled` is false
- **THEN** no OddsPortal collector task is spawned

#### Scenario: Invalid polling interval
- **WHEN** OddsPortal is enabled with `poll_interval_seconds` equal to zero
- **THEN** configuration validation fails before an OddsPortal task is spawned
