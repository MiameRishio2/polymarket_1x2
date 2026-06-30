## Context

The proxy tunnel can return the OddsPortal tournament page successfully, but live collection has
also observed response-body decoding failures. The existing client allows automatic content
encoding negotiation.

## Goals / Non-Goals

**Goals:**

- Ask OddsPortal for identity-encoded HTTP responses.
- Preserve existing proxy routing, timeouts, retries, and one-second polling.
- Verify both the request header and a live release run.

**Non-Goals:**

- Mask proxy-side DNS failures.
- Change polling cadence or add direct-network fallback.
- Change OddsPortal feed payload decoding.

## Decisions

Add `Accept-Encoding: identity` to the OddsPortal client's default headers. This is the smallest
transport-level change and applies consistently to tournament, H2H, odds, and score requests.

## Risks / Trade-offs

- Upstream or an intermediary may ignore the header. Existing retries remain responsible for
  transient transport failures.
- Identity responses may use more bandwidth, but request frequency and payload semantics remain
  unchanged.
