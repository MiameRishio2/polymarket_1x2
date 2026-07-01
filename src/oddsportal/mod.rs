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
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING, REFERER};
use tokio::time::{sleep, Duration, MissedTickBehavior};

pub(crate) const LOG_PREFIX: &str = "[oddsportal]";
pub(crate) const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn run_poll_loop(config: config::Config, interval: Duration) -> Result<()> {
    let client = build_client(&config)?;
    let discovery = discover_requests(&client, &config).await?;
    let odds_client = client.clone();
    let odds_event = discovery.0.clone();
    let score_client = client.clone();
    let score_event = discovery.0.clone();
    let score_url = discovery.1.score_url.clone();
    let score_config = config.clone();
    let append_log_path = config.log_path.clone();
    let odds_home_team = config.home_team.clone();
    let odds_away_team = config.away_team.clone();

    run_poll_loop_with(
        config,
        interval,
        None,
        discovery,
        move || {
            let client = odds_client.clone();
            let event = odds_event.clone();
            async move { collect_odds(&client, &event).await }
        },
        move || {
            let client = score_client.clone();
            let event = score_event.clone();
            let score_url = score_url.clone();
            let config = score_config.clone();
            async move { collect_score(&client, score_url.as_deref(), &event, &config).await }
        },
        move |records| append_odds_records(&append_log_path, records),
        move |records| {
            let observation = output::OddsPortalOddsObservation::from_records(
                records,
                &odds_home_team,
                &odds_away_team,
            )?;
            output::write_observation(&observation)
        },
        output::write_observation,
    )
    .await
}

struct CycleResult {
    odds: Option<OddsCollection>,
    score: Option<output::OddsPortalScoreObservation>,
}

#[derive(Debug)]
enum OddsCollection {
    Unavailable,
    Records(Vec<models::OddsPortalRecord>),
}

#[derive(Debug)]
struct CycleHandlingStatus {
    odds_succeeded: bool,
    odds_unavailable: bool,
    score_succeeded: bool,
}

async fn run_poll_loop_with<Odds, OddsFuture, Score, ScoreFuture, AppendOdds, EmitOdds, EmitScore>(
    _config: config::Config,
    interval: Duration,
    max_iterations: Option<usize>,
    _discovery: (models::DiscoveredMatch, models::RequestMetadata),
    mut collect_odds: Odds,
    mut collect_score: Score,
    mut append_odds: AppendOdds,
    mut emit_odds: EmitOdds,
    mut emit_score: EmitScore,
) -> Result<()>
where
    Odds: FnMut() -> OddsFuture,
    OddsFuture: Future<Output = Result<OddsCollection>>,
    Score: FnMut() -> ScoreFuture,
    ScoreFuture: Future<Output = Result<output::OddsPortalScoreObservation>>,
    AppendOdds: FnMut(&[models::OddsPortalRecord]) -> Result<()>,
    EmitOdds: FnMut(&[models::OddsPortalRecord]) -> Result<()>,
    EmitScore: FnMut(&output::OddsPortalScoreObservation) -> Result<()>,
{
    let mut completed = 0;
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        if max_iterations == Some(completed) {
            return Ok(());
        }

        ticker.tick().await;
        crate::diagnostics::write(format_args!("{LOG_PREFIX} starting collection pass"));
        let result = run_one_cycle_with(collect_odds(), collect_score()).await?;
        let status = handle_cycle_with(&result, &mut append_odds, &mut emit_odds, &mut emit_score)?;
        if status.odds_succeeded {
            if status.odds_unavailable {
                crate::diagnostics::write(format_args!("{LOG_PREFIX} no in-play odds available"));
            } else {
                let Some(OddsCollection::Records(records)) = result.odds.as_ref() else {
                    unreachable!("successful available odds must contain records");
                };
                crate::diagnostics::write(format_args!(
                    "{LOG_PREFIX} collection pass succeeded with {} records",
                    records.len()
                ));
            }
        }
        if status.score_succeeded {
            crate::diagnostics::write(format_args!("{LOG_PREFIX} score collection pass succeeded"));
        }
        completed += 1;
    }
}

