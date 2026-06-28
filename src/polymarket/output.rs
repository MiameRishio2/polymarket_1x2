use std::io::Write;

use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::polymarket::models::{MatchResult, QuoteRecord, TokenMeta};

#[derive(Debug, Serialize)]
pub struct PolymarketOddsObservation {
    pub provider: &'static str,
    #[serde(rename = "type")]
    pub record_type: &'static str,
    pub received_at: String,
    pub source_updated_at: Option<String>,
    pub event_slug: String,
    pub home_team: String,
    pub away_team: String,
    pub result: MatchResult,
    pub market_slug: String,
    pub asset_id: String,
    pub bid_price: Option<String>,
    pub bid_size: Option<String>,
    pub ask_price: Option<String>,
    pub ask_size: Option<String>,
    pub source: String,
}

impl PolymarketOddsObservation {
    pub fn from_quote(
        quote: &QuoteRecord,
        token: &TokenMeta,
        home_team: &str,
        away_team: &str,
    ) -> Option<Self> {
        let result = token.result?;
        (token.outcome.eq_ignore_ascii_case("yes") && token.asset_id == quote.asset_id).then(|| {
            Self {
                provider: "polymarket",
                record_type: "polymarket_odds",
                received_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                source_updated_at: None,
                event_slug: quote.event_slug.clone(),
                home_team: home_team.to_string(),
                away_team: away_team.to_string(),
                result,
                market_slug: quote.market_slug.clone(),
                asset_id: quote.asset_id.clone(),
                bid_price: quote.bid_price.clone(),
                bid_size: quote.bid_size.clone(),
                ask_price: quote.ask_price.clone(),
                ask_size: quote.ask_size.clone(),
                source: quote.source.clone(),
            }
        })
    }
}

pub fn write_observation<T: Serialize>(observation: &T) -> Result<()> {
    let line = serde_json::to_string(observation)?;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polymarket::models::{MatchResult, QuoteRecord, TokenMeta};

    #[test]
    fn serializes_classified_yes_quotes_for_each_match_result() {
        for (result, expected) in [
            (MatchResult::Home, "home"),
            (MatchResult::Draw, "draw"),
            (MatchResult::Away, "away"),
        ] {
            let token = token_fixture(result);
            let observation = PolymarketOddsObservation::from_quote(
                &quote_fixture(),
                &token,
                "South Africa",
                "Canada",
            )
            .unwrap();
            let json = serde_json::to_value(observation).unwrap();

            assert_eq!(json["provider"], "polymarket");
            assert_eq!(json["type"], "polymarket_odds");
            assert_eq!(json["result"], expected);
            assert_eq!(json["home_team"], "South Africa");
            assert_eq!(json["away_team"], "Canada");
            assert_eq!(json["bid_price"], "0.16");
            assert_eq!(json["ask_price"], "0.17");
        }
    }

    #[test]
    fn ignores_no_token_or_unclassified_market() {
        let mut token = token_fixture(MatchResult::Home);
        token.outcome = "No".into();
        assert!(PolymarketOddsObservation::from_quote(
            &quote_fixture(),
            &token,
            "South Africa",
            "Canada"
        )
        .is_none());

        token.outcome = "Yes".into();
        token.result = None;
        assert!(PolymarketOddsObservation::from_quote(
            &quote_fixture(),
            &token,
            "South Africa",
            "Canada"
        )
        .is_none());
    }

    fn token_fixture(result: MatchResult) -> TokenMeta {
        TokenMeta {
            market_slug: "rsa".into(),
            question: "Will South Africa win?".into(),
            outcome: "Yes".into(),
            asset_id: "11".into(),
            result: Some(result),
        }
    }

    fn quote_fixture() -> QuoteRecord {
        QuoteRecord {
            ts: "2026-06-28T12:00:00Z".into(),
            event_slug: "fifwc-rsa-can-2026-06-28".into(),
            market_slug: "rsa".into(),
            question: "Will South Africa win?".into(),
            outcome: "Yes".into(),
            asset_id: "11".into(),
            bid_price: Some("0.16".into()),
            bid_size: Some("100".into()),
            ask_price: Some("0.17".into()),
            ask_size: Some("80".into()),
            source: "book".into(),
        }
    }
}
