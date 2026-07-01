use anyhow::{anyhow, bail, Context, Result};
use html_escape::decode_html_entities;
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use crate::oddsportal::config::DEFAULT_BASE_URL;
use crate::oddsportal::models::{DiscoveredMatch, LiveOddsRequestState, RequestMetadata};

pub fn parse_tournament_match(html: &str, home: &str, away: &str) -> Result<DiscoveredMatch> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("#react-leagues-events")
        .map_err(|error| anyhow!("invalid selector: {error}"))?;
    let expected = format!("{home} - {away}");
    let reverse = format!("{away} - {home}");

    for element in document.select(&selector) {
        let Some(data) = element.value().attr("data") else {
            continue;
        };
        let decoded = decode_attr(data);
        let parsed: TournamentRows =
            serde_json::from_str(&decoded).context("failed to parse OddsPortal tournament rows")?;
        for row in parsed.rows {
            if row.event == expected || row.event == reverse {
                let h2h_url = absolute_url(DEFAULT_BASE_URL, &row.url)?;
                let encoded_event_id = extract_hash(&h2h_url)
                    .ok_or_else(|| anyhow!("OddsPortal match URL missing event hash"))?;
                return Ok(DiscoveredMatch {
                    event_name: row.event,
                    h2h_url,
                    encoded_event_id,
                });
            }
        }
    }

    if let Some(discovered) =
        parse_match_from_embedded_text(&decode_attr(html), &expected, &reverse)?
    {
        return Ok(discovered);
    }

    bail!("OddsPortal match not found for {home} - {away}")
}

pub fn parse_h2h_request_metadata(html: &str) -> Result<RequestMetadata> {
    let decoded = decode_attr(html);
    let score_url = find_request_url(&decoded, "updateScoreRequest")
        .map(|raw| raw.replace("\\/", "/"))
        .map(|raw| absolute_url(DEFAULT_BASE_URL, &raw))
        .transpose()?;

    Ok(RequestMetadata { score_url })
}

pub fn parse_live_odds_request(html: &str) -> Result<LiveOddsRequestState> {
    let decoded = decode_attr(html);
    if !find_json_bool(&decoded, "isLive").unwrap_or(false)
        || !find_json_bool(&decoded, "realLive").unwrap_or(false)
    {
        return Ok(LiveOddsRequestState::Unavailable);
    }
    let Some(raw_url) = find_request_url(&decoded, "requestLive") else {
        return Ok(LiveOddsRequestState::Unavailable);
    };
    let url = absolute_url(DEFAULT_BASE_URL, &raw_url.replace("\\/", "/"))?;
    Ok(LiveOddsRequestState::Available { url })
}

fn absolute_url(base_url: &str, maybe_relative: &str) -> Result<String> {
    let base = Url::parse(base_url).context("invalid OddsPortal base URL")?;
    Ok(base.join(maybe_relative)?.to_string())
}

fn decode_attr(value: &str) -> String {
    decode_html_entities(value)
        .replace("\\/", "/")
        .replace("\\\"", "\"")
}

fn extract_hash(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|url| {
            url.fragment()
                .map(|fragment| fragment.trim_matches('/').to_string())
        })
        .filter(|hash| !hash.is_empty())
}

