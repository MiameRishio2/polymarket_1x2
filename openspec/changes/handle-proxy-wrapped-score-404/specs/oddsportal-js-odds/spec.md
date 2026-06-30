## MODIFIED Requirements

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

#### Scenario: Proxy wraps pre-match not-found response
- **WHEN** the configured proxy returns HTTP 200 with a plain-text body identifying the score
  resource and `Status: 404`
- **THEN** stdout receives a score JSON record with `available: false` without reporting a `.dat`
  decoding error, and odds processing continues
