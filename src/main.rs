mod oddsportal;
mod polymarket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_crypto_provider();
    let oddsportal_config = oddsportal::config::Config::default();
    if let Err(error) = oddsportal::collect_once(oddsportal_config).await {
        eprintln!("OddsPortal collection failed: {error:#}");
    }

    let config = polymarket::config::Config::default();
    let event = polymarket::discovery::discover_event(&config).await?;
    polymarket::ws::run_market_stream(config, event).await
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
