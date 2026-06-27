## ADDED Requirements

### Requirement: Read-only latest quote access
The system SHALL expose an immutable latest quote snapshot for a known Polymarket asset so provider-local dry-run logic can consume the same state populated by initial CLOB order books and market WebSocket updates.

#### Scenario: Known asset has quote state
- **WHEN** an initial order book or WebSocket update has populated the selected asset
- **THEN** the caller receives a cloned latest quote record without mutating subscription or quote state

#### Scenario: Asset has no quote state
- **WHEN** the selected asset has not received an initial order book or WebSocket update
- **THEN** the caller receives no quote snapshot
