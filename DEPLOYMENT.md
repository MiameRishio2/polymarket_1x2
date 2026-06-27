# Deployment

The deployment scripts build and manage one background `polymarket-1x2`
process. They locate the repository from their own path, so they can be called
from any working directory.

## Prerequisites

- Linux with Bash, `/proc`, `nohup`, and standard core utilities.
- A Rust toolchain that provides `cargo`.
- A reviewed `config.yaml` in the repository root.

## Build

Build the release binary before starting the process:

```bash
./scripts/build.sh
```

The output binary is `target/release/polymarket-1x2`.

## Start

Start one background instance:

```bash
./scripts/start.sh
```

The script does not build implicitly. It refuses to start when the release
binary is missing, when the recorded application process is already running,
or when the PID file refers to another live executable.

## Status and Logs

The running process ID is stored in `run/polymarket-1x2.pid`. Inspect it with:

```bash
pid="$(cat run/polymarket-1x2.pid)"
ps -p "$pid" -o pid=,stat=,etime=,cmd=
```

Standard output and standard error are appended to
`logs/polymarket-1x2.out.log`:

```bash
tail -f logs/polymarket-1x2.out.log
```

Application quote logs configured by the Rust binary remain separate from
this process-output log.

## Stop

Stop the managed instance:

```bash
./scripts/stop.sh
```

The script confirms that the PID belongs to the release binary, sends
`SIGTERM`, and waits up to 10 seconds for a normal exit. It never escalates to
`SIGKILL`. A missing or stale PID file is treated as already stopped.

## Configuration and Safety

The process starts with the repository root as its working directory and reads
`config.yaml` from there. Before every deployment:

- Review the proxy, remote endpoints, market selection, and log paths.
- Review `trade.trader_mode`, `trade.account_mode`, and
  `trade.market_mode`. When all three values are `real`, the application can
  enter its explicitly gated live-order path.
- Keep private keys and API credentials out of source control, shell history,
  process arguments, logs, fixtures, and test output.

The scripts do not modify configuration and do not bypass any application
safety gate.

## Troubleshooting

### Release binary is missing

Run `./scripts/build.sh`, resolve any compiler errors, and retry
`./scripts/start.sh`.

### The process exits during startup

Inspect `logs/polymarket-1x2.out.log`. The start script removes its PID file
when the new process does not remain alive through the startup check.

### The PID file is invalid or belongs to another executable

The scripts refuse to overwrite or signal an unverified live PID. Inspect the
PID file and process manually:

```bash
cat run/polymarket-1x2.pid
readlink -f "/proc/$(cat run/polymarket-1x2.pid)/exe"
```

Remove the PID file only after confirming that it does not identify a running
`target/release/polymarket-1x2` process.

### Shutdown times out

Inspect the process and its output log. The stop script deliberately leaves
the PID file in place and does not force termination, so an operator can
diagnose the process before choosing any stronger action.
