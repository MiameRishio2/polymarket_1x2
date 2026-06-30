# Comet Design Handoff

- Change: harden-oddsportal-runtime
- Phase: design
- Mode: compact
- Context hash: 1e3364037a0fabc8d8b6b6d0ae967a0b1fc467f681e22d615f85caa9b9cf8eb1

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/harden-oddsportal-runtime/proposal.md

- Source: openspec/changes/harden-oddsportal-runtime/proposal.md
- Lines: 1-25
- SHA256: a108ef97a070dfb4be61c71c1f684956dd0f24e05e41a872eb7bed42251016ed

```md
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
```

## openspec/changes/harden-oddsportal-runtime/design.md

- Source: openspec/changes/harden-oddsportal-runtime/design.md
- Lines: 1-31
- SHA256: 3bc88a2222f24cb72bc5e6dc05ec64adea3e1422a30326f2d197def1c0991286

```md
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
```

## openspec/changes/harden-oddsportal-runtime/tasks.md

- Source: openspec/changes/harden-oddsportal-runtime/tasks.md
- Lines: 1-8
- SHA256: 2f8ccc8931e01b31b5bbd07df0e8a1f0f56a4f58debecbca40c7082f8e2d54ac

```md
## 1. Transport Fix

- [x] 1.1 Configure the OddsPortal client to request identity encoding.
- [x] 1.2 Add focused request-header and response-handling regression coverage.

## 2. Verification

- [ ] 2.1 Run formatting, the full test suite, release build, and live runtime verification.
```

## openspec/changes/harden-oddsportal-runtime/specs/oddsportal-js-odds/spec.md

- Source: openspec/changes/harden-oddsportal-runtime/specs/oddsportal-js-odds/spec.md
- Lines: 1-19
- SHA256: a454ad8d08e9d32074165c861cb5f1193eb7889d813c9096051f9955ea811c2c

```md
## MODIFIED Requirements

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, JSONL path, and positive polling
interval from configuration, SHALL use the shared root match home-team and away-team pair from root
configuration, SHALL use the root proxy setting for HTTP requests, and SHALL request identity
content encoding for those HTTP responses.

#### Scenario: Proxy-routed identity request
- **WHEN** the enabled OddsPortal collector issues an HTTP request
- **THEN** the request uses the configured root proxy and sends `Accept-Encoding: identity`

#### Scenario: Disabled OddsPortal collector
- **WHEN** `oddsportal.enabled` is false
- **THEN** no OddsPortal collector task is spawned

#### Scenario: Invalid polling interval
- **WHEN** OddsPortal is enabled with `poll_interval_seconds` equal to zero
- **THEN** configuration validation fails before an OddsPortal task is spawned
```

