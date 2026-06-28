## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket
subscription, quote normalization, and append-only logging under `src/polymarket/`. The root
orchestration layer SHALL pass the configured Polymarket event URL and quote-log path into this
provider-local workflow and SHALL start it concurrently with enabled OddsPortal collection.

#### Scenario: Running the executable with both providers configured
- **WHEN** the executable starts with both providers enabled and the Jordan–Argentina Polymarket event URL configured
- **THEN** it uses modules under `src/polymarket/` to discover that event, load and log initial quotes, and subscribe to its tokens without waiting for OddsPortal collection to finish

### Requirement: Configurable Polymarket collection target
The system SHALL load the Polymarket enabled flag, website event URL, and quote JSONL path from
the `polymarket` section of `config.yaml`, with existing provider defaults when the section is
absent.

#### Scenario: Localized event URL is configured
- **WHEN** `polymarket.url` is `https://polymarket.com/ja/sports/world-cup/fifwc-jor-arg-2026-06-27`
- **THEN** discovery extracts `fifwc-jor-arg-2026-06-27` and requests that Gamma event

#### Scenario: Quote log path is configured
- **WHEN** `polymarket.log_path` names a writable local path
- **THEN** initial and WebSocket quote records are appended to that path
