## 1. Configuration Contracts

- [x] 1.1 Add failing tests for nested provider configuration, the exact localized Australia–Egypt URL, provider enable defaults, and positive OddsPortal polling intervals.
- [x] 1.2 Add failing tests proving `trade.enabled` defaults false and is required in addition to all three `real` modes.
- [x] 1.3 Implement root-to-provider configuration conversion and validation without exposing credential fields.

## 2. Concurrent Provider Runtime

- [x] 2.1 Add failing orchestration tests for enabled-provider selection, no-provider rejection, and independent task completion handling.
- [x] 2.2 Implement the OddsPortal polling loop with success/error reporting, interval waits, and continued polling after a failed pass.
- [x] 2.3 Replace sequential startup with independently spawned enabled provider tasks that do not cancel each other on one provider's failure.

## 3. Provider-Attributed Output

- [x] 3.1 Add focused tests for provider-labelled lifecycle formatting where deterministic assertions are practical.
- [x] 3.2 Prefix Polymarket startup, discovery, snapshots, subscription, updates, reconnects, and failures without changing quote JSONL records.
- [x] 3.3 Prefix OddsPortal startup, pass status, records, retries, and failures without changing odds JSONL records.
- [x] 3.4 Prefix live-order lifecycle output separately and verify diagnostics still redact secrets.

## 4. Operational Configuration and Documentation

- [x] 4.1 Update `config.yaml` with enabled Australia–Egypt provider targets, separate log paths, a 30-second OddsPortal interval, and `trade.enabled: false`.
- [ ] 4.2 Update `ARCHITECTURE.md` for root configuration ownership and concurrent provider data flow.
- [ ] 4.3 Update `DEPLOYMENT.md` for feature gates and simultaneous provider output inspection.

## 5. Verification

- [ ] 5.1 Run formatting and the complete Rust test suite.
- [ ] 5.2 Build the release binary and run a bounded, trading-disabled smoke test against the supplied Polymarket URL.
- [ ] 5.3 Verify the captured process output contains both `[polymarket]` and `[oddsportal]` lines and verify Polymarket records reach its configured JSONL file.
