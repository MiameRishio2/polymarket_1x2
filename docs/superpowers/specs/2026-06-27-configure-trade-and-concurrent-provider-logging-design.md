---
comet_change: configure-trade-and-concurrent-provider-logging
role: technical-design
canonical_spec: openspec
---

# Configurable Trade and Concurrent Provider Logging

## Design Boundary

OpenSpec is the canonical behavioral specification. This document defines the implementation
shape for root configuration, provider concurrency, lifecycle output, and tests without
duplicating the requirements.

The current sequential dependency is:

```text
OddsPortal collect_once
        |
        v
load Polymarket config -> discover event -> snapshots -> WebSocket loop
```

The target runtime is:

```text
                       +-> Polymarket discover -> snapshots -> WebSocket loop
config.yaml -> runtime |
                       +-> OddsPortal collect -> wait -> collect -> ...
```

Trading is an optional branch inside the Polymarket task. It is not a third collector task.

## Module Responsibilities

### `src/config.rs`: root application configuration

Introduce one shared root configuration module because both providers consume the same YAML
document and root proxy.

It owns:

- reading and deserializing `config.yaml`;
- provider `enabled` selection;
- rejecting a zero-provider runtime;
- assembling optional provider runtime configs;
- delegating Polymarket account/trade validation to `polymarket::config`;
- delegating OddsPortal target/poll validation to `oddsportal::config`.

It does not expose secrets through `Debug`, print configuration values, construct network
clients, or implement provider behavior. Credential wrapper types, accounts, the live gate, and
`LiveConfig` remain under `src/polymarket/`.

The assembled runtime has this conceptual shape:

```rust
struct RuntimeConfig {
    polymarket: Option<PolymarketRuntime>,
    oddsportal: Option<OddsPortalRuntime>,
}

struct PolymarketRuntime {
    market: polymarket::config::Config,
    live: Option<polymarket::config::LiveConfig>,
}

struct OddsPortalRuntime {
    collector: oddsportal::config::Config,
    poll_interval: Duration,
}
```

The exact raw YAML structs remain private. Provider section defaults are enabled to preserve
collection behavior when older configuration omits the new sections. `trade.enabled` is a
separate boolean with a false default.

### `src/polymarket/config.rs`: Polymarket and trade conversion

The provider module owns:

- website URL and quote-log defaults;
- account and redacted secret types;
- live-trading gate and credential validation;
- conversion from root Polymarket settings to market and optional live configs.

The effective live gate is:

```text
trade.enabled
    AND trader_mode == real
    AND account_mode == real
    AND market_mode == real
```

No account selection, private-key access, or L2 credential validation occurs when this
expression is false. This keeps the Australia–Egypt smoke test read-only even if the legacy mode
fields remain `real`.

### `src/oddsportal/config.rs`: OddsPortal runtime conversion

The provider module adds deserializable settings for its enabled state, tournament URL, teams,
log path, and polling interval. It validates `poll_interval_seconds > 0` and receives the root
proxy during runtime conversion.

### `src/oddsportal/mod.rs`: polling lifecycle

Keep `collect_once` as the single-pass primitive. Add a long-running polling entry point that:

1. emits a provider-attributed pass-start line;
2. awaits `collect_once`;
3. emits record-count success or contextual failure;
4. waits for the configured interval;
5. repeats.

Both success and failure take the same interval path so a fast failure cannot create a busy
retry loop. The first pass runs immediately.

For deterministic tests, isolate the loop driver from the production collector and sleeper
behind private generic callbacks or a small internal trait. Tests can stop after a bounded
number of attempts and prove that a failure is followed by another attempt. No public generic
API is required.

### `src/main.rs`: provider supervision

After crypto-provider installation, load and validate the complete runtime before spawning
tasks. Print one target-safe startup line per enabled provider. Do not print account structures,
credential values, or signed payloads.

Use `tokio::task::JoinSet` with task outputs that carry a provider identity and `Result<()>`:

```rust
JoinSet<(Provider, anyhow::Result<()>)>
```

Spawn the Polymarket discovery/stream future and OddsPortal polling future before awaiting
either. Loop on `join_next()`:

