use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

use alloy_signer_local::PrivateKeySigner;
use anyhow::{bail, Result as AnyResult};
use rust_decimal::prelude::ToPrimitive;
use serde_json::Value;

use crate::polymarket::config::LiveConfig;
use crate::polymarket::models::DiscoveredEvent;
use crate::polymarket::order::{
    run_new_zealand_belgium_flow, ExecutorError, ExecutorFuture, LimitOrderIntent, OrderExecutor,
    OrderFlowReceipt, OrderSide,
};
use crate::polymarket::quotes::QuoteState;
use rs_clob_client_v2::types::{ApiKeyCreds, Chain, OrderType, UserLimitOrder};
use rs_clob_client_v2::ClobClient;

pub(crate) const LOG_PREFIX: &str = "[trade]";

#[derive(Clone, Debug, PartialEq)]
struct LiveLimitOrder {
    token_id: String,
    price: f64,
    size: f64,
    side: rs_clob_client_v2::types::Side,
}

type TradingFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, ExecutorError>> + Send + 'a>>;

trait TradingApi {
    fn place<'a>(&'a mut self, order: LiveLimitOrder) -> TradingFuture<'a>;
    fn cancel<'a>(&'a mut self, order_id: &'a str) -> TradingFuture<'a>;
}

struct ClobTradingApi {
    client: ClobClient,
}

impl TradingApi for ClobTradingApi {
    fn place<'a>(&'a mut self, order: LiveLimitOrder) -> TradingFuture<'a> {
        Box::pin(async move {
            let order = UserLimitOrder {
                token_id: order.token_id,
                price: order.price,
                size: order.size,
                side: order.side,
                expiration: None,
                timestamp: None,
                metadata: None,
                builder: None,
            };
            self.client
                .create_and_post_limit_order(&order, None, OrderType::Gtc)
                .await
                .map_err(|_| ExecutorError("CLOB limit order request failed".into()))
        })
    }

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> TradingFuture<'a> {
        Box::pin(async move {
            self.client
                .cancel_order(order_id)
                .await
                .map_err(|_| ExecutorError("CLOB cancellation request failed".into()))
        })
    }
}

struct LiveOrderExecutor<A> {
    api: A,
}

impl<A> LiveOrderExecutor<A> {
    fn new(api: A) -> Self {
        Self { api }
    }
}

impl<A: TradingApi + Send> OrderExecutor for LiveOrderExecutor<A> {
    fn place_limit<'a>(&'a mut self, intent: &'a LimitOrderIntent) -> ExecutorFuture<'a, String> {
        Box::pin(async move {
            let response = self.api.place(map_intent(intent)?).await?;
            accepted_order_id(&response)
        })
    }

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> ExecutorFuture<'a, ()> {
        Box::pin(async move {
            let response = self.api.cancel(order_id).await?;
            cancellation_confirmed(&response, order_id)
        })
    }
}

pub fn create_live_executor(config: &LiveConfig) -> AnyResult<Box<dyn OrderExecutor>> {
    let chain = match config.chain_id {
        137 => Chain::Polygon,
        80002 => Chain::Amoy,
        _ => bail!("unsupported long-account chain id"),
    };
    let signer = PrivateKeySigner::from_str(config.private_key.expose())
        .map_err(|_| anyhow::anyhow!("invalid long-account private key"))?;
    let credentials = ApiKeyCreds {
        key: config.api_key.expose().to_owned(),
        secret: config.api_secret.expose().to_owned(),
        passphrase: config.api_passphrase.expose().to_owned(),
    };
    let client = ClobClient::new(
        config.host.clone(),
        config.gamma_host.clone(),
        chain,
        Some(signer),
        Some(credentials),
        Some(config.signature_type),
        config.funder.clone(),
        None,
        false,
        None,
        Some(config.proxy_url.clone()),
    )
    .map_err(|_| anyhow::anyhow!("failed to initialize authenticated CLOB client"))?;

    Ok(Box::new(LiveOrderExecutor::new(ClobTradingApi { client })))
}

