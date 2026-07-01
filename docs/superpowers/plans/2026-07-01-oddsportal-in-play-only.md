# OddsPortal In-Play-Only Collection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace OddsPortal pre-match odds with target-match `requestLive` bookmaker odds and emit no OddsPortal odds outside live play.

**Architecture:** Keep tournament discovery for the target H2H URL, but refresh that H2H page during every odds operation to discover current live state and the current `requestLive.url`. Represent unavailable live odds explicitly, request available feeds with the match page as `Referer`, reuse the encrypted `.dat` decoder and 1X2 normalizer, and preserve existing JSON schemas.

**Tech Stack:** Rust, Tokio, reqwest, serde/serde_json, chrono, existing OddsPortal encrypted `.dat` decoder, built-in Rust tests.

## Global Constraints

- Do not use the global summarized `livegames` feed as the target-match odds source.
- Do not request or fall back to `requestPreMatch`.
- Do not change Polymarket collection or trading behavior.
- Do not change the existing JSON schemas.
- Do not add credentials, authentication, signing, or order placement.
- Preserve unrelated user changes in `.codex/skills/` and `config.yaml`.
- Run `cargo fmt --check` and `cargo test`.

---

### Task 1: Parse Live Match State and Remove Pre-Match Request Metadata

**Files:**
- Modify: `src/oddsportal/models.rs`
- Modify: `src/oddsportal/discovery.rs`
- Test: `src/oddsportal/discovery.rs`

**Interfaces:**
- Produces: `LiveOddsRequestState::{Unavailable, Available { url: String }}`.
- Produces: `parse_live_odds_request(html: &str) -> Result<LiveOddsRequestState>`.
- Changes: `RequestMetadata` retains only `score_url: Option<String>`.

- [ ] **Step 1: Add failing live-state parser tests**

Add tests with escaped H2H component data for:

```rust
let html = r#"<Event :data="{&quot;eventData&quot;:{
  &quot;isLive&quot;:true,&quot;realLive&quot;:true},
  &quot;requestPreMatch&quot;:{&quot;url&quot;:&quot;\/match-event\/ignored.dat?_=&quot;},
  &quot;requestLive&quot;:{&quot;url&quot;:&quot;\/feed\/live-event\/1-1-EZmXxG15-1-2-yjlive.dat?_=&amp;geo=JP&quot;}}">
</Event>"#;
assert_eq!(
    parse_live_odds_request(html).unwrap(),
    LiveOddsRequestState::Available {
        url: "https://www.oddsportal.com/feed/live-event/1-1-EZmXxG15-1-2-yjlive.dat?_=&geo=JP".into()
    }
);
```

Also assert `Unavailable` when either live boolean is false or
`requestLive` is absent, even if `requestPreMatch` exists.

- [ ] **Step 2: Run parser tests and verify RED**

Run:

```bash
cargo test oddsportal::discovery::tests::live -- --nocapture
```

Expected: compilation fails because `LiveOddsRequestState` and
`parse_live_odds_request` do not exist.

- [ ] **Step 3: Implement minimal live-state parsing**

Add the enum to `models.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiveOddsRequestState {
    Unavailable,
    Available { url: String },
}
```

In `discovery.rs`, decode HTML entities, require both exact JSON booleans:

```rust
let is_live = find_json_bool(&decoded, "isLive").unwrap_or(false);
let real_live = find_json_bool(&decoded, "realLive").unwrap_or(false);
if !is_live || !real_live {
    return Ok(LiveOddsRequestState::Unavailable);
}
```

Only then find `requestLive.url`, replace escaped slashes, convert `&amp;` via
the existing entity decoder, and absolutize it. Never inspect
`requestPreMatch`.

Change `parse_h2h_request_metadata` so it only extracts the optional
`updateScoreRequest` and returns `RequestMetadata { score_url }`.

- [ ] **Step 4: Run parser and existing discovery tests**

Run:

```bash
cargo test oddsportal::discovery -- --nocapture
```

Expected: all discovery tests pass after pre-match-specific tests are replaced
with score-metadata and live-state tests.

