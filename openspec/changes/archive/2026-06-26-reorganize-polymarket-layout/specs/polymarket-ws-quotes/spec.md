## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket subscription, quote normalization, and append-only logging under `src/polymarket/` while preserving existing runtime behavior.

#### Scenario: Running the executable after layout refactor
- **WHEN** the executable starts with the default Polymarket event URL
- **THEN** it uses the Polymarket modules under `src/polymarket/` to perform the same discovery, subscription, and logging workflow as before
