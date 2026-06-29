pub mod config;
pub mod decoder;
pub mod discovery;
pub mod logging;
pub mod models;
pub mod odds;
pub mod output;
pub mod score;

use std::future::Future;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::time::{sleep, Duration, MissedTickBehavior};

pub(crate) const LOG_PREFIX: &str = "[oddsportal]";

pub async fn run_poll_loop(config: config::Config, interval: Duration) -> Result<()> {
    let client = build_client(&config)?;
    let discovery = discover_requests(&client, &config).await?;
    let odds_client = client.clone();
    let odds_event = discovery.0.clone();
    let odds_requests = discovery.1.clone();
    let odds_config = config.clone();
    let score_client = client.clone();
    let score_event = discovery.0.clone();
    let score_url = discovery.1.score_url.clone();
    let score_config = config.clone();

    run_poll_loop_with(
        config,
        interval,
        None,
        discovery,
        move || {
            let client = odds_client.clone();
            let event = odds_event.clone();
            let requests = odds_requests.clone();
            let config = odds_config.clone();
            async move {
                let records = collect_odds(&client, &requests, &event).await?;
                append_odds_records(&config.log_path, &records)?;
                let output = output::OddsPortalOddsObservation::from_records(
                    &records,
                    &config.home_team,
                    &config.away_team,
                )?;
                output::write_observation(&output)?;
                Ok(records)
            }
        },
        move || {
            let client = score_client.clone();
            let event = score_event.clone();
            let score_url = score_url.clone();
            let config = score_config.clone();
            async move {
                let record = collect_score(&client, score_url.as_deref(), &event, &config).await?;
                output::write_observation(&record)?;
                Ok(record)
            }
        },
    )
    .await
}

struct CycleResult {
    odds: Option<Vec<models::OddsPortalRecord>>,
    score: Option<output::OddsPortalScoreObservation>,
}

async fn run_poll_loop_with<Odds, OddsFuture, Score, ScoreFuture>(
    _config: config::Config,
    interval: Duration,
    max_iterations: Option<usize>,
    _discovery: (models::DiscoveredMatch, models::RequestMetadata),
    mut collect_odds: Odds,
    mut collect_score: Score,
) -> Result<()>
where
    Odds: FnMut() -> OddsFuture,
    OddsFuture: Future<Output = Result<Vec<models::OddsPortalRecord>>>,
    Score: FnMut() -> ScoreFuture,
    ScoreFuture: Future<Output = Result<output::OddsPortalScoreObservation>>,
{
    let mut completed = 0;
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        if max_iterations == Some(completed) {
            return Ok(());
        }

        ticker.tick().await;
        println!("{LOG_PREFIX} starting collection pass");
        let result = run_one_cycle_with(collect_odds(), collect_score()).await?;
        if let Some(records) = result.odds {
            println!(
                "{LOG_PREFIX} collection pass succeeded with {} records",
                records.len()
            );
        }
        if result.score.is_some() {
            println!("{LOG_PREFIX} score collection pass succeeded");
        }
        completed += 1;
    }
}

async fn run_one_cycle_with<OddsFuture, ScoreFuture>(
    odds_future: OddsFuture,
    score_future: ScoreFuture,
) -> Result<CycleResult>
where
    OddsFuture: Future<Output = Result<Vec<models::OddsPortalRecord>>>,
    ScoreFuture: Future<Output = Result<output::OddsPortalScoreObservation>>,
{
    let (odds_result, score_result) = tokio::join!(odds_future, score_future);
    let odds = match odds_result {
        Ok(records) => Some(records),
        Err(error) => {
            eprintln!("{LOG_PREFIX} odds collection failed: {error:#}");
            None
        }
    };
    let score = match score_result {
        Ok(record) => Some(record),
        Err(error) => {
            eprintln!("{LOG_PREFIX} score collection failed: {error:#}");
            None
        }
    };
    Ok(CycleResult { odds, score })
}

fn build_client(config: &config::Config) -> Result<reqwest::Client> {
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
    Ok(client)
}

async fn discover_requests(
    client: &reqwest::Client,
    config: &config::Config,
) -> Result<(models::DiscoveredMatch, models::RequestMetadata)> {
    let target = config.target_match();

    let tournament_html =
        get_text_with_retries(client, &config.tournament_url, "OddsPortal tournament").await?;
    let event =
        discovery::parse_tournament_match(&tournament_html, &target.home_team, &target.away_team)?;
    let h2h_html =
        get_text_with_retries(client, &http_request_url(&event.h2h_url), "OddsPortal H2H").await?;
    let requests = discovery::parse_h2h_request_metadata(&h2h_html)?;
    Ok((event, requests))
}

