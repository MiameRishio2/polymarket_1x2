# Polymarket 1X2 Architecture

## Project Identity

`polymarket-1x2` is a Rust command-line collector for Polymarket 1X2 quote data and OddsPortal football bookmaker odds and scores. It discovers one configured team pair independently at both providers, subscribes to Polymarket market and sports WebSockets, and polls OddsPortal odds and score resources. Normalized observations are written as JSON Lines to stdout; provider-specific diagnostics are written as text to stderr. Existing quote records are also appended to provider JSONL files.

## Source Tree

```text
src/
├── main.rs                  # Binary entry point and top-level orchestration
├── config.rs                # Root config.yaml ownership and provider runtime construction
├── diagnostics.rs           # Shared timestamped stderr diagnostic boundary
├── polymarket/              # Polymarket provider implementation
│   ├── mod.rs
│   ├── clob.rs              # rs-clob-client-v2 setup and order book adaptation
│   ├── config.rs            # Polymarket runtime types, live gate, accounts, endpoints, and defaults
│   ├── discovery.rs         # Gamma team-name event discovery and 1X2 classification
│   ├── live.rs              # Proxied authenticated client, live executor, and one-shot orchestration
│   ├── logging.rs           # Append-only quote JSONL logger
│   ├── models.rs            # Polymarket quote and token data structures
│   ├── order.rs             # Abstract executor and read-only order lifecycle simulation
│   ├── output.rs            # Polymarket stdout observation models
│   ├── quotes.rs            # In-memory latest bid/ask state
│   ├── sports.rs            # Public sports-score WebSocket
│   └── ws.rs                # Market WebSocket subscription and message parsing
└── oddsportal/              # OddsPortal provider implementation
    ├── mod.rs
    ├── config.rs            # OddsPortal URLs, target match, user agent, and log path defaults
    ├── decoder.rs           # Internal .dat payload decoding
    ├── discovery.rs         # Tournament/H2H embedded state parsing
    ├── logging.rs           # Append-only OddsPortal JSONL logger
    ├── models.rs            # OddsPortal match, request, and odds record structures
    ├── odds.rs              # 1X2 bookmaker odds normalization
    ├── output.rs            # Grouped odds and score observation models and JSON line output
    └── score.rs             # Score payload normalization and unavailable-score construction
```

Provider-specific code must stay in separate source subtrees:

- Polymarket code goes under `src/polymarket/`.
- OddsPortal code goes under `src/oddsportal/`.

Shared code should remain absent until both providers need it. When that happens, introduce narrowly scoped shared modules instead of moving provider-specific parsing or transport details into a generic layer.

## Components

### Binary Entry Point

`src/main.rs` installs the Rustls crypto provider, loads the root runtime configuration, and starts each enabled provider as an independent local task. A Tokio `LocalSet` accommodates provider futures that are not `Send`, while a `JoinSet` supervises both collectors concurrently. The supervisor attributes every terminal result or panic to its provider and keeps waiting for remaining tasks; one provider stopping does not cancel the other.

### Root Configuration

`src/config.rs` owns deserialization of the repository-root `config.yaml`, provider enable flags, and conversion into provider-local runtime values. It rejects a configuration with both collectors disabled, validates the OddsPortal polling interval, and delegates Polymarket market-data and live-account construction to `src/polymarket/config.rs`.

The required root `match.home_team` and `match.away_team` pair is validated once and injected into both provider runtimes. It replaces provider-specific target ownership: Polymarket discovers a unique active football 1X2 event by normalized team names, while OddsPortal discovers the same configured pair on its tournament page.

Both provider enable flags default to `true` when omitted. Live trading fails closed: `trade.enabled` defaults to `false`, and live configuration is constructed only when it is explicitly `true` and `trade.trader_mode`, `trade.account_mode`, and `trade.market_mode` are all `real`. Therefore, an older configuration containing only the three `real` modes migrates to read-only collection until `trade.enabled: true` is deliberately added.

### Polymarket Provider

The Polymarket provider owns all Gamma API, CLOB REST, CLOB WebSocket, public Sports WebSocket,
quote-state, and log-writing details. Its public surface is intentionally small: config creation
and dual-stream execution after one team-name event discovery pass. The market and sports
WebSockets reconnect independently and emit `polymarket_odds` and `polymarket_score`
observations respectively.

The provider exposes an executor-independent order lifecycle with a deterministic local mock for tests. It also implements the executor boundary with a live `rs-clob-client-v2` adapter. Live execution requires `trade.enabled: true` in addition to `trader_mode`, `account_mode`, and `market_mode` all being `real`; it selects exactly one `type: long` account and uses its configured signer, L2 credentials, signature type, funder, host, chain, and the root proxy. The client never calls SDK credential creation or derivation methods because that dependency logs L1 authentication headers on its create path.

### OddsPortal Provider

