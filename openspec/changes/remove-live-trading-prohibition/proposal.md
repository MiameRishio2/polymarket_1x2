## Why

Repository guidance currently prohibits credentials, private-key handling, and order placement outright. That policy blocks future explicitly requested live-trading work even when it is designed with appropriate secret handling and safety controls.

## What Changes

- Remove the blanket prohibition on live-trading-related implementation from `AGENTS.md`.
- Clarify in `ARCHITECTURE.md` that current execution remains read-only while future authenticated Polymarket trading paths are permitted.
- Retain security expectations for secrets, signing, and order lifecycle boundaries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change updates contributor and architecture policy only; it does not change runtime requirements or behavior.

## Impact

Only `AGENTS.md` and `ARCHITECTURE.md` are affected. No Rust source, API, dependency, or runtime behavior changes.
