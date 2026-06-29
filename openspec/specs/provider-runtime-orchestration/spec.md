## Purpose

Define root runtime configuration, shared match targeting, concurrent task supervision, failure
isolation, and provider-attributed process output.

## Requirements

### Requirement: Independently configurable runtime features
The system SHALL load one shared football match target plus independently enabled Polymarket
collection, OddsPortal collection, and live trading features from `config.yaml`. It SHALL fail
startup with a clear diagnostic when both provider collectors are disabled or when the shared
match target is invalid.

#### Scenario: Both collectors enabled
- **WHEN** `polymarket.enabled` and `oddsportal.enabled` are both `true`
- **THEN** the system starts one Polymarket collector task and one OddsPortal collector task using
  the same configured team pair

#### Scenario: One collector enabled
- **WHEN** exactly one provider `enabled` value is `true`
- **THEN** the system starts only that provider and does not require disabled-provider transport
  settings

#### Scenario: No collector enabled
- **WHEN** both provider `enabled` values are `false`
- **THEN** startup fails before spawning any provider task

#### Scenario: Shared match target is invalid
- **WHEN** either team name is blank or both normalized team names are equal
- **THEN** startup fails before spawning any provider task

### Requirement: Concurrent provider startup
The system SHALL start enabled Polymarket and OddsPortal collection without waiting for either
provider to finish a network request or collection pass before the other provider starts.

#### Scenario: OddsPortal request is slow
- **WHEN** the OddsPortal collector is waiting for an HTTP response
- **THEN** Polymarket discovery and WebSocket startup can proceed concurrently

#### Scenario: Polymarket stream remains connected
- **WHEN** the Polymarket collector is waiting for WebSocket messages
- **THEN** the OddsPortal collector continues its configured polling passes

### Requirement: Provider task failure isolation
The system SHALL report an enabled provider task's terminal failure without cancelling another
provider task that is still running.

#### Scenario: Polymarket task exits with an error
- **WHEN** the OddsPortal polling task is still running
- **THEN** the system emits a Polymarket-attributed terminal error and leaves OddsPortal running

#### Scenario: OddsPortal pass fails
- **WHEN** an OddsPortal collection pass returns an error
- **THEN** the polling task emits an OddsPortal-attributed error, waits for the configured interval, and tries another pass without interrupting Polymarket

### Requirement: Provider-attributed process output
The system SHALL write collector data observations as complete JSON objects to stdout and SHALL
write provider-attributed lifecycle, retry, and failure diagnostics to stderr without printing
secrets.

#### Scenario: Both providers emit data
- **WHEN** Polymarket and OddsPortal produce odds or score observations
- **THEN** each stdout JSON object identifies its provider and record type without cross-provider
  aggregation

#### Scenario: Provider emits a diagnostic
- **WHEN** a provider starts, reconnects, retries, or fails
- **THEN** stderr carries its stable `[polymarket]`, `[oddsportal]`, or `[trade]` prefix while
  stdout remains data-only
