# Polymarket 1X2 Architecture

## Project Identity

`polymarket-1x2` is a Rust command-line collector for Polymarket 1X2 quote data and OddsPortal football bookmaker odds. It discovers tokens for a configured Polymarket event, subscribes to market WebSocket updates, normalizes latest bid and ask values, collects configured OddsPortal 1X2 prices through the site's embedded JavaScript state and internal odds data response, and appends JSON lines to local log files.

## Source Tree

```text
src/
├── main.rs                  # Binary entry point and top-level orchestration
├── polymarket/              # Polymarket provider implementation
│   ├── mod.rs
│   ├── clob.rs              # rs-clob-client-v2 setup and order book adaptation
│   ├── config.rs            # Typed config.yaml loading, live gate, accounts, endpoints, and defaults
│   ├── discovery.rs         # Event slug extraction and Gamma event discovery
│   ├── live.rs              # Proxied authenticated client, live executor, and one-shot orchestration
│   ├── logging.rs           # Append-only quote JSONL logger
│   ├── models.rs            # Polymarket quote and token data structures
│   ├── order.rs             # Abstract executor and read-only order lifecycle simulation
│   ├── quotes.rs            # In-memory latest bid/ask state
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

`src/main.rs` installs the Rustls crypto provider, runs one OddsPortal collection pass for the configured match, loads `config.yaml`, derives the Polymarket market and optional live-account configuration, discovers the configured event, and starts the Polymarket market stream.

### Polymarket Provider

The Polymarket provider owns all Gamma API, CLOB REST, CLOB WebSocket, quote-state, and log-writing details. Its public surface is intentionally small: config creation, event discovery, and market stream execution.

The provider exposes an executor-independent order lifecycle with a deterministic local mock for tests. It also implements the executor boundary with a live `rs-clob-client-v2` adapter. Live execution is enabled only when `trader_mode`, `account_mode`, and `market_mode` are all `real`; it selects exactly one `type: long` account and uses its configured signer, L2 credentials, signature type, funder, host, chain, and the root proxy. The client never calls SDK credential creation or derivation methods because that dependency logs L1 authentication headers on its create path.

### OddsPortal Provider

The OddsPortal provider owns tournament/H2H page fetching, embedded state parsing, internal `.dat` odds request discovery, compressed payload decoding, 1X2 bookmaker odds normalization, and append-only odds logging. It is read-only and unauthenticated.

## Data Flow

```text
Default config
    |
    v
Polymarket URL slug extraction
    |
    v
Gamma event request through proxy
    |
    v
Child market token metadata
    |
    v
Initial CLOB order book snapshots
    |
    v
Market WebSocket subscription
    |
    v
QuoteRecord normalization
    |
    v
logs/polymarket_quotes.log
```

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
config.yaml three-mode gate
    |
    +-- any mode not real --> read-only collection
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

OddsPortal data flow:

```text
Default OddsPortal config
    |
    v
Tournament page embedded state parsing
    |
    v
Norway - France H2H URL and event hash
    |
    v
H2H page requestPreMatch metadata
    |
    v
Internal /match-event/...dat request
    |
    v
Decoded odds JSON
    |
    v
1X2 bookmaker odds normalization
    |
    v
logs/oddsportal_odds.log
```

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
