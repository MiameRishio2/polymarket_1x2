## MODIFIED Requirements

### Requirement: Configurable OddsPortal collection target
The system SHALL load the OddsPortal enabled flag, tournament URL, JSONL path, and positive polling
interval from the `oddsportal` section of `config.yaml`, SHALL receive the shared configured
home-team and away-team pair from root configuration, and SHALL use the root proxy setting for
HTTP requests.

#### Scenario: South Africa Canada target is configured
- **WHEN** the shared configured home team is South Africa and away team is Canada
- **THEN** each discovery pass searches the configured tournament state for South Africa - Canada

#### Scenario: Polling interval is invalid
- **WHEN** `oddsportal.poll_interval_seconds` is zero
- **THEN** configuration validation fails before an OddsPortal task is spawned

### Requirement: Repeated OddsPortal collection
The system SHALL run non-overlapping OddsPortal polling cycles at the configured interval while
its provider task remains enabled. Each cycle SHALL request the discovered 1X2 odds and score
resources concurrently, process their results independently, append every successful normalized
odds pass to the provider-local JSONL log, and write successful observations to stdout.

#### Scenario: One-second collection succeeds
- **WHEN** `oddsportal.poll_interval_seconds` is `1` and both requests succeed
- **THEN** the task emits one odds JSON line and one score JSON line without starting another
  cycle before the next one-second tick

#### Scenario: Odds request fails and score succeeds
- **WHEN** the odds resource fails but the score resource returns a valid observation
- **THEN** the task reports the odds error to stderr, emits the score JSON line, and retries both
  resources on a later tick

#### Scenario: Score request fails and odds succeeds
- **WHEN** the score resource fails but the odds resource returns normalized 1X2 records
- **THEN** the task reports the score error to stderr, logs and emits the odds data, and retries
  both resources on a later tick

### Requirement: Visible OddsPortal lifecycle
The system SHALL emit `[oddsportal]`-prefixed diagnostics to stderr for discovery, polling,
retries, and failures. It SHALL emit machine-readable OddsPortal odds and score observations as
complete JSON lines to stdout.

#### Scenario: Polling cycle succeeds
- **WHEN** an OddsPortal odds or score observation is parsed
- **THEN** stdout receives the corresponding provider-labelled JSON record without a textual
  prefix

#### Scenario: Polling cycle fails
- **WHEN** an OddsPortal request, decoding step, or normalization step fails
- **THEN** stderr receives a prefixed contextual diagnostic without diagnostic text on stdout

## ADDED Requirements

### Requirement: OddsPortal score request discovery
The system SHALL derive the OddsPortal score request from the H2H page's embedded
`updateScoreRequest.url` metadata independently of `requestPreMatch.url`.

#### Scenario: H2H page exposes both request URLs
- **WHEN** embedded H2H metadata contains an odds request URL and a score request URL
- **THEN** discovery returns both absolute URLs associated with the same encoded event ID

#### Scenario: Score request metadata is absent
- **WHEN** the H2H page has no `updateScoreRequest.url`
- **THEN** score discovery reports a contextual absence without preventing odds request discovery

### Requirement: Structured OddsPortal odds output
The system SHALL group each successful normalized 1X2 pass by bookmaker and write one
`oddsportal_odds` JSON object containing all available bookmaker `1`, `X`, and `2` values to
stdout.

#### Scenario: Multiple bookmakers are normalized
- **WHEN** a pass contains 1X2 prices from multiple bookmakers
- **THEN** one JSON record contains the event identity, configured teams, local receipt time, and
  every bookmaker's available outcome values

### Requirement: OddsPortal score output
The system SHALL decode available OddsPortal score responses and write one `oddsportal_score` JSON
object per polling cycle with event identity, configured teams, availability, score state, source
time when present, and local receipt time.

#### Scenario: Live score is available
- **WHEN** the score resource returns a valid current match score
- **THEN** stdout receives a score JSON record with `available: true` and the parsed score fields

#### Scenario: Score is not yet available
- **WHEN** the score resource returns the expected pre-match not-found response
- **THEN** stdout receives a score JSON record with `available: false` and odds processing
  continues
