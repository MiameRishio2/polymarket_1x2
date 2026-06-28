# Comet Design Handoff

- Change: configure-trade-and-concurrent-provider-logging
- Phase: design
- Mode: compact
- Context hash: 0e47804fd7b34825d87781499a23abda525b5165a05047589f3fd06e2f1b020e

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/configure-trade-and-concurrent-provider-logging/proposal.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/proposal.md
- Lines: 1-50
- SHA256: d32be6564773ec67a549d7ef5c569b49695beec5e44eafc35bb42c67fab0b3ce

```md
## Why

The binary currently completes one OddsPortal collection before it starts the long-running
Polymarket stream, so operators cannot observe both providers running at the same time and a
slow or failing OddsPortal request delays all Polymarket output. Trading is also inferred only
from the three `real` mode values instead of being an independently enabled feature, while the
provider targets needed for operational testing are not represented together in `config.yaml`.

## What Changes

- Add explicit runtime sections in `config.yaml` for Polymarket and OddsPortal collection,
  including enable flags, provider targets, log paths, and an OddsPortal polling interval.
- Add `trade.enabled` as a separate, fail-closed live-trading switch; authenticated trading is
  allowed only when it is enabled and the existing three-mode `real` gate also passes.
- Start enabled Polymarket and OddsPortal collectors concurrently so either provider can emit
  records without waiting for the other.
- Add provider-prefixed startup, progress, retry, and failure output so the managed process log
  shows what each provider is doing even before quote or odds records arrive.
- Configure the supplied Australia–Egypt World Cup Polymarket URL as the test target and verify
  that Polymarket and OddsPortal output can be observed in the same process log.
- Preserve provider-specific parsing, transport, and JSONL logging under their existing source
  subtrees; keep top-level task orchestration in `src/main.rs`.

## Capabilities

### New Capabilities

- `provider-runtime-orchestration`: Root configuration loading, independent feature gates,
  concurrent provider task supervision, and provider-attributed operational output.

### Modified Capabilities

- `polymarket-live-trading`: Require an explicit independent trade enable flag in addition to
  the existing three-mode live gate.
- `polymarket-ws-quotes`: Load the Polymarket event URL and quote-log destination from root
  configuration and expose provider-attributed lifecycle output while collecting.
- `oddsportal-js-odds`: Load OddsPortal targets and logging settings from root configuration and
  support repeated collection while the Polymarket stream runs.

## Impact

- `config.yaml` gains structured provider runtime settings and an explicit trade enable flag.
- `src/config.rs` becomes the shared root YAML loader and runtime feature assembler.
- `src/main.rs` changes from sequential startup to concurrent orchestration.
- `src/polymarket/config.rs` retains account, credential, trade-gate, and Polymarket runtime
  conversion, while authenticated trading remains isolated under `src/polymarket/`.
- `src/oddsportal/config.rs` gains deserializable runtime settings and polling behavior remains
  provider-local.
- `ARCHITECTURE.md` and `DEPLOYMENT.md` must describe the new configuration, concurrent process
  behavior, provider-prefixed output, and log paths.
```

## openspec/changes/configure-trade-and-concurrent-provider-logging/design.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/design.md
- Lines: 1-162
- SHA256: 9e385160f8bae60c0c58cdfbcd2527e929c609ed7a28674c9e88124dc1d112d9

[TRUNCATED]

