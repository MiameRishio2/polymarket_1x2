# Comet Design Handoff

- Change: remove-live-trading-prohibition
- Phase: design
- Mode: compact
- Context hash: aeedb2f61807e3ba8fa8e5a34bbfedc34d44034de200b55a3be3282e881d2edd

Generated-by: comet-handoff.sh

OpenSpec remains the canonical capability spec. This handoff is a deterministic, source-traceable context pack, not an agent-authored summary.

## openspec/changes/remove-live-trading-prohibition/proposal.md

- Source: openspec/changes/remove-live-trading-prohibition/proposal.md
- Lines: 1-23
- SHA256: 09fb53cd42b5c7ac06c0aeb64751737e3fb9ab0c31632e7ff51ebd203101c530

```md
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
```

## openspec/changes/remove-live-trading-prohibition/design.md

- Source: openspec/changes/remove-live-trading-prohibition/design.md
- Lines: 1-13
- SHA256: dd147a5cb00de93f9b31cf539fe5cab0ffb0a9a477fe456ce2eb87c285103213

```md
## Context

The codebase currently implements quote collection and a local order lifecycle simulation. Its contributor and architecture guidance also forbids any future credential, signing, or order-placement implementation.

## Implementation

Keep descriptions of current runtime behavior accurate, but replace blanket future prohibitions with scoped security and architecture rules. Explicitly requested live-trading work may introduce authenticated Polymarket paths, while secrets must remain outside source control and logs and trading concerns must remain within the Polymarket provider boundary.

## Non-Goals

- Implementing live trading.
- Changing current runtime behavior.
- Relaxing the read-only boundary of the OddsPortal provider.
```

## openspec/changes/remove-live-trading-prohibition/tasks.md

- Source: openspec/changes/remove-live-trading-prohibition/tasks.md
- Lines: 1-8
- SHA256: 2605bdaffbfe951681b04b718f71a7810d54620658f8730d12af3d9c92a868bd

```md
## 1. Documentation

- [x] 1.1 Replace the blanket live-trading prohibition in `AGENTS.md` with scoped security requirements.
- [x] 1.2 Update `ARCHITECTURE.md` to permit future authenticated Polymarket trading paths while accurately describing current behavior.

## 2. Verification

- [x] 2.1 Verify the two documents are consistent and no Rust source or runtime behavior changed.
```

