# Deployment Scripts Design

## Scope

Add a small, repository-local deployment interface for building, starting, and
stopping the `polymarket-1x2` binary. Document the interface in
`DEPLOYMENT.md` and require agents to read that document before changing
deployment behavior.

This change does not alter Rust source code, application configuration, live
trading gates, or provider behavior.

## Files

- `scripts/build.sh`: compile the release binary.
- `scripts/start.sh`: start one background collector instance.
- `scripts/stop.sh`: stop the instance started by `start.sh`.
- `DEPLOYMENT.md`: document prerequisites, configuration, commands, runtime
  files, logs, and failure behavior.
- `AGENTS.md`: add the deployment-document dependency and synchronization rule.

## Script Contract

All scripts use Bash strict mode and calculate the repository root from their
own location. They therefore behave consistently when called outside the
repository root.

### Build

`scripts/build.sh` runs `cargo build --release` from the repository root. The
expected artifact is `target/release/polymarket-1x2`.

### Start

`scripts/start.sh`:

1. Requires the release binary to exist and be executable; it does not compile
   implicitly.
2. Creates `run/` and `logs/` when needed.
3. Refuses to start a second instance when the recorded process is alive and
   its executable matches the release binary.
4. Removes a stale PID file when its process no longer exists.
5. Refuses to overwrite a PID file when that PID belongs to a different
   executable.
6. Starts the binary from the repository root with `nohup`, appends stdout and
   stderr to `logs/polymarket-1x2.out.log`, and records the PID in
   `run/polymarket-1x2.pid`.
7. Verifies that the new process remains alive immediately after startup. On
   failure, it removes the PID file and directs the operator to the log.

Linux `/proc/<pid>/exe` is used to validate process identity before treating a
PID as this application.

### Stop

`scripts/stop.sh`:

1. Reports success when no PID file exists.
2. Removes a stale PID file when the recorded process no longer exists.
3. Refuses to signal a live process whose executable does not match the release
   binary.
4. Sends `SIGTERM` to a matching process and waits for a bounded period for
   normal shutdown.
5. Removes the PID file after confirmed exit.
6. Returns an error on timeout and does not escalate to `SIGKILL`.

## Documentation and Agent Guidance

`DEPLOYMENT.md` describes:

- Linux, Bash, and Rust/Cargo prerequisites.
- The required build-before-start workflow.
- The repository-root `config.yaml` dependency and the existing live-trading
  safety gates.
- Start, status inspection, log inspection, and stop commands.
- Runtime paths and expected recovery steps for startup or shutdown errors.

`AGENTS.md` states that agents must read `DEPLOYMENT.md` before changing
deployment documentation, scripts, or runtime operations, and must keep the
document synchronized with those changes.

## Validation

- Run `bash -n` on all three scripts.
- Run `cargo build --release`.
- Exercise script safety without launching the real collector against external
  services: missing-binary handling, missing/stale PID handling, duplicate PID
  protection, and mismatched-process protection where practical.
- Inspect the final diff and executable bits.

No Git commit is created because the repository instructions require explicit
user authorization before committing.
