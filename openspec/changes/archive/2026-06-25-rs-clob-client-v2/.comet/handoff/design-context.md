# Comet Design Handoff

- Change: rs-clob-client-v2
- Phase: design
- Mode: compact
- Context hash: e51a157d350e521c2c7be8ec986144db0a2776c1a8f9bab2fb86a3b538f4ed31

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/rs-clob-client-v2/proposal.md

- Source: openspec/changes/rs-clob-client-v2/proposal.md
- Lines: 1-29
- SHA256: db475aacb868973cb4632a6f8648bfb3836f37a1f02fe7a4f1fc76ede2c7b2c1

```md
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
```

## openspec/changes/rs-clob-client-v2/design.md

- Source: openspec/changes/rs-clob-client-v2/design.md
- Lines: 1-67
- SHA256: 6b63f33f10ba46eb9a213cf168740d4616a3e22833b6df2ebd041d5e4e013a5f

```md
## Context

The repository currently has no Rust application code. The requested source URL is `https://polymarket.com/ja/sports/world-cup/fifwc-ecu-ger-2026-06-25`. A direct Gamma market lookup for `fifwc-ecu-ger-2026-06-25` returns not found, while `GET https://gamma-api.polymarket.com/events/slug/fifwc-ecu-ger-2026-06-25` returns the event "Ecuador vs. Germany" with three child markets: draw, Germany win, and Ecuador win. Each child market has two CLOB token IDs.

Polymarket's current documentation identifies CLOB V2 production at `https://clob.polymarket.com`, documents the market WebSocket subscription shape as `{"assets_ids":[...],"type":"market"}`, and lists `book`, `price_change`, and `best_bid_ask` messages as market-data sources. The requested Rust dependency is the crates.io package `rs-clob-client-v2`.

## Goals / Non-Goals

**Goals:**

- Build a Rust CLI in this repository that can run with one command.
- Use proxy `http://10.32.110.233:7890` for outbound HTTP and WebSocket traffic.
- Resolve the supplied event URL into event metadata and all child market CLOB token IDs.
- Subscribe to the Polymarket market WebSocket channel and maintain latest bid/ask state per token.
- Log every useful quote update to a file with timestamp, market slug/question, outcome, asset ID, bid price/size, and ask price/size where available.
- Keep the implementation unauthenticated and read-only.

**Non-Goals:**

- Placing, signing, or cancelling orders.
- Persisting data to a database.
- Building a UI or service daemon.
- Solving all Polymarket event URL shapes beyond practical slug extraction from the supplied style of URL.

## Decisions

### 1. Treat the URL as an event URL, not a market URL

The slug in the provided URL resolves through the Gamma event endpoint and contains multiple child markets. The CLI will extract the final path segment, request `/events/slug/{slug}`, and subscribe to every `clobTokenIds` entry from all returned markets.

Alternative considered: call `/markets/slug/{slug}` only. That fails for this URL and would miss 1X2 event composition.

### 2. Subscribe by CLOB token IDs and compute top of book locally

The WebSocket stream can send full `book` snapshots, `price_change` updates, and `best_bid_ask` events. The CLI will parse these event types. `book` snapshots provide price and size on both sides, so they are the source of truth for quantities. `price_change` and `best_bid_ask` refresh top prices; when size is absent, the CLI keeps the last known size or logs an empty size.

Alternative considered: only log `best_bid_ask`. That gives latest prices but not quantities, so it does not satisfy the requested "price and quantity" output.

### 3. Install `rs-clob-client-v2`, but keep wire parsing explicit if needed

The project will install `rs-clob-client-v2 = "0.2.2"` as requested. If the SDK API does not expose the exact unauthenticated market-channel and proxy needs cleanly, the executable can use `tokio-tungstenite`/`reqwest` directly while retaining `rs-clob-client-v2` as the Polymarket CLOB dependency in the project. This avoids blocking on SDK ergonomics while keeping the dependency installed.

