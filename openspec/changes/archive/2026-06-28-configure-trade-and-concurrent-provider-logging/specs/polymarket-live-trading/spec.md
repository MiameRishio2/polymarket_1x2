## MODIFIED Requirements

### Requirement: Three-mode live-trading gate
The system SHALL enable authenticated order placement only when `trade.enabled` is explicitly
`true` and `trade.trader_mode`, `trade.account_mode`, and `trade.market_mode` in `config.yaml`
are all exactly `real`. A missing `trade.enabled` value SHALL default to `false`. The
`trade.order_mode` field SHALL remain unsupported.

#### Scenario: Explicit enable and all modes allow live trading
- **WHEN** `trade.enabled` is `true` and all three retained trade modes equal `real`
- **THEN** the system may initialize the authenticated client and run the fixed live flow

#### Scenario: Explicit trade enable is absent or false
- **WHEN** `trade.enabled` is absent or is `false`
- **THEN** the system does not parse a signer, derive API credentials, place an order, or cancel an order

#### Scenario: Any non-real mode prevents writes
- **WHEN** `trade.enabled` is `true` but at least one retained trade mode is absent or differs from `real`
- **THEN** the system does not parse a signer, derive API credentials, place an order, or cancel an order
