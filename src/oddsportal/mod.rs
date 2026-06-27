pub mod config;
pub mod decoder;
pub mod discovery;
pub mod logging;
pub mod models;
pub mod odds;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::time::{sleep, Duration};

pub async fn collect_once(config: config::Config) -> Result<Vec<models::OddsPortalRecord>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Requested-With",
        HeaderValue::from_static("XMLHttpRequest"),
    );
    let mut client = reqwest::Client::builder()
        .http1_only()
        .default_headers(headers)
        .user_agent(config.user_agent.clone());
    if let Some(proxy_url) = &config.proxy_url {
        client = client.proxy(
            reqwest::Proxy::all(proxy_url)
                .with_context(|| format!("invalid OddsPortal proxy URL {proxy_url}"))?,
        );
    }
    let client = client
        .build()
        .context("failed to build OddsPortal HTTP client")?;
    let target = config.target_match();

    let tournament_html =
        get_text_with_retries(&client, &config.tournament_url, "OddsPortal tournament").await?;
    let discovered =
        discovery::parse_tournament_match(&tournament_html, &target.home_team, &target.away_team)?;

    let h2h_request_url = http_request_url(&discovered.h2h_url);
    let h2h_html = get_text_with_retries(&client, &h2h_request_url, "OddsPortal H2H").await?;
    let request = discovery::parse_h2h_request_metadata(&h2h_html)?;
    let records = collect_dat_records(&client, &request, &discovered).await?;

    let mut logger = logging::OddsPortalLogger::new(&config.log_path)?;
    for record in &records {
        println!(
            "oddsportal {} {} {} {}",
            record.event_name, record.bookmaker_name, record.outcome, record.decimal_odds
        );
        logger.append(record)?;
    }

    Ok(records)
}

async fn collect_dat_records(
    client: &reqwest::Client,
    request: &models::RequestMetadata,
    discovered: &models::DiscoveredMatch,
) -> Result<Vec<models::OddsPortalRecord>> {
    let mut urls = vec![request.pre_match_url.clone()];
    if let Some(fallback_url) = &request.fallback_pre_match_url {
        urls.push(fallback_url.clone());
    }

    let mut last_error = None;
    for url in urls {
        let source_url = cache_busted_url(&url);
        match collect_dat_records_from_url(client, &source_url, discovered).await {
            Ok(records) if !records.is_empty() => return Ok(records),
            Ok(_) => {
                last_error = Some(anyhow!("OddsPortal .dat response contained no 1X2 odds"));
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no OddsPortal .dat URLs were available")))
}

async fn collect_dat_records_from_url(
    client: &reqwest::Client,
    source_url: &str,
    discovered: &models::DiscoveredMatch,
) -> Result<Vec<models::OddsPortalRecord>> {
    let dat_body = get_text_with_retries(client, source_url, "OddsPortal .dat").await?;
    let decoded = decoder::decode_dat_payload(&dat_body)
        .with_context(|| format!("failed to decode OddsPortal .dat response for {source_url}"))?;
    odds::normalize_1x2_odds(&decoded, &discovered.event_name, source_url).with_context(|| {
        format!(
            "failed to normalize OddsPortal odds for {} ({})",
            discovered.event_name, discovered.encoded_event_id
        )
    })
}

async fn get_text_with_retries(client: &reqwest::Client, url: &str, label: &str) -> Result<String> {
    let mut last_error = None;
    for attempt in 1..=3 {
        let result = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("{label} request failed: {url}"))
            .and_then(|response| {
                response
                    .error_for_status()
                    .with_context(|| format!("{label} returned error: {url}"))
            });

        match result {
            Ok(response) => match response.text().await {
                Ok(text) => return Ok(text),
                Err(error) => {
                    last_error = Some(anyhow!("failed to read {label} response: {error}"));
                }
            },
            Err(error) => last_error = Some(error),
        }

        if attempt < 3 {
            sleep(Duration::from_millis(500 * attempt)).await;
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("{label} request failed without error: {url}")))
}

fn cache_busted_url(url: &str) -> String {
    if !url.ends_with("_=") {
        return url.to_string();
    }
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{url}{millis}")
}

fn http_request_url(url: &str) -> String {
    let Some((without_fragment, _fragment)) = url.split_once('#') else {
        return url.to_string();
    };
    without_fragment.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_busted_url_appends_timestamp_to_open_cache_param() {
        let url = cache_busted_url("https://www.oddsportal.com/match-event/test.dat?_=");

        assert!(url.starts_with("https://www.oddsportal.com/match-event/test.dat?_="));
        assert!(url.len() > "https://www.oddsportal.com/match-event/test.dat?_=".len());
    }

    #[test]
    fn cache_busted_url_leaves_complete_url_unchanged() {
        let url = "https://www.oddsportal.com/match-event/test.dat?_=123";

        assert_eq!(cache_busted_url(url), url);
    }

    #[test]
    fn http_request_url_removes_fragment() {
        let url = http_request_url(
            "https://www.oddsportal.com/football/h2h/france-QkGeVG1n/norway-8rP6JO0H/#bsJSJ30L",
        );

        assert_eq!(
            url,
            "https://www.oddsportal.com/football/h2h/france-QkGeVG1n/norway-8rP6JO0H/"
        );
    }
}
