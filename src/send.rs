use anyhow::Result;

use crate::config;
use crate::discord;
use crate::markdown;
use crate::ntfy;
use crate::telegram;

const TELEGRAM_MAX_LENGTH: usize = 4096;
const DISCORD_MAX_LENGTH: usize = 2000;
const NTFY_MAX_LENGTH: usize = 4096;

pub async fn send(topic: &str, message: &str) -> Result<()> {
    let config = config::load_config()?;

    let telegram_enabled = config.telegram.as_ref().is_some_and(|t| t.enabled);
    let discord_enabled = config.discord_webhook.as_ref().is_some_and(|d| d.enabled);
    let ntfy_enabled = config.ntfy.as_ref().is_some_and(|n| n.enabled);

    if !telegram_enabled && !discord_enabled && !ntfy_enabled {
        anyhow::bail!(
            "No notification backends are enabled.\n\
             Run `pygmy init <backend>` to set one up,\n\
             or `pygmy enable <backend>` to re-enable a configured one."
        );
    }

    let mut errors: Vec<String> = Vec::new();
    let mut any_success = false;

    if let Some(tg) = &config.telegram
        && tg.enabled
    {
        match send_telegram(tg, topic, message).await {
            Ok(()) => any_success = true,
            Err(e) => errors.push(format!("telegram: {e:#}")),
        }
    }

    if let Some(dw) = &config.discord_webhook
        && dw.enabled
    {
        match send_discord(dw, topic, message).await {
            Ok(()) => any_success = true,
            Err(e) => errors.push(format!("discord-webhook: {e:#}")),
        }
    }

    if let Some(ntfy_config) = &config.ntfy
        && ntfy_config.enabled
    {
        match send_ntfy(ntfy_config, topic, message).await {
            Ok(()) => any_success = true,
            Err(e) => errors.push(format!("ntfy: {e:#}")),
        }
    }

    for err in &errors {
        eprintln!("Warning: {err}");
    }

    if any_success {
        Ok(())
    } else {
        anyhow::bail!("All backends failed.")
    }
}

async fn send_telegram(config: &config::TelegramConfig, topic: &str, message: &str) -> Result<()> {
    let token = &config.bot_token;
    let group_id = &config.group_id;

    let mut thread_id = resolve_topic(token, group_id, topic).await?;
    let html = markdown::to_telegram_html(message);
    let chunks = chunk_message(&html, TELEGRAM_MAX_LENGTH);

    let first_result = telegram::send_message(token, group_id, &chunks[0], Some(thread_id)).await;

    if let Err(e) = &first_result {
        if e.to_string().contains("thread_not_found") {
            // Cached topic was deleted on Telegram's side — recreate it.
            thread_id = recreate_topic(token, group_id, topic).await?;
            telegram::send_message(token, group_id, &chunks[0], Some(thread_id)).await?;
        } else {
            first_result?;
        }
    }

    for chunk in &chunks[1..] {
        telegram::send_message(token, group_id, chunk, Some(thread_id)).await?;
    }

    Ok(())
}

async fn send_discord(
    config: &config::DiscordWebhookConfig,
    topic: &str,
    message: &str,
) -> Result<()> {
    let converted = markdown::to_discord_markdown(message);
    let prefixed = format!("**[{topic}]**\n{converted}");
    let chunks = chunk_message(&prefixed, DISCORD_MAX_LENGTH);

    for chunk in &chunks {
        discord::send_message(&config.url, chunk).await?;
    }

    Ok(())
}

async fn send_ntfy(config: &config::NtfyConfig, topic: &str, message: &str) -> Result<()> {
    let chunks = chunk_message(message, NTFY_MAX_LENGTH);

    for chunk in &chunks {
        ntfy::send_message(config, topic, chunk).await?;
    }

    Ok(())
}

async fn recreate_topic(token: &str, group_id: &str, topic: &str) -> Result<i64> {
    let mut cache = config::load_topics();
    cache.topics.remove(topic);

    let thread_id = telegram::create_forum_topic(token, group_id, topic).await?;
    cache.topics.insert(topic.to_string(), thread_id);
    config::save_topics(&cache)?;

    Ok(thread_id)
}

async fn resolve_topic(token: &str, group_id: &str, topic: &str) -> Result<i64> {
    let mut cache = config::load_topics();

    if let Some(&thread_id) = cache.topics.get(topic) {
        return Ok(thread_id);
    }

    let thread_id = telegram::create_forum_topic(token, group_id, topic).await?;
    cache.topics.insert(topic.to_string(), thread_id);
    config::save_topics(&cache)?;

    Ok(thread_id)
}

fn chunk_message(text: &str, max_length: usize) -> Vec<String> {
    if text.len() <= max_length {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_length {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a line break near the limit to split on.
        let split_at = remaining[..max_length]
            .rfind('\n')
            .map(|i| i + 1) // include the newline in the current chunk
            .unwrap_or(max_length);

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_message_not_chunked() {
        let chunks = chunk_message("hello", TELEGRAM_MAX_LENGTH);
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn long_message_splits_on_newline_telegram() {
        let line = "a".repeat(100);
        let text = (0..50)
            .map(|_| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_message(&text, TELEGRAM_MAX_LENGTH);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MAX_LENGTH);
        }
    }

    #[test]
    fn long_message_splits_on_newline_discord() {
        let line = "a".repeat(100);
        let text = (0..30)
            .map(|_| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_message(&text, DISCORD_MAX_LENGTH);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= DISCORD_MAX_LENGTH);
        }
    }

    #[test]
    fn discord_chunk_limit_is_2000() {
        assert_eq!(DISCORD_MAX_LENGTH, 2000);
    }
}
