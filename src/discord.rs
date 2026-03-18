use anyhow::{Context, Result};

pub async fn send_message(webhook_url: &str, content: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "content": content,
    });

    let resp = client
        .post(webhook_url)
        .json(&body)
        .send()
        .await
        .context("failed to reach Discord webhook")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Discord webhook error ({}): {}", status, body);
    }
    Ok(())
}
