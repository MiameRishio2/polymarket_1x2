## Purpose

Define provider-local Polymarket quote collection and immutable access to the latest normalized quote state.

## Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket subscription, quote normalization, and append-only logging under `src/polymarket/` while preserving existing Polymarket runtime behavior when OddsPortal logging is added to binary orchestration.

#### Scenario: Running the executable with both providers configured
- **WHEN** the executable starts with the default Polymarket event URL and default OddsPortal match configuration
- **THEN** it uses the Polymarket modules under `src/polymarket/` to perform the same discovery, subscription, and logging workflow as before while also starting OddsPortal odds logging

### Requirement: Read-only latest quote access
The system SHALL expose an immutable latest quote snapshot for a known Polymarket asset so provider-local dry-run logic can consume the same state populated by initial CLOB order books and market WebSocket updates.

#### Scenario: Known asset has quote state
- **WHEN** an initial order book or WebSocket update has populated the selected asset
- **THEN** the caller receives a cloned latest quote record without mutating subscription or quote state

#### Scenario: Asset has no quote state
- **WHEN** the selected asset has not received an initial order book or WebSocket update
- **THEN** the caller receives no quote snapshot
