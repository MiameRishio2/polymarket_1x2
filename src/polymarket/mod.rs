pub mod clob;
pub mod config;
pub mod discovery;
pub mod live;
pub mod logging;
pub mod models;
pub mod order;
pub mod output;
pub mod quotes;
pub mod sports;
pub mod ws;

pub(crate) const LOG_PREFIX: &str = "[polymarket]";

pub async fn run(config: config::Config, live: Option<config::LiveConfig>) -> anyhow::Result<()> {
    let event = discovery::discover_event(&config).await?;
    let clob = ws::run_market_stream(config.clone(), live, event.clone());
    let scores = sports::run_score_stream(config, event);
    tokio::try_join!(clob, scores)?;
    Ok(())
}
