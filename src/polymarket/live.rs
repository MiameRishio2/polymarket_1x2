use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

use alloy_signer_local::PrivateKeySigner;
use anyhow::{bail, Result as AnyResult};
use rust_decimal::prelude::ToPrimitive;
use serde_json::Value;

use crate::polymarket::config::LiveConfig;
use crate::polymarket::order::{
    ExecutorError, ExecutorFuture, LimitOrderIntent, OrderExecutor, OrderSide,
};
use rs_clob_client_v2::types::{ApiKeyCreds, Chain, OrderType, UserLimitOrder};
use rs_clob_client_v2::ClobClient;

#[derive(Clone, Debug, PartialEq)]
struct LiveLimitOrder {
    token_id: String,
    price: f64,
    size: f64,
    side: rs_clob_client_v2::types::Side,
}

type TradingFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Value, ExecutorError>> + Send + 'a>>;

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
    fn place_limit<'a>(
        &'a mut self,
        intent: &'a LimitOrderIntent,
    ) -> ExecutorFuture<'a, String> {
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

    Ok(Box::new(LiveOrderExecutor::new(ClobTradingApi {
        client,
    })))
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
    use crate::polymarket::order::{LimitOrderIntent, OrderExecutor, OrderSide};
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::collections::VecDeque;

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
        assert_eq!(executor.api.placed[0].side, rs_clob_client_v2::types::Side::Buy);
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
}
