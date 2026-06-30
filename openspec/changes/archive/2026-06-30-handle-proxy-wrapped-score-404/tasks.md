## 1. OddsPortal Score Response

- [x] 1.1 Add a failing regression test for a proxy-wrapped score 404, then classify the exact
  wrapper as an unavailable score before `.dat` decoding.

## 2. Configuration Safety Test

- [x] 2.1 Remove mutable match-value assertions from the committed configuration safety test
  while preserving provider, interval, and read-only assertions.

## 3. Verification

- [x] 3.1 Run formatting, focused tests, the full Rust test suite, and a release build.
