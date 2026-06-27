use std::fmt;
use std::future::Future;
use std::pin::Pin;

#[cfg(test)]
use std::collections::VecDeque;

use rust_decimal::Decimal;

use crate::polymarket::quotes::QuoteState;

pub const SCENARIO_NAME: &str = "New Zealand vs Belgium";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitOrderIntent {
    pub asset_id: String,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderValidationError {
    EmptyAssetId,
    PriceOutOfRange,
    NonPositiveSize,
}

impl fmt::Display for OrderValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAssetId => formatter.write_str("asset id must not be empty"),
            Self::PriceOutOfRange => {
                formatter.write_str("limit price must be strictly between zero and one")
            }
            Self::NonPositiveSize => formatter.write_str("order size must be positive"),
        }
    }
}

impl std::error::Error for OrderValidationError {}

impl LimitOrderIntent {
    pub fn new(
        asset_id: impl Into<String>,
        side: OrderSide,
        price: Decimal,
        size: Decimal,
    ) -> Result<Self, OrderValidationError> {
        let asset_id = asset_id.into();
        if asset_id.trim().is_empty() {
            return Err(OrderValidationError::EmptyAssetId);
        }
        if price <= Decimal::ZERO || price >= Decimal::ONE {
            return Err(OrderValidationError::PriceOutOfRange);
        }
        if size <= Decimal::ZERO {
            return Err(OrderValidationError::NonPositiveSize);
        }

        Ok(Self {
            asset_id,
            side,
            price,
            size,
        })
    }
}

pub type ExecutorFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ExecutorError>> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutorError(pub String);

impl fmt::Display for ExecutorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ExecutorError {}

pub trait OrderExecutor {
    fn place_limit<'a>(&'a mut self, intent: &'a LimitOrderIntent) -> ExecutorFuture<'a, String>;

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> ExecutorFuture<'a, ()>;
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutorCall {
    Place(LimitOrderIntent),
    Cancel(String),
}

#[cfg(test)]
pub struct MockOrderExecutor {
    placements: VecDeque<Result<String, ExecutorError>>,
    cancellations: VecDeque<Result<(), ExecutorError>>,
    calls: Vec<ExecutorCall>,
}

#[cfg(test)]
impl MockOrderExecutor {
    pub fn scripted(
        placements: impl IntoIterator<Item = Result<String, ExecutorError>>,
        cancellations: impl IntoIterator<Item = Result<(), ExecutorError>>,
    ) -> Self {
        Self {
            placements: placements.into_iter().collect(),
            cancellations: cancellations.into_iter().collect(),
            calls: Vec::new(),
        }
    }

    pub fn calls(&self) -> &[ExecutorCall] {
        &self.calls
    }
}

#[cfg(test)]
impl OrderExecutor for MockOrderExecutor {
    fn place_limit<'a>(&'a mut self, intent: &'a LimitOrderIntent) -> ExecutorFuture<'a, String> {
        self.calls.push(ExecutorCall::Place(intent.clone()));
        let response = self
            .placements
            .pop_front()
            .unwrap_or_else(|| Err(ExecutorError("unscripted placement".to_string())));
        Box::pin(async move { response })
    }

