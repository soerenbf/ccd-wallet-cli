use anyhow::{Result, bail};
use std::io::{self, Write};

pub trait CallbackReceiver {
    fn receive(&self, browser_url: &str) -> Result<String>;
}

pub struct ManualPasteReceiver;

impl CallbackReceiver for ManualPasteReceiver {
    fn receive(&self, browser_url: &str) -> Result<String> {
        println!("Open this URL in your browser:\n\n{browser_url}\n");
        print!("Paste the final redirect URL: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        parse_callback_url(input.trim())
    }
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
}
