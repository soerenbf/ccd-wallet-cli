use concordium_rust_sdk::v2;

pub const NODE_ENDPOINT_ENV: &str = "CCD_WALLET_NODE_ENDPOINT";

pub fn endpoint_label(endpoint: &v2::Endpoint) -> String {
    endpoint.uri().to_string()
}
