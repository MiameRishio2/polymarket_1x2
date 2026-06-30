## Context

`collect_score` already maps a real HTTP 404 to an unavailable score observation. With the
configured HTTP proxy, the same upstream response may arrive as HTTP 200 with a short plain-text
body shaped as `URL:<path> Status: 404`. Passing that body to the shared `.dat` decoder produces
the misleading base64 error seen in runtime logs.

The repository intentionally permits operators to change `match.home_team` and
`match.away_team`. One configuration test currently treats the previously committed values as
fixed, coupling a safety test to mutable operational input.

## Goals / Non-Goals

**Goals:**

- Map the proxy's narrowly identified wrapped score-feed 404 to the same unavailable observation
  as a direct HTTP 404.
- Preserve contextual decoding errors for every other malformed HTTP 200 body.
- Keep the committed configuration test focused on read-only and provider safety invariants.
- Cover the wrapped-response behavior with a deterministic HTTP test.

**Non-Goals:**

- Generalize proxy error translation for every endpoint or status code.
- Change OddsPortal retry cadence, odds collection, or encrypted feed decoding.
- Modify deployment scripts or attempt to manage host processes across PID namespaces.
- Change configuration schema or default match selection.

## Decisions

1. Detect the wrapped not-found body in `collect_score` before `.dat` decoding. The predicate will
   require the proxy response shape (`URL:` prefix and `Status: 404` suffix after trimming), so
   unrelated malformed payloads continue to fail visibly. Treating all decoder failures as
   unavailable was rejected because it would hide encryption/key regressions.
2. Reuse `score::unavailable_score` for both direct and wrapped 404 paths. This keeps stdout shape
   and downstream behavior identical and avoids a second representation of pre-match absence.
3. Exercise the behavior through the existing local HTTP test harness at the `collect_score`
   boundary. A decoder-only test was rejected because the defect is response classification, not
   base64 decoding.
4. Remove only the exact team-value assertions from the committed configuration test while
   retaining enabled-provider, interval, and trade-disabled assertions. Replacing them with the
   new Netherlands/Morocco values would merely move the same coupling.

## Risks / Trade-offs

- A future proxy may use a different wrapped-error format → unmatched formats remain explicit
  errors, providing evidence before broadening classification.
- A legitimate score payload matching the exact wrapper shape would be classified unavailable →
  the required prefix/suffix and score-feed request context make this collision negligible.
- The configuration test no longer detects accidental match-value edits → match selection is
  operational input, while parsing and validity remain covered by dedicated fixture tests.
