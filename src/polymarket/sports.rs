use anyhow::Result;
use chrono::{SecondsFormat, Utc};
use futures_util::{Sink, SinkExt, StreamExt};
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;

use crate::polymarket::config::Config;
use crate::polymarket::models::DiscoveredEvent;
use crate::polymarket::output::{write_observation, PolymarketScoreObservation};
use crate::polymarket::ws::connect_ws_via_proxy;
use crate::polymarket::LOG_PREFIX;

#[derive(Deserialize)]
struct SportsMessage {
    slug: String,
    score: Option<String>,
    status: Option<String>,
    period: Option<String>,
    elapsed: Option<String>,
    live: Option<bool>,
    ended: Option<bool>,
    #[serde(default, alias = "last_update")]
    source_updated_at: Option<String>,
}

pub enum SportsAction {
    Pong,
    Observation(PolymarketScoreObservation),
    Ignore,
}

#[derive(Debug)]
enum StreamControl {
    Continue,
    Reconnect,
}

pub fn parse_sports_message(
    text: &str,
    event: &DiscoveredEvent,
    config: &Config,
) -> Result<SportsAction> {
    if text == "ping" {
        return Ok(SportsAction::Pong);
    }

    let message: SportsMessage = serde_json::from_str(text)?;
    if message.slug != event.slug {
        return Ok(SportsAction::Ignore);
    }

    Ok(SportsAction::Observation(PolymarketScoreObservation {
        provider: "polymarket",
        record_type: "polymarket_score",
        received_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        source_updated_at: message.source_updated_at,
        event_slug: event.slug.clone(),
        home_team: config.home_team.clone(),
        away_team: config.away_team.clone(),
        score: message.score,
        status: message.status,
        period: message.period,
        elapsed: message.elapsed,
        live: message.live,
        ended: message.ended,
    }))
}

async fn handle_text_message<W>(
    write: &mut W,
    text: &str,
    event: &DiscoveredEvent,
    config: &Config,
) -> Result<StreamControl>
where
    W: Sink<Message> + Unpin,
    W::Error: std::fmt::Display,
{
    handle_text_message_with(write, text, event, config, write_observation).await
}

async fn handle_text_message_with<W, Emit>(
    write: &mut W,
    text: &str,
    event: &DiscoveredEvent,
    config: &Config,
    mut emit: Emit,
) -> Result<StreamControl>
where
    W: Sink<Message> + Unpin,
    W::Error: std::fmt::Display,
    Emit: FnMut(&PolymarketScoreObservation) -> Result<()>,
{
    match parse_sports_message(text, event, config) {
        Ok(SportsAction::Pong) => {
            if let Err(error) = write.send(Message::Text("pong".into())).await {
                crate::diagnostics::write(format_args!(
                    "{LOG_PREFIX} sports websocket send failed: {error}"
                ));
                return Ok(StreamControl::Reconnect);
            }
        }
        Ok(SportsAction::Observation(record)) => emit(&record)?,
        Ok(SportsAction::Ignore) => {}
        Err(error) => crate::diagnostics::write(format_args!(
            "{LOG_PREFIX} invalid sports update: {error:#}"
        )),
    }
    Ok(StreamControl::Continue)
}

