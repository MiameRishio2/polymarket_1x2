use anyhow::{anyhow, Result};
use chrono::Utc;
use percent_encoding::percent_decode_str;
use serde_json::Value;

use crate::oddsportal::models::OddsPortalRecord;

const OUTCOMES: [(&str, &str); 3] = [("0", "1"), ("1", "X"), ("2", "2")];

pub fn normalize_1x2_odds(
    decoded: &Value,
    event_name: &str,
    source_url: &str,
) -> Result<Vec<OddsPortalRecord>> {
    let data = decoded
        .get("d")
        .ok_or_else(|| anyhow!("decoded OddsPortal response missing d object"))?;
    let event_id = string_field(data, "encodeventId")
        .or_else(|| string_field(data, "eventId"))
        .unwrap_or_default();
    let provider_names = data.get("providersNames").and_then(Value::as_object);
    let back = data
        .get("oddsdata")
        .and_then(|oddsdata| oddsdata.get("back"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            let keys = data
                .as_object()
                .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            anyhow!("decoded OddsPortal response missing oddsdata.back; keys: {keys}")
        })?;

    let received_at = Utc::now().to_rfc3339();
    let mut records = Vec::new();
    for market in back.values() {
        if market
            .get("bettingTypeId")
            .is_some_and(|value| !value_is_id(value, 1))
            || market
                .get("scopeId")
                .is_some_and(|value| !value_is_id(value, 2))
        {
            continue;
        }
        let Some(odds_by_bookmaker) = market.get("odds").and_then(Value::as_object) else {
            continue;
        };
        let active = market.get("act").and_then(Value::as_object);
        let betslips = market.get("bs").and_then(Value::as_object);

        for (bookmaker_id, prices) in odds_by_bookmaker {
            if matches!(
                active.and_then(|act| act.get(bookmaker_id)),
                Some(Value::Bool(false))
            ) {
                continue;
            }
            let Some(prices) = prices.as_object() else {
                continue;
            };
            for (column, outcome) in OUTCOMES {
                let Some(decimal_odds) = prices.get(column).and_then(value_to_string) else {
                    continue;
                };
                records.push(OddsPortalRecord {
                    ts: received_at.clone(),
                    provider: "oddsportal".to_string(),
                    event_id: event_id.clone(),
                    event_name: event_name.to_string(),
                    bookmaker_id: bookmaker_id.clone(),
                    bookmaker_name: provider_names
                        .and_then(|names| names.get(bookmaker_id))
                        .and_then(value_to_string)
                        .or_else(|| bookmaker_name_from_betslip(betslips, bookmaker_id))
                        .unwrap_or_else(|| bookmaker_id.clone()),
                    outcome: outcome.to_string(),
                    decimal_odds,
                    source_url: source_url.to_string(),
                });
            }
        }
    }

    Ok(records)
}

fn value_is_id(value: &Value, expected: u64) -> bool {
    value.as_u64() == Some(expected)
        || value.as_str().and_then(|value| value.parse::<u64>().ok()) == Some(expected)
}

