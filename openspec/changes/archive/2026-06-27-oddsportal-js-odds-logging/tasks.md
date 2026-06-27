## 1. OddsPortal Fixtures And Decoding

- [x] 1.1 Add captured fixture coverage for the Norway - France tournament/H2H embedded state and compressed `.dat` response shape.
- [x] 1.2 Implement a JXG-compatible `.dat` decoder with unit tests for base64, inflate, URL-decode, and JSON parse behavior.

## 2. OddsPortal Provider

- [x] 2.1 Add OddsPortal config and data models under `src/oddsportal/`.
- [x] 2.2 Implement tournament page match discovery from embedded state.
- [x] 2.3 Implement H2H page request metadata extraction and internal pre-match `.dat` URL construction.
- [x] 2.4 Implement 1X2 odds extraction and normalization from decoded OddsPortal data.
- [x] 2.5 Implement append-only OddsPortal JSONL logging.

## 3. Binary Orchestration

- [x] 3.1 Wire OddsPortal collection into `src/main.rs` while preserving existing Polymarket behavior.
- [x] 3.2 Ensure provider failures include provider-specific context in errors or logs.
- [x] 3.3 Update `ARCHITECTURE.md` if the runtime data flow or provider responsibilities change.

## 4. Verification

- [x] 4.1 Run focused OddsPortal parser/decoder tests.
- [x] 4.2 Run `cargo test`.
- [x] 4.3 Perform a network smoke test when OddsPortal is reachable.
