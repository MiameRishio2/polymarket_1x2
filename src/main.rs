mod config;
mod oddsportal;
mod polymarket;

use tokio::task::JoinSet;

#[derive(Clone, Copy, Debug)]
enum Provider {
    Polymarket,
    OddsPortal,
}

impl Provider {
    fn prefix(self) -> &'static str {
        match self {
            Self::Polymarket => "[polymarket]",
            Self::OddsPortal => "[oddsportal]",
        }
    }
}

async fn supervise(mut tasks: JoinSet<(Provider, anyhow::Result<()>)>) -> anyhow::Result<()> {
    let mut terminal_errors = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok((provider, Ok(()))) => {
                let error = format!("{} provider stopped unexpectedly", provider.prefix());
                eprintln!("{error}");
                terminal_errors.push(error);
            }
            Ok((provider, Err(error))) => {
                let error = format!("{} provider failed: {error:#}", provider.prefix());
                eprintln!("{error}");
                terminal_errors.push(error);
            }
            Err(error) => {
                let error = format!("provider task failed: {error}");
                eprintln!("{error}");
                terminal_errors.push(error);
            }
        }
    }
    Err(anyhow::anyhow!(
        "all provider tasks stopped: {}",
        terminal_errors.join("; ")
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_crypto_provider();
    let runtime = config::FileConfig::load("config.yaml")?.into_runtime()?;
    tokio::task::LocalSet::new()
        .run_until(async move {
            let mut tasks = JoinSet::new();

            if let Some(runtime) = runtime.polymarket {
                println!(
                    "[polymarket] starting collector for {}",
                    runtime.config.polymarket_url
                );
                tasks.spawn_local(async move {
                    let result = async {
                        let event = polymarket::discovery::discover_event(&runtime.config).await?;
                        polymarket::ws::run_market_stream(runtime.config, runtime.live, event).await
                    }
                    .await;
                    (Provider::Polymarket, result)
                });
            }

            if let Some(runtime) = runtime.oddsportal {
                println!(
                    "[oddsportal] starting collector for {} vs {}",
                    runtime.config.home_team, runtime.config.away_team
                );
                tasks.spawn_local(async move {
                    let result =
                        oddsportal::run_poll_loop(runtime.config, runtime.poll_interval).await;
                    (Provider::OddsPortal, result)
                });
            }

            supervise(tasks).await
        })
        .await
}

fn install_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::anyhow;

    #[test]
    fn crypto_provider_install_is_idempotent() {
        install_crypto_provider();
        install_crypto_provider();

        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }

    #[tokio::test]
    async fn supervisor_waits_for_remaining_provider_after_one_fails() {
        let completed = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&completed);
        let mut tasks = JoinSet::new();
        tasks.spawn(async { (Provider::Polymarket, Err(anyhow!("expected failure"))) });
        tasks.spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            observed.store(true, Ordering::SeqCst);
            (Provider::OddsPortal, Ok(()))
        });

        assert!(supervise(tasks).await.is_err());
        assert!(completed.load(Ordering::SeqCst));
    }
}
