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

OddsPortal advertises an approximately 15-second upstream refresh. Each configured
non-overlapping one-second tick starts the odds and score operations concurrently. Depending on
score URL availability and whether the primary odds request needs its single fallback, this is
one to three HTTP calls per cycle and normally two. Polling cannot force new source data and may
encounter repeated values or rate limiting.

## Self-review

- Documentation examples were checked against the serialized Rust observation structs.
- `trade.enabled` remains `false`; no trading code or credential handling changed.
- No runtime logs, temporary configurations, credentials, target artifacts, or controller-owned
  `.comet.yaml` changes are included in the task diff.
- The verification evidence supports documentation task 5.2 and execution of the required
  validation/smoke procedure for task 5.3, with the live external blocker stated rather than
  converted into a success claim.

## Review correction verification

The documentation was corrected after review to state the exact OddsPortal request behavior:
each non-overlapping tick starts an odds operation and a score operation concurrently; the score
operation makes zero or one HTTP call, and the odds operation makes one primary call plus at most
one fallback after a failed or empty primary. This is one to three HTTP calls per cycle and
normally two.

### Live smoke and secret scan

A further bounded read-only smoke used a temporary copy of `config.yaml`, a non-user-info
environment proxy, and the following run command:

```bash
timeout 20s cargo run --manifest-path /root/polymarket_1x2/Cargo.toml \
  >stdout.jsonl 2>stderr.log
```

Result: exit 124, zero stdout lines, and no JSON records to validate. Both providers logged
startup for South Africa versus Canada, then OddsPortal reported `OddsPortal match not found for
South Africa - Canada`. This live run remains upstream-blocked and is not a dual-provider live
success.

The known-value pattern file was built from the committed `proxy`, `private_key`, `api_key`,
`api_secret`, and `api_passphrase` values without printing them:

```bash
awk '/^[[:space:]]*(proxy|private_key|api_key|api_secret|api_passphrase):/ {
  value=$0
  sub(/^[^:]*:[[:space:]]*/, "", value)
  gsub(/^"|"$/, "", value)
  print value
}' config.yaml | sort -u >known-values.txt
```

Both captured streams were then scanned. These commands use exit 0 to mean clean:

```bash
if grep -F -f known-values.txt stdout.jsonl stderr.log >/dev/null; then exit 1; else exit 0; fi
if grep -Eiq '(private_key|api_key|api_secret|api_passphrase|authorization|signature)' \
  stdout.jsonl stderr.log; then exit 1; else exit 0; fi
if grep -Eq '\[trade\].*(plac|cancel)' stderr.log; then exit 1; else exit 0; fi
```

Results: known-value scan exit 0, sensitive-marker scan exit 0, and trade-action scan exit 0.
Thus the live smoke stdout and stderr contained none of the committed placeholder/credential
values, sensitive field markers, authorization/signature markers, or placement/cancellation
messages.

### Deterministic four-observation scan

Because the upstream-blocked smoke emitted no observations, the deterministic subprocess helper
was captured:

```bash
ODDSPORTAL_POLLING_OUTPUT_HELPER=1 \
  cargo test oddsportal::tests::polling_output_helper -- --exact --nocapture \
  >helper.stdout 2>helper.stderr
grep '^{' helper.stdout >observations.jsonl
jq -e . observations.jsonl >/dev/null
```

The test, JSON extraction, and `jq` commands each exited 0. Exactly four observation lines were
captured: `polymarket_odds`, `polymarket_score`, `oddsportal_odds`, and `oddsportal_score`.
Applying the same known-value and sensitive-marker scans to `helper.stdout` and `helper.stderr`
returned exit 0 for both. This deterministic observation-path evidence proves the emitted data
records contain no checked credential keys or values; it does not substitute for dual-provider
live success.