fn append_odds_records(
    log_path: &std::path::Path,
    records: &[models::OddsPortalRecord],
) -> Result<()> {
    let mut logger = logging::OddsPortalLogger::new(log_path)?;
    for record in records {
        println!(
            "{LOG_PREFIX} {} {} {} {}",
            record.event_name, record.bookmaker_name, record.outcome, record.decimal_odds
        );
        logger.append(record)?;
    }
    Ok(())
}

async fn collect_odds(
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
    let dat_body = get_text_once(client, source_url, "OddsPortal .dat").await?;
    let decoded = decoder::decode_dat_payload(&dat_body)
        .with_context(|| format!("failed to decode OddsPortal .dat response for {source_url}"))?;
    odds::normalize_1x2_odds(&decoded, &discovered.event_name, source_url).with_context(|| {
        format!(
            "failed to normalize OddsPortal odds for {} ({})",
            discovered.event_name, discovered.encoded_event_id
        )
    })
}

async fn collect_score(
    client: &reqwest::Client,
    score_url: Option<&str>,
    event: &models::DiscoveredMatch,
    config: &config::Config,
) -> Result<output::OddsPortalScoreObservation> {
    let Some(url) = score_url else {
        return Ok(score::unavailable_score(
            event,
            &config.home_team,
            &config.away_team,
        ));
    };
    let response = client
        .get(cache_busted_url(url))
        .send()
        .await
        .with_context(|| format!("OddsPortal score request failed: {url}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(score::unavailable_score(
            event,
            &config.home_team,
            &config.away_team,
        ));
    }
    let body = response
        .error_for_status()
        .with_context(|| format!("OddsPortal score returned error: {url}"))?
        .text()
        .await
        .context("failed to read OddsPortal score response")?;
    let decoded = decoder::decode_dat_payload(&body)
        .with_context(|| format!("failed to decode OddsPortal score response for {url}"))?;
    score::parse_score_payload(&decoded, event, &config.home_team, &config.away_team)
}