```md
## Context

`main` currently invokes `oddsportal::collect_once(...).await` before it loads the Polymarket
configuration. OddsPortal network latency therefore blocks Polymarket discovery and WebSocket
startup. After the one OddsPortal pass completes, only the Polymarket future remains alive, so
the process cannot produce fresh OddsPortal records alongside WebSocket quote records.

The root YAML parser is owned by `src/polymarket/config.rs` because it was introduced for
authenticated Polymarket execution. It currently creates a Polymarket market-data config from
hard-coded defaults and derives live-trading activation solely from three mode strings.
OddsPortal uses `Config::default()` directly and cannot be configured from the file.

Provider implementation boundaries in `ARCHITECTURE.md` must remain intact. Top-level lifecycle
coordination belongs in `src/main.rs`; provider-specific configuration conversion, network
requests, parsing, and JSONL writing remain in each provider subtree. Trading must remain
fail-closed, credential-safe, and one-shot.

## Goals / Non-Goals

**Goals:**

- Represent enabled Polymarket collection, enabled OddsPortal collection, and enabled live
  trading as three independently configured runtime features.
- Start the two enabled collectors without a sequential dependency.
- Keep an OddsPortal polling loop alive while the Polymarket WebSocket stream runs.
- Make startup and provider lifecycle visible in the shared stdout/stderr process log with
  unambiguous provider prefixes.
- Make the Australia–Egypt Polymarket event URL and matching OddsPortal target configurable and
  testable without enabling authenticated writes.
- Preserve the existing provider-local JSONL formats and paths by default.

**Non-Goals:**

- Combining Polymarket and OddsPortal records into one file or introducing a shared record model.
- Adding order placement for OddsPortal or changing the fixed Polymarket live-order strategy.
- Building a general-purpose task scheduler, logging framework, or dynamic configuration reload.
- Retrying live trading after any startup or stream failure.

## Decisions

### 1. Root YAML owns feature selection; provider modules own typed provider settings

The root file configuration will deserialize these explicit sections:

```yaml
polymarket:
  enabled: true
  url: https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03
  log_path: logs/polymarket_quotes.log

oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  home_team: Australia
  away_team: Egypt
  log_path: logs/oddsportal_odds.log
  poll_interval_seconds: 30

trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
```

The single root loader moves to `src/config.rs` because accounts, proxy, both providers, and
trade gates must be assembled together. It delegates credential and trade validation to
`src/polymarket/config.rs` and OddsPortal settings conversion to `src/oddsportal/config.rs`.
This is the narrow shared module permitted by the architecture because both providers consume
the same root configuration and proxy. Missing provider sections retain existing collector
defaults and remain enabled for compatibility. Missing `trade.enabled` is treated as `false`
because live writes require explicit authorization.

Alternatives considered: keeping the cross-provider root type under `src/polymarket/`, or parsing
the same YAML a second time in OddsPortal. Both were rejected because they either violate the
provider boundary or duplicate parsing and validation.

### 2. `trade.enabled` is an additional gate, not a replacement gate

Live configuration is produced only when `trade.enabled == true` and all three existing modes
```

Full source: openspec/changes/configure-trade-and-concurrent-provider-logging/design.md

## openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/tasks.md
- Lines: 1-30
- SHA256: 576a83f679f2330d318d3eed445cb050ff9d0ae3de9f503dd1a81d4ddd1392c1

```md
## 1. Configuration Contracts

- [ ] 1.1 Add failing tests for nested provider configuration, the exact localized Australia–Egypt URL, provider enable defaults, and positive OddsPortal polling intervals.
- [ ] 1.2 Add failing tests proving `trade.enabled` defaults false and is required in addition to all three `real` modes.
- [ ] 1.3 Implement root-to-provider configuration conversion and validation without exposing credential fields.

## 2. Concurrent Provider Runtime

- [ ] 2.1 Add failing orchestration tests for enabled-provider selection, no-provider rejection, and independent task completion handling.
- [ ] 2.2 Implement the OddsPortal polling loop with success/error reporting, interval waits, and continued polling after a failed pass.
- [ ] 2.3 Replace sequential startup with independently spawned enabled provider tasks that do not cancel each other on one provider's failure.

## 3. Provider-Attributed Output

- [ ] 3.1 Add focused tests for provider-labelled lifecycle formatting where deterministic assertions are practical.
- [ ] 3.2 Prefix Polymarket startup, discovery, snapshots, subscription, updates, reconnects, and failures without changing quote JSONL records.
- [ ] 3.3 Prefix OddsPortal startup, pass status, records, retries, and failures without changing odds JSONL records.
- [ ] 3.4 Prefix live-order lifecycle output separately and verify diagnostics still redact secrets.

## 4. Operational Configuration and Documentation

- [ ] 4.1 Update `config.yaml` with enabled Australia–Egypt provider targets, separate log paths, a 30-second OddsPortal interval, and `trade.enabled: false`.
- [ ] 4.2 Update `ARCHITECTURE.md` for root configuration ownership and concurrent provider data flow.
- [ ] 4.3 Update `DEPLOYMENT.md` for feature gates and simultaneous provider output inspection.

## 5. Verification

- [ ] 5.1 Run formatting and the complete Rust test suite.
- [ ] 5.2 Build the release binary and run a bounded, trading-disabled smoke test against the supplied Polymarket URL.
- [ ] 5.3 Verify the captured process output contains both `[polymarket]` and `[oddsportal]` lines and verify Polymarket records reach its configured JSONL file.
```

