---
comet_change: discover-match-by-team-names
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-29-discover-match-by-team-names
status: final
---

# Team-Name Market and Score Observation Design

## 1. Objective

Accept one football match in root configuration:

```yaml
match:
  home_team: South Africa
  away_team: Canada
```

Use that pair to discover the matching Polymarket and OddsPortal events. Emit four independent,
timestamped JSON observation types without correlating or aggregating the providers:

1. `polymarket_odds`
2. `polymarket_score`
3. `oddsportal_odds`
4. `oddsportal_score`

The OpenSpec delta specs are the canonical behavioral requirements. This document defines the
implementation boundaries and protocol handling.

## 2. Constraints

- Polymarket quote and score collection remain unauthenticated, provider-local, and WebSocket
  driven.
- OddsPortal collection remains unauthenticated and provider-local.
- The OddsPortal recurring cycle sends one odds request and one score request per second.
- Live trading stays behind the existing explicit gate and is not invoked by observation output.
- Existing provider JSONL files retain their schemas.
- stdout contains complete data JSON objects only. Human-readable diagnostics use stderr.
- No root aggregation state, cross-provider join, comparison, or arbitrage logic is introduced.

## 3. Component Boundaries

### 3.1 Root configuration

`src/config.rs` owns a `MatchSection` with `home_team` and `away_team`. Conversion trims both
values and rejects a blank value or names equal under ASCII-case-insensitive, whitespace-collapsed
comparison.

The validated pair is cloned into both enabled provider runtime configurations. Provider sections
continue to own only provider-specific switches and transport/log settings:

```yaml
polymarket:
  enabled: true
  log_path: logs/polymarket_quotes.log

oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  log_path: logs/oddsportal_odds.log
  poll_interval_seconds: 1
```

The old configured Polymarket URL and provider-local OddsPortal team fields are removed rather than
kept as competing target sources.

### 3.2 Polymarket discovery

`src/polymarket/discovery.rs` gains a paginated active-event discovery path:

```text
GET {gamma_host}/events
  ?active=true
  &closed=false
  &limit=100
  &offset=N
```

Candidate comparison:

1. Trim and collapse whitespace.
2. Compare case-insensitively.
3. Treat `vs`, `vs.`, and surrounding punctuation/spacing as equivalent.
4. Accept either displayed team order.
5. Require exactly one home-win question, one draw question, and one away-win question within the
   event.
6. Require usable CLOB token IDs and identify only the `Yes` token for each classified market.

Pagination stops on a short page. Zero matches returns a target-specific error. More than one
match returns an ambiguity error listing candidate slugs and titles. Discovery never chooses the
first fuzzy candidate.

The current exact-event JSON parser remains a pure helper reused for each candidate. URL-slug
extraction remains tested but is no longer used by runtime configuration.

### 3.3 Polymarket CLOB observations

The existing initial order-book load and market WebSocket continue to update `QuoteState` and
append all normalized records to the Polymarket JSONL file.

A provider-local classifier maps the three discovered Yes-token asset IDs to:

- `home`
- `draw`
- `away`

Initial snapshots and subsequent WebSocket updates write a console observation only when the
record belongs to one of these Yes tokens. No six-token Yes/No output is produced.

### 3.4 Polymarket score observations

A new provider-local sports stream connects through the configured proxy to:

```text
wss://sports-api.polymarket.com/ws
```

The Sports WebSocket requires no subscription. It broadcasts active events globally. The loop:

1. Reads messages continuously.
2. Replies `pong` when the text payload is exactly `ping`.
3. Parses JSON score messages.
4. Discards every message whose `slug` differs from the discovered event slug.
5. Emits a `polymarket_score` record immediately for a matching message.
6. Logs disconnects/errors and reconnects with bounded delay.

The score stream and CLOB stream execute concurrently inside `src/polymarket/`. Each stream owns
its reconnect loop, so a transient disconnect on one does not stop the other.

### 3.5 OddsPortal request discovery

`src/oddsportal/discovery.rs` continues to find the H2H event by the configured names in either
order. Its H2H metadata result expands to keep two independent request descriptors:

- `requestPreMatch.url` for 1X2 odds
- `updateScoreRequest.url` for score state

Both relative URLs become absolute OddsPortal URLs and retain the same event identity. Missing
score metadata is represented explicitly; it does not invalidate odds discovery.

### 3.6 OddsPortal recurring collection

The provider performs tournament and H2H discovery once before entering the recurring loop.
Discovery is repeated only after a contextual event/request invalidation, not twice per second.

The poll loop uses `tokio::time::interval` with missed ticks skipped. On every tick it starts one
odds future and one score future and awaits them concurrently. The next cycle cannot begin until
both futures settle, so cycles never overlap.

Startup page discovery may retain bounded retries. Recurring odds and score requests make one
attempt per cycle; the next tick is the natural retry. This prevents retry backlogs from
destroying the observation cadence.

The two results are matched independently:

- Odds success: normalize, append existing per-outcome JSONL records, and emit one grouped stdout
  record.
- Score success: decode and emit one stdout record.
- Expected pre-match score 404: emit `available: false`.
- One-side failure: log only that error; still process and emit the other side.

OddsPortal's embedded metadata currently advertises a 15-second refresh hint. Polling once per
second measures when the collector can observe changes; it cannot force upstream score or price
updates.

## 4. Output Contracts

All timestamps are UTC RFC 3339 strings. `received_at` is assigned immediately after a response or
WebSocket message is successfully parsed, before formatting and writing. `source_updated_at`
preserves the provider timestamp verbatim when present and is otherwise `null`.

