## ADDED Requirements

### Requirement: Validated dry-run limit intents
The system SHALL represent dry-run limit-order intents with a Polymarket asset ID, side, decimal price strictly between zero and one, and positive decimal size.

#### Scenario: Valid initial buy
- **WHEN** the dry-run builds a buy intent at price `0.01` and size `5`
- **THEN** it accepts the intent without floating-point conversion

#### Scenario: Invalid intent
- **WHEN** the dry-run receives a non-positive size or a price outside the open interval from zero to one
- **THEN** it rejects the intent before generating a simulated order ID

### Requirement: Deterministic New Zealand versus Belgium sequence
The system SHALL model the New Zealand vs Belgium dry-run as a buy at price `0.01` and size `5`, followed only after simulated buy acceptance by a sell for the same asset and size at price `0.11`.

#### Scenario: Accepted buy advances to sell
- **WHEN** the simulator accepts the initial buy and returns a simulated order ID
- **THEN** the workflow records that order ID and submits exactly one simulated sell intent at price `0.11` and size `5`

#### Scenario: Buy is not accepted
- **WHEN** the simulator rejects the initial buy without returning an order ID
- **THEN** the workflow terminates without creating a sell intent or retrying the buy

### Requirement: Fail-closed cancellation
The system SHALL attempt simulated cancellation at most once after a lifecycle failure when an accepted simulated order ID is available, and SHALL never retry placement.

#### Scenario: Sell fails after buy acceptance
- **WHEN** the initial buy has a simulated order ID and the sell step fails
- **THEN** the workflow attempts one simulated cancellation for the accepted buy order ID and terminates

#### Scenario: Cancellation fails
- **WHEN** the single simulated cancellation attempt fails
- **THEN** the workflow reports both failures and performs no additional placement or cancellation attempt

### Requirement: No live trading capability
The dry-run SHALL operate without loading accounts, credentials, private keys, signatures, balances, allowances, or write-side exchange clients.

#### Scenario: Running the dry-run
- **WHEN** the dry-run scenario is executed in tests
- **THEN** it uses only local quote data and simulated responses and sends no create-order or cancel-order network request

### Requirement: Abstract order executor
The lifecycle orchestration SHALL depend on an asynchronous order executor interface for limit placement and cancellation, and the change SHALL provide only a configurable mock implementation.

#### Scenario: Successful mock placement
- **WHEN** the mock executor is configured to accept a limit intent
- **THEN** it returns a synthetic order ID through the same executor interface used by lifecycle orchestration

#### Scenario: Executor call audit
- **WHEN** the mock workflow completes or fails
- **THEN** tests can inspect the exact placement and cancellation call order without any network activity

### Requirement: Quote-gated simulation
The system SHALL require a latest quote snapshot for the selected asset before starting the simulated lifecycle.

#### Scenario: Quote is available
- **WHEN** the existing CLOB snapshot or market WebSocket pipeline has populated a quote for the selected asset
- **THEN** the dry-run may validate and execute the local simulation

#### Scenario: Quote is unavailable
- **WHEN** no quote snapshot exists for the selected asset
- **THEN** the dry-run fails before generating a simulated order ID