## openspec/changes/configure-trade-and-concurrent-provider-logging/specs/oddsportal-js-odds/spec.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/specs/oddsportal-js-odds/spec.md
- Lines: 1-35
- SHA256: a5bdc4edd221aeb970f4fff16fdc8dfc445aff276265506a8f92b3d2023cb91a

```md
## ADDED Requirements

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, home team, away team, JSONL
path, and positive polling interval from the `oddsportal` section of `config.yaml`, while using
the root proxy setting for HTTP requests.

#### Scenario: Australia Egypt target is configured
- **WHEN** the configured home team is Australia and away team is Egypt
- **THEN** each collection pass searches the configured tournament state for Australia - Egypt

#### Scenario: Polling interval is invalid
- **WHEN** `oddsportal.poll_interval_seconds` is zero
- **THEN** configuration validation fails before an OddsPortal task is spawned

### Requirement: Repeated OddsPortal collection
The system SHALL run OddsPortal collection repeatedly at the configured interval while its
provider task remains enabled and SHALL append every successful pass to the provider-local JSONL
log.

#### Scenario: Collection pass succeeds
- **WHEN** a pass normalizes and logs one or more 1X2 records
- **THEN** the task reports the completed pass, waits for the configured interval, and starts another pass

#### Scenario: Collection pass fails
- **WHEN** discovery, request, decoding, normalization, or logging fails
- **THEN** the task reports the contextual error, waits for the configured interval, and starts another pass

### Requirement: Visible OddsPortal lifecycle
The system SHALL emit `[oddsportal]`-prefixed process output for startup, pass completion, odds
records, retries, and failures.

#### Scenario: Polymarket is also running
- **WHEN** an OddsPortal polling pass executes while Polymarket waits for WebSocket messages
- **THEN** the shared process output unambiguously attributes OddsPortal progress and records to OddsPortal
```

## openspec/changes/configure-trade-and-concurrent-provider-logging/specs/polymarket-live-trading/spec.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/specs/polymarket-live-trading/spec.md
- Lines: 1-19
- SHA256: 991bfc80c403312788f527621db9089d5ead8f79c063bb3577e615788866d138

```md
## MODIFIED Requirements

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
```

## openspec/changes/configure-trade-and-concurrent-provider-logging/specs/polymarket-ws-quotes/spec.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/specs/polymarket-ws-quotes/spec.md
- Lines: 1-39
- SHA256: 10a09f32707e17b5c624f19696546bf205554002131ac5d8747d63f73abbcad3