async fn get_text_once(client: &reqwest::Client, url: &str, label: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("{label} request failed: {url}"))?
        .error_for_status()
        .with_context(|| format!("{label} returned error: {url}"))?
        .text()
        .await
        .with_context(|| format!("failed to read {label} response"))
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
            if let Some(error) = &last_error {
                eprintln!("{LOG_PREFIX} {label} attempt {attempt} failed; retrying: {error:#}");
            }
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
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn polling_status_uses_stable_provider_prefix() {
        assert_eq!(LOG_PREFIX, "[oddsportal]");
    }

    #[tokio::test]
    async fn polling_continues_after_failed_pass() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&attempts);
        run_poll_loop_with(
            config::Config::default(),
            Duration::from_millis(1),
            Some(2),
            test_discovery(),
            move || {
                observed.fetch_add(1, Ordering::SeqCst);
                async { Err(anyhow!("expected test failure")) }
            },
            || async { Ok(score_fixture()) },
        )
        .await
        .unwrap();

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn polling_reports_pass_start_before_awaiting_collector() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "oddsportal::tests::polling_output_helper",
                "--nocapture",
            ])
            .env("ODDSPORTAL_POLLING_OUTPUT_HELPER", "1")
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let start = stdout
            .find("[oddsportal] starting collection pass")
            .expect("missing pass-start output");
        let collector = stdout
            .find("test collector entered")
            .expect("missing collector output");

        assert!(start < collector, "{stdout}");
    }

    #[tokio::test]
    async fn polling_output_helper() {
        if std::env::var_os("ODDSPORTAL_POLLING_OUTPUT_HELPER").is_none() {
            return;
        }
        run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(60),
            Some(1),
            test_discovery(),
            || async {
                println!("test collector entered");
                Ok(Vec::new())
            },
            || async { Ok(score_fixture()) },
        )
        .await
        .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn polling_runs_first_pass_immediately() {
        let started = tokio::time::Instant::now();
        let (first_pass, observed) = tokio::sync::oneshot::channel();
        let mut first_pass = Some(first_pass);
        let task = tokio::spawn(run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(30),
            Some(1),
            test_discovery(),
            move || {
                first_pass.take().unwrap().send(()).unwrap();
                async { Ok(Vec::new()) }
            },
            || async { Ok(score_fixture()) },
        ));

        observed.await.unwrap();
        task.await.unwrap().unwrap();
        assert_eq!(tokio::time::Instant::now() - started, Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn polling_waits_then_runs_again_after_success() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&attempts);
        let task = tokio::spawn(run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(30),
            Some(2),
            test_discovery(),
            move || {
                observed.fetch_add(1, Ordering::SeqCst);
                async { Ok(Vec::new()) }
            },
            || async { Ok(score_fixture()) },
        ));

        tokio::task::yield_now().await;
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        tokio::time::advance(Duration::from_secs(29)).await;
        tokio::task::yield_now().await;
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        tokio::time::advance(Duration::from_secs(1)).await;
        task.await.unwrap().unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn cycle_starts_odds_and_score_together_without_overlap() {
        let odds_calls = Arc::new(AtomicUsize::new(0));
        let score_calls = Arc::new(AtomicUsize::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let odds = counting_odds(
            Arc::clone(&odds_calls),
            Arc::clone(&active),
            Arc::clone(&max_active),
        );
        let scores = counting_scores(
            Arc::clone(&score_calls),
            Arc::clone(&active),
            Arc::clone(&max_active),
        );

        let task = tokio::spawn(run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(1),
            Some(2),
            test_discovery(),
            odds,
            scores,
        ));
        tokio::task::yield_now().await;
        assert_eq!(odds_calls.load(Ordering::SeqCst), 1);
        assert_eq!(score_calls.load(Ordering::SeqCst), 1);

        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        task.await.unwrap().unwrap();

        assert_eq!(odds_calls.load(Ordering::SeqCst), 2);
        assert_eq!(score_calls.load(Ordering::SeqCst), 2);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn emits_successful_side_when_peer_request_fails() {
        let result = run_one_cycle_with(async { Err(anyhow!("odds failed")) }, async {
            Ok(score_fixture())
        })
        .await
        .unwrap();

        assert!(result.odds.is_none());
        assert!(result.score.is_some());
    }

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

    type BoxResultFuture<T> = std::pin::Pin<Box<dyn Future<Output = Result<T>> + Send + 'static>>;

    fn counting_odds(
        calls: Arc<AtomicUsize>,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
    ) -> impl FnMut() -> BoxResultFuture<Vec<models::OddsPortalRecord>> {
        move || {
            let calls = Arc::clone(&calls);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                tokio::task::yield_now().await;
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(vec![odds_fixture()])
            })
        }
    }

    fn counting_scores(
        calls: Arc<AtomicUsize>,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
    ) -> impl FnMut() -> BoxResultFuture<output::OddsPortalScoreObservation> {
        move || {
            let calls = Arc::clone(&calls);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                tokio::task::yield_now().await;
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(score_fixture())
            })
        }
    }

    fn test_discovery() -> (models::DiscoveredMatch, models::RequestMetadata) {
        (
            models::DiscoveredMatch {
                event_name: "South Africa - Canada".to_string(),
                h2h_url: "https://www.oddsportal.com/football/h2h/test/#EZmXxG15".to_string(),
                encoded_event_id: "EZmXxG15".to_string(),
            },
            models::RequestMetadata {
                pre_match_url: "https://www.oddsportal.com/match-event/test.dat".to_string(),
                fallback_pre_match_url: None,
                score_url: Some(
                    "https://www.oddsportal.com/feed/postmatch-score/test.dat".to_string(),
                ),
            },
        )
    }

    fn odds_fixture() -> models::OddsPortalRecord {
        models::OddsPortalRecord {
            ts: "2026-06-28T12:00:00Z".to_string(),
            provider: "oddsportal".to_string(),
            event_id: "EZmXxG15".to_string(),
            event_name: "South Africa - Canada".to_string(),
            bookmaker_id: "16".to_string(),
            bookmaker_name: "bet365".to_string(),
            outcome: "1".to_string(),
            decimal_odds: "5.50".to_string(),
            source_url: "https://www.oddsportal.com/match-event/test.dat".to_string(),
        }
    }

    fn score_fixture() -> output::OddsPortalScoreObservation {
        score::unavailable_score(&test_discovery().0, "South Africa", "Canada")
    }
}
