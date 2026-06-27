## ADDED Requirements

### Requirement: Provider source boundaries
The system SHALL keep provider-specific source code under provider-specific directories below `src/`.

#### Scenario: Polymarket code location
- **WHEN** code implements Polymarket Gamma, CLOB, WebSocket, quote state, or quote logging behavior
- **THEN** it resides under `src/polymarket/`

#### Scenario: OddsPortal code location
- **WHEN** code implements OddsPortal scraping, API collection, parsing, or odds normalization behavior
- **THEN** it resides under `src/oddsportal/`

### Requirement: Architecture documentation
The repository SHALL include top-level architecture documentation that identifies the project purpose, module layout, provider boundaries, data flow, external integrations, and development workflow.

#### Scenario: Reading architecture guidance
- **WHEN** a contributor opens `ARCHITECTURE.md`
- **THEN** they can identify where Polymarket code belongs and where OddsPortal code belongs before adding provider code

### Requirement: Agent guidance
The repository SHALL include top-level coding-agent guidance that explicitly depends on the architecture document as the canonical source for project structure and module ownership.

#### Scenario: Agent starts work
- **WHEN** a coding agent starts a repository task
- **THEN** `AGENTS.md` instructs the agent to read `ARCHITECTURE.md` before changing source layout, provider boundaries, or agent guidance
