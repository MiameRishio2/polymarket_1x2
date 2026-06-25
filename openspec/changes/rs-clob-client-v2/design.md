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
