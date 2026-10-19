use anyhow::{Context, Result};

use crate::config::NtfyConfig;

pub async fn send_message(config: &NtfyConfig, title: &str, content: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}", config.server.trim_end_matches('/'), config.topic);

    let mut req = client
        .post(&url)
        .header("Markdown", "yes")
        .header("Title", title)
        .body(content.to_string());

    if let Some(token) = &config.token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req.send().await.context("failed to reach ntfy server")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("ntfy error ({}): {}", status, body);
    }
    Ok(())
}

/// Upload a file as an ntfy attachment: PUT with the raw bytes as the body.
/// An empty `message` omits the `Message` header (ntfy then shows the file name).
pub async fn send_file(
    config: &NtfyConfig,
    title: &str,
    message: &str,
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/{}", config.server.trim_end_matches('/'), config.topic);

    let mut req = client
        .put(&url)
        .header("Filename", encode_header(file_name))
        .header("Title", encode_header(title))
        .body(bytes);

    if !message.is_empty() {
        req = req
            .header("Markdown", "yes")
            .header("Message", encode_header(message));
    }

    if let Some(token) = &config.token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req.send().await.context("failed to reach ntfy server")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("ntfy error ({}): {}", status, body);
    }
    Ok(())
}

/// HTTP header values must be visible ASCII. Anything else (UTF-8, newlines)
/// is sent as an RFC 2047 encoded word, which ntfy decodes.
fn encode_header(value: &str) -> String {
    let plain = value.bytes().all(|b| (0x20..0x7f).contains(&b));
    if plain && !value.starts_with("=?") {
        value.to_string()
    } else {
        format!("=?UTF-8?B?{}?=", base64(value.as_bytes()))
    }
}

fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn ascii_header_left_alone() {
        assert_eq!(encode_header("report.pdf"), "report.pdf");
    }

    #[test]
    fn non_ascii_or_multiline_header_encoded() {
        assert_eq!(encode_header("é"), "=?UTF-8?B?w6k=?=");
        assert_eq!(encode_header("a\nb"), "=?UTF-8?B?YQpi?=");
    }
}
