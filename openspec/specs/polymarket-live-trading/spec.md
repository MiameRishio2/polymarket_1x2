## Purpose

Define the explicitly gated, proxy-routed Polymarket fixed-flow live trading capability, including configuration, authentication, order placement, cancellation, one-shot activation, and credential-safe diagnostics.

## Requirements

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

### Requirement: Long-account selection
The system SHALL select exactly one configured account whose `type` is `long` and SHALL fail before authentication when no such account or more than one such account exists.

#### Scenario: One long account is selected
- **WHEN** configuration contains exactly one account with `type: long`
- **THEN** that account supplies the private key, signature type, optional funder, CLOB host, and chain ID

#### Scenario: Long account selection is ambiguous
- **WHEN** configuration contains zero or multiple accounts with `type: long`
- **THEN** startup fails without constructing an authenticated client or issuing a write request

### Requirement: Configured proxied wallet authentication
The system SHALL use `rs-clob-client-v2` with `config.yaml.proxy`, construct the signer from the selected account's configured private key, load `api_key`, `api_secret`, and `api_passphrase` from that account, map a null signature type to EOA type `0`, accept configured signature types `0` through `3`, and apply the optional funder. It MUST NOT call the SDK's API-key creation or derivation methods.

#### Scenario: Null signature type uses EOA
- **WHEN** the selected long account has `signature_type: null`
- **THEN** the authenticated client is constructed with signature type `0`

#### Scenario: Explicit signature type and funder are forwarded
- **WHEN** the selected long account provides a supported signature type and funder address
- **THEN** both values are passed to the CLOB client used for signing and order creation

#### Scenario: Configured L2 credentials are forwarded
- **WHEN** the selected long account provides non-empty API key, secret, and passphrase values
- **THEN** those values initialize the L2-authenticated client without invoking credential creation or derivation

#### Scenario: Authentication configuration fails safely
- **WHEN** the private key is invalid or any configured L2 credential is missing
- **THEN** startup fails without placing an order and without including the private key in output

#### Scenario: Configured proxy is used
- **WHEN** the authenticated CLOB client is constructed
- **THEN** its HTTP transport uses the root `config.yaml.proxy` value

### Requirement: First-token fixed live flow
The system SHALL use the first token returned by event discovery and the initial quote snapshot to run one `0.01 × 5` limit buy followed immediately after accepted placement by one `0.11 × 5` limit sell.

#### Scenario: Fixed flow succeeds
- **WHEN** live trading is enabled, the event has at least one token with an initial quote, and both placements return valid order IDs
- **THEN** the system posts the buy and sell once each and returns both live order IDs

#### Scenario: Event has no token
- **WHEN** live trading is enabled but event discovery returns no token
- **THEN** startup fails before authentication or order placement

#### Scenario: First token has no quote
- **WHEN** live trading is enabled but no initial quote exists for the first token
- **THEN** the lifecycle fails before any order placement

### Requirement: Live limit-order mapping
The live executor SHALL map each validated `LimitOrderIntent` to an `rs-clob-client-v2` signed GTC limit order with the same asset ID and side and a checked, deterministic conversion of the validated fixed decimal price and size at the SDK's `f64` boundary. Placement SHALL succeed only when the response reports success and includes a non-empty order ID.

#### Scenario: Buy intent is posted as GTC
- **WHEN** the lifecycle submits the fixed buy intent
- **THEN** the live executor signs and posts one GTC buy with asset ID from the first token, price `0.01`, and size `5`

#### Scenario: Placement response lacks an order ID
- **WHEN** the CLOB accepts the HTTP request but the response has no usable order ID
- **THEN** the placement is treated as failed and no synthetic order ID is generated

#### Scenario: Placement response reports failure
- **WHEN** the CLOB response reports `success: false` even if the HTTP request completed
- **THEN** the placement is treated as failed and the response is not converted into an accepted order ID

### Requirement: Sell failure cancellation
The live executor SHALL preserve the existing fail-closed lifecycle: when the accepted buy has an order ID and the immediate sell fails, it attempts cancellation of that buy exactly once and performs no placement retry.

#### Scenario: Sell fails after accepted buy
- **WHEN** buy placement returns an order ID and sell placement fails
- **THEN** the system calls live cancellation once for the buy order ID and terminates the lifecycle with the sell and cancellation results

#### Scenario: Cancellation response does not confirm the order ID
- **WHEN** cancellation returns successfully at the HTTP layer but the requested buy order ID is absent from `canceled` or present in `not_canceled`
- **THEN** the lifecycle records cancellation as failed

#### Scenario: Buy placement fails
- **WHEN** the buy cannot be created, signed, posted, or parsed into an order ID
- **THEN** the system performs no sell and no cancellation

### Requirement: One-shot process activation
The live flow SHALL run at most once per process start after initial order books are loaded and SHALL never be retried by the market WebSocket reconnect loop.

#### Scenario: WebSocket reconnects
- **WHEN** the market WebSocket disconnects and reconnects after the live flow has run
- **THEN** no additional live order lifecycle is started

### Requirement: Credential-safe diagnostics and tests
The system MUST NOT log private keys, API secrets, passphrases, or complete signed order payloads. Automated tests SHALL use invalid placeholder credentials and SHALL not send live create-order or cancel-order requests.

#### Scenario: Live initialization or request fails
- **WHEN** an authentication, signing, placement, or cancellation error is reported
- **THEN** the diagnostic identifies the failed stage without exposing credential values or signed payloads

#### Scenario: Test suite executes
- **WHEN** automated tests exercise configuration, gating, mapping, response parsing, and lifecycle failures
- **THEN** all network writes are replaced by mocks or local fixtures
