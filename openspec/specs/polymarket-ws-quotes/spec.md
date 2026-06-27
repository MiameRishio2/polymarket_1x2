## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket subscription, quote normalization, and append-only logging under `src/polymarket/` while preserving existing Polymarket runtime behavior when OddsPortal logging is added to binary orchestration.

#### Scenario: Running the executable with both providers configured
- **WHEN** the executable starts with the default Polymarket event URL and default OddsPortal match configuration
- **THEN** it uses the Polymarket modules under `src/polymarket/` to perform the same discovery, subscription, and logging workflow as before while also starting OddsPortal odds logging
