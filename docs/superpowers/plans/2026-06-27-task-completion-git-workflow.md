# Task Completion Git Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make validated task completion automatically commit task-scoped changes, integrate them into `main`, and push `origin/main`.

**Architecture:** Replace the conflicting no-commit bullet in `AGENTS.md` with one dedicated completion-workflow section. The section defines the happy path and explicit stop conditions without changing application or deployment behavior.

**Tech Stack:** Markdown, Git

## Global Constraints

- Validate before committing or reporting completion.
- Commit only the current task's changes and preserve unrelated user work.
- Integrate non-`main` work with a normal, non-destructive merge.
- Push only the resulting `main` branch to `origin/main`.
- Never force-push, discard changes, or rewrite shared history.
- Stop and report if validation, commit, merge, or push fails.

---

### Task 1: Define and publish the completion workflow

**Files:**
- Modify: `AGENTS.md`
- Create: `docs/superpowers/plans/2026-06-27-task-completion-git-workflow.md`

**Interfaces:**
- Consumes: the repository guidance dependency on `ARCHITECTURE.md`.
- Produces: unambiguous default Git integration instructions for future agents.

- [ ] **Step 1: Verify the current guidance lacks the new workflow**

Run:

```bash
! rg -q '^## Completion Workflow$' AGENTS.md
rg -n 'Do not commit changes unless the user explicitly asks' AGENTS.md
```

Expected: both commands exit zero, proving the new section is absent and the
conflicting rule is present.

- [ ] **Step 2: Replace the conflicting rule**

Remove this bullet from `Validation`:

```markdown
- Do not commit changes unless the user explicitly asks.
```

Add this section after `Validation`:

```markdown
## Completion Workflow

- Before marking a task complete, run all validation appropriate to the files changed and require it to pass.
- Stage and commit only changes that belong to the current task; preserve unrelated and pre-existing user changes.
- If already on `main`, commit there. If working on another branch, integrate it into `main` with a normal, non-destructive merge.
- Push the resulting `main` branch to `origin/main` without requiring another confirmation.
- If validation, commit, merge, or push fails, stop and report the failure instead of claiming completion.
- Never force-push, discard changes, rewrite shared history, or include secrets and runtime files to complete this workflow.
```

- [ ] **Step 3: Verify the guidance is complete and non-contradictory**

Run:

```bash
rg -n '^## Completion Workflow$' AGENTS.md
rg -n 'origin/main' AGENTS.md
! rg -q 'Do not commit changes unless the user explicitly asks' AGENTS.md
git diff --check
```

Expected: all commands exit zero; the section and push target are present, the
old restriction is absent, and the diff has no whitespace errors.

- [ ] **Step 4: Commit only the task files**

Run:

```bash
git add AGENTS.md docs/superpowers/plans/2026-06-27-task-completion-git-workflow.md
git diff --cached --check
git commit -m "docs: require completed tasks to reach main"
```

Expected: one commit containing only the `AGENTS.md` change and this plan.

- [ ] **Step 5: Push and verify the remote**

Run:

```bash
git push origin main
git status --short
git log -1 --oneline --decorate
```

Expected: push succeeds, the worktree is clean, and local `main` and
`origin/main` point to the new completion-workflow commit.
