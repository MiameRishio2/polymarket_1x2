mod config;
mod oddsportal;
mod polymarket;

use std::collections::HashMap;
use std::future::Future;

use tokio::task::{Id, JoinSet};

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

#[cfg(test)]
fn spawn_provider<F>(
    tasks: &mut JoinSet<(Provider, anyhow::Result<()>)>,
    providers: &mut HashMap<Id, Provider>,
    provider: Provider,
    future: F,
) where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let handle = tasks.spawn(async move { (provider, future.await) });
    providers.insert(handle.id(), provider);
}

fn spawn_local_provider<F>(
    tasks: &mut JoinSet<(Provider, anyhow::Result<()>)>,
    providers: &mut HashMap<Id, Provider>,
    provider: Provider,
    future: F,
) where
    F: Future<Output = anyhow::Result<()>> + 'static,
{
    let handle = tasks.spawn_local(async move { (provider, future.await) });
    providers.insert(handle.id(), provider);
}

async fn supervise(
    mut tasks: JoinSet<(Provider, anyhow::Result<()>)>,
    mut providers: HashMap<Id, Provider>,
) -> anyhow::Result<()> {
    let mut terminal_errors = Vec::new();
    while let Some(joined) = tasks.join_next_with_id().await {
        match joined {
            Ok((task_id, (provider, Ok(())))) => {
                providers.remove(&task_id);
                let error = format!("{} provider stopped unexpectedly", provider.prefix());
                eprintln!("{error}");
                terminal_errors.push(error);
            }
            Ok((task_id, (provider, Err(error)))) => {
                providers.remove(&task_id);
                let error = format!("{} provider failed: {error:#}", provider.prefix());
                eprintln!("{error}");
                terminal_errors.push(error);
            }
            Err(error) => {
                let prefix = providers
                    .remove(&error.id())
                    .map(Provider::prefix)
                    .unwrap_or("[runtime]");
                let error = format!("{prefix} provider task failed: {error}");
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
            let mut providers = HashMap::new();

            if let Some(runtime) = runtime.polymarket {
                eprintln!(
                    "[polymarket] starting collector for {} vs {}",
                    runtime.config.home_team, runtime.config.away_team
                );
                spawn_local_provider(
                    &mut tasks,
                    &mut providers,
                    Provider::Polymarket,
                    polymarket::run(runtime.config, runtime.live),
                );
            }

            if let Some(runtime) = runtime.oddsportal {
                eprintln!(
                    "[oddsportal] starting collector for {} vs {}",
                    runtime.config.home_team, runtime.config.away_team
                );
                spawn_local_provider(
                    &mut tasks,
                    &mut providers,
                    Provider::OddsPortal,
                    oddsportal::run_poll_loop(runtime.config, runtime.poll_interval),
                );
            }

            supervise(tasks, providers).await
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
    use std::process::Command;
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

    #[test]
    fn provider_log_prefixes_are_stable() {
        assert_eq!(polymarket::LOG_PREFIX, "[polymarket]");
        assert_eq!(oddsportal::LOG_PREFIX, "[oddsportal]");
        assert_eq!(polymarket::live::LOG_PREFIX, "[trade]");
    }

    #[test]
    fn observation_helper_keeps_stdout_json_and_diagnostics_on_stderr() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "oddsportal::tests::polling_output_helper",
                "--nocapture",
            ])
            .env("ODDSPORTAL_POLLING_OUTPUT_HELPER", "1")
            .output()
            .unwrap();
        assert!(output.status.success());

        let stdout = String::from_utf8(output.stdout).unwrap();
        let observations = stdout
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(observations.len(), 4, "{stdout}");
        assert!(
            observations.iter().all(|observation| {
                observation["provider"].is_string() && observation["type"].is_string()
            }),
            "{observations:?}"
        );
        for prefix in ["[polymarket]", "[oddsportal]", "[trade]"] {
            assert!(!stdout.contains(prefix), "{stdout}");
        }

        let stderr = String::from_utf8(output.stderr).unwrap();
        for prefix in ["[polymarket]", "[oddsportal]", "[trade]"] {
            assert!(stderr.contains(prefix), "{stderr}");
        }
    }

    #[tokio::test]
    async fn supervisor_waits_for_remaining_provider_after_one_fails() {
        let completed = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&completed);
        let mut tasks = JoinSet::new();
        let mut providers = HashMap::new();
        spawn_provider(&mut tasks, &mut providers, Provider::Polymarket, async {
            Err(anyhow!("expected failure"))
        });
        spawn_provider(
            &mut tasks,
            &mut providers,
            Provider::OddsPortal,
            async move {
                tokio::time::sleep(Duration::from_millis(1)).await;
                observed.store(true, Ordering::SeqCst);
                Ok(())
            },
        );

        assert!(supervise(tasks, providers).await.is_err());
        assert!(completed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn supervisor_attributes_panics_to_the_provider() {
        let mut tasks = JoinSet::new();
        let mut providers = HashMap::new();
        spawn_provider(&mut tasks, &mut providers, Provider::OddsPortal, async {
            panic!("expected provider panic")
        });

        let error = supervise(tasks, providers).await.unwrap_err().to_string();

        assert!(error.contains("[oddsportal]"), "{error}");
    }
}
