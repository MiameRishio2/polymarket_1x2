---
comet_change: oddsportal-js-odds-logging
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-27-oddsportal-js-odds-logging
status: final
---

# OddsPortal JS Odds Logging Technical Design

## Context

The binary currently runs a read-only Polymarket quote collector. It discovers a configured event through Gamma, loads initial CLOB books, subscribes to market WebSocket updates, and appends quote records to a JSONL log. The source tree now reserves `src/oddsportal/` for provider-specific OddsPortal code.

OddsPortal exposes the needed Norway - France match metadata in page-embedded state. The World Championship 2026 tournament page includes the H2H URL and encoded event hash, and the H2H page includes `requestPreMatch.url`, which points at an internal `/match-event/...dat` odds payload. That `.dat` payload is decoded by the site's `JXG.decompress` JavaScript flow rather than presented as plain JSON in the rendered DOM.

## Design

Implement OddsPortal as a separate provider module with narrow submodules:

- `config.rs`: default tournament URL, target teams, base URL, user agent, proxy option, and log path.
- `models.rs`: target match, discovered match metadata, decoded odds records, and log record structs.
- `discovery.rs`: fetch tournament/H2H HTML and extract embedded state needed to locate the target event and internal odds request.
- `decoder.rs`: convert `.dat` response text into JSON using the observed `JXG.decompress` pipeline.
- `odds.rs`: parse decoded OddsPortal JSON into normalized 1X2 bookmaker odds.
- `logging.rs`: append normalized OddsPortal records as JSONL.
- `mod.rs`: provider facade with one public `collect_once(config) -> Result<Vec<OddsPortalRecord>>` style entry point.

Keep `src/main.rs` as orchestration. It should install Rustls once, start existing Polymarket behavior, and trigger OddsPortal collection/logging without moving provider-specific parsing into root code.

## Data Flow

```text
OddsPortal Config
    |
    v
GET tournament page
    |
    v
Extract target H2H URL + event hash from embedded state
    |
    v
GET H2H page
    |
    v
Extract requestPreMatch.url
    |
    v
GET /match-event/...dat
    |
    v
Decode base64 + inflate + URL-decode payload
    |
    v
Parse JSON oddsdata/provider metadata
    |
    v
Normalize bookmaker 1X2 odds
    |
    v
Append provider-tagged JSONL records
```

## Key Decisions

### Use Embedded State And Internal `.dat` Requests

The implementation will not scrape rendered odds table cells. It will parse page source for stable data-bearing attributes and component props, then call the internal request URL that the site itself exposes to its frontend.

This keeps the collector lightweight and testable while matching the user's requirement to collect from internal JavaScript data rather than the visible webpage.

### Decode In Rust Behind A Small Boundary

The observed browser decoder does:

1. base64 decode,
2. unzip/inflate,
3. raw URL decode,
4. parse JSON.

The Rust code should implement that pipeline directly with small tests and fixtures. If standard zlib/zip crates do not match a fixture, keep fallback logic inside `decoder.rs` only.

### Normalize Only Football 1X2

This change targets pre-match football 1X2 odds. The parser should only emit outcomes `1`, `X`, and `2`, preserving bookmaker names and IDs when available. Other betting types should be ignored rather than partially modeled.

### Logging Shape

OddsPortal records should include enough metadata to compare with Polymarket output:

- provider: `oddsportal`
- event ID/hash
- event name
- bookmaker ID/name
- outcome: `1`, `X`, or `2`
- decimal odds
- source URL
- captured timestamp

Polymarket log shape should remain unchanged unless a shared output file is selected; if a shared file is used, provider tagging is mandatory for OddsPortal records.

## Error Handling

- Missing target match: return an error that names the configured teams and tournament URL.
- Missing `requestPreMatch.url`: return an error that names the H2H URL and event hash.
- Decode failure: include which decode phase failed without dumping the entire payload.
- Odds shape mismatch: include the event hash and top-level JSON keys found.
- Network failures: wrap with provider and URL context.

## Testing Strategy

Use focused tests before implementation code:

1. Tournament fixture parsing discovers Norway - France and `bsJSJ30L`.
2. H2H fixture parsing extracts `requestPreMatch.url`.
3. `.dat` fixture decoding returns parseable JSON.
4. Odds parser emits one record per bookmaker outcome for `1`, `X`, and `2`.
5. Logger appends valid JSON lines with `provider = "oddsportal"`.
6. `cargo test` verifies Polymarket tests still pass.

Network smoke testing should be separate from unit tests, because the OddsPortal endpoint and bookmaker hash can change.

## Scope Controls

Do not add account login, cookies, credential storage, order placement, betslip behavior, or broad OddsPortal market coverage. If implementation reveals a need for a generic provider abstraction, defer it unless both providers demonstrably need the same code path.