async fn run_one_cycle_with<OddsFuture, ScoreFuture>(
    odds_future: OddsFuture,
    score_future: ScoreFuture,
) -> Result<CycleResult>
where
    OddsFuture: Future<Output = Result<OddsCollection>>,
    ScoreFuture: Future<Output = Result<output::OddsPortalScoreObservation>>,
{
    let (odds_result, score_result) = tokio::join!(odds_future, score_future);
    let odds = match odds_result {
        Ok(records) => Some(records),
        Err(error) => {
            crate::diagnostics::write(format_args!(
                "{LOG_PREFIX} odds collection failed: {error:#}"
            ));
            None
        }
    };
    let score = match score_result {
        Ok(record) => Some(record),
        Err(error) => {
            crate::diagnostics::write(format_args!(
                "{LOG_PREFIX} score collection failed: {error:#}"
            ));
            None
        }
    };
    Ok(CycleResult { odds, score })
}

fn handle_cycle_with<AppendOdds, EmitOdds, EmitScore>(
    result: &CycleResult,
    mut append_odds: AppendOdds,
    mut emit_odds: EmitOdds,
    mut emit_score: EmitScore,
) -> Result<CycleHandlingStatus>
where
    AppendOdds: FnMut(&[models::OddsPortalRecord]) -> Result<()>,
    EmitOdds: FnMut(&[models::OddsPortalRecord]) -> Result<()>,
    EmitScore: FnMut(&output::OddsPortalScoreObservation) -> Result<()>,
{
    let mut first_error = None;
    let mut odds_succeeded = false;
    let mut odds_unavailable = false;
    if let Some(odds) = result.odds.as_ref() {
        match odds {
            OddsCollection::Unavailable => {
                odds_succeeded = true;
                odds_unavailable = true;
            }
            OddsCollection::Records(records) => match append_odds(records) {
                Ok(()) => match emit_odds(records) {
                    Ok(()) => odds_succeeded = true,
                    Err(error) => {
                        crate::diagnostics::write(format_args!(
                            "{LOG_PREFIX} odds output failed: {error:#}"
                        ));
                        first_error = Some(error);
                    }
                },
                Err(error) => {
                    crate::diagnostics::write(format_args!(
                        "{LOG_PREFIX} odds append failed: {error:#}"
                    ));
                    first_error = Some(error);
                }
            },
        }
    }

    let mut score_succeeded = false;
    if let Some(record) = result.score.as_ref() {
        match emit_score(record) {
            Ok(()) => score_succeeded = true,
            Err(error) => {
                crate::diagnostics::write(format_args!(
                    "{LOG_PREFIX} score output failed: {error:#}"
                ));
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }
    Ok(CycleHandlingStatus {
        odds_succeeded,
        odds_unavailable,
        score_succeeded,
    })
}

fn build_client(config: &config::Config) -> Result<reqwest::Client> {
    build_client_with_timeouts(config, HTTP_CONNECT_TIMEOUT, HTTP_REQUEST_TIMEOUT)
}

fn build_client_with_timeouts(
    config: &config::Config,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-Requested-With",
        HeaderValue::from_static("XMLHttpRequest"),
    );
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    let mut client = reqwest::Client::builder()
        .http1_only()
        .default_headers(headers)
        .user_agent(config.user_agent.clone())
        .connect_timeout(connect_timeout)
        .timeout(request_timeout);
    if let Some(proxy_url) = &config.proxy_url {
        client = client.proxy(
            reqwest::Proxy::all(proxy_url)
                .with_context(|| format!("invalid OddsPortal proxy URL {proxy_url}"))?,
        );
    } else {
        client = client.no_proxy();
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
        crate::diagnostics::write(format_args!(
            "{LOG_PREFIX} {} {} {} {}",
            record.event_name, record.bookmaker_name, record.outcome, record.decimal_odds
        ));
        logger.append(record)?;
    }
    Ok(())
}

async fn collect_odds(
    client: &reqwest::Client,
    discovered: &models::DiscoveredMatch,
) -> Result<OddsCollection> {
    let h2h_url = http_request_url(&discovered.h2h_url);
    let h2h_html = get_text_with_retries(client, &h2h_url, "OddsPortal H2H").await?;
    let live_url = match discovery::parse_live_odds_request(&h2h_html)? {
        models::LiveOddsRequestState::Unavailable => return Ok(OddsCollection::Unavailable),
        models::LiveOddsRequestState::Available { url } => cache_busted_url(&url),
    };
    let response = client
        .get(&live_url)
        .header(REFERER, &h2h_url)
        .send()
        .await
        .with_context(|| format!("OddsPortal live odds request failed: {live_url}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(OddsCollection::Unavailable);
    }
    let dat_body = response
        .error_for_status()
        .with_context(|| format!("OddsPortal live odds returned error: {live_url}"))?
        .text()
        .await
        .context("failed to read OddsPortal live odds response")?;
    let trimmed = dat_body.trim();
    if trimmed.starts_with("URL:") && trimmed.ends_with("Status: 404") {
        return Ok(OddsCollection::Unavailable);
    }
    let decoded = decoder::decode_dat_payload(&dat_body).with_context(|| {
        format!("failed to decode OddsPortal live odds response for {live_url}")
    })?;
    let records = odds::normalize_1x2_odds(&decoded, &discovered.event_name, &live_url)
        .with_context(|| {
            format!(
                "failed to normalize OddsPortal live odds for {} ({})",
                discovered.event_name, discovered.encoded_event_id
            )
        })?;
    if records.is_empty() {
        return Err(anyhow!(
            "OddsPortal live odds response contained no active 1X2 odds"
        ));
    }
    Ok(OddsCollection::Records(records))
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
    let trimmed = body.trim();
    if trimmed.starts_with("URL:") && trimmed.ends_with("Status: 404") {
        return Ok(score::unavailable_score(
            event,
            &config.home_team,
            &config.away_team,
        ));
    }
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
                crate::diagnostics::write(format_args!(
                    "{LOG_PREFIX} {label} attempt {attempt} failed; retrying: {error:#}"
                ));
            }
            sleep(Duration::from_millis(500 * attempt)).await;
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("{label} request failed without error: {url}")))
}