```md
## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket quote discovery, initial order book loading, WebSocket
subscription, quote normalization, and append-only logging under `src/polymarket/`. The root
orchestration layer SHALL pass the configured Polymarket event URL and quote-log path into this
provider-local workflow and SHALL start it concurrently with enabled OddsPortal collection.

#### Scenario: Running the executable with both providers configured
- **WHEN** the executable starts with both providers enabled and the Australia–Egypt Polymarket event URL configured
- **THEN** it uses modules under `src/polymarket/` to discover that event, load and log initial quotes, and subscribe to its tokens without waiting for OddsPortal collection to finish

## ADDED Requirements

### Requirement: Configurable Polymarket collection target
The system SHALL load the Polymarket enabled flag, website event URL, and quote JSONL path from
the `polymarket` section of `config.yaml`, with existing provider defaults when the section is
absent.

#### Scenario: Localized event URL is configured
- **WHEN** `polymarket.url` is `https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03`
- **THEN** discovery extracts `fifwc-aus-egy-2026-07-03` and requests that Gamma event

#### Scenario: Quote log path is configured
- **WHEN** `polymarket.log_path` names a writable local path
- **THEN** initial and WebSocket quote records are appended to that path

### Requirement: Visible Polymarket lifecycle
The system SHALL emit `[polymarket]`-prefixed process output for startup, event discovery,
initial quote records, WebSocket subscription, quote updates, disconnections, and terminal
errors.

#### Scenario: No WebSocket update has arrived
- **WHEN** event discovery and initial CLOB snapshots succeed
- **THEN** the process log already contains Polymarket-attributed lifecycle and initial quote output

#### Scenario: WebSocket reconnects
- **WHEN** a market WebSocket connection fails or closes
- **THEN** the process log identifies the Polymarket connection failure or reconnect attempt
```

## openspec/changes/configure-trade-and-concurrent-provider-logging/specs/provider-runtime-orchestration/spec.md

- Source: openspec/changes/configure-trade-and-concurrent-provider-logging/specs/provider-runtime-orchestration/spec.md
- Lines: 1-54
- SHA256: 1e76279cea3d2951c1c624968a1eb6121894f22cec88f5e59cca45db096ad1c9

```md
## ADDED Requirements

### Requirement: Independently configurable runtime features
The system SHALL load Polymarket collection, OddsPortal collection, and live trading as
independently enabled features from `config.yaml`. It SHALL fail startup with a clear diagnostic
when both provider collectors are disabled.

#### Scenario: Both collectors enabled
- **WHEN** `polymarket.enabled` and `oddsportal.enabled` are both `true`
- **THEN** the system starts one Polymarket collector task and one OddsPortal collector task

#### Scenario: One collector enabled
- **WHEN** exactly one provider `enabled` value is `true`
- **THEN** the system starts only that provider and does not require the disabled provider's runtime target

#### Scenario: No collector enabled
- **WHEN** both provider `enabled` values are `false`
- **THEN** startup fails before spawning any provider task

### Requirement: Concurrent provider startup
The system SHALL start enabled Polymarket and OddsPortal collection without waiting for either
provider to finish a network request or collection pass before the other provider starts.

#### Scenario: OddsPortal request is slow
- **WHEN** the OddsPortal collector is waiting for an HTTP response
- **THEN** Polymarket discovery and WebSocket startup can proceed concurrently

#### Scenario: Polymarket stream remains connected
- **WHEN** the Polymarket collector is waiting for WebSocket messages
- **THEN** the OddsPortal collector continues its configured polling passes

### Requirement: Provider task failure isolation
The system SHALL report an enabled provider task's terminal failure without cancelling another
provider task that is still running.

#### Scenario: Polymarket task exits with an error
- **WHEN** the OddsPortal polling task is still running
- **THEN** the system emits a Polymarket-attributed terminal error and leaves OddsPortal running

#### Scenario: OddsPortal pass fails
- **WHEN** an OddsPortal collection pass returns an error
- **THEN** the polling task emits an OddsPortal-attributed error, waits for the configured interval, and tries another pass without interrupting Polymarket

### Requirement: Provider-attributed process output
The system SHALL identify every collector startup, progress, retry, record, and failure line in
the shared process output as Polymarket, OddsPortal, or trade output without printing secrets.

#### Scenario: Process starts both providers
- **WHEN** both collectors are enabled
- **THEN** the process output contains target-safe startup lines prefixed `[polymarket]` and `[oddsportal]`

#### Scenario: Both providers emit records
- **WHEN** Polymarket normalizes a quote and OddsPortal normalizes bookmaker odds
- **THEN** their process-output lines carry distinct provider prefixes while their JSONL records remain in separate configured files
```
