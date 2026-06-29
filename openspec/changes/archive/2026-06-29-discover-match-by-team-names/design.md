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

Lifecycle, retry, discovery, and error messages move to stderr so stdout remains parseable.
Provider-local JSONL files retain their existing schemas and append behavior.

### One-second OddsPortal scheduling

The configured interval is one second. On each tick, launch exactly one odds request and one score
request concurrently. Do not overlap poll cycles. Startup discovery requests may use bounded
retries; recurring odds and score requests use the next tick as their retry opportunity so a
single failure cannot create multi-second retry backlog.

Before the score resource becomes available, including an expected pre-match 404, emit an
`oddsportal_score` record with `available: false`; report unexpected transport or decoding errors
to stderr. The odds side remains independent.

## Risks / Trade-offs

- **Gamma may expose several events containing the same country names** → Require an active
  football 1X2 shape and uniqueness; list candidates instead of guessing.
- **OddsPortal advertises a 15-second refresh hint** → One-second requests cannot force upstream
  changes and may be rate-limited; keep cycles non-overlapping and make errors visible.
- **The OddsPortal score endpoint can be unavailable before kickoff** → Model absence explicitly
  rather than treating it as provider failure.
- **The Polymarket Sports WebSocket broadcasts all live sports** → Filter immediately by exact
  discovered slug and retain no unrelated events.
- **Provider clocks are not comparable** → Preserve provider time when present and always stamp
  local receipt time; downstream analysis decides which clock to use.
- **Moving lifecycle text to stderr changes stream placement** → Keep stable provider prefixes on
  diagnostics and document stdout as JSONL observations.

## Migration Plan

1. Add and validate the root match target.
2. Move South Africa and Canada from provider-local target fields into the root target; set the
   OddsPortal interval to one second.
3. Add name-based Gamma discovery and Polymarket Sports WebSocket score output.
4. Add OddsPortal score request discovery, one-second dual polling, and grouped odds JSON output.
5. Update documentation and verify all behavior with trading disabled.

Rollback requires restoring the previous binary and configuration together. Existing provider
JSONL files require no migration.

## Open Questions

None.
