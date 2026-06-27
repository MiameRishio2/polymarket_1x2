use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::polymarket::models::QuoteRecord;

pub struct QuoteLogger {
    file: File,
}

impl QuoteLogger {
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

    pub fn append(&mut self, record: &QuoteRecord) -> Result<()> {
        serde_json::to_writer(&mut self.file, record)?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_parent_directory_and_appends_json_line() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("logs/polymarket_quotes.log");
        let mut logger = QuoteLogger::new(&path).unwrap();

        logger
            .append(&QuoteRecord {
                ts: "2026-06-25T15:00:00Z".to_string(),
                event_slug: "event".to_string(),
                market_slug: "market".to_string(),
                question: "question".to_string(),
                outcome: "Yes".to_string(),
                asset_id: "101".to_string(),
                bid_price: Some("0.62".to_string()),
                bid_size: Some("10".to_string()),
                ask_price: Some("0.63".to_string()),
                ask_size: Some("20".to_string()),
                source: "book".to_string(),
            })
            .unwrap();

        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("\"asset_id\":\"101\""));
        assert!(contents.ends_with('\n'));
    }
}