Alternative considered: use only raw WebSocket dependencies and skip the CLOB client crate. That conflicts with the user's explicit dependency installation request.

### 4. File logging is append-only JSON lines plus readable tracing

The implementation will create `logs/` if missing and append structured quote records to a log file. JSON lines are easier to inspect and replay than free-form text. Console output can mirror concise updates.

Alternative considered: human-only text logs. That is easier initially but worse for downstream analysis.

## Risks / Trade-offs

- Polymarket schema drift -> Parse only fields required for quote logging and preserve raw event snippets in debug logs when parsing fails.
- WebSocket reconnects -> Add a retry loop with short backoff so transient disconnects do not terminate the process immediately.
- Proxy incompatibility with WebSocket -> Build the WebSocket connector through a client/proxy stack that supports HTTP CONNECT; verify with a live smoke test through `http://10.32.110.233:7890`.
- Quantity freshness -> `best_bid_ask` does not include size; full bid/ask quantity accuracy depends on `book` and `price_change` messages. The logger will clearly omit size when unknown rather than inventing a value.

## Migration Plan

1. Add the Rust project and dependencies.
2. Implement discovery, subscription, quote state, and logging.
3. Verify with `cargo fmt`, `cargo check`, and a short live run through the proxy that produces log entries.
4. Rollback is deleting the added Rust project files and OpenSpec change if the approach is rejected before merge.

## Open Questions

- Whether the final log format should be JSON lines only or both JSON lines and a human-readable file. The implementation will default to JSON lines unless changed during review.
```

## openspec/changes/rs-clob-client-v2/tasks.md

- Source: openspec/changes/rs-clob-client-v2/tasks.md
- Lines: 1-30
- SHA256: 4250a73a018671c068e794d8ade037ed7d2d7994ded1f356104bfa25993a5825

```md
## 1. Rust Project Setup

- [ ] 1.1 Create the Rust binary project files in the current repository.
- [ ] 1.2 Add `rs-clob-client-v2` and supporting async, HTTP, WebSocket, JSON, URL, decimal, and logging dependencies.
- [ ] 1.3 Configure default constants for the supplied Polymarket URL, proxy URL, Gamma event endpoint, market WebSocket endpoint, and log file path.

## 2. Discovery

- [ ] 2.1 Implement URL slug extraction for the supplied event URL shape.
- [ ] 2.2 Implement Gamma event lookup through the configured proxy.
- [ ] 2.3 Parse child markets, outcomes, and CLOB token IDs into typed runtime metadata.

## 3. WebSocket Quotes

- [ ] 3.1 Connect to the Polymarket market WebSocket through the proxy.
- [ ] 3.2 Send the market subscription payload for all discovered token IDs.
- [ ] 3.3 Parse `book`, `price_change`, and `best_bid_ask` messages relevant to bid/ask updates.
- [ ] 3.4 Maintain latest bid/ask state per token, including quantities when available.

## 4. Logging and Runtime Behavior

- [ ] 4.1 Create the `logs` directory when missing and append quote updates as JSON lines.
- [ ] 4.2 Mirror concise quote updates to stdout for live visibility.
- [ ] 4.3 Add reconnect/backoff behavior for transient WebSocket failures.

## 5. Verification

- [ ] 5.1 Run `cargo fmt`.
- [ ] 5.2 Run `cargo check`.
- [ ] 5.3 Run a short live smoke test through `http://10.32.110.233:7890` and confirm bid/ask updates are written to the log file.
```

## openspec/changes/rs-clob-client-v2/specs/polymarket-ws-quotes/spec.md

- Source: openspec/changes/rs-clob-client-v2/specs/polymarket-ws-quotes/spec.md
- Lines: 1-47
- SHA256: ab0012a00f0a9041f52d04d86d8011080554c568ddf5226588844dd8a41c4074

```md
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
```

