use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Result};
use serde::Deserialize;

use crate::oddsportal::models::TargetMatch;

pub const DEFAULT_BASE_URL: &str = "https://www.oddsportal.com";
pub const DEFAULT_TOURNAMENT_URL: &str =
    "https://www.oddsportal.com/football/world/world-championship-2026/";
pub const DEFAULT_HOME_TEAM: &str = "Norway";
pub const DEFAULT_AWAY_TEAM: &str = "France";
pub const DEFAULT_LOG_PATH: &str = "logs/oddsportal_odds.log";
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126 Safari/537.36";
const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 30;

#[derive(Deserialize)]
pub struct FileConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_tournament_url")]
    tournament_url: String,
    #[serde(default = "default_log_path")]
    log_path: PathBuf,
    #[serde(default = "default_poll_interval_seconds")]
    poll_interval_seconds: u64,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tournament_url: default_tournament_url(),
            log_path: default_log_path(),
            poll_interval_seconds: DEFAULT_POLL_INTERVAL_SECONDS,
        }
    }
}

impl FileConfig {
    pub fn into_runtime(
        self,
        proxy_url: Option<String>,
        home_team: String,
        away_team: String,
    ) -> Result<(bool, Config, Duration)> {
        if self.enabled && self.poll_interval_seconds == 0 {
            bail!("oddsportal.poll_interval_seconds must be greater than zero");
        }

        let defaults = Config::default();
        Ok((
            self.enabled,
            Config {
                tournament_url: self.tournament_url,
                home_team,
                away_team,
                proxy_url,
                log_path: self.log_path,
                ..defaults
            },
            Duration::from_secs(self.poll_interval_seconds),
        ))
    }
}

fn default_true() -> bool {
    true
}

fn default_tournament_url() -> String {
    DEFAULT_TOURNAMENT_URL.to_string()
}

fn default_log_path() -> PathBuf {
    PathBuf::from(DEFAULT_LOG_PATH)
}

fn default_poll_interval_seconds() -> u64 {
    DEFAULT_POLL_INTERVAL_SECONDS
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub base_url: String,
    pub tournament_url: String,
    pub home_team: String,
    pub away_team: String,
    pub user_agent: String,
    pub proxy_url: Option<String>,
    pub log_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            tournament_url: DEFAULT_TOURNAMENT_URL.to_string(),
            home_team: DEFAULT_HOME_TEAM.to_string(),
            away_team: DEFAULT_AWAY_TEAM.to_string(),
            user_agent: DEFAULT_USER_AGENT.to_string(),
            proxy_url: None,
            log_path: PathBuf::from(DEFAULT_LOG_PATH),
        }
    }
}

impl Config {
    pub fn target_match(&self) -> TargetMatch {
        TargetMatch {
            home_team: self.home_team.clone(),
            away_team: self.away_team.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn file_config_builds_australia_egypt_runtime() {
        let file: FileConfig = serde_yaml::from_str(
            r#"
enabled: true
tournament_url: https://www.oddsportal.com/football/world/world-championship-2026/
log_path: logs/oddsportal.log
poll_interval_seconds: 30
"#,
        )
        .unwrap();
        let (enabled, config, interval) = file
            .into_runtime(
                Some("http://proxy:7890".into()),
                "Australia".into(),
                "Egypt".into(),
            )
            .unwrap();

        assert!(enabled);
        assert_eq!(config.home_team, "Australia");
        assert_eq!(config.away_team, "Egypt");
        assert_eq!(config.proxy_url.as_deref(), Some("http://proxy:7890"));
        assert_eq!(interval.as_secs(), 30);
    }

    #[test]
    fn zero_poll_interval_is_rejected() {
        let file: FileConfig = serde_yaml::from_str("poll_interval_seconds: 0").unwrap();
        assert_eq!(
            file.into_runtime(None, "Australia".into(), "Egypt".into())
                .unwrap_err()
                .to_string(),
            "oddsportal.poll_interval_seconds must be greater than zero"
        );
    }

    #[test]
    fn default_config_targets_norway_france_world_championship() {
        let config = Config::default();

        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.tournament_url, DEFAULT_TOURNAMENT_URL);
        assert_eq!(config.home_team, "Norway");
        assert_eq!(config.away_team, "France");
        assert_eq!(config.log_path, PathBuf::from(DEFAULT_LOG_PATH));
        assert!(config.proxy_url.is_none());
    }
}
