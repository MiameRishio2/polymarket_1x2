## 1. Shared Match Configuration

- [x] 1.1 Add failing configuration tests for one validated shared team pair, provider injection,
  and the committed South Africa/Canada one-second target.
- [x] 1.2 Implement root match configuration, remove duplicated provider target ownership, and
  update `config.yaml` without weakening live-trading gates.

## 2. Polymarket Name Discovery

- [x] 2.1 Add fixture-driven tests for paginated active-event responses, normalized reversed team
  order, football 1X2 classification, no match, and ambiguous matches.
- [x] 2.2 Implement team-name Gamma discovery and retain exact-slug response parsing as a focused
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
