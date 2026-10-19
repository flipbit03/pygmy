use anyhow::{Context, Result};

pub async fn send_message(webhook_url: &str, content: &str) -> Result<()> {
    let client = crate::http::client();
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

/// Upload a single file to the webhook, with `content` as the message text.
pub async fn send_file(
    webhook_url: &str,
    content: &str,
    file_name: &str,
    bytes: Vec<u8>,
) -> Result<()> {
    let client = crate::http::client();
    let payload = serde_json::json!({ "content": content });
    let part = reqwest::multipart::Part::bytes(bytes).file_name(file_name.to_string());
    let form = reqwest::multipart::Form::new()
        .text("payload_json", payload.to_string())
        .part("files[0]", part);

    let resp = client
        .post(webhook_url)
        .multipart(form)
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
