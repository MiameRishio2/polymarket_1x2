use anyhow::{anyhow, bail, Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::client_async_tls_with_config;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use crate::logging::QuoteLogger;
use crate::models::{DiscoveredEvent, PriceLevel, QuoteRecord};
use crate::quotes::QuoteState;

pub fn subscription_payload(asset_ids: &[String]) -> Value {
    serde_json::json!({
        "assets_ids": asset_ids,
        "type": "market"
    })
}

pub fn parse_market_message(value: &Value, state: &mut QuoteState) -> Vec<QuoteRecord> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .flat_map(|item| parse_market_message(item, state))
            .collect();
    }

    let event_type = string_field(value, &["event_type", "type"]);
    match event_type.as_deref() {
        Some("book") => parse_book(value, state).into_iter().collect(),
        Some("best_bid_ask") => parse_best_bid_ask(value, state).into_iter().collect(),
        Some("price_change") => parse_price_change(value, state),
        _ => Vec::new(),
    }
}

pub async fn run_market_stream(
    config: crate::config::Config,
    event: DiscoveredEvent,
) -> Result<()> {
    let asset_ids: Vec<String> = event
        .tokens
        .iter()
        .map(|token| token.asset_id.clone())
        .collect();
    if asset_ids.is_empty() {
        bail!("no CLOB token ids discovered");
    }

    let mut logger = QuoteLogger::new(&config.log_path)?;
    let mut state = QuoteState::new(event.slug.clone(), event.tokens.clone());
    let clob_client = crate::clob::create_client(&config)?;
    for record in crate::clob::load_initial_orderbooks(&clob_client, &event, &mut state).await? {
        println!(
            "{} {} {} bid={:?}/{:?} ask={:?}/{:?}",
            record.market_slug,
            record.outcome,
            record.asset_id,
            record.bid_price,
            record.bid_size,
            record.ask_price,
            record.ask_size
        );
        logger.append(&record)?;
    }

    let payload = Message::Text(subscription_payload(&asset_ids).to_string().into());

    loop {
        match connect_ws_via_proxy(&config.market_ws_url, &config.proxy_url).await {
            Ok((ws, _response)) => {
                let (mut write, mut read) = ws.split();
                write.send(payload.clone()).await?;
                println!("subscribed to {} Polymarket CLOB tokens", asset_ids.len());

                while let Some(message) = read.next().await {
                    match message? {
                        Message::Text(text) => {
                            let value: Value = serde_json::from_str(&text)?;
                            for record in parse_market_message(&value, &mut state) {
                                println!(
                                    "{} {} {} bid={:?}/{:?} ask={:?}/{:?}",
                                    record.market_slug,
                                    record.outcome,
                                    record.asset_id,
                                    record.bid_price,
                                    record.bid_size,
                                    record.ask_price,
                                    record.ask_size
                                );
                                logger.append(&record)?;
                            }
                        }
                        Message::Ping(payload) => write.send(Message::Pong(payload)).await?,
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            }
            Err(error) => eprintln!("websocket connection failed: {error:#}"),
        }

        sleep(Duration::from_secs(3)).await;
    }
}

fn parse_book(value: &Value, state: &mut QuoteState) -> Option<QuoteRecord> {
    let asset_id = string_field(value, &["asset_id", "token_id", "market"]);
    let bids = parse_levels(value.get("bids"));
    let asks = parse_levels(value.get("asks"));
    state.apply_book(&asset_id?, bids, asks, "book")
}

fn parse_best_bid_ask(value: &Value, state: &mut QuoteState) -> Option<QuoteRecord> {
    let asset_id = string_field(value, &["asset_id", "token_id", "market"]);
    let bid = string_field(value, &["bid", "best_bid"]);
    let ask = string_field(value, &["ask", "best_ask"]);
    state.apply_best_bid_ask(&asset_id?, bid, ask, "best_bid_ask")
}

fn parse_price_change(value: &Value, state: &mut QuoteState) -> Vec<QuoteRecord> {
    if let Some(changes) = value.get("changes").and_then(Value::as_array) {
        return changes
            .iter()
            .filter_map(|change| {
                let asset_id = string_field(change, &["asset_id", "token_id", "market"])
                    .or_else(|| string_field(value, &["asset_id", "token_id", "market"]))?;
                let side = string_field(change, &["side"])?;
                let price = string_field(change, &["price", "best_bid", "best_ask"]);
                let size = string_field(change, &["size"]);
                state.apply_side_update(&asset_id, &side, price, size, "price_change")
            })
            .collect();
    }

    let Some(asset_id) = string_field(value, &["asset_id", "token_id", "market"]) else {
        return Vec::new();
    };
    let Some(side) = string_field(value, &["side"]) else {
        return Vec::new();
    };
    let price = string_field(value, &["price", "best_bid", "best_ask"]);
    let size = string_field(value, &["size"]);
    state
        .apply_side_update(&asset_id, &side, price, size, "price_change")
        .into_iter()
        .collect()
}

fn parse_levels(value: Option<&Value>) -> Vec<PriceLevel> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|level| {
            let price = string_field(level, &["price"]);
            let size = string_field(level, &["size"]);
            Some(PriceLevel {
                price: price?,
                size: size?,
            })
        })
        .collect()
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        value.get(name).and_then(|field| match field {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        })
    })
}

async fn connect_ws_via_proxy(
    ws_url: &str,
    proxy_url: &str,
) -> Result<(
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    tokio_tungstenite::tungstenite::handshake::client::Response,
)> {
    let target = Url::parse(ws_url).context("invalid WebSocket URL")?;
    let proxy = Url::parse(proxy_url).context("invalid proxy URL")?;
    let target_host = target
        .host_str()
        .ok_or_else(|| anyhow!("WebSocket URL missing host"))?;
    let target_port = target.port_or_known_default().unwrap_or(443);
    let proxy_host = proxy
        .host_str()
        .ok_or_else(|| anyhow!("proxy URL missing host"))?;
    let proxy_port = proxy.port_or_known_default().unwrap_or(8080);

    let mut stream = TcpStream::connect((proxy_host, proxy_port))
        .await
        .with_context(|| format!("failed to connect proxy {proxy_host}:{proxy_port}"))?;
    async_http_proxy::http_connect_tokio(&mut stream, target_host, target_port)
        .await
        .with_context(|| format!("HTTP CONNECT to {target_host}:{target_port} failed"))?;

    let (ws, response) = client_async_tls_with_config(ws_url, stream, None, None).await?;
    Ok((ws, response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TokenMeta;

    #[test]
    fn subscription_payload_uses_assets_ids_and_market_type() {
        let payload = subscription_payload(&["101".to_string(), "128".to_string()]);

        assert_eq!(
            payload,
            serde_json::json!({
                "assets_ids": ["101", "128"],
                "type": "market"
            })
        );
    }

    #[test]
    fn parses_book_message_into_quote_record() {
        let mut state = QuoteState::new(
            "event",
            vec![TokenMeta {
                market_slug: "market".to_string(),
                question: "question".to_string(),
                outcome: "Yes".to_string(),
                asset_id: "101".to_string(),
            }],
        );
        let value = serde_json::json!({
            "event_type": "book",
            "asset_id": "101",
            "bids": [{"price": "0.61", "size": "10"}],
            "asks": [{"price": "0.63", "size": "20"}]
        });

        let records = parse_market_message(&value, &mut state);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].bid_price.as_deref(), Some("0.61"));
        assert_eq!(records[0].ask_size.as_deref(), Some("20"));
    }
}
