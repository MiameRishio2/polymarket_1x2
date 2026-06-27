mod config;
mod oddsportal;
mod polymarket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_crypto_provider();
    let oddsportal_config = oddsportal::config::Config::default();
    if let Err(error) = oddsportal::collect_once(oddsportal_config).await {
        eprintln!("OddsPortal collection failed: {error:#}");
    }

    let runtime = config::FileConfig::load("config.yaml")?.into_runtime()?;
    let polymarket = runtime
        .polymarket
        .ok_or_else(|| anyhow::anyhow!("polymarket collector must be enabled"))?;
    let config::PolymarketRuntime { config, live } = polymarket;
    let event = polymarket::discovery::discover_event(&config).await?;
    polymarket::ws::run_market_stream(config, live, event).await
}

fn install_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_provider_install_is_idempotent() {
        install_crypto_provider();
        install_crypto_provider();

        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
