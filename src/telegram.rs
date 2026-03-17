use anyhow::{Context, Result};
use serde::Deserialize;

const BASE_URL: &str = "https://api.telegram.org";

fn bot_url(token: &str, method: &str) -> String {
    format!("{BASE_URL}/bot{token}/{method}")
}

pub async fn send_message(
    token: &str,
    chat_id: &str,
    text: &str,
    thread_id: Option<i64>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML",
    });
    if let Some(tid) = thread_id {
        body["message_thread_id"] = serde_json::json!(tid);
    }

    let resp = client
        .post(bot_url(token, "sendMessage"))
        .json(&body)
        .send()
        .await
        .context("failed to reach Telegram API")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.contains("message thread not found") || body.contains("thread not found") {
            anyhow::bail!("thread_not_found");
        }
        anyhow::bail!("Telegram API error ({}): {}", status, body);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct CreateForumTopicResponse {
    ok: bool,
    result: Option<ForumTopic>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForumTopic {
    message_thread_id: i64,
}

pub async fn create_forum_topic(token: &str, chat_id: &str, name: &str) -> Result<i64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "chat_id": chat_id,
        "name": name,
    });

    let resp: CreateForumTopicResponse = client
        .post(bot_url(token, "createForumTopic"))
        .json(&body)
        .send()
        .await
        .context("failed to reach Telegram API")?
        .json()
        .await
        .context("failed to parse Telegram response")?;

    if !resp.ok {
        let desc = resp.description.unwrap_or_default();
        if desc.contains("not enough rights") || desc.contains("CHAT_ADMIN_REQUIRED") {
            anyhow::bail!(
                "Bot lacks admin rights to create topics.\n\
                 Fix: Enable Topics in group settings first, then add the bot as admin."
            );
        }
        if desc.contains("FORUM_DISABLED") || desc.contains("topics must be enabled") {
            anyhow::bail!(
                "Forum topics are not enabled on this group.\n\
                 Fix: Open your Telegram group → Settings → Topics → Enable."
            );
        }
        anyhow::bail!("Failed to create topic: {}", desc);
    }

    resp.result
        .map(|t| t.message_thread_id)
        .context("Telegram returned ok but no topic data")
}

#[derive(Debug, Deserialize)]
struct GetUpdatesResponse {
    ok: bool,
    result: Option<Vec<Update>>,
}

#[derive(Debug, Deserialize)]
pub struct Update {
    pub message: Option<Message>,
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
