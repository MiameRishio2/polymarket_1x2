use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetMatch {
    pub home_team: String,
    pub away_team: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredMatch {
    pub event_name: String,
    pub h2h_url: String,
    pub encoded_event_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestMetadata {
    pub pre_match_url: String,
    pub fallback_pre_match_url: Option<String>,
    pub score_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OddsPortalRecord {
    pub ts: String,
    pub provider: String,
    pub event_id: String,
    pub event_name: String,
    pub bookmaker_id: String,
    pub bookmaker_name: String,
    pub outcome: String,
    pub decimal_odds: String,
    pub source_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oddsportal_record_serializes_provider_tag() {
        let record = OddsPortalRecord {
            ts: "2026-06-26T12:00:00Z".to_string(),
            provider: "oddsportal".to_string(),
            event_id: "bsJSJ30L".to_string(),
            event_name: "Norway - France".to_string(),
            bookmaker_id: "16".to_string(),
            bookmaker_name: "bet365".to_string(),
            outcome: "X".to_string(),
            decimal_odds: "3.70".to_string(),
            source_url: "https://www.oddsportal.com/match-event/test.dat".to_string(),
        };

        let json = serde_json::to_value(&record).unwrap();

        assert_eq!(json["provider"], "oddsportal");
        assert_eq!(json["outcome"], "X");
        assert_eq!(json["ts"], "2026-06-26T12:00:00Z");
        chrono::DateTime::parse_from_rfc3339(json["ts"].as_str().unwrap()).unwrap();
    }
}
