use anyhow::Result;

use crate::config;
use crate::markdown;
use crate::telegram;

const TELEGRAM_MAX_LENGTH: usize = 4096;

pub async fn send(topic: &str, message: &str) -> Result<()> {
    let config = config::load_config()?;
    let token = &config.telegram.bot_token;
    let group_id = &config.telegram.group_id;

    let mut thread_id = resolve_topic(token, group_id, topic).await?;
    let html = markdown::to_telegram_html(message);
    let chunks = chunk_message(&html);

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

fn chunk_message(text: &str) -> Vec<String> {
    if text.len() <= TELEGRAM_MAX_LENGTH {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= TELEGRAM_MAX_LENGTH {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a line break near the limit to split on.
        let split_at = remaining[..TELEGRAM_MAX_LENGTH]
            .rfind('\n')
            .map(|i| i + 1) // include the newline in the current chunk
            .unwrap_or(TELEGRAM_MAX_LENGTH);

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
        let chunks = chunk_message("hello");
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn long_message_splits_on_newline() {
        let line = "a".repeat(100);
        let text = (0..50)
            .map(|_| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_message(&text);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MAX_LENGTH);
        }
    }
}
