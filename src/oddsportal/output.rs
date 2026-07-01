use std::collections::BTreeMap;
use std::io::Write;

use anyhow::{bail, ensure, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::oddsportal::models::OddsPortalRecord;

#[derive(Debug, Serialize)]
pub struct BookmakerOdds {
    pub bookmaker_id: String,
    pub bookmaker_name: String,
    pub outcomes: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct OddsPortalOddsObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_id: String,
    pub event_name: String,
    pub home_team: String,
    pub away_team: String,
    pub bookmakers: Vec<BookmakerOdds>,
}

#[derive(Debug, Serialize)]
pub struct OddsPortalScoreObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_id: String,
    pub event_name: String,
    pub home_team: String,
    pub away_team: String,
    pub available: bool,
    pub score: Option<String>,
    pub status: Option<String>,
    pub period: Option<String>,
    pub elapsed: Option<String>,
}

impl OddsPortalOddsObservation {
    pub fn from_records(
        records: &[OddsPortalRecord],
        home_team: &str,
        away_team: &str,
    ) -> Result<Self> {
        let Some(first) = records.first() else {
            bail!("cannot create OddsPortal odds observation from empty record batch");
        };
        ensure!(
            records.iter().all(|record| record.ts == first.ts),
            "cannot create OddsPortal odds observation from records with inconsistent receipt timestamps"
        );
        let mut grouped = BTreeMap::<(String, String), BTreeMap<String, String>>::new();
        for record in records {
            grouped
                .entry((record.bookmaker_id.clone(), record.bookmaker_name.clone()))
                .or_default()
                .insert(record.outcome.clone(), record.decimal_odds.clone());
        }
        let bookmakers = grouped
            .into_iter()
            .map(|((bookmaker_id, bookmaker_name), outcomes)| BookmakerOdds {
                bookmaker_id,
                bookmaker_name,
                outcomes,
            })
            .collect();

        Ok(Self {
            provider: "oddsportal",
            record_type: "oddsportal_odds",
            received_at: first.ts.clone(),
            source_updated_at: None,
            event_id: first.event_id.clone(),
            event_name: first.event_name.clone(),
            home_team: home_team.to_string(),
            away_team: away_team.to_string(),
            bookmakers,
        })
    }
}

#[allow(dead_code)]
pub fn write_observation<T: Serialize>(observation: &T) -> Result<()> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    write_observation_to(&mut lock, observation)
}

pub(crate) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn write_observation_to<W: Write, T: Serialize>(writer: &mut W, observation: &T) -> Result<()> {
    let line = serde_json::to_string(observation)?;
    writeln!(writer, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_all_bookmakers_into_one_odds_record() {
        let records = vec![
            odds_record("16", "bet365", "1", "5.50"),
            odds_record("16", "bet365", "X", "3.80"),
            odds_record("16", "bet365", "2", "1.62"),
            odds_record("18", "Pinnacle", "1", "5.60"),
        ];

        let output =
            OddsPortalOddsObservation::from_records(&records, "South Africa", "Canada").unwrap();

        assert_eq!(output.bookmakers.len(), 2);
        assert_eq!(output.bookmakers[0].bookmaker_id, "16");
        assert_eq!(output.bookmakers[0].outcomes["X"], "3.80");
        assert_eq!(output.bookmakers[1].bookmaker_id, "18");
        assert_eq!(output.received_at, records[0].ts);
    }

    #[test]
    fn rejects_odds_batches_with_inconsistent_receipt_times() {
        let mut records = vec![
            odds_record("16", "bet365", "1", "5.50"),
            odds_record("16", "bet365", "X", "3.80"),
        ];
        records[1].ts = "2026-06-28T12:00:01Z".to_string();

        let error = OddsPortalOddsObservation::from_records(&records, "South Africa", "Canada")
            .unwrap_err();

        assert!(error.to_string().contains("receipt timestamps"));
    }

    #[test]
    fn rejects_empty_odds_record_batch() {
        let error =
            OddsPortalOddsObservation::from_records(&[], "South Africa", "Canada").unwrap_err();

        assert!(error.to_string().contains("empty"));
    }

    #[test]
    fn writes_one_complete_json_line() {
        let observation = serde_json::json!({
            "provider": "oddsportal",
            "type": "oddsportal_odds"
        });
        let mut sink = Vec::new();

        write_observation_to(&mut sink, &observation).unwrap();

        assert_eq!(sink.iter().filter(|byte| **byte == b'\n').count(), 1);
        assert_eq!(sink.last(), Some(&b'\n'));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&sink).unwrap(),
            observation
        );
    }

    fn odds_record(
        bookmaker_id: &str,
        bookmaker_name: &str,
        outcome: &str,
        decimal_odds: &str,
    ) -> OddsPortalRecord {
        OddsPortalRecord {
            ts: "2026-06-28T12:00:00Z".to_string(),
            provider: "oddsportal".to_string(),
            event_id: "EZmXxG15".to_string(),
            event_name: "South Africa - Canada".to_string(),
            bookmaker_id: bookmaker_id.to_string(),
            bookmaker_name: bookmaker_name.to_string(),
            outcome: outcome.to_string(),
            decimal_odds: decimal_odds.to_string(),
            source_url: "https://www.oddsportal.com/feed/live-event/test.dat".to_string(),
        }
    }
}
