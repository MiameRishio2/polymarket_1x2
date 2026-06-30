# OddsPortal In-Play-Only Collection

## Goal

Replace OddsPortal pre-match odds collection with target-match in-play 1X2
odds collection. When the target is not live or OddsPortal offers no live
feed, emit no OddsPortal odds and never fall back to pre-match prices.

## Verified Provider Behavior

The OddsPortal football live page advertises a global
`/feed/livegames/liveOdds/` feed, but that feed contains only summarized
maximum and average outcomes. A live match page separately exposes:

```text
requestLive.url = /feed/live-event/...dat
```

The decoded `requestLive` response contains per-bookmaker 1X2 prices under
`d.oddsdata.back`, matching the core structure already handled by the
collector. Live bookmaker names can be recovered from bookmaker slugs in each
market's `bs` betslip URLs. The live request requires the public match page as
its HTTP `Referer`.

## Collection Flow

Tournament discovery continues to identify the configured target and its
public H2H URL. Odds collection no longer stores or requests
`requestPreMatch.url`.

At each non-overlapping polling tick, the odds operation:

1. Fetches the target H2H page.
2. Parses `eventData.isLive`, `eventData.realLive`, and `requestLive.url`.
3. Returns an unavailable result without an odds observation unless the match
   is live and a live URL is present.
4. Absolutizes the live URL, appends the cache-busting timestamp, and requests
   it with the H2H URL as `Referer`.
5. Treats a live-feed 404 as unavailable for that tick.
6. Decodes the encrypted response and normalizes active per-bookmaker 1X2
   odds.

Refreshing the H2H page every tick is intentional: it observes the transition
into live play and obtains the current live feed hash instead of guessing it
from a pre-match URL.

The score operation remains independent and runs concurrently with the odds
operation. A missing or unavailable live odds feed does not suppress a valid
score observation.

## Models and Output

Request metadata will retain score discovery data but will no longer require
pre-match odds URLs. A dedicated live-request parser will return one of:

- `Unavailable`, for a non-live match or absent `requestLive`;
- `Available { url }`, for a currently live target with a live feed.

The odds collection result similarly distinguishes:

- no live odds available for this tick;
- a non-empty batch of normalized live odds.

Unavailable is normal state, not a provider failure. It produces no
`oddsportal_odds` stdout object and appends no detailed odds records.

Available odds preserve the existing output contracts:

- stdout type remains `oddsportal_odds`;
- `received_at` remains the local receipt timestamp;
- detailed records retain `ts` and `source_url`;
- bookmaker/outcome representation remains unchanged.

No new pre-match/live discriminator is needed because the collector becomes
in-play-only and the documentation defines the record semantics.

## Bookmaker Names

The live feed may omit `providersNames`. When absent, normalization extracts a
slug from a betslip URL such as:

```text
/bookmakers/22bet/betslip/l/
```

and converts it into a readable bookmaker name. If neither
`providersNames` nor a usable betslip slug exists, the bookmaker ID remains the
fallback name.

## Errors and Diagnostics

- Non-live state, missing `requestLive`, and live-feed HTTP 404 are logged as
  ordinary unavailability and produce no odds output.
- H2H transport failures, non-404 live-feed failures, decode failures, and
  malformed live odds remain odds-operation failures for that tick.
- Odds failures do not suppress the concurrently collected score.
- All diagnostics continue through the shared RFC 3339 timestamped stderr
  boundary.

## Boundaries

- Do not use the global summarized `livegames` feed as the target-match odds
  source.
- Do not request or fall back to `requestPreMatch`.
- Do not change Polymarket collection or trading behavior.
- Do not change the existing JSON schemas.
- Do not add credentials, authentication, signing, or order placement.

## Verification

- Parse saved H2H fixtures for non-live, live-with-feed, and live-without-feed
  states.
- Assert that only `requestLive.url` is selected and that
  `requestPreMatch.url` is ignored.
- Assert that live requests include the H2H `Referer`.
- Decode and normalize a saved encrypted live-event fixture containing
  per-bookmaker 1X2 odds.
- Verify bookmaker-name extraction from `bs` URLs and ID fallback.
- Verify non-live, missing-feed, and live-feed 404 ticks emit no odds and no
  detailed odds records while score output still proceeds.
- Verify live odds preserve the existing JSON schemas and timestamps.
- Run `cargo fmt --check` and `cargo test`.
- Update `README.md`, `ARCHITECTURE.md`, and `DEPLOYMENT.md` to describe
  in-play-only behavior and request counts.
