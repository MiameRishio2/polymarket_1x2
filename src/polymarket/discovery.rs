use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::polymarket::config::Config;
use crate::polymarket::models::{DiscoveredEvent, MatchResult, TokenMeta};
use crate::polymarket::LOG_PREFIX;

pub async fn discover_event(config: &Config) -> Result<DiscoveredEvent> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&config.proxy_url)?)
        .build()?;
    let mut offset = 0_u32;
    let mut matches = Vec::new();

    loop {
        let offset_value = offset.to_string();
        let body = client
            .get(format!(
                "{}/events",
                config.gamma_api_url.trim_end_matches('/')
            ))
            .query(&[
                ("active", "true"),
                ("closed", "false"),
                ("limit", "100"),
                ("offset", offset_value.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let page_count = serde_json::from_str::<Vec<serde_json::Value>>(&body)
            .context("failed to parse Gamma event page")?
            .len();
        matches.extend(parse_event_page(
            &body,
            &config.home_team,
            &config.away_team,
        )?);
        if page_count < 100 {
            break;
        }
        offset += 100;
    }

    let event = select_unique_event(matches, &config.home_team, &config.away_team)?;
    println!(
        "{LOG_PREFIX} discovered event {} with {} tokens",
        event.slug,
        event.tokens.len()
    );
    Ok(event)
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
                result: None,
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

pub fn parse_event_page(
    body: &str,
    home_team: &str,
    away_team: &str,
) -> Result<Vec<DiscoveredEvent>> {
    let events: Vec<GammaEvent> =
        serde_json::from_str(body).context("failed to parse Gamma event page")?;
    events
        .into_iter()
        .filter_map(|event| classify_event(event, home_team, away_team).transpose())
        .collect()
}

fn classify_event(
    event: GammaEvent,
    home_team: &str,
    away_team: &str,
) -> Result<Option<DiscoveredEvent>> {
    if !event.active || event.closed {
        return Ok(None);
    }

    let normalized_title = normalize_words(&event.title);
    let normalized_home = normalize_words(home_team);
    let normalized_away = normalize_words(away_team);
    if !normalized_title.contains(&normalized_home) || !normalized_title.contains(&normalized_away)
    {
        return Ok(None);
    }

    let market_results: Vec<_> = event
        .markets
        .iter()
        .map(|market| classify_question(&market.question, &normalized_home, &normalized_away))
        .collect();
    if [MatchResult::Home, MatchResult::Draw, MatchResult::Away]
        .into_iter()
        .any(|result| {
            market_results
                .iter()
                .filter(|candidate| **candidate == Some(result))
                .count()
                != 1
        })
    {
        return Ok(None);
    }

    let mut tokens = Vec::new();
    for (market, result) in event.markets.into_iter().zip(market_results) {
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

        for (outcome, asset_id) in outcomes.into_iter().zip(asset_ids) {
            let token_result = result.filter(|_| outcome.eq_ignore_ascii_case("yes"));
            tokens.push(TokenMeta {
                market_slug: market.slug.clone(),
                question: market.question.clone(),
                outcome,
                asset_id,
                result: token_result,
            });
        }
    }

    Ok(Some(DiscoveredEvent {
        slug: event.slug,
        title: event.title,
        tokens,
    }))
}

fn classify_question(
    question: &str,
    normalized_home: &str,
    normalized_away: &str,
) -> Option<MatchResult> {
    let question = normalize_words(question);
    if question.contains("draw") {
        Some(MatchResult::Draw)
    } else if question.contains("win") && question.contains(normalized_home) {
        Some(MatchResult::Home)
    } else if question.contains("win") && question.contains(normalized_away) {
        Some(MatchResult::Away)
    } else {
        None
    }
}

fn normalize_words(value: &str) -> String {
    value
        .to_lowercase()
        .replace("vs.", "vs")
        .replace(['-', '–', '—'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn select_unique_event(
    matches: Vec<DiscoveredEvent>,
    home_team: &str,
    away_team: &str,
) -> Result<DiscoveredEvent> {
    match matches.as_slice() {
        [event] => Ok(event.clone()),
        [] => bail!("no active Polymarket football 1X2 event found for {home_team} vs {away_team}"),
        _ => bail!(
            "ambiguous Polymarket events for {home_team} vs {away_team}: {}",
            matches
                .iter()
                .map(|event| event.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
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
    active: bool,
    #[serde(default)]
    closed: bool,
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
    use crate::polymarket::models::{MatchResult, TokenMeta};

    fn event_object_fixture(slug: &str, title: &str) -> String {
        format!(
            r#"{{
                "slug":"{slug}",
                "title":"{title}",
                "active":true,
                "closed":false,
                "markets":[
                    {{"slug":"home","question":"Will South Africa win on 2026-06-28?",
                      "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"11\",\"12\"]"}},
                    {{"slug":"draw","question":"Will South Africa vs. Canada end in a draw?",
                      "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"21\",\"22\"]"}},
                    {{"slug":"away","question":"Will Canada win on 2026-06-28?",
                      "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"31\",\"32\"]"}}
                ]
            }}"#
        )
    }

    fn event_fixture(slug: &str, title: &str, complete: bool) -> String {
        let mut event = event_object_fixture(slug, title);
        if !complete {
            event = event.replace(
                r#",
                    {"slug":"away","question":"Will Canada win on 2026-06-28?",
                      "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"31\",\"32\"]"}"#,
                "",
            );
        }
        format!("[{event}]")
    }

    #[test]
    fn finds_reversed_team_title_on_later_page_and_classifies_yes_tokens() {
        let page = r#"[{
          "slug":"fifwc-rsa-can-2026-06-28",
          "title":"Canada vs. South Africa",
          "active":true,
          "closed":false,
          "markets":[
            {"slug":"rsa","question":"Will South Africa win on 2026-06-28?",
             "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"11\",\"12\"]"},
            {"slug":"draw","question":"Will South Africa vs. Canada end in a draw?",
             "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"21\",\"22\"]"},
            {"slug":"can","question":"Will Canada win on 2026-06-28?",
             "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"31\",\"32\"]"}
          ]
        }]"#;

        let matches = parse_event_page(page, "South Africa", "Canada").unwrap();
        assert_eq!(matches.len(), 1);
        let yes_results: Vec<_> = matches[0]
            .tokens
            .iter()
            .filter(|token| token.outcome == "Yes")
            .filter_map(|token| token.result)
            .collect();
        assert_eq!(
            yes_results,
            vec![MatchResult::Home, MatchResult::Draw, MatchResult::Away]
        );
    }

    #[test]
    fn rejects_incomplete_and_ambiguous_1x2_candidates() {
        let incomplete = event_fixture("a", "South Africa vs Canada", false);
        assert!(parse_event_page(&incomplete, "South Africa", "Canada")
            .unwrap()
            .is_empty());

        let ambiguous = format!(
            "[{},{}]",
            event_object_fixture("a", "South Africa vs Canada"),
            event_object_fixture("b", "Canada vs. South Africa")
        );
        let error = select_unique_event(
            parse_event_page(&ambiguous, "South Africa", "Canada").unwrap(),
            "South Africa",
            "Canada",
        )
        .unwrap_err();
        assert!(error.to_string().contains("ambiguous"));
        assert!(error.to_string().contains('a'));
        assert!(error.to_string().contains('b'));
    }

    #[test]
    fn reports_target_pair_when_no_active_event_matches() {
        let inactive = event_fixture("inactive", "South Africa vs Canada", true)
            .replace(r#""active":true"#, r#""active":false"#);
        let matches = parse_event_page(&inactive, "South Africa", "Canada").unwrap();
        let error = select_unique_event(matches, "South Africa", "Canada").unwrap_err();

        assert!(error.to_string().contains("South Africa vs Canada"));
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
                        result: None,
                    },
                    TokenMeta {
                        market_slug: "fifwc-ecu-ger-2026-06-25-ger".to_string(),
                        question: "Will Germany win on 2026-06-25?".to_string(),
                        outcome: "No".to_string(),
                        asset_id: "128".to_string(),
                        result: None,
                    },
                ],
            }
        );
    }
}
