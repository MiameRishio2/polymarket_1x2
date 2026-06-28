# Comet Design Handoff

- Change: discover-match-by-team-names
- Phase: design
- Mode: compact
- Context hash: 619bdbf9f0490a7702c8d45f04498e3429cff8f3313f9181b9540127d1a5eed1

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/discover-match-by-team-names/proposal.md

- Source: openspec/changes/discover-match-by-team-names/proposal.md
- Lines: 1-43
- SHA256: 499df3981084e46fb24e4daac7715b9af7f78dd2a5166d45b735ab82b8d73225

```md
## Why

The two collectors currently use separate target configuration: OddsPortal accepts team names,
while Polymarket requires an exact event URL. Operators should be able to name one football match
once and have both providers discover and monitor the corresponding markets without manually
finding provider-specific URLs.

## What Changes

- Add one shared configured home/away team pair for the monitored football match.
- Discover the matching Polymarket event automatically from the configured team names instead of
  requiring an exact Polymarket event URL.
- Use the same team pair for OddsPortal match discovery and request its 1X2 odds and score
  independently once per second.
- Keep Polymarket on its existing CLOB WebSocket subscription and add its public Sports WebSocket
  for score updates.
- Print four independent JSON record types for Polymarket odds, Polymarket scores, OddsPortal odds,
  and OddsPortal scores without cross-provider aggregation.
- Configure the committed target as South Africa versus Canada with trading still disabled.
- Preserve append-only provider JSONL logging and all live-trading safety gates.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `polymarket-ws-quotes`: Discover the configured football event from team names, emit structured
  Yes-token quote records, and stream the corresponding Polymarket score.
- `oddsportal-js-odds`: Consume the shared team pair and independently collect and emit the
  corresponding 1X2 bookmaker prices and score at a one-second interval.
- `provider-runtime-orchestration`: Supply one validated team pair to both provider collectors
  while preserving provider boundaries and failure isolation.

## Impact

The change affects root configuration conversion and orchestration, Polymarket event discovery,
OddsPortal request discovery, command-line output, focused provider/configuration tests,
`config.yaml`, and synchronized architecture/user documentation. It does not place orders, expose
credentials, correlate provider records, change provider JSONL record formats, or replace a
Polymarket WebSocket with HTTP polling.
```

## openspec/changes/discover-match-by-team-names/design.md

- Source: openspec/changes/discover-match-by-team-names/design.md
- Lines: 1-125
- SHA256: 50a9f9515d41dd63303188651ef365d13c16b1c0bca3464f4c12d840ba58eff9

[TRUNCATED]

```md
## Context

The runtime currently represents the same match twice: OddsPortal receives `home_team` and
`away_team`, while Polymarket receives a manually copied event URL. OddsPortal repeatedly fetches
bookmaker odds, and Polymarket prints CLOB quote changes from an in-task `QuoteState`. Neither
provider currently emits score observations in a stable machine-readable console format.

The operator wants raw observations for independent downstream analysis, not a cross-provider
aggregate. OddsPortal odds and score are separate HTTP resources. Polymarket provides separate
public CLOB and Sports WebSockets.

## Goals / Non-Goals

**Goals:**

- Configure South Africa versus Canada once as a shared home/away pair.
- Discover one Polymarket football event and one OddsPortal football match from that pair.
- Poll OddsPortal odds and score independently once per second.
- Stream Polymarket Yes-token quotes and scores without HTTP polling.
- Emit four independent timestamped JSON record types suitable for latency analysis.
- Fail safely when event discovery is missing or ambiguous.

**Non-Goals:**

- Aggregating, correlating, ranking, or comparing provider records in the collector.
- Placing, signing, or changing live orders.
- Calculating arbitrage or converting bookmaker odds to implied probabilities.
- Replacing provider JSONL files or scraping rendered OddsPortal DOM rows.

## Decisions

### One root-owned match target

Add a required root match target containing `home_team` and `away_team`, validate both names, and
inject cloned values into each enabled provider runtime. Provider sections retain
transport-specific settings such as the OddsPortal tournament URL and provider log paths.

Keeping separate team fields in both provider sections was rejected because they can drift.
Making Polymarket read OddsPortal configuration was rejected because it violates provider
ownership.

### Strict Polymarket name discovery

At startup, query paginated active, non-closed Gamma events. Normalize case, whitespace, the
separator, and the optional period after `vs`, then find the configured names in either displayed
order. Accept only a unique event with football 1X2 market shape: one home-win question, one draw
question, and one away-win question. Missing or ambiguous matches are contextual startup errors.

Guessing a slug from abbreviations and a date was rejected because the two names provide neither
stable abbreviations nor the match date. Retaining the exact-slug response parser as an internal
helper keeps existing fixture coverage without retaining URL configuration.

### Four provider-local observation streams

Polymarket retains the CLOB stream and adds the unauthenticated Sports WebSocket. It filters the
global sports feed by the discovered event slug, responds to `ping` with `pong`, and reconnects
without terminating the CLOB stream. Quote output includes only the Yes token for the classified
home, draw, or away market.

OddsPortal H2H metadata discovery extracts both `requestPreMatch` and `updateScoreRequest`. Each
poll tick requests the odds and score resources concurrently. Results are handled independently:
one success is emitted even if the other request fails.

Root orchestration passes configuration and supervises providers but does not receive, merge, or
store their observations.

### JSON stdout and diagnostic stderr

Data records are complete JSON objects written one per stdout line:

- `polymarket_odds`
- `polymarket_score`
- `oddsportal_odds`
- `oddsportal_score`

Every record carries provider, type, event identity, configured team names, and an RFC 3339
`received_at` assigned immediately after parsing. Source timestamps are preserved as
`source_updated_at` when supplied. Polymarket odds records identify `home`, `draw`, or `away` and
carry Yes-token bid/ask prices and sizes. One OddsPortal odds record contains all bookmakers and
their available `1`, `X`, and `2` values for that pass.
```

