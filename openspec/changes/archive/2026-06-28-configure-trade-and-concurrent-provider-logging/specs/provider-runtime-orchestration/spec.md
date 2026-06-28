## ADDED Requirements

### Requirement: Independently configurable runtime features
The system SHALL load Polymarket collection, OddsPortal collection, and live trading as
independently enabled features from `config.yaml`. It SHALL fail startup with a clear diagnostic
when both provider collectors are disabled.

#### Scenario: Both collectors enabled
- **WHEN** `polymarket.enabled` and `oddsportal.enabled` are both `true`
- **THEN** the system starts one Polymarket collector task and one OddsPortal collector task

#### Scenario: One collector enabled
- **WHEN** exactly one provider `enabled` value is `true`
- **THEN** the system starts only that provider and does not require the disabled provider's runtime target

#### Scenario: No collector enabled
- **WHEN** both provider `enabled` values are `false`
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
The system SHALL identify every collector startup, progress, retry, record, and failure line in
the shared process output as Polymarket, OddsPortal, or trade output without printing secrets.

#### Scenario: Process starts both providers
- **WHEN** both collectors are enabled
- **THEN** the process output contains target-safe startup lines prefixed `[polymarket]` and `[oddsportal]`

#### Scenario: Both providers emit records
- **WHEN** Polymarket normalizes a quote and OddsPortal normalizes bookmaker odds
- **THEN** their process-output lines carry distinct provider prefixes while their JSONL records remain in separate configured files
