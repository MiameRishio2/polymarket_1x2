## ADDED Requirements

### Requirement: Event URL discovery
The system SHALL accept the supplied Polymarket event URL and resolve the final URL path segment as an event slug through the Polymarket Gamma event endpoint.

#### Scenario: Resolve Ecuador Germany event
- **WHEN** the executable runs with `https://polymarket.com/ja/sports/world-cup/fifwc-ecu-ger-2026-06-25`
- **THEN** it discovers the `fifwc-ecu-ger-2026-06-25` event and its child markets

### Requirement: CLOB token subscription
The system SHALL subscribe to all CLOB token IDs returned by the event's child markets using the Polymarket market WebSocket channel.

#### Scenario: Subscribe all 1X2 outcomes
- **WHEN** the event contains Draw, Germany, and Ecuador child markets
- **THEN** the system subscribes to the six outcome token IDs from those markets

### Requirement: Proxy usage
The system SHALL route outbound Gamma API and WebSocket connections through `http://10.32.110.233:7890` by default.

#### Scenario: Default proxy configured
- **WHEN** the executable starts without a custom proxy argument
- **THEN** HTTP discovery and WebSocket subscription attempts use `http://10.32.110.233:7890`

### Requirement: Latest bid and ask logging
The system SHALL log latest bid and ask prices and quantities for each subscribed token whenever WebSocket messages provide updated quote data.

#### Scenario: Book snapshot contains bid and ask levels
- **WHEN** a WebSocket `book` message contains bid and ask levels for a subscribed token
- **THEN** the system logs the best bid price and size and the best ask price and size for that token

#### Scenario: Price update lacks quantity
- **WHEN** a WebSocket update provides a latest bid or ask price without size
- **THEN** the system logs the updated price and leaves quantity empty unless a previous quantity is known

### Requirement: Append-only log file
The system SHALL write quote updates to an append-only log file under a local `logs` directory.

#### Scenario: Log directory missing
- **WHEN** the executable starts and `logs` does not exist
- **THEN** the system creates `logs` before writing quote updates

### Requirement: Read-only market data client
The system SHALL remain unauthenticated and read-only.

#### Scenario: Running the executable
- **WHEN** the executable connects to Polymarket services
- **THEN** it does not request private keys, API credentials, or order placement permissions