Full source: openspec/changes/discover-match-by-team-names/design.md

## openspec/changes/discover-match-by-team-names/tasks.md

- Source: openspec/changes/discover-match-by-team-names/tasks.md
- Lines: 1-42
- SHA256: dd226128ffcf93a971468f7d6d57e941d51b4038262458c686642a7fd6f3583f

```md
## 1. Shared Match Configuration

- [ ] 1.1 Add failing configuration tests for one validated shared team pair, provider injection,
  and the committed South Africa/Canada one-second target.
- [ ] 1.2 Implement root match configuration, remove duplicated provider target ownership, and
  update `config.yaml` without weakening live-trading gates.

## 2. Polymarket Name Discovery

- [ ] 2.1 Add fixture-driven tests for paginated active-event responses, normalized reversed team
  order, football 1X2 classification, no match, and ambiguous matches.
- [ ] 2.2 Implement team-name Gamma discovery and retain exact-slug response parsing as a focused
  helper.

## 3. Polymarket Observation Streams

- [ ] 3.1 Add serialization and filtering tests for home/draw/away Yes-token odds JSON records.
- [ ] 3.2 Emit initial and changed Yes-token CLOB observations as JSON stdout while preserving
  quote JSONL logging.
- [ ] 3.3 Add fixture and protocol tests for Sports WebSocket score parsing, slug filtering,
  heartbeat response, and reconnect behavior.
- [ ] 3.4 Implement the provider-local Sports WebSocket score stream independently of the CLOB
  stream.

## 4. OddsPortal Dual Polling

- [ ] 4.1 Add H2H fixture tests for independent odds and score request URL discovery.
- [ ] 4.2 Add decoder and serialization tests for available and pre-match-unavailable score
  responses plus grouped all-bookmaker 1X2 JSON output.
- [ ] 4.3 Add paused-time tests proving one-second non-overlapping cycles, concurrent odds/score
  requests, and preservation of either successful side when the other fails.
- [ ] 4.4 Implement independent odds and score collection with JSON stdout and unchanged odds
  JSONL logging.

## 5. Output, Documentation, and Verification

- [ ] 5.1 Route machine-readable observations to stdout and provider-prefixed diagnostics to
  stderr, with focused process-output tests.
- [ ] 5.2 Synchronize `ARCHITECTURE.md`, configuration examples, and user-facing runtime/output
  documentation with name discovery, dual score feeds, and output schemas.
- [ ] 5.3 Run formatting, focused tests, full `cargo test`, strict OpenSpec validation, and a
  bounded trading-disabled South Africa/Canada smoke test.
```

## openspec/changes/discover-match-by-team-names/specs/oddsportal-js-odds/spec.md

- Source: openspec/changes/discover-match-by-team-names/specs/oddsportal-js-odds/spec.md
- Lines: 1-88
- SHA256: 1d0e1cebe8d87e6fddd94c592097e7b4d95a730656d79381cc6dc1b179e8c93d

[TRUNCATED]