### 4.1 Polymarket odds

```json
{
  "provider": "polymarket",
  "type": "polymarket_odds",
  "received_at": "2026-06-28T12:00:00.123Z",
  "source_updated_at": null,
  "event_slug": "fifwc-rsa-can-2026-06-28",
  "home_team": "South Africa",
  "away_team": "Canada",
  "result": "home",
  "market_slug": "fifwc-rsa-can-2026-06-28-rsa",
  "asset_id": "123",
  "bid_price": "0.16",
  "bid_size": "100",
  "ask_price": "0.17",
  "ask_size": "80",
  "source": "best_bid_ask"
}
```

`result` is exactly `home`, `draw`, or `away`. Missing bid/ask fields serialize as `null`.

### 4.2 Polymarket score

```json
{
  "provider": "polymarket",
  "type": "polymarket_score",
  "received_at": "2026-06-28T12:00:01.100Z",
  "source_updated_at": "2026-06-28T12:00:01.050Z",
  "event_slug": "fifwc-rsa-can-2026-06-28",
  "home_team": "South Africa",
  "away_team": "Canada",
  "score": "1-0",
  "status": "InProgress",
  "period": "1H",
  "elapsed": "32:15",
  "live": true,
  "ended": false
}
```

Provider fields absent from a valid message serialize as `null` where appropriate.

### 4.3 OddsPortal odds

```json
{
  "provider": "oddsportal",
  "type": "oddsportal_odds",
  "received_at": "2026-06-28T12:00:01.300Z",
  "source_updated_at": null,
  "event_id": "EZmXxG15",
  "event_name": "South Africa - Canada",
  "home_team": "South Africa",
  "away_team": "Canada",
  "bookmakers": [
    {
      "bookmaker_id": "16",
      "bookmaker_name": "bet365",
      "outcomes": {
        "1": "5.50",
        "X": "3.80",
        "2": "1.62"
      }
    }
  ]
}
```

The array contains every bookmaker returned by normalization. A missing outcome is omitted from
that bookmaker's `outcomes` object, not invented or copied.

### 4.4 OddsPortal score

```json
{
  "provider": "oddsportal",
  "type": "oddsportal_score",
  "received_at": "2026-06-28T12:00:01.310Z",
  "source_updated_at": null,
  "event_id": "EZmXxG15",
  "event_name": "South Africa - Canada",
  "home_team": "South Africa",
  "away_team": "Canada",
  "available": true,
  "score": "1-0",
  "status": "live",
  "period": "1H",
  "elapsed": "32:00"
}
```

Before the source exposes a score:

```json
{
  "provider": "oddsportal",
  "type": "oddsportal_score",
  "received_at": "2026-06-28T11:59:00.000Z",
  "source_updated_at": null,
  "event_id": "EZmXxG15",
  "event_name": "South Africa - Canada",
  "home_team": "South Africa",
  "away_team": "Canada",
  "available": false,
  "score": null,
  "status": null,
  "period": null,
  "elapsed": null
}
```

## 5. Output and Failure Semantics

- Serialize each data object fully before one stdout write, preventing interleaved partial JSON
  from concurrent tasks.
- A serialization or stdout write error is terminal for that provider because silently dropping
  observations invalidates latency analysis.
- Expected score unavailability is data, not an error.
- Malformed score data, HTTP failures, decode failures, and reconnects are diagnostics on stderr.
- Diagnostics retain stable provider prefixes. Values derived from credentials, headers,
  signatures, private keys, and authenticated payloads are never printed.
- Existing quote/odds JSONL logger failures remain terminal because persistence was requested by
  existing specifications.

## 6. Testing Strategy

### Configuration

- Shared pair is injected into both enabled runtimes.
- Blank names and equal normalized names fail before task spawn.
- Disabled-provider transport settings remain optional.
- Committed configuration selects South Africa/Canada, one-second OddsPortal polling, and disabled
  trading.

### Polymarket discovery and odds

- Paginated Gamma fixtures find a match on a later page.
- Team order and `vs` punctuation normalize correctly.
- Non-football lookalikes and incomplete market shapes are rejected.
- Zero and multiple candidates produce deterministic errors.
- Only classified Yes-token records serialize to stdout.
- Initial order books emit before the first CLOB WebSocket message.

### Polymarket score

- Matching slug messages serialize all score timing fields.
- Unrelated slugs produce no output.
- `ping` produces `pong`.
- Disconnect triggers reconnect without stopping a test CLOB task.

### OddsPortal

- H2H fixtures extract both request URLs independently.
- Score fixtures cover live, scheduled, malformed, and expected 404 states.
- Multiple bookmakers group into one record with available `1/X/2` fields.
- Paused Tokio time proves first-cycle behavior, one-second ticks, skipped missed ticks, and no
  overlap.
- Odds failure/score success and score failure/odds success both emit the successful side.

### Process streams

- Captured stdout contains only parseable JSON lines for data-producing test helpers.
- Captured stderr retains provider prefixes for diagnostics.
- JSONL log fixtures remain byte-compatible with their existing record schemas.

Full verification runs formatting, focused tests, `cargo test`, strict OpenSpec validation, and a
bounded trading-disabled live smoke test where network access permits.

## 7. Documentation Impact

Update `ARCHITECTURE.md` because root configuration ownership, Polymarket score transport, and
OddsPortal score transport change. Update `README.md` with the new configuration and four output
schemas. Deployment scripts and process-management behavior do not change, so `DEPLOYMENT.md`
requires review but no planned behavioral edit.