fn bookmaker_name_from_betslip(
    betslips: Option<&serde_json::Map<String, Value>>,
    bookmaker_id: &str,
) -> Option<String> {
    let urls = betslips?.get(bookmaker_id)?.as_object()?;
    urls.values().filter_map(Value::as_str).find_map(|url| {
        let mut segments = url.split('/').filter(|segment| !segment.is_empty());
        while let Some(segment) = segments.next() {
            if segment == "bookmakers" {
                let slug = segments.next()?;
                let decoded = percent_decode_str(slug).decode_utf8().ok()?;
                let name = decoded.replace('-', " ");
                return (!name.is_empty()).then_some(name);
            }
        }
        None
    })
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(value_to_string)
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_one_x_two_bookmaker_odds() {
        let decoded = serde_json::json!({
            "d": {
                "encodeventId": "bsJSJ30L",
                "oddsdata": {
                    "back": {
                        "0": {
                            "odds": {
                                "16": {"0": "4.20", "1": "3.70", "2": "1.85"}
                            },
                            "act": {"16": true}
                        }
                    }
                },
                "providersNames": {"16": "bet365"}
            }
        });

        let records = normalize_1x2_odds(
            &decoded,
            "Norway - France",
            "https://www.oddsportal.com/match-event/test.dat",
        )
        .unwrap();

        assert_eq!(records.len(), 3);
        assert!(records.iter().any(|record| {
            record.bookmaker_name == "bet365"
                && record.outcome == "X"
                && record.decimal_odds == "3.70"
        }));
    }

    #[test]
    fn skips_inactive_bookmaker_odds() {
        let decoded = serde_json::json!({
            "d": {
                "encodeventId": "bsJSJ30L",
                "oddsdata": {
                    "back": {
                        "0": {
                            "odds": {
                                "16": {"0": "4.20", "1": "3.70", "2": "1.85"}
                            },
                            "act": {"16": false}
                        }
                    }
                },
                "providersNames": {"16": "bet365"}
            }
        });

        let records =
            normalize_1x2_odds(&decoded, "Norway - France", "https://example.test").unwrap();

        assert!(records.is_empty());
    }

    #[test]
    fn assigns_one_receipt_timestamp_to_the_normalized_batch() {
        let decoded = serde_json::json!({
            "d": {
                "encodeventId": "bsJSJ30L",
                "oddsdata": {
                    "back": {
                        "0": {
                            "odds": {
                                "16": {"0": "4.20", "1": "3.70", "2": "1.85"},
                                "18": {"0": "4.25", "1": "3.75", "2": "1.80"}
                            }
                        }
                    }
                }
            }
        });

        let records =
            normalize_1x2_odds(&decoded, "Norway - France", "https://example.test").unwrap();

        assert_eq!(records.len(), 6);
        assert!(records.iter().all(|record| record.ts == records[0].ts));
    }

    #[test]
    fn derives_live_bookmaker_name_from_betslip() {
        let decoded = serde_json::json!({
            "d": {
                "encodeventId": "EZmXxG15",
                "oddsdata": {
                    "back": {
                        "E-1-2-0-0-0": {
                            "bettingTypeId": 1,
                            "scopeId": 2,
                            "odds": {
                                "417": {"0": 2.87, "1": 3.60, "2": 2.25}
                            },
                            "bs": {
                                "417": {
                                    "0": "/bookmakers/1xbet/betslip/l/Football/example/"
                                }
                            },
                            "act": {"417": true}
                        }
                    }
                }
            }
        });

        let records =
            normalize_1x2_odds(&decoded, "Home - Away", "https://example.test/live.dat").unwrap();

        assert_eq!(records.len(), 3);
        assert!(records
            .iter()
            .all(|record| record.bookmaker_name == "1xbet"));
    }

    #[test]
    fn ignores_non_one_x_two_live_markets() {
        let decoded = serde_json::json!({
            "d": {
                "encodeventId": "EZmXxG15",
                "oddsdata": {
                    "back": {
                        "E-1-2-0-0-0": {
                            "bettingTypeId": 1,
                            "scopeId": 2,
                            "odds": {
                                "16": {"0": 2.87, "1": 3.60, "2": 2.25}
                            }
                        },
                        "E-5-2-0-0-0": {
                            "bettingTypeId": 5,
                            "scopeId": 2,
                            "odds": {
                                "16": {"0": 1.80, "1": 2.00, "2": 2.10}
                            }
                        }
                    }
                }
            }
        });

        let records =
            normalize_1x2_odds(&decoded, "Home - Away", "https://example.test/live.dat").unwrap();

        assert_eq!(records.len(), 3);
        assert!(records.iter().all(|record| record.decimal_odds != "1.8"));
    }
}