fn cache_busted_url(url: &str) -> String {
    let open_param = if url.ends_with("_=") {
        Some(url.len())
    } else {
        url.find("_=&").map(|index| index + 2)
    };
    let Some(insert_at) = open_param else {
        return url.to_string();
    };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let mut cache_busted = url.to_string();
    cache_busted.insert_str(insert_at, &millis.to_string());
    cache_busted
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
    use base64::Engine;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    use std::io::Write;
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
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
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
        let stderr = String::from_utf8(output.stderr).unwrap();
        let start = stderr
            .find("[oddsportal] starting collection pass")
            .expect("missing pass-start output");
        let collector = stderr
            .find("test collector entered")
            .expect("missing collector output");

        assert!(start < collector, "{stderr}");
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
                crate::diagnostics::write(format_args!("{LOG_PREFIX} test collector entered"));
                crate::polymarket::output::write_observation(&serde_json::json!({
                    "provider": "polymarket",
                    "type": "polymarket_odds",
                    "received_at": "2026-06-30T12:00:00.000Z"
                }))
                .unwrap();
                crate::polymarket::output::write_observation(&serde_json::json!({
                    "provider": "polymarket",
                    "type": "polymarket_score",
                    "received_at": "2026-06-30T12:00:00.000Z"
                }))
                .unwrap();
                Ok(OddsCollection::Records(vec![odds_fixture()]))
            },
            || async { Ok(score_fixture()) },
            |_| Ok(()),
            |_| {
                output::write_observation(&serde_json::json!({
                    "provider": "oddsportal",
                    "type": "oddsportal_odds",
                    "received_at": "2026-06-30T12:00:00.000Z"
                }))
            },
            |_| {
                output::write_observation(&serde_json::json!({
                    "provider": "oddsportal",
                    "type": "oddsportal_score",
                    "received_at": "2026-06-30T12:00:00.000Z"
                }))
            },
        )
        .await
        .unwrap();
        crate::diagnostics::write(format_args!("[polymarket] helper diagnostic"));
        crate::diagnostics::write(format_args!("[trade] helper diagnostic"));
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
                async { Ok(OddsCollection::Unavailable) }
            },
            || async { Ok(score_fixture()) },
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
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
                async { Ok(OddsCollection::Unavailable) }
            },
            || async { Ok(score_fixture()) },
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
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
        let gate = Arc::new(tokio::sync::Semaphore::new(0));
        let odds = gated_odds(
            Arc::clone(&odds_calls),
            Arc::clone(&active),
            Arc::clone(&max_active),
            Arc::clone(&gate),
        );
        let scores = gated_scores(
            Arc::clone(&score_calls),
            Arc::clone(&active),
            Arc::clone(&max_active),
            Arc::clone(&gate),
        );

        let task = tokio::spawn(run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(1),
            Some(2),
            test_discovery(),
            odds,
            scores,
            |_| Ok(()),
            |_| Ok(()),
            |_| Ok(()),
        ));
        tokio::task::yield_now().await;
        assert_eq!(odds_calls.load(Ordering::SeqCst), 1);
        assert_eq!(score_calls.load(Ordering::SeqCst), 1);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);

        tokio::time::advance(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;
        assert_eq!(odds_calls.load(Ordering::SeqCst), 1);
        assert_eq!(score_calls.load(Ordering::SeqCst), 1);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);

        gate.add_permits(2);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(odds_calls.load(Ordering::SeqCst), 2);
        assert_eq!(score_calls.load(Ordering::SeqCst), 2);

        gate.add_permits(2);
        tokio::task::yield_now().await;
        task.await.unwrap().unwrap();

        assert_eq!(odds_calls.load(Ordering::SeqCst), 2);
        assert_eq!(score_calls.load(Ordering::SeqCst), 2);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn odds_failure_still_emits_score() {
        let score_outputs = AtomicUsize::new(0);
        let result = run_one_cycle_with(async { Err(anyhow!("odds failed")) }, async {
            Ok(score_fixture())
        })
        .await
        .unwrap();

        handle_cycle_with(
            &result,
            |_| unreachable!("failed odds must not be appended"),
            |_| unreachable!("failed odds must not be emitted"),
            |_| {
                score_outputs.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(score_outputs.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn score_failure_still_appends_and_emits_odds() {
        let odds_appends = AtomicUsize::new(0);
        let odds_outputs = AtomicUsize::new(0);
        let result = run_one_cycle_with(
            async { Ok(OddsCollection::Records(vec![odds_fixture()])) },
            async { Err(anyhow!("score failed")) },
        )
        .await
        .unwrap();

        handle_cycle_with(
            &result,
            |_| {
                odds_appends.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            |_| {
                odds_outputs.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            |_| unreachable!("failed score must not be emitted"),
        )
        .unwrap();

        assert_eq!(odds_appends.load(Ordering::SeqCst), 1);
        assert_eq!(odds_outputs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn unavailable_odds_skips_odds_sinks_and_emits_score() {
        let score_outputs = AtomicUsize::new(0);
        let result = CycleResult {
            odds: Some(OddsCollection::Unavailable),
            score: Some(score_fixture()),
        };

        let status = handle_cycle_with(
            &result,
            |_| unreachable!("unavailable odds must not be appended"),
            |_| unreachable!("unavailable odds must not be emitted"),
            |_| {
                score_outputs.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap();

        assert!(status.odds_succeeded);
        assert!(status.odds_unavailable);
        assert_eq!(score_outputs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn odds_append_failure_short_circuits_grouped_output_and_processes_score() {
        let grouped_outputs = AtomicUsize::new(0);
        let score_outputs = AtomicUsize::new(0);
        let result = CycleResult {
            odds: Some(OddsCollection::Records(vec![odds_fixture()])),
            score: Some(score_fixture()),
        };

        let error = handle_cycle_with(
            &result,
            |_| Err(anyhow!("append failed")),
            |_| {
                grouped_outputs.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            |_| {
                score_outputs.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("append failed"));
        assert_eq!(grouped_outputs.load(Ordering::SeqCst), 0);
        assert_eq!(score_outputs.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn stdout_sink_failures_mark_each_side_failed() {
        let result = CycleResult {
            odds: Some(OddsCollection::Records(vec![odds_fixture()])),
            score: Some(score_fixture()),
        };

        let status = handle_cycle_with(
            &result,
            |_| Ok(()),
            |_| Err(anyhow!("odds stdout failed")),
            |_| Err(anyhow!("score stdout failed")),
        )
        .unwrap_err();

        assert!(status.to_string().contains("odds stdout failed"));
    }

    #[test]
    fn first_sink_failure_is_returned_after_peer_processing() {
        let score_outputs = AtomicUsize::new(0);
        let result = CycleResult {
            odds: Some(OddsCollection::Records(vec![odds_fixture()])),
            score: Some(score_fixture()),
        };

        let error = handle_cycle_with(
            &result,
            |_| Err(anyhow!("first append failure")),
            |_| unreachable!("append failure must short-circuit grouped output"),
            |_| {
                score_outputs.fetch_add(1, Ordering::SeqCst);
                Err(anyhow!("later score failure"))
            },
        )
        .unwrap_err();

        assert_eq!(score_outputs.load(Ordering::SeqCst), 1);
        assert!(error.to_string().contains("first append failure"));
    }

    #[tokio::test]
    async fn oddsportal_client_requests_identity_encoding() {
        let server = TestHttpServer::start(200, "ok").await;
        let client = build_client_with_timeouts(
            &config::Config::default(),
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .unwrap();
        client.get(&server.url).send().await.unwrap();

        assert!(server
            .last_request()
            .to_ascii_lowercase()
            .contains("accept-encoding: identity\r\n"));
    }

    #[tokio::test]
    async fn polling_returns_terminal_sink_error_after_peer_processing() {
        let score_outputs = Arc::new(AtomicUsize::new(0));
        let observed_scores = Arc::clone(&score_outputs);

        let error = run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(60),
            Some(1),
            test_discovery(),
            || async { Ok(OddsCollection::Records(vec![odds_fixture()])) },
            || async { Ok(score_fixture()) },
            |_| Err(anyhow!("terminal append failure")),
            |_| unreachable!("append failure must short-circuit grouped output"),
            move |_| {
                observed_scores.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .await
        .unwrap_err();

        assert_eq!(score_outputs.load(Ordering::SeqCst), 1);
        assert!(error.to_string().contains("terminal append failure"));
    }

    #[test]
    fn terminal_sink_error_does_not_log_cycle_success() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "oddsportal::tests::terminal_sink_error_output_helper",
                "--nocapture",
            ])
            .env("ODDSPORTAL_TERMINAL_SINK_HELPER", "1")
            .output()
            .unwrap();

        assert!(output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("[oddsportal] odds append failed"));
        assert!(!stderr.contains("collection pass succeeded"), "{stderr}");
        assert!(
            !stderr.contains("score collection pass succeeded"),
            "{stderr}"
        );
    }

    #[tokio::test]
    async fn terminal_sink_error_output_helper() {
        if std::env::var_os("ODDSPORTAL_TERMINAL_SINK_HELPER").is_none() {
            return;
        }

        let error = run_poll_loop_with(
            config::Config::default(),
            Duration::from_secs(60),
            Some(1),
            test_discovery(),
            || async { Ok(OddsCollection::Records(vec![odds_fixture()])) },
            || async { Ok(score_fixture()) },
            |_| Err(anyhow!("terminal append failure")),
            |_| unreachable!("append failure must short-circuit grouped output"),
            |_| Ok(()),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("terminal append failure"));
    }

    #[tokio::test]
    async fn missing_score_url_returns_unavailable_without_http() {
        let client = test_client();

        let score = collect_score(
            &client,
            None,
            &test_discovery().0,
            &config::Config::default(),
        )
        .await
        .unwrap();

        assert!(!score.available);
    }

    #[tokio::test]
    async fn score_404_returns_unavailable_after_one_attempt() {
        let server = TestHttpServer::start(404, "").await;
        let client = test_client();

        let score = collect_score(
            &client,
            Some(&server.url),
            &test_discovery().0,
            &config::Config::default(),
        )
        .await
        .unwrap();

        assert!(!score.available);
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn proxy_wrapped_score_404_returns_unavailable_without_decoding() {
        let server = TestHttpServer::start(
            200,
            "URL:/feed/postmatch-score/1-EZmXxG15-yj93f.dat?_= Status: 404",
        )
        .await;
        let client = test_client();

        let score = collect_score(
            &client,
            Some(&server.url),
            &test_discovery().0,
            &config::Config::default(),
        )
        .await
        .unwrap();

        assert!(!score.available);
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn score_error_is_not_retried_within_cycle() {
        let server = TestHttpServer::start(500, "nope").await;
        let client = test_client();

        let error = collect_score(
            &client,
            Some(&server.url),
            &test_discovery().0,
            &config::Config::default(),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("returned error"));
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn stalled_response_is_bounded_by_total_request_timeout() {
        let server = TestHttpServer::start_stalled().await;
        let client = build_client_with_timeouts(
            &config::Config::default(),
            Duration::from_millis(50),
            Duration::from_millis(50),
        )
        .unwrap();

        let result = tokio::time::timeout(
            Duration::from_secs(1),
            get_text_once(&client, &server.url, "stalled test"),
        )
        .await
        .expect("client request timeout must settle the stalled response");

        let error = result.unwrap_err();
        assert!(error.to_string().contains("request failed"), "{error:#}");
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn live_odds_request_uses_h2h_referer() {
        let live = TestHttpServer::start(200, &encoded_odds_payload()).await;
        let h2h_body = format!(
            r#"<Event :data="{{&quot;eventData&quot;:{{
              &quot;isLive&quot;:true,&quot;realLive&quot;:true}},
              &quot;requestLive&quot;:{{
              &quot;url&quot;:&quot;{}&quot;}}}}">
              </Event>"#,
            live.url
        );
        let h2h = TestHttpServer::start(200, &h2h_body).await;
        let client = test_client();
        let mut event = test_discovery().0;
        event.h2h_url = h2h.url.clone();

        let result = collect_odds(&client, &event).await.unwrap();

        assert!(matches!(result, OddsCollection::Records(records) if records.len() == 3));
        assert_eq!(h2h.request_count(), 1);
        assert_eq!(live.request_count(), 1);
        assert!(live
            .last_request()
            .to_ascii_lowercase()
            .contains(&format!("referer: {}", h2h.url).to_ascii_lowercase()));
    }

    #[tokio::test]
    async fn non_live_match_returns_unavailable_without_live_request() {
        let h2h = TestHttpServer::start(
            200,
            r#"<Event :data="{&quot;eventData&quot;:{
              &quot;isLive&quot;:false,&quot;realLive&quot;:false}}">
              </Event>"#,
        )
        .await;
        let client = test_client();
        let mut event = test_discovery().0;
        event.h2h_url = h2h.url.clone();

        let result = collect_odds(&client, &event).await.unwrap();

        assert!(matches!(result, OddsCollection::Unavailable));
        assert_eq!(h2h.request_count(), 1);
    }

    #[tokio::test]
    async fn live_odds_404_returns_unavailable() {
        let live = TestHttpServer::start(404, "not found").await;
        let h2h_body = format!(
            r#"<Event :data="{{&quot;eventData&quot;:{{
              &quot;isLive&quot;:true,&quot;realLive&quot;:true}},
              &quot;requestLive&quot;:{{
              &quot;url&quot;:&quot;{}&quot;}}}}">
              </Event>"#,
            live.url
        );
        let h2h = TestHttpServer::start(200, &h2h_body).await;
        let client = test_client();
        let mut event = test_discovery().0;
        event.h2h_url = h2h.url.clone();

        let result = collect_odds(&client, &event).await.unwrap();

        assert!(matches!(result, OddsCollection::Unavailable));
        assert_eq!(live.request_count(), 1);
    }

    #[tokio::test]
    async fn proxy_wrapped_live_odds_404_returns_unavailable() {
        let live =
            TestHttpServer::start(200, "URL:/feed/live-event/1-EZmXxG15.dat?_= Status: 404").await;
        let h2h_body = format!(
            r#"<Event :data="{{&quot;eventData&quot;:{{
              &quot;isLive&quot;:true,&quot;realLive&quot;:true}},
              &quot;requestLive&quot;:{{
              &quot;url&quot;:&quot;{}&quot;}}}}">
              </Event>"#,
            live.url
        );
        let h2h = TestHttpServer::start(200, &h2h_body).await;
        let client = test_client();
        let mut event = test_discovery().0;
        event.h2h_url = h2h.url.clone();

        let result = collect_odds(&client, &event).await.unwrap();

        assert!(matches!(result, OddsCollection::Unavailable));
    }

    #[test]
    fn cache_busted_url_appends_timestamp_to_open_cache_param() {
        let url = cache_busted_url("https://www.oddsportal.com/match-event/test.dat?_=");

        assert!(url.starts_with("https://www.oddsportal.com/match-event/test.dat?_="));
        assert!(url.len() > "https://www.oddsportal.com/match-event/test.dat?_=".len());
    }

    #[test]
    fn cache_busted_url_fills_open_param_before_geo_query() {
        let url = cache_busted_url("https://www.oddsportal.com/feed/live-event/test.dat?_=&geo=JP");

        assert!(url.starts_with("https://www.oddsportal.com/feed/live-event/test.dat?_="));
        assert!(url.ends_with("&geo=JP"));
        assert!(!url.contains("?_=&geo=JP"));
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

    fn gated_odds(
        calls: Arc<AtomicUsize>,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
        gate: Arc<tokio::sync::Semaphore>,
    ) -> impl FnMut() -> BoxResultFuture<OddsCollection> {
        move || {
            let calls = Arc::clone(&calls);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let gate = Arc::clone(&gate);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                let permit = gate.acquire().await.unwrap();
                permit.forget();
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(OddsCollection::Records(vec![odds_fixture()]))
            })
        }
    }

    fn gated_scores(
        calls: Arc<AtomicUsize>,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
        gate: Arc<tokio::sync::Semaphore>,
    ) -> impl FnMut() -> BoxResultFuture<output::OddsPortalScoreObservation> {
        move || {
            let calls = Arc::clone(&calls);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let gate = Arc::clone(&gate);
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                let permit = gate.acquire().await.unwrap();
                permit.forget();
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

    struct TestHttpServer {
        url: String,
        requests: Arc<AtomicUsize>,
        last_request: Arc<std::sync::Mutex<String>>,
        task: tokio::task::JoinHandle<()>,
    }

    impl TestHttpServer {
        async fn start(status: u16, body: &str) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(AtomicUsize::new(0));
            let observed = Arc::clone(&requests);
            let last_request = Arc::new(std::sync::Mutex::new(String::new()));
            let observed_request = Arc::clone(&last_request);
            let body = body.to_string();
            let task = tokio::spawn(async move {
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    observed.fetch_add(1, Ordering::SeqCst);
                    stream.readable().await.unwrap();
                    let mut request = [0_u8; 4096];
                    let read = stream.try_read(&mut request).unwrap_or(0);
                    *observed_request.lock().unwrap() =
                        String::from_utf8_lossy(&request[..read]).into_owned();
                    let reason = match status {
                        200 => "OK",
                        404 => "Not Found",
                        500 => "Internal Server Error",
                        _ => "Test",
                    };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let mut remaining = response.as_bytes();
                    while !remaining.is_empty() {
                        stream.writable().await.unwrap();
                        match stream.try_write(remaining) {
                            Ok(written) => remaining = &remaining[written..],
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(error) => panic!("test server write failed: {error}"),
                        }
                    }
                }
            });
            Self {
                url: format!("http://{address}/data"),
                requests,
                last_request,
                task,
            }
        }

        async fn start_stalled() -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(AtomicUsize::new(0));
            let observed = Arc::clone(&requests);
            let last_request = Arc::new(std::sync::Mutex::new(String::new()));
            let observed_request = Arc::clone(&last_request);
            let task = tokio::spawn(async move {
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
                    observed.fetch_add(1, Ordering::SeqCst);
                    stream.readable().await.unwrap();
                    let mut request = [0_u8; 4096];
                    let read = stream.try_read(&mut request).unwrap_or(0);
                    *observed_request.lock().unwrap() =
                        String::from_utf8_lossy(&request[..read]).into_owned();
                    std::future::pending::<()>().await;
                }
            });
            Self {
                url: format!("http://{address}/stalled"),
                requests,
                last_request,
                task,
            }
        }

        fn request_count(&self) -> usize {
            self.requests.load(Ordering::SeqCst)
        }

        fn last_request(&self) -> String {
            self.last_request.lock().unwrap().clone()
        }
    }

    impl Drop for TestHttpServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn encoded_odds_payload() -> String {
        let json = r#"{"d":{"encodeventId":"EZmXxG15","oddsdata":{"back":{"0":{"odds":{"16":{"0":"5.50","1":"3.80","2":"1.62"}},"act":{"16":true}}}},"providersNames":{"16":"bet365"}}}"#;
        encode_dat_payload(json)
    }

    fn encode_dat_payload(json: &str) -> String {
        let encoded = utf8_percent_encode(json, NON_ALPHANUMERIC).to_string();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(encoded.as_bytes()).unwrap();
        base64::engine::general_purpose::STANDARD.encode(encoder.finish().unwrap())
    }

    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .http1_only()
            .no_proxy()
            .build()
            .unwrap()
    }
}
