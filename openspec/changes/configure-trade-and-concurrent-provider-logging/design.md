## Context

`main` currently invokes `oddsportal::collect_once(...).await` before it loads the Polymarket
configuration. OddsPortal network latency therefore blocks Polymarket discovery and WebSocket
startup. After the one OddsPortal pass completes, only the Polymarket future remains alive, so
the process cannot produce fresh OddsPortal records alongside WebSocket quote records.

The root YAML parser is owned by `src/polymarket/config.rs` because it was introduced for
authenticated Polymarket execution. It currently creates a Polymarket market-data config from
hard-coded defaults and derives live-trading activation solely from three mode strings.
OddsPortal uses `Config::default()` directly and cannot be configured from the file.

Provider implementation boundaries in `ARCHITECTURE.md` must remain intact. Top-level lifecycle
coordination belongs in `src/main.rs`; provider-specific configuration conversion, network
requests, parsing, and JSONL writing remain in each provider subtree. Trading must remain
fail-closed, credential-safe, and one-shot.

## Goals / Non-Goals

**Goals:**

- Represent enabled Polymarket collection, enabled OddsPortal collection, and enabled live
  trading as three independently configured runtime features.
- Start the two enabled collectors without a sequential dependency.
- Keep an OddsPortal polling loop alive while the Polymarket WebSocket stream runs.
- Make startup and provider lifecycle visible in the shared stdout/stderr process log with
  unambiguous provider prefixes.
- Make the Australia–Egypt Polymarket event URL and matching OddsPortal target configurable and
  testable without enabling authenticated writes.
- Preserve the existing provider-local JSONL formats and paths by default.

**Non-Goals:**

- Combining Polymarket and OddsPortal records into one file or introducing a shared record model.
- Adding order placement for OddsPortal or changing the fixed Polymarket live-order strategy.
- Building a general-purpose task scheduler, logging framework, or dynamic configuration reload.
- Retrying live trading after any startup or stream failure.

## Decisions

### 1. Root YAML owns feature selection; provider modules own typed provider settings

The root file configuration will deserialize these explicit sections:

```yaml
polymarket:
  enabled: true
  url: https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03
  log_path: logs/polymarket_quotes.log

oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  home_team: Australia
  away_team: Egypt
  log_path: logs/oddsportal_odds.log
  poll_interval_seconds: 30

trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
```

The single root loader moves to `src/config.rs` because accounts, proxy, both providers, and
trade gates must be assembled together. It delegates credential and trade validation to
`src/polymarket/config.rs` and OddsPortal settings conversion to `src/oddsportal/config.rs`.
This is the narrow shared module permitted by the architecture because both providers consume
the same root configuration and proxy. Missing provider sections retain existing collector
defaults and remain enabled for compatibility. Missing `trade.enabled` is treated as `false`
because live writes require explicit authorization.

Alternatives considered: keeping the cross-provider root type under `src/polymarket/`, or parsing
the same YAML a second time in OddsPortal. Both were rejected because they either violate the
provider boundary or duplicate parsing and validation.

### 2. `trade.enabled` is an additional gate, not a replacement gate

Live configuration is produced only when `trade.enabled == true` and all three existing modes
are exactly `real`. Account selection and credential validation occur only after both gates pass.
The supplied test configuration sets `trade.enabled: false`, so the requested public market-data
test cannot place or cancel orders even though legacy mode fields remain `real`.

Alternative considered: replacing the three modes with one boolean. Rejected because it removes
the existing defense in depth and changes a documented safety contract beyond this request.

### 3. Main starts independent long-lived provider futures

`main` constructs enabled provider runtimes first, prints their selected targets without secrets,
then spawns each provider as an independently owned Tokio task. It waits for task completions
without cancelling the remaining provider when one exits. Startup fails clearly if no collector
is enabled.

The Polymarket task performs discovery once and then enters its existing WebSocket reconnect
loop. The optional live flow remains inside this task and can execute at most once.

The OddsPortal task owns `run_poll_loop`: call `collect_once`, report success or a contextual
error, sleep for the configured interval, and repeat. A failed pass does not terminate the loop
or block Polymarket. The polling interval must be greater than zero.

Alternative considered: `tokio::try_join!` over the current one-shot functions. Rejected because
OddsPortal would finish immediately and a provider error would cancel or terminate supervision
of the other provider.

### 4. Operational output uses stable textual provider prefixes

Existing `println!` and `eprintln!` calls on the affected paths will use `[polymarket]`,
`[oddsportal]`, or `[trade]` prefixes. Each collector prints at least:

- startup with a non-secret target;
- successful discovery/subscription or completed polling pass;
- retry/disconnection/failure context.

Quote and odds record output also carries its provider prefix. JSONL files remain provider-local
and unchanged. This uses the current standard-output mechanism rather than adding a logging
dependency solely for attribution.

Alternative considered: adopting `tracing` and structured subscribers. Rejected as unnecessary
scope for a small binary whose deployment already captures stdout/stderr in one file.

### 5. Verification separates deterministic tests from the live smoke test

Unit tests cover YAML defaults and overrides, the independent trade gate, invalid polling
intervals, enabled-provider selection, and orchestration helpers without external network writes.
After `cargo test`, a bounded smoke run uses the supplied Australia–Egypt URL with trading
disabled. Verification checks the process output for both provider prefixes and the Polymarket
JSONL path for records. If external network or proxy access prevents live verification, that is
reported as a verification failure rather than replaced with a claim based only on unit tests.

## Risks / Trade-offs

- [OddsPortal polling can trigger upstream throttling] → Use a configurable positive interval,
  retain request retries, and default to a conservative 30 seconds.
- [One provider task can terminate while the process remains alive] → Print a provider-attributed
  terminal error; keep the other task alive as required instead of hiding the degraded state.
- [A localized Polymarket URL may parse differently] → Retain slug-based extraction and add a
  focused test for the exact supplied `/ja/sports/...` URL.
- [Legacy configs with all three modes set to `real` stop trading] → This is intentional
  fail-closed migration; operators must add `trade.enabled: true` explicitly.
- [Concurrent stdout lines can interleave] → Each record is emitted with one line-level
  `println!`/`eprintln!` call and a stable provider prefix.
- [A bounded live test may see no WebSocket update] → Initial CLOB snapshots must also emit
  Polymarket output and JSONL records, avoiding dependence on a later market tick.

## Migration Plan

1. Add nested provider settings and `trade.enabled: false` to the committed configuration.
2. Deploy the updated binary and confirm both configured startup lines in
   `logs/polymarket-1x2.out.log`.
3. Confirm provider JSONL files update independently.
4. Enable trading only in a separately reviewed configuration by setting `trade.enabled: true`
   while preserving all three `real` modes and valid credentials.

Rollback uses the previous binary and previous configuration together. The new binary accepts
missing provider sections through defaults, but the old binary ignores provider sections and
does not understand the new independent trade authorization semantics.

## Open Questions

None for implementation. The selected initial OddsPortal polling interval is 30 seconds and can
be changed in `config.yaml`.
