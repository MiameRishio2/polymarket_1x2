use anyhow::{anyhow, bail, Result};
use serde_json::Value;

use crate::oddsportal::models::DiscoveredMatch;
use crate::oddsportal::output::{now_rfc3339, OddsPortalScoreObservation};

pub fn parse_score_payload(
    decoded: &Value,
    event: &DiscoveredMatch,
    home_team: &str,
    away_team: &str,
) -> Result<OddsPortalScoreObservation> {
    let data = match decoded.get("d") {
        Some(value) => value
            .as_object()
            .ok_or_else(|| anyhow!("OddsPortal score payload field d must be an object"))?,
        None => decoded
            .as_object()
            .ok_or_else(|| anyhow!("OddsPortal score payload must be an object"))?,
    };

    let score = score_field(data, &["score", "result"])?;
    let status = score_field(data, &["status", "state"])?;
    let period = score_field(data, &["period", "stage"])?;
    let elapsed = score_field(data, &["elapsed", "time"])?;
    if score.is_none() && status.is_none() && period.is_none() && elapsed.is_none() {
        let keys = data.keys().cloned().collect::<Vec<_>>().join(", ");
        bail!("OddsPortal score payload contains no recognized score-state field; keys: [{keys}]");
    }

    Ok(OddsPortalScoreObservation {
        provider: "oddsportal",
        record_type: "oddsportal_score",
        received_at: now_rfc3339(),
        source_updated_at: score_field(data, &["lastUpdate", "updatedAt", "ts"])?,
        event_id: event.encoded_event_id.clone(),
        event_name: event.event_name.clone(),
        home_team: home_team.to_string(),
        away_team: away_team.to_string(),
        available: true,
        score,
        status,
        period,
        elapsed,
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

fn score_field(value: &serde_json::Map<String, Value>, names: &[&str]) -> Result<Option<String>> {
    for name in names {
        match value.get(*name) {
            Some(Value::String(text)) => return Ok(Some(text.clone())),
            Some(Value::Number(number)) => return Ok(Some(number.to_string())),
            Some(Value::Null) | None => {}
            Some(other) => {
                bail!(
                    "OddsPortal score payload field {name} must be a string, number, or null; got {other}"
                )
            }
        }
    }
    Ok(None)
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
    fn parses_scheduled_score_state_without_a_score() {
        let decoded = serde_json::json!({
            "d": {
                "status": "scheduled",
                "lastUpdate": "2026-06-28T11:00:00Z"
            }
        });

        let record =
            parse_score_payload(&decoded, &match_fixture(), "South Africa", "Canada").unwrap();

        assert_eq!(record.status.as_deref(), Some("scheduled"));
        assert!(record.score.is_none());
    }

    #[test]
    fn rejects_empty_unknown_and_timestamp_only_score_payloads() {
        for decoded in [
            serde_json::json!({}),
            serde_json::json!({"d": {}}),
            serde_json::json!({"d": {"lastUpdate": "2026-06-28T12:00:01Z"}}),
            serde_json::json!({"d": {"unrecognized": "value"}}),
        ] {
            let error = parse_score_payload(&decoded, &match_fixture(), "South Africa", "Canada")
                .unwrap_err();
            assert!(error.to_string().contains("score payload"), "{error:#}");
        }
    }

    #[test]
    fn rejects_non_object_and_wrong_type_score_payloads() {
        for decoded in [
            serde_json::json!([]),
            serde_json::json!({"d": "not-an-object"}),
            serde_json::json!({"d": {"score": true}}),
            serde_json::json!({"d": {"status": ["live"]}}),
            serde_json::json!({"d": {"period": {"name": "1H"}}}),
            serde_json::json!({"d": {"elapsed": false}}),
        ] {
            let error = parse_score_payload(&decoded, &match_fixture(), "South Africa", "Canada")
                .unwrap_err();
            assert!(error.to_string().contains("score payload"), "{error:#}");
        }
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
