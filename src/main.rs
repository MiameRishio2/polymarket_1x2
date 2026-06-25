mod config;
mod discovery;
mod logging;
mod models;
mod quotes;
mod ws;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_crypto_provider();
    let config = config::Config::default();
    let event = discovery::discover_event(&config).await?;
    ws::run_market_stream(config, event).await
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
