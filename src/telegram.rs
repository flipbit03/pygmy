use anyhow::{Context, Result};
use serde::Deserialize;

const BASE_URL: &str = "https://api.telegram.org";

fn bot_url(token: &str, method: &str) -> String {
    format!("{BASE_URL}/bot{token}/{method}")
}

pub async fn send_message(token: &str, chat_id: &str, text: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML",
    });

    let resp = client
        .post(bot_url(token, "sendMessage"))
        .json(&body)
        .send()
        .await
        .context("failed to reach Telegram API")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Telegram API error ({}): {}", status, body);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct GetUpdatesResponse {
    ok: bool,
    result: Option<Vec<Update>>,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub message: Option<Message>,
    pub channel_post: Option<Message>,
    pub my_chat_member: Option<ChatMemberUpdated>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub chat: Chat,
}

#[derive(Debug, Deserialize)]
pub struct ChatMemberUpdated {
    pub chat: Chat,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub chat_type: String,
}

pub async fn get_updates(token: &str) -> Result<Vec<Update>> {
    let client = reqwest::Client::new();
    let resp: GetUpdatesResponse = client
        .post(bot_url(token, "getUpdates"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .context("failed to reach Telegram API")?
        .json()
        .await
        .context("failed to parse Telegram response")?;

    if !resp.ok {
        anyhow::bail!("Telegram getUpdates failed");
    }

    Ok(resp.result.unwrap_or_default())
}
