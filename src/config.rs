use std::path::PathBuf;

pub const DEFAULT_POLYMARKET_URL: &str =
    "https://polymarket.com/sports/world-cup/fifwc-nor-fra-2026-06-26";
pub const DEFAULT_PROXY_URL: &str = "http://10.32.110.233:7890";
pub const DEFAULT_CLOB_API_URL: &str = "https://clob.polymarket.com";
pub const DEFAULT_GAMMA_API_URL: &str = "https://gamma-api.polymarket.com";
pub const DEFAULT_GAMMA_EVENT_BASE: &str = "https://gamma-api.polymarket.com/events/slug/";
pub const DEFAULT_MARKET_WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
pub const DEFAULT_LOG_PATH: &str = "logs/polymarket_quotes.log";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub polymarket_url: String,
    pub proxy_url: String,
    pub clob_api_url: String,
    pub gamma_api_url: String,
    pub gamma_event_base: String,
    pub market_ws_url: String,
    pub log_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            polymarket_url: DEFAULT_POLYMARKET_URL.to_string(),
            proxy_url: DEFAULT_PROXY_URL.to_string(),
            clob_api_url: DEFAULT_CLOB_API_URL.to_string(),
            gamma_api_url: DEFAULT_GAMMA_API_URL.to_string(),
            gamma_event_base: DEFAULT_GAMMA_EVENT_BASE.to_string(),
            market_ws_url: DEFAULT_MARKET_WS_URL.to_string(),
            log_path: PathBuf::from(DEFAULT_LOG_PATH),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_requested_values() {
        let config = Config::default();

        assert_eq!(config.polymarket_url, DEFAULT_POLYMARKET_URL);
        assert_eq!(config.proxy_url, DEFAULT_PROXY_URL);
        assert_eq!(config.clob_api_url, DEFAULT_CLOB_API_URL);
        assert_eq!(config.gamma_api_url, DEFAULT_GAMMA_API_URL);
        assert_eq!(config.gamma_event_base, DEFAULT_GAMMA_EVENT_BASE);
        assert_eq!(config.market_ws_url, DEFAULT_MARKET_WS_URL);
        assert_eq!(config.log_path, PathBuf::from(DEFAULT_LOG_PATH));
    }
}
