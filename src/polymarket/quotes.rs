use std::collections::HashMap;
use std::str::FromStr;

use chrono::Utc;
use rust_decimal::Decimal;

use crate::polymarket::models::{PriceLevel, QuoteRecord, TokenMeta};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct CurrentQuote {
    bid_price: Option<String>,
    bid_size: Option<String>,
    ask_price: Option<String>,
    ask_size: Option<String>,
}

pub struct QuoteState {
    event_slug: String,
    tokens: HashMap<String, TokenMeta>,
    quotes: HashMap<String, CurrentQuote>,
}

impl QuoteState {
    pub fn new(event_slug: impl Into<String>, tokens: Vec<TokenMeta>) -> Self {
        Self {
            event_slug: event_slug.into(),
            tokens: tokens
                .into_iter()
                .map(|token| (token.asset_id.clone(), token))
                .collect(),
            quotes: HashMap::new(),
        }
    }

    pub fn apply_book(
        &mut self,
        asset_id: &str,
        bids: Vec<PriceLevel>,
        asks: Vec<PriceLevel>,
        source: &str,
    ) -> Option<QuoteRecord> {
        let best_bid = best_level(bids, Side::Bid);
        let best_ask = best_level(asks, Side::Ask);
        let quote = self.quotes.entry(asset_id.to_string()).or_default();

        if let Some(level) = best_bid {
            quote.bid_price = Some(level.price);
            quote.bid_size = Some(level.size);
        }
        if let Some(level) = best_ask {
            quote.ask_price = Some(level.price);
            quote.ask_size = Some(level.size);
        }

        self.record(asset_id, source)
    }

    pub fn apply_best_bid_ask(
        &mut self,
        asset_id: &str,
        bid: Option<String>,
        ask: Option<String>,
        source: &str,
    ) -> Option<QuoteRecord> {
        let quote = self.quotes.entry(asset_id.to_string()).or_default();
        if let Some(bid) = bid {
            quote.bid_price = Some(bid);
        }
        if let Some(ask) = ask {
            quote.ask_price = Some(ask);
        }

        self.record(asset_id, source)
    }

    pub fn apply_side_update(
        &mut self,
        asset_id: &str,
        side: &str,
        price: Option<String>,
        size: Option<String>,
        source: &str,
    ) -> Option<QuoteRecord> {
        let quote = self.quotes.entry(asset_id.to_string()).or_default();
        match side.to_ascii_uppercase().as_str() {
            "BUY" | "BID" => {
                if let Some(price) = price {
                    quote.bid_price = Some(price);
                }
                if let Some(size) = size {
                    quote.bid_size = Some(size);
                }
            }
            "SELL" | "ASK" => {
                if let Some(price) = price {
                    quote.ask_price = Some(price);
                }
                if let Some(size) = size {
                    quote.ask_size = Some(size);
                }
            }
            _ => return None,
        }

        self.record(asset_id, source)
    }

    pub fn latest_quote(&self, asset_id: &str) -> Option<QuoteRecord> {
        self.record(asset_id, "snapshot")
    }

    fn record(&self, asset_id: &str, source: &str) -> Option<QuoteRecord> {
        let token = self.tokens.get(asset_id)?;
        let quote = self.quotes.get(asset_id)?;

        Some(QuoteRecord {
            ts: Utc::now().to_rfc3339(),
            event_slug: self.event_slug.clone(),
            market_slug: token.market_slug.clone(),
            question: token.question.clone(),
            outcome: token.outcome.clone(),
            asset_id: token.asset_id.clone(),
            bid_price: quote.bid_price.clone(),
            bid_size: quote.bid_size.clone(),
            ask_price: quote.ask_price.clone(),
            ask_size: quote.ask_size.clone(),
            source: source.to_string(),
        })
    }
}

#[derive(Clone, Copy)]
enum Side {
    Bid,
    Ask,
}

fn best_level(levels: Vec<PriceLevel>, side: Side) -> Option<PriceLevel> {
    levels
        .into_iter()
        .filter(|level| parse_decimal(&level.price).is_some())
        .fold(None, |best, level| match best {
            None => Some(level),
            Some(current) => {
                let current_price = parse_decimal(&current.price)?;
                let candidate_price = parse_decimal(&level.price)?;
                let candidate_is_better = match side {
                    Side::Bid => candidate_price > current_price,
                    Side::Ask => candidate_price < current_price,
                };
                Some(if candidate_is_better { level } else { current })
            }
        })
}

fn parse_decimal(value: &str) -> Option<Decimal> {
    Decimal::from_str(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> TokenMeta {
        TokenMeta {
            market_slug: "fifwc-ecu-ger-2026-06-25-ger".to_string(),
            question: "Will Germany win on 2026-06-25?".to_string(),
            outcome: "Yes".to_string(),
            asset_id: "101".to_string(),
            result: None,
        }
    }

    #[test]
    fn book_update_selects_best_bid_and_ask_with_sizes() {
        let mut state = QuoteState::new("event", vec![token()]);

        let record = state
            .apply_book(
                "101",
                vec![
                    PriceLevel {
                        price: "0.61".to_string(),
                        size: "10".to_string(),
                    },
                    PriceLevel {
                        price: "0.63".to_string(),
                        size: "20".to_string(),
                    },
                ],
                vec![
                    PriceLevel {
                        price: "0.65".to_string(),
                        size: "30".to_string(),
                    },
                    PriceLevel {
                        price: "0.64".to_string(),
                        size: "40".to_string(),
                    },
                ],
                "book",
            )
            .unwrap();

        assert_eq!(record.bid_price.as_deref(), Some("0.63"));
        assert_eq!(record.bid_size.as_deref(), Some("20"));
        assert_eq!(record.ask_price.as_deref(), Some("0.64"));
        assert_eq!(record.ask_size.as_deref(), Some("40"));
    }

    #[test]
    fn price_only_update_preserves_previous_sizes() {
        let mut state = QuoteState::new("event", vec![token()]);
        state.apply_book(
            "101",
            vec![PriceLevel {
                price: "0.61".to_string(),
                size: "10".to_string(),
            }],
            vec![PriceLevel {
                price: "0.64".to_string(),
                size: "40".to_string(),
            }],
            "book",
        );

        let record = state
            .apply_best_bid_ask(
                "101",
                Some("0.62".to_string()),
                Some("0.63".to_string()),
                "best_bid_ask",
            )
            .unwrap();

        assert_eq!(record.bid_price.as_deref(), Some("0.62"));
        assert_eq!(record.bid_size.as_deref(), Some("10"));
        assert_eq!(record.ask_price.as_deref(), Some("0.63"));
        assert_eq!(record.ask_size.as_deref(), Some("40"));
    }

    #[test]
    fn latest_quote_returns_a_clone_without_mutating_state() {
        let mut state = QuoteState::new("event", vec![token()]);
        state.apply_book(
            "101",
            vec![PriceLevel {
                price: "0.61".to_string(),
                size: "10".to_string(),
            }],
            vec![PriceLevel {
                price: "0.64".to_string(),
                size: "40".to_string(),
            }],
            "book",
        );

        let first = state.latest_quote("101").unwrap();
        let second = state.latest_quote("101").unwrap();

        assert_eq!(first.asset_id, "101");
        assert_eq!(first.bid_price.as_deref(), Some("0.61"));
        assert_eq!(second.ask_price.as_deref(), Some("0.64"));
    }

    #[test]
    fn latest_quote_is_none_before_an_asset_update() {
        let state = QuoteState::new("event", vec![token()]);

        assert!(state.latest_quote("101").is_none());
        assert!(state.latest_quote("unknown").is_none());
    }
}
