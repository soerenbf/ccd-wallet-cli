use anyhow::{Context, Result, bail};
use std::{
    io::{self, Write},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use uuid::Uuid;

pub const MANUAL_REDIRECT_URI: &str = "ConcordiumRedirectToken";

const CALLBACK_PAGE: &str = include_str!("callback_page.html");

pub enum CallbackSession {
    Manual(ManualPasteSession),
    Loopback(LoopbackCallbackSession),
}

impl CallbackSession {
    pub fn redirect_uri(&self) -> &str {
        match self {
            Self::Manual(session) => session.redirect_uri(),
            Self::Loopback(session) => session.redirect_uri(),
        }
    }

    pub async fn receive(self, browser_url: &str) -> Result<String> {
        match self {
            Self::Manual(session) => session.receive(browser_url).await,
            Self::Loopback(session) => session.receive(browser_url).await,
        }
    }
}

pub struct ManualPasteSession;

impl ManualPasteSession {
    pub fn redirect_uri(&self) -> &str {
        MANUAL_REDIRECT_URI
    }

    pub async fn receive(self, browser_url: &str) -> Result<String> {
        println!("Open this URL in your browser:\n\n{browser_url}\n");
        print!("Paste the final redirect URL: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        parse_callback_url(input.trim())
    }
}

pub struct LoopbackCallbackSession {
    listener: TcpListener,
    redirect_uri: String,
    callback_path: String,
    complete_path: String,
    timeout: Duration,
}

impl LoopbackCallbackSession {
    pub async fn bind(timeout: Duration) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind local identity callback receiver to 127.0.0.1")?;
        let addr = listener
            .local_addr()
            .context("failed to read local identity callback address")?;
        let nonce = Uuid::new_v4().simple().to_string();
        let callback_path = format!("/callback/{nonce}");
        let complete_path = format!("{callback_path}/complete");
        let redirect_uri = format!("http://127.0.0.1:{}{callback_path}", addr.port());

        Ok(Self {
            listener,
            redirect_uri,
            callback_path,
            complete_path,
            timeout,
        })
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub async fn receive(self, _browser_url: &str) -> Result<String> {
        timeout(self.timeout, self.receive_inner())
            .await
            .context(
                "timed out waiting for browser callback; retry with --manual-callback if loopback callback is not available",
            )?
    }

    async fn receive_inner(self) -> Result<String> {
        loop {
            let (stream, _) = self
                .listener
                .accept()
                .await
                .context("failed to accept local identity callback connection")?;

            if let Some(result) =
                handle_callback_connection(stream, &self.callback_path, &self.complete_path).await?
            {
                return result;
            }
        }
    }
}

async fn handle_callback_connection(
    mut stream: TcpStream,
    callback_path: &str,
    complete_path: &str,
) -> Result<Option<Result<String>>> {
    let request = read_http_request(&mut stream).await?;

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", path) if path == callback_path => {
            write_response(&mut stream, 200, "text/html; charset=utf-8", CALLBACK_PAGE).await?;
            Ok(None)
        }
        ("POST", path) if path == complete_path => {
            let callback = parse_callback_url(request.body.trim());
            let (status, body) = if callback.is_ok() {
                (200, "Identity callback received. You can close this tab.")
            } else {
                (400, "Identity callback failed. Return to ccd-wallet.")
            };
            write_response(&mut stream, status, "text/plain; charset=utf-8", body).await?;
            Ok(Some(callback))
        }
        _ => {
            write_response(&mut stream, 404, "text/plain; charset=utf-8", "not found").await?;
            Ok(None)
        }
    }
}

struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

async fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .context("failed to read local identity callback request")?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if header_end(&buffer).is_some() {
            break;
        }
        if buffer.len() > 64 * 1024 {
            bail!("local identity callback request was too large");
        }
    }

    let headers_end = header_end(&buffer).context("malformed local identity callback request")?;
    let headers = std::str::from_utf8(&buffer[..headers_end])
        .context("local identity callback request headers were not UTF-8")?;
    let mut lines = headers.lines();
    let request_line = lines
        .next()
        .context("local identity callback request was missing request line")?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .context("local identity callback request was missing method")?
        .to_owned();
    let path = request_parts
        .next()
        .context("local identity callback request was missing path")?
        .to_owned();

    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>())
        .transpose()
        .context("local identity callback request had invalid Content-Length")?
        .unwrap_or(0);

    let body_start = headers_end + 4;
    while buffer.len() < body_start + content_length {
        let read = stream
            .read(&mut chunk)
            .await
            .context("failed to read local identity callback request body")?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > 64 * 1024 {
            bail!("local identity callback request was too large");
        }
    }

    let body_end = std::cmp::min(buffer.len(), body_start + content_length);
    let body = std::str::from_utf8(&buffer[body_start..body_end])
        .context("local identity callback request body was not UTF-8")?
        .to_owned();

    Ok(HttpRequest { method, path, body })
}

