---
comet_change: rs-clob-client-v2
role: technical-design
canonical_spec: openspec
---

# rs-clob-client-v2 Technical Design

## Architecture

The project will be a single Rust binary with small internal modules:

- `config`: default Polymarket URL, proxy URL, Gamma endpoint, WebSocket endpoint, and log path.
- `discovery`: URL slug extraction and Gamma event lookup through the configured proxy.
- `models`: typed event, market, outcome token, order book, and quote update structures.
- `ws`: unauthenticated market WebSocket connection, subscription, message parsing, ping handling, and reconnect loop.
- `quotes`: latest bid/ask state per token, including best known sizes.
- `logging`: append-only JSON-lines quote output and concise stdout mirroring.

`main` wires these together: load defaults, discover tokens, open the log file, connect to the market WebSocket, subscribe to all token IDs, update in-memory quote state from each message, and append each useful quote update.

## Data Flow

```text
Polymarket event URL
  -> extract final path segment: fifwc-ecu-ger-2026-06-25
  -> GET https://gamma-api.polymarket.com/events/slug/{slug}
  -> parse child markets and clobTokenIds
  -> WSS wss://ws-subscriptions-clob.polymarket.com/ws/market
  -> send {"assets_ids":[...],"type":"market"}
  -> parse book / price_change / best_bid_ask
  -> update latest quote state
  -> append JSON line to logs/polymarket_quotes.log
```

All outbound network calls use `http://10.32.110.233:7890` by default. The HTTP client can use `reqwest::Proxy`. The WebSocket client will use `tokio-tungstenite` over an HTTP CONNECT tunnel established with `async-http-proxy`, while retaining `rs-clob-client-v2` as an installed dependency.

## Message Handling

The implementation will support these market-channel event types:

- `book`: parse all bid and ask levels, choose the best bid and best ask, and record both price and size.
- `price_change`: update matching side prices and sizes when the message includes side, price, and size fields. If size is absent, retain the last known size for that side.
- `best_bid_ask`: update top bid and ask prices. If no sizes are present, keep previous sizes or log `null`.

Unknown events are ignored after a debug-level trace. Malformed messages do not stop the process unless parsing failures become fatal during subscription setup.

## Logging Contract

Each quote update is appended as one JSON object per line:

```json
{
  "ts": "2026-06-25T15:00:00.000Z",
  "event_slug": "fifwc-ecu-ger-2026-06-25",
  "market_slug": "fifwc-ecu-ger-2026-06-25-ger",
  "question": "Will Germany win on 2026-06-25?",
  "outcome": "Yes",
  "asset_id": "101279...",
  "bid_price": "0.627",
  "bid_size": "125.5",
  "ask_price": "0.628",
  "ask_size": "98.0",
  "source": "book"
}
```

Prices and sizes will remain strings in logs to avoid decimal precision loss.

## Error Handling

- Discovery failures return a clear error and exit non-zero.
- Missing `clobTokenIds` on all markets is a fatal configuration/data error.
- WebSocket disconnects trigger reconnect with short backoff.
- File logging failures are fatal because the requested output is the log file.
- Keyboard interruption ends the process without deleting existing logs.

## Testing Strategy

- Unit-test slug extraction from localized Polymarket paths.
- Unit-test Gamma event parsing, especially JSON-string encoded `clobTokenIds`.
- Unit-test best bid/ask calculation from book levels.
- Unit-test quote-state updates for book and price-only updates.
- Verify with `cargo fmt`, `cargo check`, and a bounded live smoke test through `http://10.32.110.233:7890` that creates log entries.

## Implementation Notes

- The default run path should not require command-line arguments.
- The code should allow optional override arguments or environment variables only if that stays simple; defaults are the priority.
- The executable remains read-only and unauthenticated. It must not prompt for private keys or API credentials.
