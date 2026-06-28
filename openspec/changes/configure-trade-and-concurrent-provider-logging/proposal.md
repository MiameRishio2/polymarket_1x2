## Why

The binary currently completes one OddsPortal collection before it starts the long-running
Polymarket stream, so operators cannot observe both providers running at the same time and a
slow or failing OddsPortal request delays all Polymarket output. Trading is also inferred only
from the three `real` mode values instead of being an independently enabled feature, while the
provider targets needed for operational testing are not represented together in `config.yaml`.

## What Changes

- Add explicit runtime sections in `config.yaml` for Polymarket and OddsPortal collection,
  including enable flags, provider targets, log paths, and an OddsPortal polling interval.
- Add `trade.enabled` as a separate, fail-closed live-trading switch; authenticated trading is
  allowed only when it is enabled and the existing three-mode `real` gate also passes.
- Start enabled Polymarket and OddsPortal collectors concurrently so either provider can emit
  records without waiting for the other.
- Add provider-prefixed startup, progress, retry, and failure output so the managed process log
  shows what each provider is doing even before quote or odds records arrive.
- Configure the supplied Australia–Egypt World Cup Polymarket URL as the test target and verify
  that Polymarket and OddsPortal output can be observed in the same process log.
- Preserve provider-specific parsing, transport, and JSONL logging under their existing source
  subtrees; keep top-level task orchestration in `src/main.rs`.

## Capabilities

### New Capabilities

- `provider-runtime-orchestration`: Root configuration loading, independent feature gates,
  concurrent provider task supervision, and provider-attributed operational output.

### Modified Capabilities

- `polymarket-live-trading`: Require an explicit independent trade enable flag in addition to
  the existing three-mode live gate.
- `polymarket-ws-quotes`: Load the Polymarket event URL and quote-log destination from root
  configuration and expose provider-attributed lifecycle output while collecting.
- `oddsportal-js-odds`: Load OddsPortal targets and logging settings from root configuration and
  support repeated collection while the Polymarket stream runs.

## Impact

- `config.yaml` gains structured provider runtime settings and an explicit trade enable flag.
- `src/config.rs` becomes the shared root YAML loader and runtime feature assembler.
- `src/main.rs` changes from sequential startup to concurrent orchestration.
- `src/polymarket/config.rs` retains account, credential, trade-gate, and Polymarket runtime
  conversion, while authenticated trading remains isolated under `src/polymarket/`.
- `src/oddsportal/config.rs` gains deserializable runtime settings and polling behavior remains
  provider-local.
- `ARCHITECTURE.md` and `DEPLOYMENT.md` must describe the new configuration, concurrent process
  behavior, provider-prefixed output, and log paths.
