use concordium_rust_sdk::v2;

pub const DEFAULT_NODE_ENDPOINT: &str = "http://localhost:20000";
pub const NODE_ENDPOINT_ENV: &str = "CCD_WALLET_NODE_ENDPOINT";

pub fn endpoint_label(endpoint: &v2::Endpoint) -> String {
    endpoint.uri().to_string()
}
