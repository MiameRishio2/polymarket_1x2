## MODIFIED Requirements

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
