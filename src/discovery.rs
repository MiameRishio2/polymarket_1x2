use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use url::Url;

use crate::config::Config;
use crate::models::{DiscoveredEvent, TokenMeta};

pub fn extract_slug(input: &str) -> Result<String> {
    let url = Url::parse(input).context("invalid Polymarket URL")?;
    url.path_segments()
        .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
        .map(str::to_string)
        .filter(|slug| !slug.is_empty())
        .ok_or_else(|| anyhow!("URL does not contain an event slug"))
}

pub async fn discover_event(config: &Config) -> Result<DiscoveredEvent> {
    let slug = extract_slug(&config.polymarket_url)?;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&config.proxy_url)?)
        .build()?;
    let body = client
        .get(format!("{}{}", config.gamma_event_base, slug))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    parse_event_response(&body)
}

pub fn parse_event_response(body: &str) -> Result<DiscoveredEvent> {
    let event: GammaEvent = serde_json::from_str(body).context("failed to parse Gamma event")?;
    let mut tokens = Vec::new();

    for market in event.markets {
        let outcomes = parse_json_string_array(&market.outcomes)
            .with_context(|| format!("failed to parse outcomes for {}", market.slug))?;
        let asset_ids = parse_json_string_array(&market.clob_token_ids)
            .with_context(|| format!("failed to parse clobTokenIds for {}", market.slug))?;

        if outcomes.len() != asset_ids.len() {
            bail!(
                "market {} has {} outcomes but {} token ids",
                market.slug,
                outcomes.len(),
                asset_ids.len()
            );
        }

        for (outcome, asset_id) in outcomes.into_iter().zip(asset_ids.into_iter()) {
            tokens.push(TokenMeta {
                market_slug: market.slug.clone(),
                question: market.question.clone(),
                outcome,
                asset_id,
            });
        }
    }

    if tokens.is_empty() {
        bail!("event did not contain any CLOB token ids");
    }

    Ok(DiscoveredEvent {
        slug: event.slug,
        title: event.title,
        tokens,
    })
}

fn parse_json_string_array(value: &serde_json::Value) -> Result<Vec<String>> {
    match value {
        serde_json::Value::String(encoded) => Ok(serde_json::from_str(encoded)?),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("expected string array item"))
            })
            .collect(),
        _ => bail!("expected JSON string array or array"),
    }
}

#[derive(Debug, Deserialize)]
struct GammaEvent {
    slug: String,
    title: String,
    #[serde(default)]
    markets: Vec<GammaMarket>,
}

#[derive(Debug, Deserialize)]
struct GammaMarket {
    slug: String,
    question: String,
    outcomes: serde_json::Value,
    #[serde(rename = "clobTokenIds")]
    clob_token_ids: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TokenMeta;

    #[test]
    fn extracts_slug_from_localized_polymarket_url() {
        let slug = extract_slug("https://polymarket.com/sports/world-cup/fifwc-nor-fra-2026-06-26")
            .unwrap();

        assert_eq!(slug, "fifwc-nor-fra-2026-06-26");
    }

    #[test]
    fn parses_event_markets_and_json_encoded_token_ids() {
        let json = r#"{
            "slug": "fifwc-ecu-ger-2026-06-25",
            "title": "Ecuador vs. Germany",
            "markets": [{
                "slug": "fifwc-ecu-ger-2026-06-25-ger",
                "question": "Will Germany win on 2026-06-25?",
                "outcomes": "[\"Yes\", \"No\"]",
                "clobTokenIds": "[\"101\", \"128\"]"
            }]
        }"#;

        let event = parse_event_response(json).unwrap();

        assert_eq!(
            event,
            DiscoveredEvent {
                slug: "fifwc-ecu-ger-2026-06-25".to_string(),
                title: "Ecuador vs. Germany".to_string(),
                tokens: vec![
                    TokenMeta {
                        market_slug: "fifwc-ecu-ger-2026-06-25-ger".to_string(),
                        question: "Will Germany win on 2026-06-25?".to_string(),
                        outcome: "Yes".to_string(),
                        asset_id: "101".to_string(),
                    },
                    TokenMeta {
                        market_slug: "fifwc-ecu-ger-2026-06-25-ger".to_string(),
                        question: "Will Germany win on 2026-06-25?".to_string(),
                        outcome: "No".to_string(),
                        asset_id: "128".to_string(),
                    },
                ],
            }
        );
    }
}
