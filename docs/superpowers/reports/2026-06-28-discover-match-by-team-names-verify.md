# Discover Match by Team Names Verification

Verified on 2026-06-29 in branch `discover-match-by-team-names`.

## Documentation

- Updated `ARCHITECTURE.md` for the shared match pair, Gamma team-name discovery, independent
  Polymarket market and sports WebSockets, concurrent OddsPortal odds and score requests, the four
  stdout observation types, and provider-prefixed stderr diagnostics.
- Expanded `README.md` with build/run commands, the exact shared `match`, `polymarket`, and
  `oddsportal` configuration, read-only trade gate, output examples, and the OddsPortal refresh
  and rate-limit caveat.
- Updated `DEPLOYMENT.md` because `scripts/start.sh` combines stdout and stderr in one file. The
  guide now distinguishes stdout observation JSONL, stderr diagnostic text, and legacy detailed
  quote logs.

## Commands and results

| Command | Exit | Result |
| --- | ---: | --- |
| `cargo fmt --all -- --check` | 0 | Formatting check passed. |
| `git diff --check` | 0 | No whitespace errors. |
| Task-brief `rg -n 'polymarket\\.url\|oddsportal:\\n.*home_team\|cross-provider.*aggregat' ... \|\| true` | 0 | `rg` itself rejected literal `\n` without multiline mode; `|| true` made the shell command exit 0. |
| Corrected `rg -Un 'polymarket\\.url\|oddsportal:\\n.*home_team\|cross-provider.*aggregat' ... \|\| true` | 0 | One intentional proposal sentence matched: “without cross-provider aggregation.” No configured `polymarket.url`, duplicated OddsPortal team target, or aggregation requirement was found. |
| `cargo test output` | 0 | 8 focused output tests passed; 0 failed. |
| `cargo test` | 0 | 103 tests passed; 0 failed. |
| `openspec validate discover-match-by-team-names --strict` | 0 | `Change 'discover-match-by-team-names' is valid`. |

## Bounded read-only smoke

The committed placeholder proxy was not changed. Two `timeout 20s cargo run --manifest-path
/root/polymarket_1x2/Cargo.toml` attempts ran from temporary directories with a reachable,
non-user-info environment proxy substituted into temporary untracked copies of `config.yaml`.
Both retained `trade.enabled: false`; temporary configurations and captured streams were removed
after inspection.

First attempt:

- Exit 124, the expected timeout termination for a continuing collector.
- 12 stdout lines; every line passed `jq -e .`.
- Observed only supported types: `oddsportal_odds` and `oddsportal_score`.
- No `[trade]` placement or cancellation message.
- Both provider startup markers named South Africa and Canada, but no Polymarket observation was
  emitted during the window.
- The post-run diagnostic-tail sanitizer was invoked with an invalid `sed -F` option. This did
  not alter the already captured exit, JSON, type, or trade-message checks, but it prevented that
  attempt's detailed diagnostic tail from being displayed.

Second attempt:

- Exit 124, again due to the timeout.
- Both providers logged startup for South Africa versus Canada.
- OddsPortal terminated with `OddsPortal match not found for South Africa - Canada`.
- No stdout observations arrived, so `jq` had no records to validate.
- No `[trade]` placement or cancellation message and no checked secret marker appeared.

Live smoke is therefore **partial/blocked by current upstream match availability**. The first
attempt proves the bounded, trading-disabled process can emit valid OddsPortal observations; the
second proves the target was transiently absent at OddsPortal. Neither attempt produced
Polymarket observations, so this report does not claim successful live discovery and output from
both providers. The absence of `polymarket_score` is permitted when the match is not live, and an
unavailable OddsPortal score is represented with `available: false`.

OddsPortal advertises an approximately 15-second upstream refresh. The configured one-second,
two-request non-overlapping cycle cannot force new source data and may encounter repeated values
or rate limiting.

## Self-review

- Documentation examples were checked against the serialized Rust observation structs.
- `trade.enabled` remains `false`; no trading code or credential handling changed.
- No runtime logs, temporary configurations, credentials, target artifacts, or controller-owned
  `.comet.yaml` changes are included in the task diff.
- The verification evidence supports documentation task 5.2 and execution of the required
  validation/smoke procedure for task 5.3, with the live external blocker stated rather than
  converted into a success claim.
