# Timestamped Diagnostics and Odds Semantics

## Goal

Make every runtime log line carry an explicit timestamp without breaking the
existing machine-readable JSONL contracts, and document whether the collected
OddsPortal prices are pre-match or in-play odds.

## Current Behavior

- Human-readable diagnostics are written to stderr with a provider prefix but
  no timestamp.
- Stdout observation JSONL already includes `received_at`.
- Provider-specific quote JSONL already includes `ts`.
- OddsPortal request discovery reads `requestPreMatch.url` into
  `pre_match_url`, then collects prices from that `/match-event/...dat`
  resource.

The current OddsPortal prices are therefore pre-match odds. The collector does
not discover or request an in-play odds feed.

## Design

Add one shared diagnostic-output helper at the binary orchestration layer. It
will write stderr lines in this format:

```text
2026-06-30T12:34:56.789Z [oddsportal] starting collection pass
```

The timestamp is the UTC emission time in RFC 3339 format with millisecond
precision. Polymarket, OddsPortal, trade, runtime-supervision, and terminal
diagnostics will all use this helper.

Stdout observations and provider quote files remain valid JSONL:

- stdout observations keep their existing `received_at` field;
- detailed quote records keep their existing `ts` field;
- no text prefix is added before a JSON object.

This gives every persisted line an explicit time while preserving consumers
that parse stdout or quote files as JSON.

## Boundaries

- Do not add in-play OddsPortal collection.
- Do not change odds values, polling behavior, provider discovery, or output
  schemas.
- Do not introduce a logging framework solely for this change.
- Keep provider prefixes unchanged after the timestamp.

## Error Handling

Diagnostic writes retain stderr's current best-effort behavior. Formatting the
timestamp has no fallible application-level path. Existing provider and
supervisor error propagation remains unchanged.

## Verification

- Unit-test the diagnostic formatter for an RFC 3339 millisecond timestamp and
  unchanged provider prefix/message.
- Update subprocess output tests to require timestamps on all diagnostic
  stderr lines.
- Keep tests that parse every stdout observation line as JSON.
- Assert that stdout observations contain `received_at` and detailed odds
  records contain `ts`.
- Add or retain a focused test proving OddsPortal discovery selects
  `requestPreMatch.url`.
- Run `cargo test`.
- Update architecture, deployment, and user documentation to state the log
  timestamp contract and that OddsPortal odds are pre-match rather than
  in-play.
