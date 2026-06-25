## Why

We need a small Rust executable that can monitor live Polymarket CLOB quotes for the Ecuador vs. Germany World Cup event from the supplied Polymarket URL. The first useful version should run through the required HTTP proxy, subscribe over WebSocket, and persist latest bid/ask updates to a log file for later inspection.

## What Changes

- Create a Rust project in the current repository.
- Add the requested Polymarket Rust CLOB V2 dependency (`rs-clob-client-v2`) plus supporting async/logging dependencies.
- Configure the executable to use HTTP proxy `http://10.32.110.233:7890` for HTTP discovery and WebSocket traffic.
- Resolve the supplied Polymarket event URL to the event slug `fifwc-ecu-ger-2026-06-25`, discover all child markets and CLOB token IDs, then subscribe to those token IDs through the Polymarket market WebSocket channel.
- Print and persist latest bid/ask price and quantity updates to a log file.
- No breaking changes; there is no existing Rust application surface in this repository.

## Capabilities

### New Capabilities

- `polymarket-ws-quotes`: Rust CLI capability for resolving a Polymarket event URL, subscribing to CLOB market WebSocket data, and logging latest bid/ask quotes.

### Modified Capabilities

None.

## Impact

- Adds Rust project files (`Cargo.toml`, `Cargo.lock`, `src/**`) to the current repository.
- Adds dependencies for async runtime, HTTP/WebSocket client behavior, JSON parsing, URL parsing, decimal handling, and file logging.
- Calls Polymarket Gamma API for event/market discovery and Polymarket CLOB market WebSocket for live market data.
- Writes runtime logs to a local log file such as `logs/polymarket_quotes.log`.
