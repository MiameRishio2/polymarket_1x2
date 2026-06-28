use anyhow::Result;
use serde_json::Value;

use crate::oddsportal::models::DiscoveredMatch;
use crate::oddsportal::output::{now_rfc3339, OddsPortalScoreObservation};

pub fn parse_score_payload(
    decoded: &Value,
    event: &DiscoveredMatch,
    home_team: &str,
    away_team: &str,
) -> Result<OddsPortalScoreObservation> {
    let data = decoded.get("d").unwrap_or(decoded);
    Ok(OddsPortalScoreObservation {
        provider: "oddsportal",
        record_type: "oddsportal_score",
        received_at: now_rfc3339(),
        source_updated_at: field(data, &["lastUpdate", "updatedAt", "ts"]),
        event_id: event.encoded_event_id.clone(),
        event_name: event.event_name.clone(),
        home_team: home_team.to_string(),
        away_team: away_team.to_string(),
        available: true,
        score: field(data, &["score", "result"]),
        status: field(data, &["status", "state"]),
        period: field(data, &["period", "stage"]),
        elapsed: field(data, &["elapsed", "time"]),
    })
}

pub fn unavailable_score(
    event: &DiscoveredMatch,
    home_team: &str,
    away_team: &str,
) -> OddsPortalScoreObservation {
    OddsPortalScoreObservation {
        provider: "oddsportal",
        record_type: "oddsportal_score",
        received_at: now_rfc3339(),
        source_updated_at: None,
        event_id: event.encoded_event_id.clone(),
        event_name: event.event_name.clone(),
        home_team: home_team.to_string(),
        away_team: away_team.to_string(),
        available: false,
        score: None,
        status: None,
        period: None,
        elapsed: None,
    }
}

fn field(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| match value.get(name) {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_available_score_fields() {
        let decoded = serde_json::json!({
            "d": {
                "score": "1-0",
                "status": "live",
                "period": "1H",
                "elapsed": 32,
                "lastUpdate": "2026-06-28T12:00:01Z"
            }
        });

        let record =
            parse_score_payload(&decoded, &match_fixture(), "South Africa", "Canada").unwrap();

        assert!(record.available);
        assert_eq!(record.score.as_deref(), Some("1-0"));
        assert_eq!(record.status.as_deref(), Some("live"));
        assert_eq!(record.period.as_deref(), Some("1H"));
        assert_eq!(record.elapsed.as_deref(), Some("32"));
        assert_eq!(
            record.source_updated_at.as_deref(),
            Some("2026-06-28T12:00:01Z")
        );
    }

    #[test]
    fn models_pre_match_score_as_unavailable() {
        let record = unavailable_score(&match_fixture(), "South Africa", "Canada");
        let json = serde_json::to_value(&record).unwrap();

        assert_eq!(json["provider"], "oddsportal");
        assert_eq!(json["type"], "oddsportal_score");
        assert_eq!(json["available"], false);
        assert!(json["score"].is_null());
        assert!(json["status"].is_null());
        assert!(json["period"].is_null());
        assert!(json["elapsed"].is_null());
    }

    fn match_fixture() -> DiscoveredMatch {
        DiscoveredMatch {
            event_name: "South Africa - Canada".to_string(),
            h2h_url: "https://www.oddsportal.com/football/h2h/test/#EZmXxG15".to_string(),
            encoded_event_id: "EZmXxG15".to_string(),
        }
    }
}