pub async fn run_fixed_live_flow(
    live: Option<&LiveConfig>,
    event: &DiscoveredEvent,
    state: &QuoteState,
) -> AnyResult<Option<OrderFlowReceipt>> {
    maybe_run_fixed_live_flow(live, event, state, create_live_executor).await
}

async fn maybe_run_fixed_live_flow<F>(
    live: Option<&LiveConfig>,
    event: &DiscoveredEvent,
    state: &QuoteState,
    create_executor: F,
) -> AnyResult<Option<OrderFlowReceipt>>
where
    F: FnOnce(&LiveConfig) -> AnyResult<Box<dyn OrderExecutor>>,
{
    let Some(live) = live else {
        return Ok(None);
    };
    let token = event
        .tokens
        .first()
        .ok_or_else(|| anyhow::anyhow!("live trading event has no tokens"))?;
    if state.latest_quote(&token.asset_id).is_none() {
        bail!("missing initial quote for live asset {}", token.asset_id);
    }

    let mut executor = create_executor(live)?;
    let receipt = run_new_zealand_belgium_flow(state, &token.asset_id, executor.as_mut())
        .await
        .map_err(|_| anyhow::anyhow!("fixed live order flow failed"))?;
    Ok(Some(receipt))
}

fn map_intent(intent: &LimitOrderIntent) -> Result<LiveLimitOrder, ExecutorError> {
    let price = intent
        .price
        .to_f64()
        .ok_or_else(|| ExecutorError("limit price cannot be represented by CLOB client".into()))?;
    let size = intent
        .size
        .to_f64()
        .ok_or_else(|| ExecutorError("limit size cannot be represented by CLOB client".into()))?;
    let side = match intent.side {
        OrderSide::Buy => rs_clob_client_v2::types::Side::Buy,
        OrderSide::Sell => rs_clob_client_v2::types::Side::Sell,
    };

    Ok(LiveLimitOrder {
        token_id: intent.asset_id.clone(),
        price,
        size,
        side,
    })
}

fn accepted_order_id(value: &Value) -> Result<String, ExecutorError> {
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(ExecutorError("CLOB placement rejected".into()));
    }

    value
        .get("orderID")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|order_id| !order_id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ExecutorError("CLOB placement response missing order ID".into()))
}

