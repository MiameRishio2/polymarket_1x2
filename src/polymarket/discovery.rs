use std::future::Future;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::polymarket::config::Config;
use crate::polymarket::models::{DiscoveredEvent, MatchResult, TokenMeta};
use crate::polymarket::LOG_PREFIX;

pub async fn discover_event(config: &Config) -> Result<DiscoveredEvent> {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(&config.proxy_url)?)
        .build()?;
    let events_url = format!("{}/events", config.gamma_api_url.trim_end_matches('/'));
    let event = discover_event_pages(&config.home_team, &config.away_team, move |offset| {
        let client = client.clone();
        let events_url = events_url.clone();
        async move {
            Ok(client
                .get(events_url)
                .query(&event_query(offset))
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?)
        }
    })
    .await?;

    crate::diagnostics::write(format_args!(
        "{LOG_PREFIX} discovered event {} with {} tokens",
        event.slug,
        event.tokens.len()
    ));
    Ok(event)
}

async fn discover_event_pages<F, Fut>(
    home_team: &str,
    away_team: &str,
    mut fetch_page: F,
) -> Result<DiscoveredEvent>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let mut offset = 0_u32;
    let mut matches = Vec::new();

    loop {
        let body = fetch_page(offset).await?;
        let page_count = serde_json::from_str::<Vec<serde_json::Value>>(&body)
            .context("failed to parse Gamma event page")?
            .len();
        matches.extend(parse_event_page(&body, home_team, away_team)?);
        if page_count < 100 {
            break;
        }
        offset += 100;
    }

    select_unique_event(matches, home_team, away_team)
}

fn event_query(offset: u32) -> [(&'static str, String); 5] {
    [
        ("active", "true".to_string()),
        ("closed", "false".to_string()),
        ("tag_slug", "soccer".to_string()),
        ("limit", "100".to_string()),
        ("offset", offset.to_string()),
    ]
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
    if !matches_team_pair(&normalized_title, &normalized_home, &normalized_away) {
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
        if result.is_some()
            && outcomes
                .iter()
                .filter(|outcome| outcome.eq_ignore_ascii_case("yes"))
                .count()
                != 1
        {
            return Ok(None);
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
    if let Some(team) = question
        .strip_prefix("will ")
        .and_then(|value| value.split_once(" win"))
        .map(|(team, _)| team)
    {
        return match team {
            team if team == normalized_home => Some(MatchResult::Home),
            team if team == normalized_away => Some(MatchResult::Away),
            _ => None,
        };
    }

    question
        .strip_prefix("will ")
        .and_then(|value| value.split_once(" end in a draw"))
        .map(|(pair, _)| pair)
        .filter(|pair| matches_team_pair(pair, normalized_home, normalized_away))
        .map(|_| MatchResult::Draw)
}

fn matches_team_pair(value: &str, normalized_home: &str, normalized_away: &str) -> bool {
    value.split_once(" vs ").is_some_and(|(left, right)| {
        (left == normalized_home && right == normalized_away)
            || (left == normalized_away && right == normalized_home)
    })
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
    use std::cell::RefCell;
    use std::rc::Rc;

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
    fn classifies_overlapping_team_names_by_exact_team_phrase() {
        let page = r#"[{
          "slug":"guinea-equatorial-guinea",
          "title":"Guinea vs. Equatorial Guinea",
          "active":true,
          "closed":false,
          "markets":[
            {"slug":"guinea","question":"Will Guinea win on 2026-06-28?",
             "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"11\",\"12\"]"},
            {"slug":"draw","question":"Will Guinea vs. Equatorial Guinea end in a draw?",
             "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"21\",\"22\"]"},
            {"slug":"equatorial-guinea","question":"Will Equatorial Guinea win on 2026-06-28?",
             "outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"31\",\"32\"]"}
          ]
        }]"#;

        let matches = parse_event_page(page, "Guinea", "Equatorial Guinea").unwrap();
        let yes_results: Vec<_> = matches[0]
            .tokens
            .iter()
            .filter_map(|token| token.result)
            .collect();

        assert_eq!(
            yes_results,
            vec![MatchResult::Home, MatchResult::Draw, MatchResult::Away]
        );
    }

    #[test]
    fn rejects_result_market_without_a_yes_token() {
        let page = event_fixture("missing-yes", "South Africa vs Canada", true).replace(
            r#""outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"11\",\"12\"]""#,
            r#""outcomes":"[\"No\"]","clobTokenIds":"[\"12\"]""#,
        );

        assert!(parse_event_page(&page, "South Africa", "Canada")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn rejects_result_market_with_duplicate_yes_tokens() {
        let page = event_fixture("duplicate-yes", "South Africa vs Canada", true).replace(
            r#""outcomes":"[\"Yes\",\"No\"]","clobTokenIds":"[\"31\",\"32\"]""#,
            r#""outcomes":"[\"Yes\",\"YES\",\"No\"]","clobTokenIds":"[\"31\",\"33\",\"32\"]""#,
        );

        assert!(parse_event_page(&page, "South Africa", "Canada")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn gamma_event_query_filters_soccer_and_carries_offset() {
        assert_eq!(
            event_query(100),
            [
                ("active", "true".to_string()),
                ("closed", "false".to_string()),
                ("tag_slug", "soccer".to_string()),
                ("limit", "100".to_string()),
                ("offset", "100".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn paginates_full_page_before_selecting_later_match() {
        let first_page = serde_json::to_string(
            &(0..100)
                .map(|index| {
                    serde_json::json!({
                        "slug": format!("unrelated-{index}"),
                        "title": "Japan vs Brazil",
                        "active": true,
                        "closed": false,
                        "markets": []
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let second_page =
            event_fixture("fifwc-rsa-can-2026-06-28", "Canada vs. South Africa", true);
        let offsets = Rc::new(RefCell::new(Vec::new()));
        let recorded_offsets = Rc::clone(&offsets);

        let event = discover_event_pages("South Africa", "Canada", move |offset| {
            recorded_offsets.borrow_mut().push(offset);
            std::future::ready(Ok(if offset == 0 {
                first_page.clone()
            } else {
                second_page.clone()
            }))
        })
        .await
        .unwrap();

        assert_eq!(event.slug, "fifwc-rsa-can-2026-06-28");
        assert_eq!(*offsets.borrow(), vec![0, 100]);
    }

    #[test]
    fn rejects_incomplete_and_ambiguous_1x2_candidates() {
        let incomplete = event_fixture("a", "South Africa vs Canada", false);
        assert!(parse_event_page(&incomplete, "South Africa", "Canada")
            .unwrap()
            .is_empty());

        let ambiguous = format!(
            "[{},{}]",
            event_object_fixture("south-africa-canada-a", "South Africa vs Canada"),
            event_object_fixture("canada-south-africa-b", "Canada vs. South Africa")
        );
        let error = select_unique_event(
            parse_event_page(&ambiguous, "South Africa", "Canada").unwrap(),
            "South Africa",
            "Canada",
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "ambiguous Polymarket events for South Africa vs Canada: \
             south-africa-canada-a, canada-south-africa-b"
        );
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