    fn cancel<'a>(&'a mut self, order_id: &'a str) -> ExecutorFuture<'a, ()> {
        self.calls.push(ExecutorCall::Cancel(order_id.to_string()));
        let response = self
            .cancellations
            .pop_front()
            .unwrap_or_else(|| Err(ExecutorError("unscripted cancellation".to_string())));
        Box::pin(async move { response })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderFlowReceipt {
    pub buy_order_id: String,
    pub sell_order_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderFlowError {
    MissingQuote(String),
    InvalidIntent(OrderValidationError),
    BuyFailed(ExecutorError),
    SellFailed {
        buy_order_id: String,
        sell_error: ExecutorError,
        cancellation: Result<(), ExecutorError>,
    },
}

pub async fn run_new_zealand_belgium_flow(
    quotes: &QuoteState,
    asset_id: &str,
    executor: &mut dyn OrderExecutor,
) -> Result<OrderFlowReceipt, OrderFlowError> {
    quotes
        .latest_quote(asset_id)
        .ok_or_else(|| OrderFlowError::MissingQuote(asset_id.to_string()))?;

    let size = Decimal::new(5, 0);
    let buy_price = Decimal::new(1, 2);
    let sell_price = buy_price + Decimal::new(1, 1);
    let buy = LimitOrderIntent::new(asset_id, OrderSide::Buy, buy_price, size)
        .map_err(OrderFlowError::InvalidIntent)?;
    let buy_order_id = executor
        .place_limit(&buy)
        .await
        .map_err(OrderFlowError::BuyFailed)?;

    let sell = LimitOrderIntent::new(asset_id, OrderSide::Sell, sell_price, size)
        .map_err(OrderFlowError::InvalidIntent)?;
    match executor.place_limit(&sell).await {
        Ok(sell_order_id) => Ok(OrderFlowReceipt {
            buy_order_id,
            sell_order_id,
        }),
        Err(sell_error) => {
            let cancellation = executor.cancel(&buy_order_id).await;
            Err(OrderFlowError::SellFailed {
                buy_order_id,
                sell_error,
                cancellation,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polymarket::models::{PriceLevel, TokenMeta};
    use crate::polymarket::quotes::QuoteState;
    use rust_decimal::Decimal;

    fn quote_state() -> QuoteState {
        let mut state = QuoteState::new(
            "new-zealand-vs-belgium",
            vec![TokenMeta {
                market_slug: "new-zealand-vs-belgium".to_string(),
                question: SCENARIO_NAME.to_string(),
                outcome: "New Zealand".to_string(),
                asset_id: "asset-101".to_string(),
            }],
        );
        state.apply_book(
            "asset-101",
            vec![PriceLevel {
                price: "0.01".to_string(),
                size: "5".to_string(),
            }],
            vec![PriceLevel {
                price: "0.02".to_string(),
                size: "5".to_string(),
            }],
            "book",
        );
        state
    }

    #[test]
    fn accepts_required_buy_intent_without_floating_point() {
        let intent = LimitOrderIntent::new(
            "asset-101",
            OrderSide::Buy,
            Decimal::new(1, 2),
            Decimal::new(5, 0),
        )
        .unwrap();

        assert_eq!(intent.price, Decimal::new(1, 2));
        assert_eq!(intent.size, Decimal::new(5, 0));
        assert_eq!(intent.side, OrderSide::Buy);
    }

    #[test]
    fn rejects_empty_asset_invalid_price_and_non_positive_size() {
        assert_eq!(
            LimitOrderIntent::new("", OrderSide::Buy, Decimal::new(1, 2), Decimal::new(5, 0),),
            Err(OrderValidationError::EmptyAssetId)
        );
        assert_eq!(
            LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::ZERO,
                Decimal::new(5, 0),
            ),
            Err(OrderValidationError::PriceOutOfRange)
        );
        assert_eq!(
            LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::ONE,
                Decimal::new(5, 0),
            ),
            Err(OrderValidationError::PriceOutOfRange)
        );
        assert_eq!(
            LimitOrderIntent::new(
                "asset-101",
                OrderSide::Buy,
                Decimal::new(1, 2),
                Decimal::ZERO,
            ),
            Err(OrderValidationError::NonPositiveSize)
        );
    }

    #[tokio::test]
    async fn accepted_buy_is_followed_by_decimal_sell() {
        let state = quote_state();
        let mut executor = MockOrderExecutor::scripted(
            [Ok("sim-buy-1".to_string()), Ok("sim-sell-1".to_string())],
            [],
        );

        let receipt = run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap();

        assert_eq!(receipt.buy_order_id, "sim-buy-1");
        assert_eq!(receipt.sell_order_id, "sim-sell-1");
        assert_eq!(executor.calls().len(), 2);
        let ExecutorCall::Place(sell) = &executor.calls()[1] else {
            panic!("second call must place the sell");
        };
        assert_eq!(sell.side, OrderSide::Sell);
        assert_eq!(sell.price, Decimal::new(11, 2));
        assert_eq!(sell.size, Decimal::new(5, 0));
    }

    #[tokio::test]
    async fn sell_failure_cancels_buy_once_without_retry() {
        let state = quote_state();
        let mut executor = MockOrderExecutor::scripted(
            [
                Ok("sim-buy-1".to_string()),
                Err(ExecutorError("sell rejected".to_string())),
            ],
            [Ok(())],
        );

        let error = run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

        assert_eq!(
            error,
            OrderFlowError::SellFailed {
                buy_order_id: "sim-buy-1".to_string(),
                sell_error: ExecutorError("sell rejected".to_string()),
                cancellation: Ok(()),
            }
        );
        assert_eq!(
            executor.calls(),
            &[
                ExecutorCall::Place(
                    LimitOrderIntent::new(
                        "asset-101",
                        OrderSide::Buy,
                        Decimal::new(1, 2),
                        Decimal::new(5, 0),
                    )
                    .unwrap()
                ),
                ExecutorCall::Place(
                    LimitOrderIntent::new(
                        "asset-101",
                        OrderSide::Sell,
                        Decimal::new(11, 2),
                        Decimal::new(5, 0),
                    )
                    .unwrap()
                ),
                ExecutorCall::Cancel("sim-buy-1".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn buy_failure_stops_after_one_placement() {
        let state = quote_state();
        let mut executor =
            MockOrderExecutor::scripted([Err(ExecutorError("buy rejected".to_string()))], []);

        let error = run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

        assert_eq!(
            error,
            OrderFlowError::BuyFailed(ExecutorError("buy rejected".to_string()))
        );
        assert_eq!(executor.calls().len(), 1);
    }

    #[tokio::test]
    async fn cancellation_failure_is_reported_without_another_call() {
        let state = quote_state();
        let mut executor = MockOrderExecutor::scripted(
            [
                Ok("sim-buy-1".to_string()),
                Err(ExecutorError("sell rejected".to_string())),
            ],
            [Err(ExecutorError("cancel rejected".to_string()))],
        );

        let error = run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

        assert_eq!(
            error,
            OrderFlowError::SellFailed {
                buy_order_id: "sim-buy-1".to_string(),
                sell_error: ExecutorError("sell rejected".to_string()),
                cancellation: Err(ExecutorError("cancel rejected".to_string())),
            }
        );
        assert_eq!(executor.calls().len(), 3);
    }

    #[tokio::test]
    async fn missing_quote_stops_before_executor_calls() {
        let state = QuoteState::new("new-zealand-vs-belgium", Vec::new());
        let mut executor = MockOrderExecutor::scripted([], []);

        let error = run_new_zealand_belgium_flow(&state, "asset-101", &mut executor)
            .await
            .unwrap_err();

        assert_eq!(error, OrderFlowError::MissingQuote("asset-101".to_string()));
        assert!(executor.calls().is_empty());
    }
}
