---
comet_change: switch-target-to-jordan-argentina
role: technical-design
canonical_spec: openspec
---

# Switch Provider Target to Jordan–Argentina

## Scope

Change the existing provider target values without changing provider interfaces, runtime
orchestration, parsing, logging, JSONL schemas, or trading behavior.

The existing target-specific acceptance examples in `polymarket-ws-quotes` and
`oddsportal-js-odds` are updated to the same Jordan–Argentina values.

## Configuration

The committed runtime target is:

```yaml
polymarket:
  enabled: true
  url: https://polymarket.com/ja/sports/world-cup/fifwc-jor-arg-2026-06-27

oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  home_team: Jordan
  away_team: Argentina

trade:
  enabled: false
```

Provider log paths and the 30-second OddsPortal polling interval remain unchanged. The committed
proxy remains an invalid placeholder; deployment must supply a reachable proxy.

## Test Strategy

- A root configuration test reads committed `config.yaml`, constructs both provider runtimes,
  and asserts the exact Polymarket URL, team order, and disabled live-trading runtime.
- The localized URL test asserts the slug `fifwc-jor-arg-2026-06-27`.
- The complete Rust suite and release build must pass.
- A bounded read-only smoke test temporarily uses the documented reachable proxy, then restores
  the placeholder. It must show both provider prefixes, successful records in both JSONL files,
  and no trade-placement output.

## Verification Evidence

The completed smoke run lasted 90 seconds and exited through the timeout bound:

- Polymarket emitted 438 prefixed lines, discovered 6 tokens, and grew its JSONL file.
- OddsPortal emitted 85 prefixed lines, completed two 39-record passes, and grew its JSONL file.
- No trade-placement line was emitted.
- The temporary proxy override was restored before commit.

## Risks

- Team ordering is explicit: Jordan is home, Argentina is away.
- The Polymarket slug date is 2026-06-27 even when local OddsPortal display time crosses into
  2026-06-28.
- Switching the target cannot fix a deployment proxy DNS failure; operators must replace
  `YOUR_PROXY_URL`.
