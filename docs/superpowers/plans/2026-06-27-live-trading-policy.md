---
change: remove-live-trading-prohibition
design-doc: docs/superpowers/specs/2026-06-27-live-trading-policy-design.md
base-ref: 89ebd963f43f173b191021966bd546083b01d25e
---

# Live-Trading Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permit explicitly requested Polymarket live-trading work while preserving accurate current-state documentation and security boundaries.

**Architecture:** Keep the existing provider ownership model unchanged. Update contributor policy and architecture guidance only; defer authenticated runtime execution to a separate OpenSpec change.

**Tech Stack:** Markdown, OpenSpec, Cargo verification

## Global Constraints

- Do not change Rust source, APIs, dependencies, or runtime behavior.
- Keep authenticated trading concerns under `src/polymarket/`.
- Keep secrets out of source control, logs, fixtures, and test output.
- Keep OddsPortal read-only and unauthenticated.

archived-with: 2026-06-27-remove-live-trading-prohibition
---

### Task 1: Update live-trading policy documentation

**Files:**
- Modify: `AGENTS.md`
- Modify: `ARCHITECTURE.md`

**Interfaces:**
- Consumes: Existing provider boundaries documented in `ARCHITECTURE.md`.
- Produces: Contributor and architecture rules that permit a later, explicitly requested Polymarket live-trading capability.

- [x] **Step 1: Replace the blanket contributor prohibition**

Update `AGENTS.md` so live trading, credentials, signing, and order placement require an explicit task. Add requirements that secrets remain outside source control and output and that authenticated trading stays under `src/polymarket/`.

- [x] **Step 2: Preserve current-state architecture facts**

Keep the current collector and mock executor described as unauthenticated, read-only, and simulation-only.

- [x] **Step 3: Permit future authenticated execution**

Document that a separately requested Polymarket capability may add credential loading, signing, placement, cancellation, and validation behind provider boundaries.

- [x] **Step 4: Verify documentation scope**

Run:

```bash
git diff --check
test -z "$(git diff 2e31a093aa48995643e3e2f7e6156da8614598e9...HEAD --name-only -- '*.rs')"
```

Expected: both commands exit successfully with no output.

- [x] **Step 5: Build the unchanged Rust project**

Run:

```bash
cargo build
```

Expected: exit status `0`; existing dead-code warnings are allowed.

- [x] **Step 6: Commit the documentation policy**

Commits:

```text
856a9f4 tweak: permit explicitly requested live trading
89ebd96 docs: design live trading policy boundary
```
