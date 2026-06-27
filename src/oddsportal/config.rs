use std::path::PathBuf;

use crate::oddsportal::models::TargetMatch;

pub const DEFAULT_BASE_URL: &str = "https://www.oddsportal.com";
pub const DEFAULT_TOURNAMENT_URL: &str =
    "https://www.oddsportal.com/football/world/world-championship-2026/";
pub const DEFAULT_HOME_TEAM: &str = "Norway";
pub const DEFAULT_AWAY_TEAM: &str = "France";
pub const DEFAULT_LOG_PATH: &str = "logs/oddsportal_odds.log";
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126 Safari/537.36";

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
