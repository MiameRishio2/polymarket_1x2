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
    gamma_host: Option<String>,
    host: Option<String>,
    chain_id: Option<u64>,
    #[serde(default)]
    accounts: Vec<AccountConfig>,
    #[serde(default)]
    trade: TradeConfig,
    #[serde(rename = "match")]
    match_target: MatchSection,
    #[serde(default)]
    polymarket: PolymarketSection,
    #[serde(default)]
    oddsportal: crate::oddsportal::config::FileConfig,
}

#[derive(Deserialize)]
struct PolymarketSection {
    #[serde(default = "default_true")]
    enabled: bool,
    log_path: Option<PathBuf>,
}

impl Default for PolymarketSection {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: None,
        }
    }
}

#[derive(Clone, Deserialize)]
struct MatchSection {
    home_team: String,
    away_team: String,
}

impl MatchSection {
    fn validated(self) -> Result<Self> {
        let home_team = self.home_team.trim().to_string();
        let away_team = self.away_team.trim().to_string();
        if home_team.is_empty() || away_team.is_empty() {
            bail!("match.home_team and match.away_team must not be blank");
        }
        if normalized_team_name(&home_team) == normalized_team_name(&away_team) {
            bail!("match.home_team and match.away_team must identify different teams");
        }
        Ok(Self {
            home_team,
            away_team,
        })
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
        let match_target = self.match_target.validated()?;
        let (oddsportal_enabled, oddsportal_config, oddsportal_interval) =
            self.oddsportal.into_runtime(
                Some(self.proxy.clone()),
                match_target.home_team.clone(),
                match_target.away_team.clone(),
            )?;

        if !self.polymarket.enabled && !oddsportal_enabled {
            bail!("at least one provider collector must be enabled");
        }

        let polymarket = if self.polymarket.enabled {
            let gamma_host = self.gamma_host.ok_or_else(|| {
                anyhow::anyhow!("polymarket.gamma_host is required when polymarket is enabled")
            })?;
            let host = self.host.ok_or_else(|| {
                anyhow::anyhow!("polymarket.host is required when polymarket is enabled")
            })?;
            let chain_id = self.chain_id.ok_or_else(|| {
                anyhow::anyhow!("polymarket.chain_id is required when polymarket is enabled")
            })?;
            let defaults = crate::polymarket::config::Config::default();
            let (config, live) = crate::polymarket::config::build_runtime(RuntimeInput {
                proxy_url: self.proxy.clone(),
                gamma_host,
                clob_host: host,
                chain_id,
                accounts: self.accounts,
                trade: self.trade,
                home_team: match_target.home_team,
                away_team: match_target.away_team,
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

fn normalized_team_name(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"
proxy: http://127.0.0.1:7890
gamma_host: https://gamma-api.polymarket.com
host: https://clob.polymarket.com
chain_id: 137
match:
  home_team: Australia
  away_team: Egypt
polymarket:
  enabled: true
  log_path: logs/aus-egy-polymarket.log
oddsportal:
  enabled: true
  tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
  log_path: logs/aus-egy-oddsportal.log
  poll_interval_seconds: 1
trade:
  enabled: false
  trader_mode: real
  account_mode: real
  market_mode: real
"#;

    #[test]
    fn injects_one_match_pair_into_both_providers() {
        let runtime = FileConfig::parse(BASE).unwrap().into_runtime().unwrap();
        let polymarket = runtime.polymarket.unwrap().config;
        let oddsportal = runtime.oddsportal.unwrap();

        assert_eq!(
            (polymarket.home_team.as_str(), polymarket.away_team.as_str()),
            ("Australia", "Egypt")
        );
        assert_eq!(
            (
                oddsportal.config.home_team.as_str(),
                oddsportal.config.away_team.as_str()
            ),
            ("Australia", "Egypt")
        );
        assert_eq!(oddsportal.poll_interval, Duration::from_secs(1));
    }

    #[test]
    fn rejects_blank_or_equal_match_names() {
        for yaml in [
            BASE.replace("home_team: Australia", "home_team: '  '"),
            BASE.replace("away_team: Egypt", "away_team: australia"),
        ] {
            assert!(FileConfig::parse(&yaml)
                .unwrap()
                .into_runtime()
                .unwrap_err()
                .to_string()
                .contains("match"));
        }
    }

    #[test]
    fn committed_config_targets_south_africa_canada_read_only() {
        let runtime = FileConfig::parse(include_str!("../config.yaml"))
            .unwrap()
            .into_runtime()
            .unwrap();
        let polymarket = runtime.polymarket.unwrap();
        let oddsportal = runtime.oddsportal.unwrap();

        assert_eq!(polymarket.config.home_team, "South Africa");
        assert_eq!(polymarket.config.away_team, "Canada");
        assert_eq!(oddsportal.config.home_team, "South Africa");
        assert_eq!(oddsportal.config.away_team, "Canada");
        assert_eq!(oddsportal.poll_interval, Duration::from_secs(1));
        assert!(polymarket.live.is_none());
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
        let yaml = BASE.replace("poll_interval_seconds: 1", "poll_interval_seconds: 0");

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

    #[test]
    fn oddsportal_only_does_not_require_polymarket_settings() {
        let yaml = r#"
proxy: http://127.0.0.1:7890
match:
  home_team: Australia
  away_team: Egypt
polymarket:
  enabled: false
oddsportal:
  enabled: true
  poll_interval_seconds: 1
"#;

        let runtime = FileConfig::parse(yaml).unwrap().into_runtime().unwrap();

        assert!(runtime.polymarket.is_none());
        assert!(runtime.oddsportal.is_some());
    }

    #[test]
    fn polymarket_only_ignores_invalid_disabled_oddsportal_interval() {
        let yaml = BASE
            .replace(
                "oddsportal:\n  enabled: true",
                "oddsportal:\n  enabled: false",
            )
            .replace("poll_interval_seconds: 1", "poll_interval_seconds: 0");

        let runtime = FileConfig::parse(&yaml).unwrap().into_runtime().unwrap();

        assert!(runtime.polymarket.is_some());
        assert!(runtime.oddsportal.is_none());
    }

    #[test]
    fn enabled_polymarket_requires_each_provider_setting_with_context() {
        for (field, expected) in [
            (
                "gamma_host",
                "polymarket.gamma_host is required when polymarket is enabled",
            ),
            (
                "host",
                "polymarket.host is required when polymarket is enabled",
            ),
            (
                "chain_id",
                "polymarket.chain_id is required when polymarket is enabled",
            ),
        ] {
            let yaml = BASE
                .lines()
                .filter(|line| !line.starts_with(&format!("{field}:")))
                .collect::<Vec<_>>()
                .join("\n");

            assert_eq!(
                FileConfig::parse(&yaml)
                    .unwrap()
                    .into_runtime()
                    .unwrap_err()
                    .to_string(),
                expected
            );
        }
    }
}