fn cancellation_confirmed(value: &Value, order_id: &str) -> Result<(), ExecutorError> {
    let canceled = value
        .get("canceled")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(order_id)));
    let rejected = value
        .get("not_canceled")
        .and_then(Value::as_object)
        .is_some_and(|items| items.contains_key(order_id));

    if canceled && !rejected {
        Ok(())
    } else {
        Err(ExecutorError("CLOB cancellation not confirmed".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileConfig;
    use crate::polymarket::models::{DiscoveredEvent, PriceLevel, TokenMeta};
    use crate::polymarket::order::MockOrderExecutor;
    use crate::polymarket::order::{LimitOrderIntent, OrderExecutor, OrderSide};
    use crate::polymarket::quotes::QuoteState;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn trade_log_prefix_is_stable() {
        assert_eq!(LOG_PREFIX, "[trade]");
    }

    #[derive(Default)]
    struct FakeApi {
        placement_responses: VecDeque<Result<Value, ExecutorError>>,
        cancellation_responses: VecDeque<Result<Value, ExecutorError>>,
        placed: Vec<LiveLimitOrder>,
        canceled: Vec<String>,
    }

    impl TradingApi for FakeApi {
        fn place<'a>(&'a mut self, order: LiveLimitOrder) -> TradingFuture<'a> {
            self.placed.push(order);
            let result = self
                .placement_responses
                .pop_front()
                .unwrap_or_else(|| Err(ExecutorError("unscripted placement".into())));
            Box::pin(async move { result })
        }

        fn cancel<'a>(&'a mut self, order_id: &'a str) -> TradingFuture<'a> {
            self.canceled.push(order_id.to_owned());
            let result = self
                .cancellation_responses
                .pop_front()
                .unwrap_or_else(|| Err(ExecutorError("unscripted cancellation".into())));
            Box::pin(async move { result })
        }
    }

    #[test]
    fn maps_fixed_decimal_buy_and_sell_to_sdk_orders() {
        let buy = LimitOrderIntent::new(
            "101",
            OrderSide::Buy,
            Decimal::new(1, 2),
            Decimal::new(5, 0),
        )
        .unwrap();
        let sell = LimitOrderIntent::new(
            "101",
            OrderSide::Sell,
            Decimal::new(11, 2),
            Decimal::new(5, 0),
        )
        .unwrap();

        let mapped_buy = map_intent(&buy).unwrap();
        let mapped_sell = map_intent(&sell).unwrap();

        assert_eq!(mapped_buy.token_id, "101");
        assert_eq!(mapped_buy.price, 0.01);
        assert_eq!(mapped_buy.size, 5.0);
        assert_eq!(mapped_buy.side, rs_clob_client_v2::types::Side::Buy);
        assert_eq!(mapped_sell.price, 0.11);
        assert_eq!(mapped_sell.size, 5.0);
        assert_eq!(mapped_sell.side, rs_clob_client_v2::types::Side::Sell);
    }

    #[test]
    fn placement_requires_success_and_non_empty_order_id() {
        assert_eq!(
            accepted_order_id(&json!({"success": true, "orderID": "live-1"})).unwrap(),
            "live-1"
        );
        assert_eq!(
            accepted_order_id(&json!({"success": false, "orderID": "rejected"}))
                .unwrap_err()
                .to_string(),
            "CLOB placement rejected"
        );
        assert_eq!(
            accepted_order_id(&json!({"success": true, "orderID": ""}))
                .unwrap_err()
                .to_string(),
            "CLOB placement response missing order ID"
        );
        assert_eq!(
            accepted_order_id(&json!({"success": true}))
                .unwrap_err()
                .to_string(),
            "CLOB placement response missing order ID"
        );
    }

    #[test]
    fn cancellation_requires_requested_id_in_canceled_only() {
        cancellation_confirmed(
            &json!({"canceled": ["live-1"], "not_canceled": {}}),
            "live-1",
        )
        .unwrap();

        assert_eq!(
            cancellation_confirmed(
                &json!({"canceled": [], "not_canceled": {"live-1": "not found"}}),
                "live-1",
            )
            .unwrap_err()
            .to_string(),
            "CLOB cancellation not confirmed"
        );
        assert_eq!(
            cancellation_confirmed(
                &json!({"canceled": ["other"], "not_canceled": {}}),
                "live-1",
            )
            .unwrap_err()
            .to_string(),
            "CLOB cancellation not confirmed"
        );
    }

    #[tokio::test]
    async fn live_executor_places_mapped_order_and_extracts_id() {
        let mut api = FakeApi::default();
        api.placement_responses
            .push_back(Ok(json!({"success": true, "orderID": "live-1"})));
        let mut executor = LiveOrderExecutor::new(api);
        let intent = LimitOrderIntent::new(
            "101",
            OrderSide::Buy,
            Decimal::new(1, 2),
            Decimal::new(5, 0),
        )
        .unwrap();

        let order_id = executor.place_limit(&intent).await.unwrap();

        assert_eq!(order_id, "live-1");
        assert_eq!(executor.api.placed.len(), 1);
        assert_eq!(
            executor.api.placed[0].side,
            rs_clob_client_v2::types::Side::Buy
        );
    }

    #[tokio::test]
    async fn live_executor_requires_confirmed_cancellation() {
        let mut api = FakeApi::default();
        api.cancellation_responses.push_back(Ok(
            json!({"canceled": [], "not_canceled": {"live-1": "not found"}}),
        ));
        let mut executor = LiveOrderExecutor::new(api);

        let error = executor.cancel("live-1").await.unwrap_err();

        assert_eq!(error.to_string(), "CLOB cancellation not confirmed");
        assert_eq!(executor.api.canceled, ["live-1"]);
    }

    fn event() -> DiscoveredEvent {
        DiscoveredEvent {
            slug: "test-event".into(),
            title: "Test Event".into(),
            tokens: vec![TokenMeta {
                market_slug: "test-market".into(),
                question: "Test?".into(),
                outcome: "First".into(),
                asset_id: "101".into(),
                result: None,
            }],
        }
    }

    fn quoted_state(event: &DiscoveredEvent) -> QuoteState {
        let mut state = QuoteState::new(event.slug.clone(), event.tokens.clone());
        state.apply_book(
            "101",
            vec![PriceLevel {
                price: "0.01".into(),
                size: "5".into(),
            }],
            vec![PriceLevel {
                price: "0.02".into(),
                size: "5".into(),
            }],
            "test",
        );
        state
    }

    fn live_config() -> LiveConfig {
        let yaml = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
match:
  home_team: Australia
  away_team: Egypt
accounts:
  - name: long-test
    type: long
    signature_type: null
    private_key: test-private
    api_key: test-key
    api_secret: test-secret
    api_passphrase: test-passphrase
    host: https://clob.polymarket.com
    chain_id: 137
    funder: null
trade:
  enabled: true
  trader_mode: real
  account_mode: real
  market_mode: real
"#;
        let file: FileConfig = serde_yaml::from_str(yaml).unwrap();
        file.into_runtime()
            .unwrap()
            .polymarket
            .unwrap()
            .live
            .unwrap()
    }

    #[tokio::test]
    async fn disabled_mode_does_not_create_executor() {
        let event = event();
        let state = quoted_state(&event);
        let calls = Arc::new(AtomicUsize::new(0));
        let factory_calls = Arc::clone(&calls);

        let receipt = maybe_run_fixed_live_flow(None, &event, &state, move |_| {
            factory_calls.fetch_add(1, Ordering::SeqCst);
            unreachable!("disabled mode must not create an executor")
        })
        .await
        .unwrap();

        assert!(receipt.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn empty_event_and_missing_quote_fail_before_executor_creation() {
        let live = live_config();
        let calls = Arc::new(AtomicUsize::new(0));
        let empty_event = DiscoveredEvent {
            slug: "empty".into(),
            title: "Empty".into(),
            tokens: Vec::new(),
        };
        let empty_state = QuoteState::new("empty", Vec::new());
        let factory_calls = Arc::clone(&calls);
        let empty_error =
            maybe_run_fixed_live_flow(Some(&live), &empty_event, &empty_state, move |_| {
                factory_calls.fetch_add(1, Ordering::SeqCst);
                unreachable!("empty event must fail before executor creation")
            })
            .await
            .unwrap_err();
        assert_eq!(empty_error.to_string(), "live trading event has no tokens");

        let event = event();
        let no_quote_state = QuoteState::new(event.slug.clone(), event.tokens.clone());
        let factory_calls = Arc::clone(&calls);
        let quote_error =
            maybe_run_fixed_live_flow(Some(&live), &event, &no_quote_state, move |_| {
                factory_calls.fetch_add(1, Ordering::SeqCst);
                unreachable!("missing quote must fail before executor creation")
            })
            .await
            .unwrap_err();
        assert_eq!(
            quote_error.to_string(),
            "missing initial quote for live asset 101"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn enabled_mode_runs_fixed_flow_once() {
        let live = live_config();
        let event = event();
        let state = quoted_state(&event);
        let calls = Arc::new(AtomicUsize::new(0));
        let factory_calls = Arc::clone(&calls);

        let receipt = maybe_run_fixed_live_flow(Some(&live), &event, &state, move |_| {
            factory_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(MockOrderExecutor::scripted(
                [Ok("live-buy".into()), Ok("live-sell".into())],
                [],
            )) as Box<dyn OrderExecutor>)
        })
        .await
        .unwrap()
        .unwrap();

        assert_eq!(receipt.buy_order_id, "live-buy");
        assert_eq!(receipt.sell_order_id, "live-sell");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
