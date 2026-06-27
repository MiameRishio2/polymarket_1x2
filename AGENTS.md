# Agent Instructions

## Project Scope

This repository is a Rust binary for collecting Polymarket 1X2 market quotes. Keep changes focused on the requested behavior and avoid unrelated refactors.

## Architecture Dependency

`AGENTS.md` depends on `ARCHITECTURE.md` for the canonical project structure and module ownership rules. Before changing source layout, provider boundaries, or agent guidance, read `ARCHITECTURE.md` first and keep this file consistent with it.

## Deployment Dependency

Before changing deployment documentation, build/start/stop scripts, runtime paths, or process-management behavior, read `DEPLOYMENT.md` and keep it synchronized with those changes.

## Source Layout

- Polymarket-specific code belongs under `src/polymarket/`.
- OddsPortal-specific code belongs under `src/oddsportal/`.
- Add shared modules only when at least two provider implementations need the same abstraction.
- Keep `src/main.rs` as the binary orchestration layer.

## Validation

- Run `cargo test` after Rust code changes.
- Prefer focused tests near the code being changed.

## Completion Workflow

- Before marking a task complete, run all validation appropriate to the files changed and require it to pass.
- Stage and commit only changes that belong to the current task; preserve unrelated and pre-existing user changes.
- If already on `main`, commit there. If working on another branch, integrate it into `main` with a normal, non-destructive merge.
- Push the resulting `main` branch to `origin/main` without requiring another confirmation.
- If validation, commit, merge, or push fails, stop and report the failure instead of claiming completion.
- Never force-push, discard changes, rewrite shared history, or include secrets and runtime files to complete this workflow.

## Change Discipline

- Preserve existing runtime behavior unless the task explicitly asks for a behavior change.
- Introduce live-trading behavior, credentials, private-key handling, signing, or order placement only when the task explicitly requests it.
- Keep secrets out of source control, logs, fixtures, and test output; isolate authenticated trading concerns under `src/polymarket/` and add focused safety validation.
- Keep documentation synchronized with source layout changes.
