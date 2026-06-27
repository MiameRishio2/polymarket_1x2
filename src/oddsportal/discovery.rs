use anyhow::{anyhow, bail, Context, Result};
use html_escape::decode_html_entities;
use percent_encoding::percent_decode_str;
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use crate::oddsportal::config::DEFAULT_BASE_URL;
use crate::oddsportal::models::{DiscoveredMatch, RequestMetadata};

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
    let marker = r#""requestPreMatch":{"url":""#;
    let start = decoded
        .find(marker)
        .ok_or_else(|| anyhow!("OddsPortal H2H page missing requestPreMatch.url"))?
        + marker.len();
    let tail = &decoded[start..];
    let end = tail
        .find('"')
        .ok_or_else(|| anyhow!("OddsPortal requestPreMatch.url is unterminated"))?;
    let raw_url = tail[..end].replace("\\/", "/");
    let fallback_pre_match_url = absolute_url(DEFAULT_BASE_URL, &raw_url)?;
    let pre_match_url = find_json_string(&decoded, r#""xhash":""#)
        .and_then(|hash| {
            percent_decode_str(&hash)
                .decode_utf8()
                .ok()
                .map(|hash| hash.to_string())
        })
        .filter(|hash| !hash.is_empty())
        .and_then(|hash| pre_match_url_with_hash(&fallback_pre_match_url, &hash).ok())
        .unwrap_or_else(|| fallback_pre_match_url.clone());

    Ok(RequestMetadata {
        fallback_pre_match_url: (pre_match_url != fallback_pre_match_url)
            .then_some(fallback_pre_match_url),
        pre_match_url,
    })
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

fn pre_match_url_with_hash(url: &str, hash: &str) -> Result<String> {
    let mut parsed = Url::parse(url).context("invalid OddsPortal pre-match URL")?;
    let path = parsed.path();
    let Some((prefix, _suffix)) = path.rsplit_once(".dat") else {
        return Ok(url.to_string());
    };
    let Some((base, _old_hash)) = prefix.rsplit_once('-') else {
        return Ok(url.to_string());
    };
    parsed.set_path(&format!("{base}-{hash}.dat"));
    Ok(parsed.to_string())
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

fn find_json_string(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)? + marker.len();
    let tail = &text[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
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
    fn extracts_request_pre_match_url_from_h2h_state() {
        let html = r#"<event :data="{&quot;requestPreMatch&quot;:{&quot;url&quot;:&quot;\/match-event\/1-1-bsJSJ30L-1-2-yj159.dat?_=&quot;}}"></event>"#;

        let metadata = parse_h2h_request_metadata(html).unwrap();

        assert_eq!(
            metadata.pre_match_url,
            "https://www.oddsportal.com/match-event/1-1-bsJSJ30L-1-2-yj159.dat?_="
        );
        assert_eq!(metadata.fallback_pre_match_url, None);
    }

    #[test]
    fn uses_frontend_xhash_before_request_pre_match_fallback() {
        let html = r#"<Event :data="{&quot;eventData&quot;:{&quot;xhash&quot;:&quot;%79%6a%31%35%39&quot;,&quot;xhashf&quot;:&quot;%79%6a%34%34%64&quot;},&quot;requestPreMatch&quot;:{&quot;url&quot;:&quot;\/match-event\/1-1-bsJSJ30L-1-2-yj44d.dat?_=&quot;}}"></Event>"#;

        let metadata = parse_h2h_request_metadata(html).unwrap();

        assert_eq!(
            metadata.pre_match_url,
            "https://www.oddsportal.com/match-event/1-1-bsJSJ30L-1-2-yj159.dat?_="
        );
        assert_eq!(
            metadata.fallback_pre_match_url,
            Some(
                "https://www.oddsportal.com/match-event/1-1-bsJSJ30L-1-2-yj44d.dat?_=".to_string()
            )
        );
    }

    #[test]
    fn uses_frontend_xhash_from_backslash_escaped_h2h_data() {
        let html = r#"<Event :data="{\&quot;eventData\&quot;:{\&quot;xhash\&quot;:\&quot;%79%6a%31%35%39\&quot;},\&quot;requestPreMatch\&quot;:{\&quot;url\&quot;:\&quot;\/match-event\/1-1-bsJSJ30L-1-2-yj44d.dat?_=\&quot;}}"></Event>"#;

        let metadata = parse_h2h_request_metadata(html).unwrap();

        assert_eq!(
            metadata.pre_match_url,
            "https://www.oddsportal.com/match-event/1-1-bsJSJ30L-1-2-yj159.dat?_="
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
