//! Minimal HTTP/1.1 JSON POST client for talking to the gostc-rs admin API.
//!
//! Hand-rolled on tokio to keep the dependency tree tiny; the admin server is
//! our own axum instance reached over plain HTTP on the intranet.

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// POST a JSON body to `<api_base><path>` and parse the JSON response.
/// Only plain `http://` targets are supported.
pub async fn post_json(
    api_base: &str,
    path: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value> {
    let host = api_base
        .strip_prefix("http://")
        .ok_or_else(|| anyhow::anyhow!("admin api must be http:// (got {api_base})"))?
        .trim_end_matches('/')
        .to_string();

    let mut stream = TcpStream::connect(&host)
        .await
        .with_context(|| format!("connect admin api {host}"))?;

    let body = serde_json::to_vec(body)?;
    let req = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    stream.write_all(req.as_bytes()).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;

    let split = find(&raw, b"\r\n\r\n").context("malformed http response (no header terminator)")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_ascii_lowercase();
    let mut body_bytes = raw[split + 4..].to_vec();

    let status = head
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("0")
        .to_string();
    if !status.starts_with('2') {
        bail!(
            "admin api returned HTTP {status}: {}",
            String::from_utf8_lossy(&body_bytes)
        );
    }
    if head.contains("transfer-encoding:") && head.contains("chunked") {
        body_bytes = dechunk(&body_bytes)?;
    }
    Ok(serde_json::from_slice(&body_bytes)?)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn dechunk(input: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let rel = find(&input[i..], b"\r\n").context("malformed chunked body")?;
        let nl = i + rel;
        let size_str = String::from_utf8_lossy(&input[i..nl]);
        let size = usize::from_str_radix(size_str.trim().split(';').next().unwrap_or("0"), 16)
            .context("bad chunk size")?;
        i = nl + 2;
        if size == 0 {
            break;
        }
        if i + size > input.len() {
            bail!("truncated chunked body");
        }
        out.extend_from_slice(&input[i..i + size]);
        i += size + 2; // chunk payload + trailing CRLF
    }
    Ok(out)
}
