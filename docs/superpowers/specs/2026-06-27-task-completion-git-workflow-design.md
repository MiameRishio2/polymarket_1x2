# Task Completion Git Workflow Design

## Scope

Update `AGENTS.md` so completed tasks are integrated into `main` and pushed to
`origin/main` by default. This replaces the existing rule that prohibits
commits unless the user explicitly requests one.

This change affects agent workflow only. It does not change application source,
deployment behavior, or repository architecture.

## Completion Workflow

An agent must perform these actions when a task is ready to complete:

1. Run all validation appropriate to the files changed.
2. Confirm validation succeeded before creating a commit.
3. Stage and commit only changes that belong to the current task, preserving
   unrelated and pre-existing user changes.
4. If already on `main`, commit there. If working on another branch, integrate
   that branch into `main` with a normal, non-destructive merge.
5. Push the resulting `main` branch to `origin/main`.
6. Report the commit and push result.

The workflow is automatic and does not require a second confirmation after the
task requirements have been approved.

## Failure and Safety Rules

- If validation, commit, merge, or push fails, stop and report the exact
  failure. Do not claim the task is complete.
- Do not force-push.
- Do not discard changes, rewrite shared history, or use destructive Git
  commands to complete the workflow.
- Do not include secrets, runtime files, or unrelated changes in the task
  commit.
- A remote rejection must be resolved safely; it does not authorize an
  automatic rebase, conflict resolution that changes user work, or a
  force-push.

## Documentation Change

Add a dedicated `Completion Workflow` section to `AGENTS.md` and remove the
conflicting `Do not commit changes unless the user explicitly asks` bullet from
`Validation`.

## Validation

- Re-read `AGENTS.md` to confirm there is no contradictory commit guidance.
- Verify the new workflow names `main` and `origin/main` explicitly.
- Run `git diff --check`.
- Commit only this design, plan, and `AGENTS.md` change.
- Push the resulting `main` commit to `origin/main`.