```md
## MODIFIED Requirements

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, JSONL path, and positive polling
interval from the `oddsportal` section of `config.yaml`, SHALL receive the shared configured
home-team and away-team pair from root configuration, and SHALL use the root proxy setting for
HTTP requests.

#### Scenario: South Africa Canada target is configured
- **WHEN** the shared configured home team is South Africa and away team is Canada
- **THEN** each discovery pass searches the configured tournament state for South Africa - Canada

#### Scenario: Polling interval is invalid
- **WHEN** `oddsportal.poll_interval_seconds` is zero
- **THEN** configuration validation fails before an OddsPortal task is spawned

### Requirement: Repeated OddsPortal collection
The system SHALL run non-overlapping OddsPortal polling cycles at the configured interval while
its provider task remains enabled. Each cycle SHALL request the discovered 1X2 odds and score
resources concurrently, process their results independently, append every successful normalized
odds pass to the provider-local JSONL log, and write successful observations to stdout.

#### Scenario: One-second collection succeeds
- **WHEN** `oddsportal.poll_interval_seconds` is `1` and both requests succeed
- **THEN** the task emits one odds JSON line and one score JSON line without starting another
  cycle before the next one-second tick

#### Scenario: Odds request fails and score succeeds
- **WHEN** the odds resource fails but the score resource returns a valid observation
- **THEN** the task reports the odds error to stderr, emits the score JSON line, and retries both
  resources on a later tick

#### Scenario: Score request fails and odds succeeds
- **WHEN** the score resource fails but the odds resource returns normalized 1X2 records
- **THEN** the task reports the score error to stderr, logs and emits the odds data, and retries
  both resources on a later tick

### Requirement: Visible OddsPortal lifecycle
The system SHALL emit `[oddsportal]`-prefixed diagnostics to stderr for discovery, polling,
retries, and failures. It SHALL emit machine-readable OddsPortal odds and score observations as
complete JSON lines to stdout.

#### Scenario: Polling cycle succeeds
- **WHEN** an OddsPortal odds or score observation is parsed
- **THEN** stdout receives the corresponding provider-labelled JSON record without a textual
  prefix

#### Scenario: Polling cycle fails
- **WHEN** an OddsPortal request, decoding step, or normalization step fails
- **THEN** stderr receives a prefixed contextual diagnostic without diagnostic text on stdout

## ADDED Requirements

### Requirement: OddsPortal score request discovery
The system SHALL derive the OddsPortal score request from the H2H page's embedded
`updateScoreRequest.url` metadata independently of `requestPreMatch.url`.

#### Scenario: H2H page exposes both request URLs
- **WHEN** embedded H2H metadata contains an odds request URL and a score request URL
- **THEN** discovery returns both absolute URLs associated with the same encoded event ID

#### Scenario: Score request metadata is absent
- **WHEN** the H2H page has no `updateScoreRequest.url`
- **THEN** score discovery reports a contextual absence without preventing odds request discovery

### Requirement: Structured OddsPortal odds output
The system SHALL group each successful normalized 1X2 pass by bookmaker and write one
`oddsportal_odds` JSON object containing all available bookmaker `1`, `X`, and `2` values to
stdout.

#### Scenario: Multiple bookmakers are normalized
- **WHEN** a pass contains 1X2 prices from multiple bookmakers
- **THEN** one JSON record contains the event identity, configured teams, local receipt time, and
  every bookmaker's available outcome values

### Requirement: OddsPortal score output
The system SHALL decode available OddsPortal score responses and write one `oddsportal_score` JSON
object per polling cycle with event identity, configured teams, availability, score state, source
time when present, and local receipt time.

```

Full source: openspec/changes/discover-match-by-team-names/specs/oddsportal-js-odds/spec.md

## openspec/changes/discover-match-by-team-names/specs/polymarket-ws-quotes/spec.md

- Source: openspec/changes/discover-match-by-team-names/specs/polymarket-ws-quotes/spec.md
- Lines: 1-94
- SHA256: 432878dd21c401cd01d9cb9c21e147ec8062babb274092e090d3b8ec9a060628

[TRUNCATED]

