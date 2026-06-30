use anyhow::Result;
use rs_clob_client_v2::types::{Chain, OrderBookParams, OrderBookSummary};
use rs_clob_client_v2::ClobClient;

use crate::polymarket::config::Config;
use crate::polymarket::models::{DiscoveredEvent, PriceLevel, QuoteRecord};
use crate::polymarket::quotes::QuoteState;
use crate::polymarket::LOG_PREFIX;

pub fn create_client(config: &Config) -> Result<ClobClient> {
    Ok(ClobClient::new(
        config.clob_api_url.clone(),
        config.gamma_api_url.clone(),
        Chain::Polygon,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
        Some(config.proxy_url.clone()),
    )?)
}

pub fn orderbook_params(event: &DiscoveredEvent) -> Vec<OrderBookParams> {
    event
        .tokens
        .iter()
        .map(|token| OrderBookParams {
            token_id: token.asset_id.clone(),
            side: None,
        })
        .collect()
}

pub async fn load_initial_orderbooks(
    client: &ClobClient,
    event: &DiscoveredEvent,
    state: &mut QuoteState,
) -> Result<Vec<QuoteRecord>> {
    let mut records = Vec::new();

    for (token, params) in event.tokens.iter().zip(orderbook_params(event)) {
        match client.get_order_book(&params.token_id).await {
            Ok(summary) => {
                if let Some(record) = apply_orderbook_summary(&summary, state) {
                    records.push(record);
                }
            }
            Err(error) => {
                crate::diagnostics::write(format_args!(
                    "{LOG_PREFIX} rs-clob-client-v2 orderbook unavailable for {} {}: {}",
                    token.market_slug, token.outcome, error
                ));
            }
        }
    }

    Ok(records)
}

pub fn apply_orderbook_summary(
    summary: &OrderBookSummary,
    state: &mut QuoteState,
) -> Option<QuoteRecord> {
    let bids = summary
        .bids
        .iter()
        .map(|level| PriceLevel {
            price: level.price.clone(),
            size: level.size.clone(),
        })
        .collect();
    let asks = summary
        .asks
        .iter()
        .map(|level| PriceLevel {
            price: level.price.clone(),
            size: level.size.clone(),
        })
        .collect();

    state.apply_book(&summary.asset_id, bids, asks, "rs-clob-client-v2")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polymarket::models::TokenMeta;
    use rs_clob_client_v2::types::{OrderBookSummary, OrderSummary};

    fn event() -> DiscoveredEvent {
        DiscoveredEvent {
            slug: "event".to_string(),
            title: "title".to_string(),
            tokens: vec![TokenMeta {
                market_slug: "market".to_string(),
                question: "question".to_string(),
                outcome: "Yes".to_string(),
                asset_id: "101".to_string(),
                result: None,
            }],
        }
    }

    #[test]
    fn orderbook_params_use_rs_clob_client_v2_type() {
        let params = orderbook_params(&event());

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].token_id, "101");
        assert!(params[0].side.is_none());
    }

    #[test]
    fn applies_rs_clob_client_v2_orderbook_summary() {
        let mut state = QuoteState::new("event", event().tokens);
        let summary = OrderBookSummary {
            market: "market".to_string(),
            asset_id: "101".to_string(),
            timestamp: "0".to_string(),
            bids: vec![
                OrderSummary {
                    price: "0.60".to_string(),
                    size: "10".to_string(),
                },
                OrderSummary {
                    price: "0.61".to_string(),
                    size: "20".to_string(),
                },
            ],
            asks: vec![OrderSummary {
                price: "0.63".to_string(),
                size: "30".to_string(),
            }],
            min_order_size: "5".to_string(),
            tick_size: "0.001".to_string(),
            neg_risk: true,
            hash: "hash".to_string(),
        };

        let record = apply_orderbook_summary(&summary, &mut state).unwrap();

        assert_eq!(record.asset_id, "101");
        assert_eq!(record.bid_price.as_deref(), Some("0.61"));
        assert_eq!(record.bid_size.as_deref(), Some("20"));
        assert_eq!(record.ask_price.as_deref(), Some("0.63"));
        assert_eq!(record.ask_size.as_deref(), Some("30"));
        assert_eq!(record.source, "rs-clob-client-v2");
    }
}