- a normal provider error is printed with that provider's prefix;
- a normal unexpected success is printed as a terminal exit;
- a task panic/cancellation is reported as a runtime task failure;
- the loop continues while another provider task remains.

This meets failure isolation without automatic task restart. OddsPortal pass failures are
non-terminal and handled inside its loop. Polymarket WebSocket reconnects remain inside the
existing stream loop. A pre-stream Polymarket discovery failure is terminal for that provider,
but does not cancel OddsPortal.

If all spawned tasks terminate, `main` returns an error so deployment status does not claim a
healthy collector process. Normal stop behavior remains process-level `SIGTERM`.

## Provider Output Contract

Continue using one `println!` or `eprintln!` call per line; no logging dependency is added.

Prefixes are stable:

- `[polymarket]` for discovery, initial snapshots, subscriptions, quote updates, reconnects, and
  terminal provider errors;
- `[oddsportal]` for pass starts, records, pass summaries, retries, and terminal provider errors;
- `[trade]` for gated live-flow status only.

Initial Polymarket snapshots are printed and appended before WebSocket connection, so a quiet
market still proves that Polymarket reached the quote path. Provider JSONL serialization is not
changed.

## Runtime Data and Failure Flow

```text
read config.yaml
      |
      +-- invalid YAML / no providers / zero poll interval --> fail before spawn
      |
      +-- trade disabled --> skip account and credential validation
      |
      +-- trade enabled + incomplete modes/credentials --> fail before spawn
      |
      v
spawn all enabled providers
      |
      +-- Polymarket setup error --> [polymarket] terminal error; OddsPortal continues
      |
      +-- OddsPortal pass error --> [oddsportal] error; wait; retry
      |
      +-- WebSocket error --> [polymarket] reconnect; trade does not repeat
```

Live trading stays inside `run_market_stream` after initial books load and before the WebSocket
reconnect loop. The supervisor never restarts the Polymarket task, preserving the existing
at-most-once order-flow guarantee.

## Testing Strategy

Implementation follows test-first slices.

### Configuration tests

- exact `/ja/sports/world-cup/fifwc-aus-egy-2026-07-03` slug extraction;
- provider sections override URLs, teams, log paths, and interval;
- omitted provider sections use enabled defaults;
- both disabled is rejected;
- zero OddsPortal interval is rejected;
- missing or false `trade.enabled` bypasses all account validation;
- explicit enable plus three `real` modes validates and selects exactly one long account;
- secret debug output remains redacted.

### Polling and supervision tests

- the polling driver runs immediately;
- success waits and runs another pass;
- failure waits and runs another pass;
- enabled-provider selection spawns the expected identities;
- an observed provider completion does not abort the remaining task;
- no-provider selection fails before task creation.

Tests use closures/channels and controlled completion rather than external HTTP or WebSocket
connections.

### Provider output tests

Use focused formatter/helper assertions only where extraction improves determinism. Keep
record-loop behavior covered by existing parsing and logger tests. Confirm that JSONL payloads
do not acquire process-log prefixes.

### Full verification

Run:

```text
cargo fmt --check
cargo test
./scripts/build.sh
```

Then run a bounded smoke test with both providers enabled and `trade.enabled: false`. The
configured Polymarket URL is the supplied Australia–Egypt event. Verification requires:

- `[polymarket]` and `[oddsportal]` lines in captured stdout/stderr;
- Polymarket initial snapshot output even if no later WebSocket update arrives;
- at least one Polymarket JSONL record at the configured path;
- no `[trade]` placement output.

External network or proxy failure is reported as a failed live verification, not treated as
equivalent to deterministic test success.

## Documentation and Migration

Update `ARCHITECTURE.md` because a shared root configuration module and concurrent provider data
flow are added. Update `DEPLOYMENT.md` because feature gates and process-log interpretation
change.

Existing configs without provider sections continue to start both collectors with defaults.
Existing configs do not trade until `trade.enabled: true` is added. This intentional fail-closed
migration must be explicit in deployment documentation.

No dependency, persistent data format, JSONL schema, deployment script, or process path changes.
