use anyhow::{Context, Result};
use concordium_rust_sdk::v2;
use tonic::transport::ClientTlsConfig;

pub async fn connect_v2_client(endpoint: v2::Endpoint) -> Result<v2::Client> {
    let uri = endpoint.uri().clone();
    let mut transport = tonic::transport::Endpoint::from_shared(uri.to_string())
        .context("failed to build tonic transport endpoint")?;

    if uri.scheme_str() == Some("https") {
        let tls = ClientTlsConfig::new().with_enabled_roots();
        transport = transport
            .tls_config(tls)
            .context("failed to configure TLS for Concordium node endpoint")?;
    }

    v2::Client::new(transport)
        .await
        .context("failed to connect to Concordium node")
}

pub const NODE_ENDPOINT_ENV: &str = "CCD_WALLET_NODE_ENDPOINT";

pub fn endpoint_label(endpoint: &v2::Endpoint) -> String {
    normalize_url_string(&endpoint.uri().to_string())
}

pub fn normalize_url_string(url: &str) -> String {
    url.trim_end_matches('/').to_owned()
}
