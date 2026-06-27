# Concurrent Provider Runtime Verification

Date: 2026-06-27

Status: **BLOCKED on live provider evidence**

## Deterministic verification

All deterministic commands ran from the repository root:

| Command | Exit | Evidence |
| --- | ---: | --- |
| `cargo fmt --check` | 0 | Formatting clean |
| `cargo test` | 0 | 62 passed; 0 failed; 0 ignored |
| `./scripts/build.sh` | 0 | Release binary built at `target/release/polymarket-1x2` |

OpenSpec tasks 4.2, 4.3, and 5.1 are complete.

## Read-only smoke test

The safety precheck confirmed `trade.enabled: false`. The managed PID file was
stale, and no managed collector process was running. No process was stopped or
signaled.

Command:

```bash
timeout 90s target/release/polymarket-1x2 > /tmp/polymarket-1x2-smoke.log 2>&1
```

The command ran for 90 seconds and exited 124, as expected for a process that
remained alive through the bound.

Evidence checks:

| Check | Exit | Evidence |
| --- | ---: | --- |
| Polymarket prefix search | 0 | 2 prefixed lines |
| OddsPortal prefix search | 0 | 10 prefixed lines |
| `test -s logs/polymarket_quotes.log` | 0 | File existed with 221 lines |
| No trade placement output | 0 | 0 matching placement lines |

The existing Polymarket JSONL was 84,968 bytes before the smoke run and 84,968
bytes afterward. It therefore does not prove that this run delivered a
Polymarket record.

## External blocker

The configured placeholder proxy could not resolve its underlying address.
Polymarket terminated after the Gamma request failed with a proxy tunnel DNS
error ending in `Temporary failure in name resolution`. OddsPortal remained
running and retried on its configured interval, but each tournament request
failed with the same proxy tunnel DNS error.

No proxy setting was substituted, and no credential or trading setting was
changed. Because the environment prevented either provider from reaching its
upstream and the Polymarket JSONL did not grow, OpenSpec tasks 5.2 and 5.3
remain unchecked.
