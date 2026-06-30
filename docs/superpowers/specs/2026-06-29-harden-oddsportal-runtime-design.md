---
comet_change: harden-oddsportal-runtime
role: technical-design
canonical_spec: openspec
archived-with: 2026-06-30-harden-oddsportal-runtime
status: final
---

# Harden OddsPortal Runtime Transport

## Problem

The OddsPortal client routes requests through the configured HTTP proxy. Live verification showed
that the tunnel and target can be reachable while some tournament responses still fail during
response-body decoding. These failures happen before the application-level OddsPortal feed
decoder runs.

## Design

Add `Accept-Encoding: identity` to the default headers used by the existing OddsPortal
`reqwest::Client`. Every tournament, H2H, odds, and score request already shares that client, so
the change stays inside the current provider boundary.

The root proxy, HTTP/1.1 setting, timeouts, three-attempt discovery retry, and one-second polling
interval remain unchanged. The client does not fall back to a direct connection because doing so
would violate the configured network route.

## Error Handling

Proxy-side DNS and connection failures remain visible and use the existing retry behavior. The
identity header reduces reliance on intermediary compression handling but cannot guarantee that
an upstream server or proxy honors the request.

## Testing

- Build a request through the real configured client and assert the local test server observes
  `Accept-Encoding: identity`.
- Retain coverage for response handling when a server labels a plain response as gzip.
- Run the full Rust test suite and release build.
- Run the release binary against the live OddsPortal tournament through the configured proxy and
  require successful odds output. Record transient external failures separately from deterministic
  application failures.

## Scope

No polling, configuration, public interface, dependency, source-layout, trading, or credential
change is included.
