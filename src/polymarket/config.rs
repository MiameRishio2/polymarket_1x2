use std::fmt;
use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::Deserialize;

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

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    fn require_non_empty(&self, field: &str) -> Result<()> {
        if self.0.trim().is_empty() {
            bail!("{field} must not be empty");
        }
        Ok(())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Deserialize)]
pub struct AccountConfig {
    #[serde(rename = "type")]
    pub account_type: String,
    pub signature_type: Option<u8>,
    pub private_key: SecretString,
    pub api_key: Option<SecretString>,
    pub api_secret: Option<SecretString>,
    pub api_passphrase: Option<SecretString>,
    pub host: Option<String>,
    pub chain_id: Option<u64>,
    pub funder: Option<String>,
}

#[derive(Default, Deserialize)]
pub struct TradeConfig {
    #[serde(default)]
    pub enabled: bool,
    pub trader_mode: Option<String>,
    pub account_mode: Option<String>,
    pub market_mode: Option<String>,
}

impl TradeConfig {
    fn is_live(&self) -> bool {
        self.enabled
            && [&self.trader_mode, &self.account_mode, &self.market_mode]
                .into_iter()
                .all(|mode| mode.as_deref() == Some("real"))
    }
}

#[derive(Clone, Debug)]
pub struct LiveConfig {
    pub signature_type: u8,
    pub private_key: SecretString,
    pub api_key: SecretString,
    pub api_secret: SecretString,
    pub api_passphrase: SecretString,
    pub host: String,
    pub gamma_host: String,
    pub chain_id: u64,
    pub funder: Option<String>,
    pub proxy_url: String,
}

pub struct RuntimeInput {
    pub proxy_url: String,
    pub gamma_host: String,
    pub clob_host: String,
    pub chain_id: u64,
    pub accounts: Vec<AccountConfig>,
    pub trade: TradeConfig,
    pub polymarket_url: String,
    pub log_path: PathBuf,
}

pub fn build_runtime(input: RuntimeInput) -> Result<(Config, Option<LiveConfig>)> {
    let market = Config {
        polymarket_url: input.polymarket_url,
        proxy_url: input.proxy_url.clone(),
        clob_api_url: input.clob_host.clone(),
        gamma_api_url: input.gamma_host.clone(),
        gamma_event_base: format!("{}/events/slug/", input.gamma_host.trim_end_matches('/')),
        log_path: input.log_path,
        ..Config::default()
    };

    if !input.trade.is_live() {
        return Ok((market, None));
    }

    let mut long_accounts = input
        .accounts
        .into_iter()
        .filter(|account| account.account_type == "long");
    let account = long_accounts
        .next()
        .filter(|_| long_accounts.next().is_none())
        .ok_or_else(|| {
            anyhow::anyhow!("live trading requires exactly one account with type long")
        })?;

    account
        .private_key
        .require_non_empty("long-account private_key")?;
    let api_key = required_secret(account.api_key, "long-account api_key")?;
    let api_secret = required_secret(account.api_secret, "long-account api_secret")?;
    let api_passphrase = required_secret(account.api_passphrase, "long-account api_passphrase")?;
    let signature_type = account.signature_type.unwrap_or(0);
    if signature_type > 3 {
        bail!("long-account signature_type must be between 0 and 3");
    }

    Ok((
        market,
        Some(LiveConfig {
            signature_type,
            private_key: account.private_key,
            api_key,
            api_secret,
            api_passphrase,
            host: account.host.unwrap_or(input.clob_host),
            gamma_host: input.gamma_host,
            chain_id: account.chain_id.unwrap_or(input.chain_id),
            funder: account.funder.filter(|value| !value.trim().is_empty()),
            proxy_url: input.proxy_url,
        }),
    ))
}

