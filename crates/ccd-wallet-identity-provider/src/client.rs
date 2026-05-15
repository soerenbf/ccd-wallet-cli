use anyhow::{Context, Result, bail};
use concordium_rust_sdk::id::{constants::IpPairing, types::IpInfo};
use reqwest::{Client, StatusCode, Url, redirect::Policy};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum PollResult {
    Pending,
    Done(Value),
    ProviderError(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletProxyMetadata {
    #[serde(rename = "issuanceStart")]
    pub issuance_start: String,
    pub support: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletProxyIpEntry {
    #[serde(rename = "ipInfo")]
    pub ip_info: IpInfo<IpPairing>,
    pub metadata: WalletProxyMetadata,
}

#[derive(Debug, Deserialize)]
struct PollResponse {
    status: String,
    token: Option<Value>,
    detail: Option<String>,
}

pub async fn fetch_wallet_proxy_ip_info(wallet_proxy: &str) -> Result<Vec<WalletProxyIpEntry>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build wallet proxy HTTP client")?;

    let url = Url::parse(wallet_proxy)
        .with_context(|| format!("invalid wallet proxy URL: {wallet_proxy}"))?
        .join("/v1/ip_info")
        .context("failed to build wallet proxy ip_info URL")?;

    let response = client
        .get(url.clone())
        .send()
        .await
        .with_context(|| format!("failed to reach wallet proxy at {url}"))?;

    if response.status() != StatusCode::OK {
        bail!(
            "wallet proxy returned unexpected status {} for {}",
            response.status(),
            url
        );
    }

    response
        .json()
        .await
        .with_context(|| format!("failed to parse wallet proxy response from {url}"))
}

pub fn build_issuance_url(
    base_url: &str,
    redirect_uri: &str,
    id_object_request_json: &str,
) -> Result<Url> {
    let mut current_url = Url::parse(base_url)
        .with_context(|| format!("invalid identity provider URL: {base_url}"))?;
    {
        let mut pairs = current_url.query_pairs_mut();
        pairs.append_pair("scope", "identity");
        pairs.append_pair("response_type", "code");
        pairs.append_pair("redirect_uri", redirect_uri);
        pairs.append_pair("state", id_object_request_json);
    }
    Ok(current_url)
}

pub async fn start_issuance(
    base_url: &str,
    redirect_uri: &str,
    id_object_request_json: &str,
) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(Policy::none())
        .build()
        .context("failed to build identity provider HTTP client")?;

    let mut current_url = build_issuance_url(base_url, redirect_uri, id_object_request_json)?;

    for _ in 0..10 {
        let response = client
            .get(current_url.clone())
            .send()
            .await
            .with_context(|| format!("failed to reach identity provider at {current_url}"))?;

        if !response.status().is_redirection() {
            return Ok(current_url.to_string());
        }

        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .context("identity provider redirect response was missing a Location header")?
            .to_str()
            .context("identity provider redirect Location header was not valid UTF-8")?
            .to_owned();

        if is_final_redirect_location(&location, redirect_uri) {
            return Ok(location);
        }

        current_url = current_url
            .join(&location)
            .with_context(|| format!("failed to resolve redirect location '{location}'"))?;
    }

    bail!("identity provider redirect chain exceeded 10 hops")
}

fn is_final_redirect_location(location: &str, redirect_uri: &str) -> bool {
    location.contains(redirect_uri)
        || location.contains(&percent_encode_uri_component(redirect_uri))
}

fn percent_encode_uri_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub async fn poll_code_uri(code_uri: &str) -> Result<PollResult> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build identity polling HTTP client")?;

    let response = client
        .get(code_uri)
        .send()
        .await
        .with_context(|| format!("failed to poll identity status at {code_uri}"))?;

    if response.status() != StatusCode::OK {
        bail!(
            "identity provider returned unexpected status {} while polling",
            response.status()
        );
    }

    let body: PollResponse = response
        .json()
        .await
        .with_context(|| format!("failed to parse identity status response from {code_uri}"))?;

    match body.status.as_str() {
        "pending" => Ok(PollResult::Pending),
        "done" => Ok(PollResult::Done(body.token.context(
            "identity provider response with status 'done' was missing token",
        )?)),
        "error" => Ok(PollResult::ProviderError(body.detail.unwrap_or_else(
            || "identity provider reported an unspecified error".to_owned(),
        ))),
        other => bail!("identity provider returned unknown status '{other}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn serve_once(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn start_issuance_returns_redirect_location() {
        let url = serve_once(
            "HTTP/1.1 302 Found\r\nLocation: ConcordiumRedirectToken#code_uri=https://issuer.example/code/123\r\nContent-Length: 0\r\n\r\n",
        );

        let location = start_issuance(&url, "ConcordiumRedirectToken", "{}")
            .await
            .unwrap();
        assert_eq!(
            location,
            "ConcordiumRedirectToken#code_uri=https://issuer.example/code/123"
        );
    }

    #[tokio::test]
    async fn start_issuance_stops_at_full_loopback_redirect_uri() {
        let redirect_uri = "http://127.0.0.1:38123/callback/abc123";
        let url = serve_once(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:38123/callback/abc123#code_uri=https://issuer.example/code/123\r\nContent-Length: 0\r\n\r\n",
        );

        let location = start_issuance(&url, redirect_uri, "{}").await.unwrap();
        assert_eq!(
            location,
            "http://127.0.0.1:38123/callback/abc123#code_uri=https://issuer.example/code/123"
        );
    }

    #[tokio::test]
    async fn start_issuance_stops_at_encoded_loopback_redirect_uri() {
        let redirect_uri = "http://127.0.0.1:38123/callback/abc123";
        let url = serve_once(
            "HTTP/1.1 302 Found\r\nLocation: https://issuer.example/finish?redirect_uri=http%3A%2F%2F127.0.0.1%3A38123%2Fcallback%2Fabc123\r\nContent-Length: 0\r\n\r\n",
        );

        let location = start_issuance(&url, redirect_uri, "{}").await.unwrap();
        assert_eq!(
            location,
            "https://issuer.example/finish?redirect_uri=http%3A%2F%2F127.0.0.1%3A38123%2Fcallback%2Fabc123"
        );
    }

    #[tokio::test]
    async fn start_issuance_falls_back_to_original_url_when_not_redirected() {
        let url = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");

        let browser_url = start_issuance(&url, "ConcordiumRedirectToken", "{}")
            .await
            .unwrap();
        assert!(browser_url.starts_with(&url));
        assert!(browser_url.contains("scope=identity"));
        assert!(browser_url.contains("response_type=code"));
        assert!(browser_url.contains("redirect_uri=ConcordiumRedirectToken"));
    }

    #[tokio::test]
    async fn poll_code_uri_parses_done_pending_and_error() {
        let done_url = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 47\r\n\r\n{\"status\":\"done\",\"token\":{\"identityObject\":{}}}",
        );
        match poll_code_uri(&done_url).await.unwrap() {
            PollResult::Done(value) => assert_eq!(value, serde_json::json!({"identityObject": {}})),
            other => panic!("expected done result, got {other:?}"),
        }

        let pending_url = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"status\":\"pending\"}",
        );
        assert_eq!(
            poll_code_uri(&pending_url).await.unwrap(),
            PollResult::Pending
        );

        let error_url = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 34\r\n\r\n{\"status\":\"error\",\"detail\":\"boom\"}",
        );
        assert_eq!(
            poll_code_uri(&error_url).await.unwrap(),
            PollResult::ProviderError("boom".to_owned())
        );
    }
}
