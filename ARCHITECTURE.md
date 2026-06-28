# Polymarket 1X2 Architecture

## Project Identity

`polymarket-1x2` is a Rust command-line collector for Polymarket 1X2 quote data and OddsPortal football bookmaker odds. It discovers tokens for a configured Polymarket event, subscribes to market WebSocket updates, normalizes latest bid and ask values, collects configured OddsPortal 1X2 prices through the site's embedded JavaScript state and internal odds data response, and appends JSON lines to local log files.

## Source Tree

```text
src/
├── main.rs                  # Binary entry point and top-level orchestration
├── config.rs                # Root config.yaml ownership and provider runtime construction
├── polymarket/              # Polymarket provider implementation
│   ├── mod.rs
│   ├── clob.rs              # rs-clob-client-v2 setup and order book adaptation
│   ├── config.rs            # Polymarket runtime types, live gate, accounts, endpoints, and defaults
│   ├── discovery.rs         # Event slug extraction and Gamma event discovery
│   ├── live.rs              # Proxied authenticated client, live executor, and one-shot orchestration
│   ├── logging.rs           # Append-only quote JSONL logger
│   ├── models.rs            # Polymarket quote and token data structures
│   ├── order.rs             # Abstract executor and read-only order lifecycle simulation
│   ├── quotes.rs            # In-memory latest bid/ask state
│   ├── sports.rs            # Sports score WebSocket parsing and reconnect loop
│   └── ws.rs                # Market WebSocket subscription and message parsing
└── oddsportal/              # OddsPortal provider implementation
    ├── mod.rs
    ├── config.rs            # OddsPortal URLs, target match, user agent, and log path defaults
    ├── decoder.rs           # Internal .dat payload decoding
    ├── discovery.rs         # Tournament/H2H embedded state parsing
    ├── logging.rs           # Append-only OddsPortal JSONL logger
    ├── models.rs            # OddsPortal match, request, and odds record structures
    └── odds.rs              # 1X2 bookmaker odds normalization
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

Both provider enable flags default to `true` when omitted. Live trading fails closed: `trade.enabled` defaults to `false`, and live configuration is constructed only when it is explicitly `true` and `trade.trader_mode`, `trade.account_mode`, and `trade.market_mode` are all `real`. Therefore, an older configuration containing only the three `real` modes migrates to read-only collection until `trade.enabled: true` is deliberately added.

### Polymarket Provider

The Polymarket provider owns all Gamma API, CLOB REST, CLOB WebSocket, Sports WebSocket,
quote-state, and log-writing details. Its public surface is intentionally small: config creation
and dual-stream execution after one event discovery pass.

The provider exposes an executor-independent order lifecycle with a deterministic local mock for tests. It also implements the executor boundary with a live `rs-clob-client-v2` adapter. Live execution requires `trade.enabled: true` in addition to `trader_mode`, `account_mode`, and `market_mode` all being `real`; it selects exactly one `type: long` account and uses its configured signer, L2 credentials, signature type, funder, host, chain, and the root proxy. The client never calls SDK credential creation or derivation methods because that dependency logs L1 authentication headers on its create path.

### OddsPortal Provider

The OddsPortal provider owns tournament/H2H page fetching, embedded state parsing, internal `.dat` odds request discovery, compressed payload decoding, 1X2 bookmaker odds normalization, and append-only odds logging. It is read-only and unauthenticated. Its polling loop uses the root-configured positive interval, reports each pass with the provider prefix, and retries on the next interval after a failed pass instead of terminating.

## Data Flow

```text
config.yaml -> src/config.rs -> enabled provider runtimes
                              |
                LocalSet + supervised JoinSet
                    /                     \
                   v                       v
       Polymarket local task        OddsPortal local task
                   |                       |
       URL slug extraction          configured polling loop
                   |                       |
       Gamma event discovery        tournament/H2H discovery
                   |                       |
       token metadata and           internal .dat request
       initial CLOB snapshots               |
                   |                decode and normalize 1X2
          +--------+--------+               |
          |                 |       append OddsPortal JSONL
    market WebSocket  Sports WebSocket      |
          |                 |       wait configured interval
    quote records      score records        |
          |                 |         repeat after success
          +--------+--------+            or failed pass
                   |
       reconnect each disconnected stream
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

Human-readable process output uses three stable prefixes:

- `[polymarket]` identifies market discovery, snapshots, WebSocket lifecycle, quotes, reconnects, and provider failures.
- `[oddsportal]` identifies polling startup, request retries, collected odds, pass status, and provider failures.
- `[trade]` identifies the separately gated live-order lifecycle.

These prefixes apply to process output only. The provider JSONL files retain their existing record formats without prefixes.

## External Integrations

- Polymarket website URL: source for the configured event slug.
- Polymarket Gamma API: event and child market metadata.
- Polymarket CLOB API through `rs-clob-client-v2`: initial order book snapshots.
- Polymarket authenticated CLOB API through the same proxied client: gated GTC placement and single-order cancellation.
- Polymarket market WebSocket: live market updates.
- OddsPortal tournament and H2H pages: embedded state for match and request discovery.
- OddsPortal `/match-event/...dat` endpoint: internal compressed pre-match odds payload.
- HTTP proxy default: `http://10.32.110.233:7890`.

The normal collection path remains unauthenticated and read-only. The explicitly gated live path reads test-account credentials from `config.yaml`, separates authenticated execution from public market-data collection, validates intents before submission, requires explicit placement/cancellation response confirmation, and never logs credential values or signed payloads.

## Development Workflow

- Use `cargo test` for verification after Rust changes.
- Keep provider tests with provider modules.
- Update this file when source ownership or provider boundaries change.
- Keep `AGENTS.md` aligned with practical contributor rules.