fn parse_match_from_embedded_text(
    text: &str,
    expected: &str,
    reverse: &str,
) -> Result<Option<DiscoveredMatch>> {
    for event_name in [expected, reverse] {
        let Some(position) = text
            .find(&format!(r#""name":"{event_name}""#))
            .or_else(|| text.find(&format!(r#""event":"{event_name}""#)))
        else {
            continue;
        };
        let Some(raw_url) = find_url_near(text, position) else {
            continue;
        };
        let h2h_url = absolute_url(DEFAULT_BASE_URL, &raw_url)?;
        let encoded_event_id = extract_hash(&h2h_url)
            .ok_or_else(|| anyhow!("OddsPortal match URL missing event hash"))?;
        return Ok(Some(DiscoveredMatch {
            event_name: event_name.to_string(),
            h2h_url,
            encoded_event_id,
        }));
    }

    Ok(None)
}

fn find_url_near(text: &str, position: usize) -> Option<String> {
    let window_start = position.saturating_sub(2_000);
    let window_end = (position + 2_000).min(text.len());
    let window = &text[window_start..window_end];

    find_json_strings(window, r#""url":""#)
        .into_iter()
        .map(|url| url.replace("\\/", "/"))
        .find(|url| url.contains("/football/h2h/") && url.contains('#'))
}

fn find_json_strings(text: &str, marker: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(offset) = rest.find(marker) {
        let tail = &rest[offset + marker.len()..];
        let Some(end) = tail.find('"') else {
            break;
        };
        values.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    values
}

fn find_json_bool(text: &str, field: &str) -> Option<bool> {
    let marker = format!(r#""{field}":"#);
    let value = text.get(text.find(&marker)? + marker.len()..)?;
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn find_request_url(text: &str, request_name: &str) -> Option<String> {
    let request_marker = format!(r#""{request_name}""#);
    let request = &text[text.find(&request_marker)? + request_marker.len()..];
    let object = request.trim_start().strip_prefix(':')?.trim_start();
    let object = object.strip_prefix('{')?.trim_start();
    let url = object.strip_prefix(r#""url""#)?.trim_start();
    let value = url.strip_prefix(':')?.trim_start().strip_prefix('"')?;
    Some(value[..value.find('"')?].to_string())
}

#[derive(Debug, Deserialize)]
struct TournamentRows {
    rows: Vec<TournamentRow>,
}

#[derive(Debug, Deserialize)]
struct TournamentRow {
    url: String,
    event: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_match_from_tournament_embedded_state() {
        let html = r#"<div id="react-leagues-events" data='{"rows":[{"url":"\/football\/h2h\/france-QkGeVG1n\/norway-8rP6JO0H\/#bsJSJ30L","event":"Norway - France"}]}'></div>"#;

        let found = parse_tournament_match(html, "Norway", "France").unwrap();

        assert_eq!(found.event_name, "Norway - France");
        assert_eq!(found.encoded_event_id, "bsJSJ30L");
        assert_eq!(
            found.h2h_url,
            "https://www.oddsportal.com/football/h2h/france-QkGeVG1n/norway-8rP6JO0H/#bsJSJ30L"
        );
    }

    #[test]
    fn live_match_uses_request_live_and_ignores_pre_match_url() {
        let html = r#"<Event :data="{&quot;eventData&quot;:{
          &quot;isLive&quot;:true,&quot;realLive&quot;:true},
          &quot;requestPreMatch&quot;:{
          &quot;url&quot;:&quot;\/match-event\/ignored.dat?_=&quot;},
          &quot;requestLive&quot;:{
          &quot;url&quot;:&quot;\/feed\/live-event\/1-1-EZmXxG15-1-2-yjlive.dat?_=&amp;geo=JP&quot;}}">
          </Event>"#;

        assert_eq!(
            parse_live_odds_request(html).unwrap(),
            crate::oddsportal::models::LiveOddsRequestState::Available {
                url: "https://www.oddsportal.com/feed/live-event/1-1-EZmXxG15-1-2-yjlive.dat?_=&geo=JP"
                    .to_string(),
            }
        );
    }

    #[test]
    fn non_live_match_is_unavailable_even_with_request_live() {
        let html = r#"<Event :data="{&quot;eventData&quot;:{
          &quot;isLive&quot;:false,&quot;realLive&quot;:true},
          &quot;requestLive&quot;:{
          &quot;url&quot;:&quot;\/feed\/live-event\/test.dat?_=&quot;}}">
          </Event>"#;

        assert_eq!(
            parse_live_odds_request(html).unwrap(),
            crate::oddsportal::models::LiveOddsRequestState::Unavailable
        );
    }

    #[test]
    fn live_match_without_request_live_is_unavailable() {
        let html = r#"<Event :data="{&quot;eventData&quot;:{
          &quot;isLive&quot;:true,&quot;realLive&quot;:true},
          &quot;requestPreMatch&quot;:{
          &quot;url&quot;:&quot;\/match-event\/ignored.dat?_=&quot;}}">
          </Event>"#;

        assert_eq!(
            parse_live_odds_request(html).unwrap(),
            crate::oddsportal::models::LiveOddsRequestState::Unavailable
        );
    }

    #[test]
    fn extracts_score_request_when_pre_match_data_is_present() {
        let html = r#"<Event :data="{&quot;requestPreMatch&quot;:{
          &quot;url&quot;:&quot;\/match-event\/1-1-EZmXxG15-1-2-yj93f.dat?_=&quot;},
          &quot;updateScoreRequest&quot;:{
          &quot;url&quot;:&quot;\/feed\/postmatch-score\/1-EZmXxG15-yj93f.dat?_=&quot;}}">
          </Event>"#;

        let metadata = parse_h2h_request_metadata(html).unwrap();

        assert_eq!(
            metadata.score_url.as_deref(),
            Some("https://www.oddsportal.com/feed/postmatch-score/1-EZmXxG15-yj93f.dat?_=")
        );
    }

    #[test]
    fn score_metadata_does_not_require_pre_match_request() {
        let html = r#"<Event :data="{&quot;updateScoreRequest&quot;:{
          &quot;url&quot;:&quot;\/feed\/postmatch-score\/1-EZmXxG15-yj93f.dat?_=&quot;}}">
          </Event>"#;

        let metadata = parse_h2h_request_metadata(html).unwrap();

        assert_eq!(
            metadata.score_url.as_deref(),
            Some("https://www.oddsportal.com/feed/postmatch-score/1-EZmXxG15-yj93f.dat?_=")
        );
    }

    #[test]
    fn discovers_match_from_escaped_tournament_page_state() {
        let html = r#"<star-component :sport-data="{&quot;d&quot;:{&quot;rows&quot;:[{&quot;name&quot;:&quot;Norway - France&quot;,&quot;url&quot;:&quot;\/football\/h2h\/france-QkGeVG1n\/norway-8rP6JO0H\/#bsJSJ30L&quot;}]}}"></star-component>"#;

        let found = parse_tournament_match(html, "Norway", "France").unwrap();

        assert_eq!(found.event_name, "Norway - France");
        assert_eq!(found.encoded_event_id, "bsJSJ30L");
        assert_eq!(
            found.h2h_url,
            "https://www.oddsportal.com/football/h2h/france-QkGeVG1n/norway-8rP6JO0H/#bsJSJ30L"
        );
    }

    #[test]
    fn embedded_state_fallback_uses_h2h_url_not_nearby_participant_urls() {
        let html = r#"<star-component :sport-data="{&quot;d&quot;:{&quot;rows&quot;:[{&quot;url&quot;:&quot;\/football\/h2h\/france-QkGeVG1n\/norway-8rP6JO0H\/#bsJSJ30L&quot;,&quot;homeParticipantUrl&quot;:&quot;\/football\/team\/norway\/8rP6JO0H\/&quot;,&quot;tournament-url&quot;:&quot;world-championship-2026&quot;,&quot;name&quot;:&quot;Norway - France&quot;}]}}"></star-component>"#;

        let found = parse_tournament_match(html, "Norway", "France").unwrap();

        assert_eq!(found.encoded_event_id, "bsJSJ30L");
        assert!(found.h2h_url.contains("/football/h2h/"));
    }
}
