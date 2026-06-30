## Why

Live OddsPortal collection through the configured HTTP proxy intermittently fails while reading
compressed tournament responses, even though the proxy tunnel and endpoint are reachable.

## What Changes

- Request identity-encoded OddsPortal HTTP responses.
- Keep the configured one-second polling interval and existing retry behavior unchanged.
- Add focused transport-header coverage and verify the release binary against the live endpoint.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oddsportal-js-odds`: Clarify that proxied HTTP requests ask the upstream for identity encoding.

## Impact

The change is confined to the OddsPortal HTTP client and its tests. No public API, dependency,
proxy address, polling cadence, or provider boundary changes.