fn required_secret(value: Option<SecretString>, field: &str) -> Result<SecretString> {
    let value = value.ok_or_else(|| anyhow::anyhow!("{field} must not be empty"))?;
    value.require_non_empty(field)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct RuntimeInputFixture {
        proxy: String,
        gamma_host: String,
        host: String,
        chain_id: u64,
        #[serde(default)]
        accounts: Vec<AccountConfig>,
        #[serde(default)]
        trade: TradeConfig,
    }

    fn runtime_input_from_yaml(yaml: &str) -> RuntimeInput {
        let fixture: RuntimeInputFixture = serde_yaml::from_str(yaml).unwrap();
        let defaults = Config::default();
        RuntimeInput {
            proxy_url: fixture.proxy,
            gamma_host: fixture.gamma_host,
            clob_host: fixture.host,
            chain_id: fixture.chain_id,
            accounts: fixture.accounts,
            trade: fixture.trade,
            polymarket_url: defaults.polymarket_url,
            log_path: defaults.log_path,
        }
    }

    const LIVE_YAML: &str = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
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

    #[test]
    fn all_real_modes_select_unique_long_account_without_exposing_secrets() {
        let (market, live) = build_runtime(runtime_input_from_yaml(LIVE_YAML)).unwrap();
        let live = live.unwrap();
        let debug = format!("{live:?}");

        assert_eq!(market.proxy_url, "http://127.0.0.1:7890");
        assert_eq!(market.clob_api_url, "https://clob.polymarket.com");
        assert_eq!(live.signature_type, 0);
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("test-private"));
        assert!(!debug.contains("test-key"));
        assert!(!debug.contains("test-secret"));
        assert!(!debug.contains("test-passphrase"));
    }

    #[test]
    fn any_non_real_mode_disables_live_account_validation() {
        for field in ["trader_mode", "account_mode", "market_mode"] {
            let yaml = LIVE_YAML
                .replace(&format!("  {field}: real"), &format!("  {field}: mock"))
                .replace("  - name: long-test", "  - name: short-test")
                .replace("    type: long", "    type: short");
            let (_, live) = build_runtime(runtime_input_from_yaml(&yaml)).unwrap();

            assert!(live.is_none(), "{field} must disable live trading");
        }
    }

    #[test]
    fn enabled_live_trading_requires_exactly_one_long_account() {
        let missing =
            runtime_input_from_yaml(&LIVE_YAML.replace("    type: long", "    type: short"));
        assert_eq!(
            build_runtime(missing).unwrap_err().to_string(),
            "live trading requires exactly one account with type long"
        );

        let duplicate_yaml = LIVE_YAML.replace(
            "trade:\n",
            r#"  - name: second-long
    type: long
    signature_type: 1
    private_key: other-private
    api_key: other-key
    api_secret: other-secret
    api_passphrase: other-passphrase
    host: https://clob.polymarket.com
    chain_id: 137
    funder: 0x0000000000000000000000000000000000000001
trade:
"#,
        );
        let duplicate = runtime_input_from_yaml(&duplicate_yaml);
        assert_eq!(
            build_runtime(duplicate).unwrap_err().to_string(),
            "live trading requires exactly one account with type long"
        );
    }

    #[test]
    fn live_account_rejects_missing_credentials_and_invalid_signature_type() {
        let missing_key =
            runtime_input_from_yaml(&LIVE_YAML.replace("api_key: test-key", "api_key: ''"));
        assert_eq!(
            build_runtime(missing_key).unwrap_err().to_string(),
            "long-account api_key must not be empty"
        );

        let invalid_signature = runtime_input_from_yaml(
            &LIVE_YAML.replace("signature_type: null", "signature_type: 4"),
        );
        assert_eq!(
            build_runtime(invalid_signature).unwrap_err().to_string(),
            "long-account signature_type must be between 0 and 3"
        );
    }

    #[test]
    fn missing_or_false_enabled_disables_live_account_validation() {
        for enabled_line in ["", "  enabled: false\n"] {
            let yaml = LIVE_YAML
                .replace("  enabled: true\n", enabled_line)
                .replace("    type: long", "    type: short");
            let input = runtime_input_from_yaml(&yaml);
            let (_, live) = build_runtime(input).unwrap();
            assert!(live.is_none());
        }
    }
}
