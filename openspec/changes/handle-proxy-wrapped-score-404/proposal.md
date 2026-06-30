## Why

The OddsPortal proxy can translate an upstream score-feed 404 into an outer HTTP 200 response
whose plain-text body reports `Status: 404`. The collector currently attempts to base64-decode
that body, producing noisy failures instead of the existing unavailable-score observation.

The committed configuration test also hard-codes an old match pair, so changing the supported
runtime target makes the otherwise unrelated test suite fail.

## What Changes

- Recognize the proxy's wrapped not-found score response as the existing pre-match unavailable
  state before attempting `.dat` decoding.
- Add a regression test for the wrapped response.
- Make the committed-configuration safety test validate read-only/provider invariants without
  asserting a mutable match selection.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oddsportal-js-odds`: Clarify that an expected pre-match not-found response remains unavailable
  when the configured proxy represents the upstream 404 inside an HTTP 200 plain-text body.

## Impact

The implementation is limited to OddsPortal score response handling in `src/oddsportal/mod.rs`
and the committed configuration test in `src/config.rs`. No API, configuration schema,
dependency, or live-trading behavior changes.
