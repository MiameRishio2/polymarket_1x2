use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::polymarket::config::{AccountConfig, RuntimeInput, TradeConfig};

#[derive(Debug)]
pub struct RuntimeConfig {
    pub polymarket: Option<PolymarketRuntime>,
    pub oddsportal: Option<OddsPortalRuntime>,
}

#[derive(Debug)]
pub struct PolymarketRuntime {
    pub config: crate::polymarket::config::Config,
    pub live: Option<crate::polymarket::config::LiveConfig>,
}

#[derive(Debug)]
pub struct OddsPortalRuntime {
    pub config: crate::oddsportal::config::Config,
    pub poll_interval: Duration,
}

#[derive(Deserialize)]
pub struct FileConfig {
    proxy: String,
    gamma_host: String,
    host: String,
    chain_id: u64,
    #[serde(default)]
    accounts: Vec<AccountConfig>,
    #[serde(default)]
    trade: TradeConfig,
    #[serde(default)]
    polymarket: PolymarketSection,
    #[serde(default)]
    oddsportal: crate::oddsportal::config::FileConfig,
}

#[derive(Deserialize)]
struct PolymarketSection {
    #[serde(default = "default_true")]
    enabled: bool,
    url: Option<String>,
    log_path: Option<PathBuf>,
}

impl Default for PolymarketSection {
    fn default() -> Self {
        Self {
            enabled: true,
            url: None,
            log_path: None,
        }
    }
}

impl FileConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("failed to parse {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        serde_yaml::from_str(text).context("failed to parse configuration")
    }

    pub fn into_runtime(self) -> Result<RuntimeConfig> {
        let (oddsportal_enabled, oddsportal_config, oddsportal_interval) =
            self.oddsportal.into_runtime(Some(self.proxy.clone()))?;

        if !self.polymarket.enabled && !oddsportal_enabled {
            bail!("at least one provider collector must be enabled");
        }

        let polymarket = if self.polymarket.enabled {
            let defaults = crate::polymarket::config::Config::default();
            let (config, live) = crate::polymarket::config::build_runtime(RuntimeInput {
                proxy_url: self.proxy.clone(),
                gamma_host: self.gamma_host,
                clob_host: self.host,
                chain_id: self.chain_id,
                accounts: self.accounts,
                trade: self.trade,
                polymarket_url: self.polymarket.url.unwrap_or(defaults.polymarket_url),
                log_path: self.polymarket.log_path.unwrap_or(defaults.log_path),
            })?;
            Some(PolymarketRuntime { config, live })
        } else {
            None
        };

        let oddsportal = if oddsportal_enabled {
            Some(OddsPortalRuntime {
                config: oddsportal_config,
                poll_interval: oddsportal_interval,
            })
        } else {
            None
        };

        Ok(RuntimeConfig {
            polymarket,
            oddsportal,
        })
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
polymarket:
  enabled: true
  url: https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03
  log_path: logs/aus-egy-polymarket.log
oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  home_team: Australia
  away_team: Egypt
  log_path: logs/aus-egy-oddsportal.log
  poll_interval_seconds: 30
trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
"#;

    #[test]
    fn builds_both_provider_runtimes_for_australia_egypt() {
        let runtime = FileConfig::parse(BASE).unwrap().into_runtime().unwrap();
        let polymarket = runtime.polymarket.unwrap();
        let oddsportal = runtime.oddsportal.unwrap();

        assert!(polymarket.live.is_none());
        assert_eq!(
            polymarket.config.polymarket_url,
            "https://polymarket.com/ja/sports/world-cup/fifwc-aus-egy-2026-07-03"
        );
        assert_eq!(oddsportal.config.home_team, "Australia");
        assert_eq!(oddsportal.config.away_team, "Egypt");
        assert_eq!(oddsportal.poll_interval.as_secs(), 30);
    }

    #[test]
    fn provider_enabled_flags_default_true() {
        let yaml = BASE
            .replace("  enabled: true\n", "")
            .replace("  enabled: false\n", "");
        let runtime = FileConfig::parse(&yaml).unwrap().into_runtime().unwrap();

        assert!(runtime.polymarket.is_some());
        assert!(runtime.oddsportal.is_some());
    }

    #[test]
    fn rejects_zero_oddsportal_poll_interval() {
        let yaml = BASE.replace("poll_interval_seconds: 30", "poll_interval_seconds: 0");

        assert_eq!(
            FileConfig::parse(&yaml)
                .unwrap()
                .into_runtime()
                .unwrap_err()
                .to_string(),
            "oddsportal.poll_interval_seconds must be greater than zero"
        );
    }

    #[test]
    fn rejects_runtime_when_both_collectors_are_disabled() {
        let yaml = format!("{BASE}\n")
            .replace(
                "polymarket:\n  enabled: true",
                "polymarket:\n  enabled: false",
            )
            .replace(
                "oddsportal:\n  enabled: true",
                "oddsportal:\n  enabled: false",
            );
        assert_eq!(
            FileConfig::parse(&yaml)
                .unwrap()
                .into_runtime()
                .unwrap_err()
                .to_string(),
            "at least one provider collector must be enabled"
        );
    }
}
