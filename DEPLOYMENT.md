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
tail -f logs/polymarket_quotes.log
tail -f logs/oddsportal_odds.log
```

Because `scripts/start.sh` intentionally redirects both streams to this one
file, the first command shows a mixture of machine-readable observation JSONL
from stdout and human-readable diagnostics from stderr. Diagnostic lines begin
with an RFC 3339 UTC millisecond timestamp, for example:

```text
2026-06-30T12:34:56.789Z [oddsportal] starting collection pass
```

After the timestamp, `[polymarket]` labels Polymarket discovery, WebSocket,
reconnect, and terminal diagnostics; `[oddsportal]` labels OddsPortal polling,
retry, and terminal diagnostics; `[trade]` is reserved for the separately
gated live-order lifecycle; `[runtime]` identifies a task failure that cannot
be attributed to a known provider. A terminal error from one provider can
appear while the other provider continues because enabled collectors are
supervised independently.

The other two commands follow the default detailed quote JSONL files. Their
paths can be changed with `polymarket.log_path` and `oddsportal.log_path`.
They retain their existing pure-JSON quote record formats, carry a `ts`
timestamp, and do not contain score observations. For a clean machine-readable
stream in a foreground run, keep stdout and stderr separate:

```bash
cargo run >observations.jsonl 2>diagnostics.log
```

`observations.jsonl` then contains only the four supported observation types:
`polymarket_odds`, `polymarket_score`, `oddsportal_odds`, and
`oddsportal_score`; every object carries `received_at`. Timestamped diagnostics
retain their provider prefixes only in `diagnostics.log`.

OddsPortal prices are collected only from the target match page's
`requestLive` `/feed/live-event/...dat` resource. Before kickoff, after
completion, or whenever that feed is unavailable, no `oddsportal_odds` record
is emitted. The collector never requests or falls back to `requestPreMatch`.

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
- Keep `trade.enabled: false` for read-only collection. Live trading requires
  `trade.enabled: true` in addition to `trade.trader_mode`,
  `trade.account_mode`, and `trade.market_mode` all being `real`.
- Treat an existing configuration without `trade.enabled` as read-only:
  the field defaults to `false`, even when all three modes are already `real`.
- Review `polymarket.enabled`, `oddsportal.enabled`, and the positive
  `oddsportal.poll_interval_seconds`. Enabled providers run concurrently, and
  at least one provider must be enabled.
- Each non-overlapping tick starts one OddsPortal odds operation and one score
  operation concurrently. The score side makes zero HTTP calls when no score
  URL was discovered, otherwise one. The odds side always refreshes the target
  H2H page and makes one additional `requestLive` call only while the match is
  live. This produces one to three HTTP calls in a normal cycle. H2H retries
  can add up to two calls after failures. The committed interval is one
  second, while the observed live feed advertises a ten-second refresh, so
  repeated values are expected and aggressive polling may be rate-limited.
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

If only one provider reports a terminal error, continue inspecting the log:
the other provider is not cancelled and can keep collecting. A provider that
stops is not automatically restarted inside the process. OddsPortal
collection-pass failures are non-terminal and are retried after the configured
polling interval.

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
