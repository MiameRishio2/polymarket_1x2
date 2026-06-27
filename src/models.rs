use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenMeta {
    pub market_slug: String,
    pub question: String,
    pub outcome: String,
    pub asset_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredEvent {
    pub slug: String,
    pub title: String,
    pub tokens: Vec<TokenMeta>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceLevel {
    pub price: String,
    pub size: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct QuoteRecord {
    pub ts: String,
    pub event_slug: String,
    pub market_slug: String,
    pub question: String,
    pub outcome: String,
    pub asset_id: String,
    pub bid_price: Option<String>,
    pub bid_size: Option<String>,
    pub ask_price: Option<String>,
    pub ask_size: Option<String>,
    pub source: String,
}
