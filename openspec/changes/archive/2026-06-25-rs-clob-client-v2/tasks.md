## 1. Rust Project Setup

- [x] 1.1 Create the Rust binary project files in the current repository.
- [x] 1.2 Add `rs-clob-client-v2` and supporting async, HTTP, WebSocket, JSON, URL, decimal, and logging dependencies.
- [x] 1.3 Configure default constants for the supplied Polymarket URL, proxy URL, Gamma event endpoint, market WebSocket endpoint, and log file path.

## 2. Discovery

- [x] 2.1 Implement URL slug extraction for the supplied event URL shape.
- [x] 2.2 Implement Gamma event lookup through the configured proxy.
- [x] 2.3 Parse child markets, outcomes, and CLOB token IDs into typed runtime metadata.

## 3. WebSocket Quotes

- [x] 3.1 Connect to the Polymarket market WebSocket through the proxy.
- [x] 3.2 Send the market subscription payload for all discovered token IDs.
- [x] 3.3 Parse `book`, `price_change`, and `best_bid_ask` messages relevant to bid/ask updates.
- [x] 3.4 Maintain latest bid/ask state per token, including quantities when available.

## 4. Logging and Runtime Behavior

- [x] 4.1 Create the `logs` directory when missing and append quote updates as JSON lines.
- [x] 4.2 Mirror concise quote updates to stdout for live visibility.
- [x] 4.3 Add reconnect/backoff behavior for transient WebSocket failures.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo check`.
- [x] 5.3 Run a short live smoke test through `http://10.32.110.233:7890` and confirm bid/ask updates are written to the log file.