fn header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .context("failed to write local identity callback response")
}

pub fn parse_callback_url(input: &str) -> Result<String> {
    if let Some((_, code_uri)) = input.split_once("#code_uri=") {
        if code_uri.is_empty() {
            bail!("callback URL did not contain a code_uri value")
        }
        return Ok(code_uri.to_owned());
    }

    if let Some((_, error)) = input.split_once("#error=") {
        let error = if error.is_empty() {
            "identity issuance failed"
        } else {
            error
        };
        bail!("{error}");
    }

    bail!(
        "unrecognised callback URL; paste the final redirect URL containing #code_uri= or #error="
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_session_uses_sentinel_redirect_uri() {
        let session = ManualPasteSession;
        assert_eq!(session.redirect_uri(), MANUAL_REDIRECT_URI);
    }

    #[test]
    fn parses_code_uri_from_callback_fragment() {
        let parsed =
            parse_callback_url("ConcordiumRedirectToken#code_uri=https://issuer.example/code/123")
                .unwrap();
        assert_eq!(parsed, "https://issuer.example/code/123");
    }

    #[test]
    fn returns_error_fragment_message() {
        let err = parse_callback_url("ConcordiumRedirectToken#error=cancelled").unwrap_err();
        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn rejects_unrecognised_input() {
        let err = parse_callback_url("https://example.com/nope").unwrap_err();
        assert!(err.to_string().contains("unrecognised callback URL"));
    }

    #[tokio::test]
    async fn loopback_callback_page_serving_and_completion() {
        let session = LoopbackCallbackSession::bind(Duration::from_secs(2))
            .await
            .unwrap();
        let redirect_uri = session.redirect_uri().to_owned();
        let complete_uri = format!("{redirect_uri}/complete");
        let handle = tokio::spawn(async move { session.receive("browser-url").await });

        let page = reqwest::get(&redirect_uri).await.unwrap();
        assert_eq!(page.status(), reqwest::StatusCode::OK);
        assert!(
            page.text()
                .await
                .unwrap()
                .contains("Finishing identity issuance")
        );

        let response = reqwest::Client::new()
            .post(complete_uri)
            .body("#code_uri=https://issuer.example/code/123")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let code_uri = handle.await.unwrap().unwrap();
        assert_eq!(code_uri, "https://issuer.example/code/123");
    }

    #[tokio::test]
    async fn loopback_completion_returns_provider_error() {
        let session = LoopbackCallbackSession::bind(Duration::from_secs(2))
            .await
            .unwrap();
        let complete_uri = format!("{}/complete", session.redirect_uri());
        let handle = tokio::spawn(async move { session.receive("browser-url").await });

        let response = reqwest::Client::new()
            .post(complete_uri)
            .body("#error=cancelled")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

        let err = handle.await.unwrap().unwrap_err();
        assert!(err.to_string().contains("cancelled"));
    }

    #[tokio::test]
    async fn loopback_rejects_wrong_path_and_nonce() {
        let session = LoopbackCallbackSession::bind(Duration::from_secs(2))
            .await
            .unwrap();
        let redirect_uri = session.redirect_uri().to_owned();
        let wrong_uri = redirect_uri.replace("/callback/", "/callback/wrong-");
        let complete_uri = format!("{redirect_uri}/complete");
        let handle = tokio::spawn(async move { session.receive("browser-url").await });

        let response = reqwest::get(wrong_uri).await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        reqwest::Client::new()
            .post(complete_uri)
            .body("#code_uri=https://issuer.example/code/123")
            .send()
            .await
            .unwrap();
        assert_eq!(
            handle.await.unwrap().unwrap(),
            "https://issuer.example/code/123"
        );
    }

    #[tokio::test]
    async fn loopback_is_single_use() {
        let session = LoopbackCallbackSession::bind(Duration::from_secs(2))
            .await
            .unwrap();
        let complete_uri = format!("{}/complete", session.redirect_uri());
        let handle = tokio::spawn(async move { session.receive("browser-url").await });

        let first = reqwest::Client::new()
            .post(&complete_uri)
            .body("#code_uri=https://issuer.example/code/123")
            .send()
            .await
            .unwrap();
        assert_eq!(first.status(), reqwest::StatusCode::OK);
        assert_eq!(
            handle.await.unwrap().unwrap(),
            "https://issuer.example/code/123"
        );

        let second = reqwest::Client::new()
            .post(&complete_uri)
            .body("#code_uri=https://issuer.example/code/456")
            .send()
            .await;
        assert!(second.is_err() || second.unwrap().status() != reqwest::StatusCode::OK);
    }
}
