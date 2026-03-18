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