The OddsPortal provider owns tournament/H2H page fetching, embedded state parsing, independent
odds and score request discovery, compressed payload decoding, 1X2 bookmaker odds normalization,
score normalization, and append-only odds logging. It is read-only and unauthenticated. Each
non-overlapping polling tick starts one odds operation and one score operation concurrently,
preserves either successful result when the other operation fails, and waits for both operations
to finish before the next tick. The score operation makes no HTTP call when no score URL was
discovered and otherwise makes one. The odds operation makes one primary HTTP call and may make
exactly one fallback call after a failed or empty primary response. Consequently, a cycle makes
one to three HTTP calls, normally two when a score URL exists and the primary odds request
succeeds. OddsPortal advertises an approximately 15-second upstream refresh, so one-second
polling cannot force new source data and may be rate-limited.

## Data Flow

```text
config.yaml match.home_team + match.away_team
                        |
                  src/config.rs
                        |
        inject the same pair into both providers
                  /                     \
                 v                       v
     Polymarket local task        OddsPortal local task
                 |                       |
     Gamma team-name search       tournament/H2H discovery
                 |                       |
       one football event         odds URL + score URL
                 |                       |
        +--------+--------+       one-second tick starts
        |                 |       both operations concurrently
        v                 v           (1-3 HTTP calls)
 market WebSocket   Sports WebSocket    /       \
        |                 |            v         v
 initial/changed      matching       odds      score
 Yes-token odds       event score      \         /
        |                 |        finish without overlap
        +--------+--------+                 |
                 |                     next tick
                 v                         |
       stdout observation JSONL <----------+
                 |
       polymarket_odds / polymarket_score /
       oddsportal_odds / oddsportal_score

All lifecycle, retry, discovery, reconnect, and failure diagnostics
go to stderr as provider-prefixed text.
```

Provider tasks are failure-isolated at orchestration level. A terminal Polymarket error is logged and supervised while a running OddsPortal polling task continues, and vice versa. The process exits with a combined error only after every enabled provider task has stopped.

Simulation-only order data flow:

```text
Latest QuoteState snapshot
    |
    v
Validated decimal buy intent
    |
    v
Mock OrderExecutor -> synthetic order ID
    |
    v
Validated decimal sell intent or one simulated cancellation
```

Gated live order data flow:

```text
config.yaml trade.enabled + three-mode gate
    |
    +-- enabled is false or any mode is not real --> read-only collection
    |
    v
Unique type: long account
    |
    v
Configured signer + L2 credentials + funder + proxy
    |
    v
Initial quote for first discovered token
    |
    v
One live GTC buy at 0.01 × 5
    |
    v
Immediate live GTC sell at 0.11 × 5
    |
    +-- sell failure --> one attempted buy cancellation
    |
    v
Market WebSocket reconnect loop (never repeats live flow)
```

## Runtime Output

Stdout is reserved for newline-delimited observation JSON objects. The four record types are
`polymarket_odds`, `polymarket_score`, `oddsportal_odds`, and `oddsportal_score`. Consumers can
distinguish receipt time from an optional provider timestamp through `received_at` and
`source_updated_at`. A pre-match or missing OddsPortal score resource produces an
`oddsportal_score` object with `available: false`; the Sports WebSocket may emit no
`polymarket_score` record before a match is live.

Human-readable diagnostics go to stderr. Every line starts with its UTC
emission time in RFC 3339 millisecond format, followed by a stable prefix:

- `[polymarket]` identifies market discovery, snapshots, WebSocket lifecycle, quotes, reconnects, and provider failures.
- `[oddsportal]` identifies polling startup, request retries, collected odds, pass status, and provider failures.
- `[trade]` identifies the separately gated live-order lifecycle.
- `[runtime]` identifies a task failure that cannot be attributed to a known provider.

For example:

```text
2026-06-30T12:34:56.789Z [oddsportal] starting collection pass
```

These prefixes never appear in stdout observation JSONL. Stdout records remain
pure JSON and carry `received_at`; provider quote JSONL files remain pure JSON,
carry `ts`, retain their existing record formats without prefixes, and do not
duplicate the score streams.

## External Integrations

- Polymarket Gamma API: paginated active-event discovery by configured team names and child market metadata.
- Polymarket CLOB API through `rs-clob-client-v2`: initial order book snapshots.
- Polymarket authenticated CLOB API through the same proxied client: gated GTC placement and single-order cancellation.
- Polymarket market WebSocket: live market updates.
- OddsPortal tournament and H2H pages: embedded state for match and request discovery.
- OddsPortal `requestPreMatch` `/match-event/...dat` endpoint: internal compressed pre-match odds payload. The collector does not discover or request an in-play odds feed.
- HTTP proxy default: `http://10.32.110.233:7890`.

The normal collection path remains unauthenticated and read-only. The explicitly gated live path reads test-account credentials from `config.yaml`, separates authenticated execution from public market-data collection, validates intents before submission, requires explicit placement/cancellation response confirmation, and never logs credential values or signed payloads.

## Development Workflow

- Use `cargo test` for verification after Rust changes.
- Keep provider tests with provider modules.
- Update this file when source ownership or provider boundaries change.
- Keep `AGENTS.md` aligned with practical contributor rules.