- [ ] **Step 5: Commit parser/model changes**

```bash
git add src/oddsportal/models.rs src/oddsportal/discovery.rs
git commit -m "feat: discover OddsPortal live odds requests"
```

### Task 2: Normalize Live Bookmaker Names

**Files:**
- Create: `tests/fixtures/oddsportal_live_event.dat`
- Modify: `src/oddsportal/decoder.rs`
- Modify: `src/oddsportal/odds.rs`
- Test: `src/oddsportal/decoder.rs`
- Test: `src/oddsportal/odds.rs`

**Interfaces:**
- Consumes: decoded live-event `d.oddsdata.back.*.bs` betslip mappings.
- Produces: existing `normalize_1x2_odds(...)` records with readable bookmaker names.

- [ ] **Step 1: Add a sanitized encrypted live-event fixture and failing tests**

Add a deterministic encrypted fixture whose decoded JSON contains:

```json
{
  "d": {
    "encodeventId": "EZmXxG15",
    "oddsdata": {
      "back": {
        "E-1-2-0-0-0": {
          "bettingTypeId": 1,
          "scopeId": 2,
          "odds": {
            "417": {"0": 2.87, "1": 3.60, "2": 2.25}
          },
          "bs": {
            "417": {
              "0": "/bookmakers/1xbet/betslip/l/Football/example/"
            }
          },
          "act": {"417": true}
        }
      }
    }
  }
}
```

In `decoder.rs`, load it with `include_str!` and require successful decryption
and the live `scopeId`. In `odds.rs`, add a test requiring bookmaker `417` to
normalize as `1xbet`, with outcomes `1`, `X`, and `2`.

- [ ] **Step 2: Run decoder/odds tests and verify RED**

Run:

```bash
cargo test oddsportal::decoder::tests::decodes_encrypted_live_event_fixture -- --nocapture
cargo test oddsportal::odds::tests::derives_live_bookmaker_name_from_betslip -- --nocapture
```

Expected: the decoder fixture test passes once the fixture is valid; the odds
test fails with bookmaker name `417`.

- [ ] **Step 3: Implement bookmaker slug fallback**

For each market, read `bs`. Resolve names in this order:

```rust
provider_names
    .and_then(...)
    .or_else(|| bookmaker_name_from_betslip(betslips, bookmaker_id))
    .unwrap_or_else(|| bookmaker_id.clone())
```

`bookmaker_name_from_betslip` must inspect any string URL for that bookmaker,
parse the segment immediately after `bookmakers`, percent-decode it, replace
hyphens with spaces, and return `None` for an empty/missing slug.

- [ ] **Step 4: Run decoder and normalizer tests**

Run:

```bash
cargo test oddsportal::decoder -- --nocapture
cargo test oddsportal::odds -- --nocapture
```

Expected: all tests pass, including provider-name precedence and ID fallback.

- [ ] **Step 5: Commit fixture and normalization**

```bash
git add tests/fixtures/oddsportal_live_event.dat src/oddsportal/decoder.rs src/oddsportal/odds.rs
git commit -m "feat: normalize OddsPortal live bookmaker odds"
```

### Task 3: Collect In-Play Odds Only

**Files:**
- Modify: `src/oddsportal/mod.rs`
- Modify: `src/oddsportal/models.rs`
- Test: `src/oddsportal/mod.rs`

**Interfaces:**
- Produces: internal `OddsCollection::{Unavailable, Records(Vec<OddsPortalRecord>)}`.
- Changes: `collect_odds` refreshes the H2H page and uses only `requestLive`.
- Requires: live feed GET contains `Referer: <fragment-free H2H URL>`.

- [ ] **Step 1: Add failing HTTP and cycle tests**

Use `TestHttpServer` to cover:

1. non-live H2H HTML causes one H2H request and returns `Unavailable`;
2. live H2H HTML causes a live feed request with the H2H `Referer`;
3. live feed 404 returns `Unavailable`;
4. decoded live feed records return `Records`;
5. unavailable odds call neither append nor emit while score still emits;
6. no request path contains `/match-event/`.

