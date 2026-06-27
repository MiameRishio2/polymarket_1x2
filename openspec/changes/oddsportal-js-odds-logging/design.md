## Context

The binary currently discovers one configured Polymarket event, loads initial CLOB quotes, subscribes to market WebSocket updates, and appends quote records to `logs/polymarket_quotes.log`. `src/oddsportal/` exists only as a provider boundary.

For the reference OddsPortal page, the tournament HTML exposes Norway - France as an event row URL containing `#bsJSJ30L`. The H2H page for France/Norway embeds JavaScript component data with `requestPreMatch.url` shaped like `/match-event/1-1-bsJSJ30L-1-2-<bookmaker-hash>.dat?_=`, plus base paths for match event and odds history requests. The `.dat` response is not JSON; it is base64 data decoded by `JXG.decompress` from `/js/lscompressor.min.js`, which inflates zipped data and URL-decodes the result. The Vue event bundle then reads `d.oddsdata`, provider metadata, betting type, and scope fields.

## Goals / Non-Goals

**Goals:**

- Collect OddsPortal 1X2 bookmaker odds without scraping rendered DOM table text.
- Derive the target event and internal odds request from page-embedded JavaScript/HTML state.
- Decode the internal `.dat` response and normalize bookmaker odds for outcomes `1`, `X`, and `2`.
- Log OddsPortal odds records in append-only JSONL format while keeping Polymarket logging unchanged.
- Keep all OddsPortal-specific code under `src/oddsportal/`.

**Non-Goals:**

- Do not place bets, authenticate, manage user accounts, or bypass logged-in-only data.
- Do not implement every OddsPortal market type; this change targets pre-match football 1X2 odds.
- Do not create a shared provider abstraction unless implementation proves both providers need it.
- Do not replace the existing Polymarket WebSocket collector.

## Decisions

### Approach Options

1. Parse rendered match table HTML.
   - Pros: quick if the server returns visible odds rows.
   - Cons: brittle, tightly coupled to CSS/DOM rendering, and does not satisfy the requirement to avoid direct webpage scraping.

2. Launch a browser and read hydrated JavaScript state.
   - Pros: follows the same path as the website and can handle dynamic behavior.
   - Cons: heavy dependency for a CLI collector, harder to test, and unnecessary because the needed request metadata is present in HTML.

3. Fetch HTML as text, extract embedded JavaScript state, call the internal `.dat` endpoint, and decode the response in Rust.
   - Pros: read-only, lightweight, testable with fixtures, and aligned with the user's requested data source.
   - Cons: depends on OddsPortal's private response shape and compression scheme.

Use option 3.

### Provider Structure

`src/oddsportal/` will own configuration, page discovery, internal request construction, `.dat` decoding, odds parsing, and JSONL logging. `src/main.rs` remains the orchestration layer and should only call provider-level entry points.

### Data Flow

```text
OddsPortal config
    |
    v
Tournament page fetch
    |
    v
Embedded state extraction
    |
    v
Target match event URL/hash discovery
    |
    v
H2H page fetch
    |
    v
requestPreMatch URL extraction
    |
    v
Compressed .dat fetch and JXG-compatible decode
    |
    v
1X2 odds normalization
    |
    v
append JSONL records
```

### Logging

OddsPortal should write structured records that identify the provider, event, bookmaker, outcome, decimal odds, and source request. Polymarket logging should keep its existing records and path unless implementation exposes a clear need to add a separate OddsPortal log path. If both providers write to one file, records must be distinguishable by provider.

### Decoding

The first implementation should avoid porting the full minified browser script directly. Prefer a small Rust decoder that mirrors the observed pipeline:

1. Base64-decode the `.dat` body.
2. Inflate ZIP/zlib payload bytes.
3. URL-decode the inflated text.
4. Parse the resulting JSON.

If fixture data shows a ZIP wrapper that differs from standard Rust crates, isolate the decoder behind a small function and cover it with the captured Norway - France response fixture.

## Risks / Trade-offs

- OddsPortal private endpoints or compression can change without notice -> isolate request discovery and decoding, add fixture tests, and return contextual errors.
- OddsPortal can vary bookmaker hash values per request or session -> derive `requestPreMatch.url` from the H2H page instead of hard-coding it.
- The tournament page may list "Norway - France" while the H2H path is `france-.../norway-...` -> match discovery must use event display names and URL hash, not path order alone.
- Running both providers concurrently can let one provider failure stop the other -> orchestration should log/return provider-specific context and avoid hiding errors.

## Migration Plan

1. Add OddsPortal provider modules and tests with static fixtures for page state and decoded odds.
2. Wire the provider into `main.rs` after the Polymarket setup is preserved.
3. Run `cargo test`.
4. Run the binary or focused integration path only when network access is available.
