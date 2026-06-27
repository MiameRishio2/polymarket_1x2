## Context

The codebase currently implements quote collection and a local order lifecycle simulation. Its contributor and architecture guidance also forbids any future credential, signing, or order-placement implementation.

## Implementation

Keep descriptions of current runtime behavior accurate, but replace blanket future prohibitions with scoped security and architecture rules. Explicitly requested live-trading work may introduce authenticated Polymarket paths, while secrets must remain outside source control and logs and trading concerns must remain within the Polymarket provider boundary.

## Non-Goals

- Implementing live trading.
- Changing current runtime behavior.
- Relaxing the read-only boundary of the OddsPortal provider.
