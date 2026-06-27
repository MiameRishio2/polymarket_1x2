# Concurrent Provider Runtime Verification

Date: 2026-06-27

Status: **COMPLETE**

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

The first run used the literal committed proxy placeholder and could not reach
either upstream because the proxy address failed DNS resolution. A
single-variable environmental retest temporarily replaced only that placeholder
with the documented project default proxy, `http://10.32.110.233:7890`.
Credentials, trading settings, production code, and all other configuration
were unchanged.

Command:

```bash
timeout 90s target/release/polymarket-1x2 > /tmp/polymarket-1x2-smoke.log 2>&1
```

The command ran for 90 seconds and exited 124, as expected for a process that
remained alive through the bound.

Evidence checks:

| Check | Exit | Evidence |
| --- | ---: | --- |
| Polymarket prefix search | 0 | 16 prefixed lines, including discovery and subscription |
| OddsPortal prefix search | 0 | 5 prefixed polling lines |
| `test -s logs/polymarket_quotes.log` | 0 | File contained 232 lines after the run |
| No trade placement output | 0 | 0 matching placement lines |

The Polymarket JSONL grew from 84,968 bytes to 89,231 bytes during the retest,
proving that this run delivered records to the configured file. OddsPortal
emitted its required provider prefix and continued polling after pass failures,
independently of the active Polymarket stream.

## Configuration restoration

Immediately after collecting evidence, `config.yaml` was restored with
`proxy: "YOUR_PROXY_URL"` using the same patch mechanism. A subsequent
`git diff --exit-code -- config.yaml` exited 0, confirming that no proxy change
remained. `trade.enabled` was still `false`.

The bounded smoke and output criteria pass, so OpenSpec tasks 5.2 and 5.3 are
complete.
