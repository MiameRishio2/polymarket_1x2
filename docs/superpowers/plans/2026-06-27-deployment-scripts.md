# Deployment Scripts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add documented, safe repository-local commands to build, start, and stop one background `polymarket-1x2` process.

**Architecture:** Three Bash scripts derive the repository root from their own path and share fixed binary, PID, and log locations. A shell integration test runs copied scripts against fake binaries in temporary repositories so lifecycle behavior is verified without contacting external services.

**Tech Stack:** Bash, Cargo, Linux `/proc`, coreutils

## Global Constraints

- The supported runtime is Linux with Bash and Rust/Cargo installed.
- `start.sh` must not build implicitly.
- Runtime state is stored in `run/polymarket-1x2.pid`; output is appended to `logs/polymarket-1x2.out.log`.
- A live PID must be matched through `/proc/<pid>/exe` before it is signaled.
- Shutdown uses `SIGTERM` with a bounded wait and never escalates to `SIGKILL`.
- Do not change Rust source, configuration semantics, provider behavior, or live-trading gates.
- Do not create a Git commit without explicit user authorization.

---

### Task 1: Add and test the process lifecycle scripts

**Files:**
- Create: `scripts/build.sh`
- Create: `scripts/start.sh`
- Create: `scripts/stop.sh`
- Create: `tests/deployment_scripts.sh`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: Cargo package binary `target/release/polymarket-1x2`.
- Produces: commands `scripts/build.sh`, `scripts/start.sh`, and `scripts/stop.sh`; runtime files `run/polymarket-1x2.pid` and `logs/polymarket-1x2.out.log`.

- [ ] **Step 1: Write the failing lifecycle test**

Create `tests/deployment_scripts.sh` as a strict-mode Bash test. It must copy
the three deployment scripts into a temporary repository, create fake release
binaries, and assert these cases:

```bash
assert_fails "$repo/scripts/start.sh" "missing release binary"
assert_succeeds "$repo/scripts/stop.sh" "missing PID is already stopped"
assert_succeeds "$repo/scripts/start.sh" "starts fake long-running binary"
assert_fails "$repo/scripts/start.sh" "rejects duplicate start"
assert_succeeds "$repo/scripts/stop.sh" "terminates matching fake binary"
assert_succeeds "$repo/scripts/stop.sh" "removes stale PID"
assert_fails "$repo/scripts/stop.sh" "rejects mismatched live PID"
```

The fake collector is a compiled local C program rather than a shell script so
`/proc/<pid>/exe` resolves to the expected release-binary path:

```c
#include <signal.h>
#include <unistd.h>

static volatile sig_atomic_t running = 1;
static void stop(int signal_number) {
    (void)signal_number;
    running = 0;
}

int main(void) {
    signal(SIGTERM, stop);
    while (running) {
        pause();
    }
    return 0;
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
bash tests/deployment_scripts.sh
```

Expected: non-zero exit because `scripts/build.sh`, `scripts/start.sh`, and
`scripts/stop.sh` do not exist.

- [ ] **Step 3: Implement the scripts**

Create all scripts with `#!/usr/bin/env bash`, `set -euo pipefail`, and:

```bash
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
```

`build.sh` changes to `$REPO_ROOT` and runs:

```bash
cargo build --release
```

`start.sh` uses:

```bash
BINARY="$REPO_ROOT/target/release/polymarket-1x2"
PID_FILE="$REPO_ROOT/run/polymarket-1x2.pid"
LOG_FILE="$REPO_ROOT/logs/polymarket-1x2.out.log"
```

It validates numeric PID-file content, checks process liveness with
`kill -0`, compares canonical paths from `readlink -f
"/proc/$pid/exe"` and `readlink -f "$BINARY"`, creates runtime directories,
starts with:

```bash
(
    cd -- "$REPO_ROOT"
    nohup "$BINARY" >>"$LOG_FILE" 2>&1 &
    echo "$!" >"$PID_FILE"
)
```

It then waits briefly and requires both process liveness and executable
identity. On startup failure it removes the PID file and exits non-zero.

`stop.sh` performs the same numeric/liveness/identity checks, sends:

```bash
kill -TERM "$pid"
```

It polls `kill -0` for up to 10 seconds, removes the PID file after exit, and
returns non-zero without sending `SIGKILL` on timeout.

Make all four shell files executable.

- [ ] **Step 4: Ignore runtime PID state**

Append this exact entry to `.gitignore`:

```gitignore
/run/
```

- [ ] **Step 5: Run focused verification**

Run:

```bash
bash -n scripts/build.sh scripts/start.sh scripts/stop.sh tests/deployment_scripts.sh
bash tests/deployment_scripts.sh
```

Expected: both commands exit zero and the integration test reports every
lifecycle case as passing.

### Task 2: Document and validate the deployment contract

**Files:**
- Create: `DEPLOYMENT.md`
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: the script and runtime-path contract from Task 1.
- Produces: operator instructions and mandatory agent reading guidance.

- [ ] **Step 1: Write `DEPLOYMENT.md`**

Document:

```markdown
# Deployment

## Prerequisites

- Linux with Bash and `/proc`.
- A Rust toolchain providing `cargo`.
- A reviewed `config.yaml` in the repository root.

## Build

Run `./scripts/build.sh`.

## Start

Run `./scripts/start.sh`. The command starts one background process and refuses
duplicate instances.

## Status and logs

Inspect `run/polymarket-1x2.pid`, verify it with `ps -p "$(cat ...)"`, and
follow `logs/polymarket-1x2.out.log`.

## Stop

Run `./scripts/stop.sh`. It sends `SIGTERM`, waits up to 10 seconds, and never
forces `SIGKILL`.

## Configuration and safety

The process reads `config.yaml` from the repository root. Review proxy,
endpoints, account data, and all three live-trading mode gates before starting;
never place real secrets in source control or command output.

## Troubleshooting

Explain missing binaries, startup-log inspection, stale/mismatched PID
protection, and manual investigation after a shutdown timeout.
```

- [ ] **Step 2: Add the `AGENTS.md` reading constraint**

Extend the dependency guidance with:

```markdown
## Deployment Dependency

Before changing deployment documentation, build/start/stop scripts, runtime
paths, or process-management behavior, read `DEPLOYMENT.md` and keep it
synchronized with those changes.
```

- [ ] **Step 3: Build and inspect the complete change**

Run:

```bash
./scripts/build.sh
bash -n scripts/build.sh scripts/start.sh scripts/stop.sh tests/deployment_scripts.sh
bash tests/deployment_scripts.sh
git diff --check
git status --short
```

Expected: release build succeeds, syntax and lifecycle tests pass, diff check
is clean, and only the requested documentation, scripts, test, ignore rule,
design, and plan files are changed.