```md
## MODIFIED Requirements

### Requirement: Provider-local implementation
The system SHALL implement Polymarket football event discovery, initial order book loading, CLOB
WebSocket subscription, Sports WebSocket subscription, quote and score normalization, and
append-only quote logging under `src/polymarket/`. The root orchestration layer SHALL pass the
shared configured team pair and quote-log path into this provider-local workflow and SHALL start
it concurrently with enabled OddsPortal collection.

#### Scenario: Running the executable with both providers configured
- **WHEN** both providers are enabled and South Africa versus Canada is configured
- **THEN** modules under `src/polymarket/` discover that event, load and log initial quotes, and
  stream its CLOB quotes and score without waiting for OddsPortal collection to finish

### Requirement: Configurable Polymarket collection target
The system SHALL load the Polymarket enabled flag and quote JSONL path from the `polymarket`
section of `config.yaml`, SHALL receive the shared configured team pair from root configuration,
and SHALL retain existing provider defaults when optional provider settings are absent.

#### Scenario: Team pair is configured
- **WHEN** the shared target names South Africa and Canada
- **THEN** Polymarket discovery searches for the corresponding active football event without
  requiring an exact website event URL

#### Scenario: Quote log path is configured
- **WHEN** `polymarket.log_path` names a writable local path
- **THEN** initial and WebSocket quote records are appended to that path

### Requirement: Visible Polymarket lifecycle
The system SHALL emit `[polymarket]`-prefixed diagnostics to stderr for startup, event discovery,
CLOB and Sports WebSocket lifecycle, reconnects, and terminal errors. It SHALL emit
machine-readable Polymarket odds and score observations as complete JSON lines to stdout.

#### Scenario: Initial quote is loaded
- **WHEN** event discovery and an initial CLOB snapshot succeed
- **THEN** stdout contains a `polymarket_odds` JSON record and stderr contains
  Polymarket-attributed lifecycle diagnostics

#### Scenario: A Polymarket WebSocket reconnects
- **WHEN** either the CLOB or Sports WebSocket connection fails or closes
- **THEN** stderr identifies the Polymarket connection and its reconnect attempt without writing
  diagnostic text to stdout

## ADDED Requirements

### Requirement: Polymarket event discovery by team names
The system SHALL normalize the configured team names, inspect paginated active non-closed Gamma
events, and select the unique football 1X2 event containing those two teams in either displayed
order.

#### Scenario: One matching football event exists
- **WHEN** Gamma contains one active South Africa versus Canada football event with home-win,
  draw, and away-win markets
- **THEN** discovery returns that event, classifies its three 1X2 markets, and returns their CLOB
  token metadata

#### Scenario: No matching event exists
- **WHEN** no active football 1X2 event contains both configured teams
- **THEN** discovery fails with a contextual error naming the target pair

#### Scenario: Matching event is ambiguous
- **WHEN** more than one active football 1X2 event satisfies the normalized pair
- **THEN** discovery fails and identifies the candidates instead of choosing one implicitly

### Requirement: Structured Polymarket Yes-token odds output
The system SHALL write one `polymarket_odds` JSON object to stdout for each initial or changed Yes
token quote in the classified home-win, draw, or away-win market.

#### Scenario: Home-win Yes quote changes
- **WHEN** a CLOB message changes the Yes token for the classified home-win market
- **THEN** stdout receives one JSON record identifying `home` and containing event identity,
  configured teams, bid/ask prices and sizes, source, and local receipt time

#### Scenario: No token quote changes
- **WHEN** a CLOB message does not produce a normalized Yes-token quote change
- **THEN** no `polymarket_odds` JSON record is emitted for that message

### Requirement: Polymarket live score stream
The system SHALL connect to the unauthenticated Polymarket Sports WebSocket, respond to its
heartbeat, filter updates by the discovered event slug, and write each matching score observation
```

Full source: openspec/changes/discover-match-by-team-names/specs/polymarket-ws-quotes/spec.md

## openspec/changes/discover-match-by-team-names/specs/provider-runtime-orchestration/spec.md

- Source: openspec/changes/discover-match-by-team-names/specs/provider-runtime-orchestration/spec.md
- Lines: 1-40
- SHA256: ad6df683d2750e7c67e9e718fdf3018cfccd50a0a71464d150c3569a9734300a

```md
## MODIFIED Requirements

### Requirement: Independently configurable runtime features
The system SHALL load one shared football match target plus independently enabled Polymarket
collection, OddsPortal collection, and live trading features from `config.yaml`. It SHALL fail
startup with a clear diagnostic when both provider collectors are disabled or when the shared
match target is invalid.

#### Scenario: Both collectors enabled
- **WHEN** `polymarket.enabled` and `oddsportal.enabled` are both `true`
- **THEN** the system starts one Polymarket collector task and one OddsPortal collector task using
  the same configured team pair

#### Scenario: One collector enabled
- **WHEN** exactly one provider `enabled` value is `true`
- **THEN** the system starts only that provider and does not require disabled-provider transport
  settings

#### Scenario: No collector enabled
- **WHEN** both provider `enabled` values are `false`
- **THEN** startup fails before spawning any provider task

#### Scenario: Shared match target is invalid
- **WHEN** either team name is blank or both normalized team names are equal
- **THEN** startup fails before spawning any provider task

### Requirement: Provider-attributed process output
The system SHALL write collector data observations as complete JSON objects to stdout and SHALL
write provider-attributed lifecycle, retry, and failure diagnostics to stderr without printing
secrets.

#### Scenario: Both providers emit data
- **WHEN** Polymarket and OddsPortal produce odds or score observations
- **THEN** each stdout JSON object identifies its provider and record type without cross-provider
  aggregation

#### Scenario: Provider emits a diagnostic
- **WHEN** a provider starts, reconnects, retries, or fails
- **THEN** stderr carries its stable `[polymarket]`, `[oddsportal]`, or `[trade]` prefix while
  stdout remains data-only
```

