use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::oddsportal::models::OddsPortalRecord;

pub struct OddsPortalLogger {
    file: File,
}

impl OddsPortalLogger {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create log directory {}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open log file {}", path.display()))?;

        Ok(Self { file })
    }

    pub fn append(&mut self, record: &OddsPortalRecord) -> Result<()> {
        serde_json::to_writer(&mut self.file, record)?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oddsportal::models::OddsPortalRecord;

    #[test]
    fn creates_parent_directory_and_appends_oddsportal_json_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("logs/oddsportal_odds.log");
        let mut logger = OddsPortalLogger::new(&path).unwrap();

        logger.append(&sample_record()).unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("\"provider\":\"oddsportal\""));
        assert!(contents.ends_with('\n'));
    }

    fn sample_record() -> OddsPortalRecord {
        OddsPortalRecord {
            ts: "2026-06-26T12:00:00Z".to_string(),
            provider: "oddsportal".to_string(),
            event_id: "bsJSJ30L".to_string(),
            event_name: "Norway - France".to_string(),
            bookmaker_id: "16".to_string(),
            bookmaker_name: "bet365".to_string(),
            outcome: "1".to_string(),
            decimal_odds: "4.20".to_string(),
            source_url: "https://www.oddsportal.com/match-event/test.dat".to_string(),
        }
    }
}