pub async fn run_score_stream(config: Config, event: DiscoveredEvent) -> Result<()> {
    loop {
        crate::diagnostics::write(format_args!("{LOG_PREFIX} connecting sports websocket"));
        match connect_ws_via_proxy(&config.sports_ws_url, &config.proxy_url).await {
            Ok((ws, _response)) => {
                let (mut write, mut read) = ws.split();
                while let Some(message) = read.next().await {
                    match message {
                        Ok(Message::Text(text)) => {
                            match handle_text_message(&mut write, &text, &event, &config).await {
                                Ok(StreamControl::Continue) => {}
                                Ok(StreamControl::Reconnect) => break,
                                Err(error) => {
                                    return Err(error
                                        .context("failed to write Polymarket sports observation"));
                                }
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Ok(_) => {}
                        Err(error) => {
                            crate::diagnostics::write(format_args!(
                                "{LOG_PREFIX} sports websocket read failed: {error:#}"
                            ));
                            break;
                        }
                    }
                }
                crate::diagnostics::write(format_args!(
                    "{LOG_PREFIX} sports websocket disconnected; reconnecting"
                ));
            }
            Err(error) => crate::diagnostics::write(format_args!(
                "{LOG_PREFIX} sports websocket failed: {error:#}; reconnecting"
            )),
        }
        sleep(Duration::from_secs(3)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use tokio_tungstenite::tungstenite::Error as WebSocketError;

    use super::*;
    use crate::polymarket::config::Config;
    use crate::polymarket::models::DiscoveredEvent;

    #[test]
    fn returns_pong_for_heartbeat_and_ignores_other_slugs() {
        assert!(matches!(
            parse_sports_message("ping", &event_fixture(), &config_fixture()).unwrap(),
            SportsAction::Pong
        ));
        let other = r#"{"slug":"fifwc-other","score":"0-0"}"#;
        assert!(matches!(
            parse_sports_message(other, &event_fixture(), &config_fixture()).unwrap(),
            SportsAction::Ignore
        ));
    }

    #[test]
    fn serializes_matching_score_with_source_and_receipt_times() {
        let text = r#"{
          "slug":"fifwc-rsa-can-2026-06-28",
          "score":"1-0",
          "status":"InProgress",
          "period":"1H",
          "elapsed":"32:15",
          "live":true,
          "ended":false,
          "last_update":"2026-06-28T12:00:01.050Z"
        }"#;
        let SportsAction::Observation(record) =
            parse_sports_message(text, &event_fixture(), &config_fixture()).unwrap()
        else {
            panic!("expected observation")
        };

        assert_eq!(record.score.as_deref(), Some("1-0"));
        assert_eq!(
            record.source_updated_at.as_deref(),
            Some("2026-06-28T12:00:01.050Z")
        );
        let json = serde_json::to_value(record).unwrap();
        assert_eq!(json["provider"], "polymarket");
        assert_eq!(json["type"], "polymarket_score");
        assert_eq!(json["event_slug"], "fifwc-rsa-can-2026-06-28");
        assert_eq!(json["home_team"], "South Africa");
        assert_eq!(json["away_team"], "Canada");
        assert!(json["received_at"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
    }

    #[tokio::test]
    async fn pong_send_failure_requests_reconnect_instead_of_escaping() {
        let mut sink = FailingSink;

        let control = handle_text_message(&mut sink, "ping", &event_fixture(), &config_fixture())
            .await
            .unwrap();

        assert!(matches!(control, StreamControl::Reconnect));
    }

    #[tokio::test]
    async fn observation_sink_failure_is_terminal() {
        let mut sink = FailingSink;

        let error = handle_text_message_with(
            &mut sink,
            r#"{"slug":"fifwc-rsa-can-2026-06-28","score":"1-0"}"#,
            &event_fixture(),
            &config_fixture(),
            |_| Err(anyhow::anyhow!("stdout closed")),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("stdout closed"));
    }

    fn event_fixture() -> DiscoveredEvent {
        DiscoveredEvent {
            slug: "fifwc-rsa-can-2026-06-28".into(),
            title: "South Africa vs Canada".into(),
            tokens: Vec::new(),
        }
    }

    fn config_fixture() -> Config {
        Config {
            home_team: "South Africa".into(),
            away_team: "Canada".into(),
            ..Config::default()
        }
    }

    struct FailingSink;

    impl Sink<Message> for FailingSink {
        type Error = WebSocketError;

        fn poll_ready(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, _item: Message) -> Result<(), Self::Error> {
            Err(WebSocketError::ConnectionClosed)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }
}