The Referer assertion is:

```rust
assert!(
    live_server
        .last_request()
        .to_ascii_lowercase()
        .contains(&format!("referer: {}", h2h_url).to_ascii_lowercase())
);
```

- [ ] **Step 2: Run collection tests and verify RED**

Run:

```bash
cargo test oddsportal::tests::live_odds -- --nocapture
cargo test oddsportal::tests::unavailable_odds -- --nocapture
```

Expected: tests fail because collection still reads static pre-match URLs and
cannot represent unavailable odds.

- [ ] **Step 3: Implement in-play-only request flow**

Add:

```rust
enum OddsCollection {
    Unavailable,
    Records(Vec<models::OddsPortalRecord>),
}
```

Change the odds operation to fetch `http_request_url(event.h2h_url)`, call
`parse_live_odds_request`, and return `Unavailable` unless it returns
`Available`. For an available URL, issue:

```rust
client
    .get(cache_busted_url(&url))
    .header(reqwest::header::REFERER, http_request_url(&event.h2h_url))
```

Treat both a real HTTP 404 and the existing proxy-wrapped
`URL:... Status: 404` body as `Unavailable`. All other transport/status/decode
errors remain failures. A decoded live feed with no active 1X2 records remains
an error rather than being confused with non-live state.

Remove all pre-match primary/fallback logic and tests.

- [ ] **Step 4: Propagate unavailability without output**

Change cycle generics and `CycleResult` to carry `OddsCollection`. In
`handle_cycle_with`, `Unavailable` must set a successful/unavailable status but
must not invoke append or stdout emit closures. `Records` follows the current
append-then-emit sequence. Log:

```text
[oddsportal] no in-play odds available
```

for unavailable ticks.

- [ ] **Step 5: Run all OddsPortal tests**

Run:

```bash
cargo test oddsportal -- --nocapture
```

Expected: all OddsPortal tests pass; score independence, retry bounds, timeout,
and sink error behavior remain covered.

- [ ] **Step 6: Commit in-play collection**

```bash
git add src/oddsportal/mod.rs src/oddsportal/models.rs
git commit -m "feat: collect OddsPortal in-play odds only"
```

### Task 4: Synchronize Documentation and Verify Delivery

**Files:**
- Modify: `README.md`
- Modify: `ARCHITECTURE.md`
- Modify: `DEPLOYMENT.md`

**Interfaces:**
- Documents: `requestLive` source, no pre-match fallback, unavailable behavior, Referer requirement, and per-tick request counts.

- [ ] **Step 1: Update all runtime documentation**

Replace pre-match claims with:

```text
OddsPortal odds come only from the target match page's requestLive
/feed/live-event/...dat resource. Before kickoff, after completion, or whenever
that feed is unavailable, no oddsportal_odds record is emitted. The collector
never falls back to requestPreMatch.
```

Document that an odds operation makes one H2H request when unavailable and a
second live-feed request when available; the independent score operation may
add one request.

- [ ] **Step 2: Run documentation consistency checks**

Run:

```bash
rg -n 'requestLive|requestPreMatch|in-play|pre-match|Referer' README.md ARCHITECTURE.md DEPLOYMENT.md
git diff --check
```

Expected: any `requestPreMatch` mention says it is never used; no text claims
that emitted OddsPortal odds are pre-match.

- [ ] **Step 3: Commit documentation**

```bash
git add README.md ARCHITECTURE.md DEPLOYMENT.md
git commit -m "docs: describe OddsPortal in-play-only collection"
```

- [ ] **Step 4: Run final validation**

Run:

```bash
cargo fmt --check
cargo test
rg -n 'pre_match_url|fallback_pre_match_url|/match-event/' src/oddsportal
git diff --check origin/main..HEAD
git status --short --branch
```

Expected: formatting and all tests pass; no production pre-match odds URL
fields or `/match-event/` odds request remain; only unrelated pre-existing
worktree changes are unstaged.

- [ ] **Step 5: Push verified main**

Run:

```bash
git push origin main
```

Expected: `origin/main` advances to the verified local `main` without
force-push.
