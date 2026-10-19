use anyhow::Result;

use crate::attachment::{self, Attachment};
use crate::config;
use crate::discord;
use crate::markdown;
use crate::ntfy;
use crate::telegram;

pub const TELEGRAM_MAX_LENGTH: usize = 4096;
pub const TELEGRAM_MAX_CAPTION_LENGTH: usize = 1024;
pub const DISCORD_MAX_LENGTH: usize = 2000;
pub const NTFY_MAX_LENGTH: usize = 4096;

/// Send a notification to every enabled backend.
///
/// `message` is the text (or the caption, when `attachments` is non-empty).
/// It may only be empty when there is at least one attachment.
pub async fn send(topic: &str, message: &str, attachments: &[Attachment]) -> Result<()> {
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
        let result = if attachments.is_empty() {
            send_telegram(tg, topic, message).await
        } else {
            send_telegram_files(tg, topic, message, attachments).await
        };
        match result {
            Ok(()) => any_success = true,
            Err(e) => errors.push(format!("telegram: {e:#}")),
        }
    }

    if let Some(dw) = &config.discord_webhook
        && dw.enabled
    {
        let result = if attachments.is_empty() {
            send_discord(dw, topic, message).await
        } else {
            send_discord_files(dw, topic, message, attachments).await
        };
        match result {
            Ok(()) => any_success = true,
            Err(e) => errors.push(format!("discord-webhook: {e:#}")),
        }
    }

    if let Some(ntfy_config) = &config.ntfy
        && ntfy_config.enabled
    {
        let result = if attachments.is_empty() {
            send_ntfy(ntfy_config, topic, message).await
        } else {
            send_ntfy_files(ntfy_config, topic, message, attachments).await
        };
        match result {
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
    let channel_id = &config.channel_id;

    let prefixed = format!("**[{topic}]**\n{message}");
    let html = markdown::to_telegram_html(&prefixed);
    let chunks = chunk_message(&html, TELEGRAM_MAX_LENGTH);

    for chunk in &chunks {
        telegram::send_message(token, channel_id, chunk).await?;
    }

    Ok(())
}

async fn send_telegram_files(
    config: &config::TelegramConfig,
    topic: &str,
    message: &str,
    attachments: &[Attachment],
) -> Result<()> {
    attachment::check_sizes(attachments, attachment::TELEGRAM_MAX_FILE_SIZE)?;

    let short = markdown::to_telegram_html(&format!("**[{topic}]**"));
    let full = if message.trim().is_empty() {
        short.clone()
    } else {
        markdown::to_telegram_html(&format!("**[{topic}]**\n{message}"))
    };
    let plan = plan_captions(full, short, TELEGRAM_MAX_CAPTION_LENGTH);

    if plan.text_first {
        send_telegram(config, topic, message).await?;
    }

    for (i, att) in attachments.iter().enumerate() {
        let caption = plan.caption_for(i);
        telegram::send_document(
            &config.bot_token,
            &config.channel_id,
            &att.name,
            att.read(attachment::TELEGRAM_MAX_FILE_SIZE)?,
            caption,
        )
        .await?;
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

async fn send_discord_files(
    config: &config::DiscordWebhookConfig,
    topic: &str,
    message: &str,
    attachments: &[Attachment],
) -> Result<()> {
    attachment::check_sizes(attachments, attachment::DISCORD_MAX_FILE_SIZE)?;

    let short = format!("**[{topic}]**");
    let full = if message.trim().is_empty() {
        short.clone()
    } else {
        format!("**[{topic}]**\n{}", markdown::to_discord_markdown(message))
    };
    let plan = plan_captions(full, short, DISCORD_MAX_LENGTH);

    if plan.text_first {
        send_discord(config, topic, message).await?;
    }

    // One webhook message per file keeps each request under Discord's
    // per-message upload limit regardless of how many files are sent.
    for (i, att) in attachments.iter().enumerate() {
        let bytes = att.read(attachment::DISCORD_MAX_FILE_SIZE)?;
        discord::send_file(&config.url, plan.caption_for(i), &att.name, bytes).await?;
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

async fn send_ntfy_files(
    config: &config::NtfyConfig,
    topic: &str,
    message: &str,
    attachments: &[Attachment],
) -> Result<()> {
    attachment::check_sizes(attachments, attachment::NTFY_MAX_FILE_SIZE)?;

    let plan = plan_ntfy_captions(message);

    if plan.text_first {
        send_ntfy(config, topic, message).await?;
    }

    for (i, att) in attachments.iter().enumerate() {
        let bytes = att.read(attachment::NTFY_MAX_FILE_SIZE)?;
        ntfy::send_file(config, topic, plan.caption_for(i), &att.name, bytes).await?;
    }

    Ok(())
}

/// How to caption a batch of files.
#[derive(Debug, PartialEq)]
struct CaptionPlan {
    /// The full text didn't fit in a caption: send it as a normal
    /// (chunked) message before the files.
    text_first: bool,
    /// Caption for the first file.
    first: String,
    /// Caption for every other file.
    rest: String,
}

impl CaptionPlan {
    fn caption_for(&self, index: usize) -> &str {
        if index == 0 { &self.first } else { &self.rest }
    }
}

/// `full` is the caption with the message, `short` is the topic-only caption.
/// If `full` fits in `max_length`, it goes on the first file; otherwise the text
/// is sent separately and every file gets `short`.
fn plan_captions(full: String, short: String, max_length: usize) -> CaptionPlan {
    if full.len() <= max_length {
        CaptionPlan {
            text_first: false,
            first: full,
            rest: short,
        }
    } else {
        CaptionPlan {
            text_first: true,
            first: short.clone(),
            rest: short,
        }
    }
}

/// ntfy shows the topic as the title, so the caption is just the message.
///
/// The caption travels in the `Message` header, and the ntfy server turns the
/// two characters `\n` in that header into a real newline. A caption with a
/// literal backslash-n would arrive altered, so it goes text-first instead
/// (as a normal message body, which ntfy leaves alone).
fn plan_ntfy_captions(message: &str) -> CaptionPlan {
    let full = message.trim().to_string();
    if full.contains("\\n") {
        return CaptionPlan {
            text_first: true,
            first: String::new(),
            rest: String::new(),
        };
    }
    plan_captions(full, String::new(), NTFY_MAX_LENGTH)
}

/// Split `text` into chunks of at most `max_length` bytes, preferring to cut
/// after a newline and never cutting inside a UTF-8 character.
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

        // Back off to a char boundary so the slice can't land mid-character.
        let mut cut = max_length;
        while !remaining.is_char_boundary(cut) {
            cut -= 1;
        }
        if cut == 0 {
            // max_length is smaller than the first character: take it whole.
            cut = remaining.chars().next().map_or(1, char::len_utf8);
        }

        // Find a line break near the limit to split on.
        let split_at = remaining[..cut]
            .rfind('\n')
            .map(|i| i + 1) // include the newline in the current chunk
            .unwrap_or(cut);

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
    fn non_ascii_message_splits_on_char_boundary() {
        // "é" is 2 bytes: 1100 of them is 2200 bytes, and byte 2000 is a
        // boundary, but 1001 × "é" + "a" puts the limit mid-character.
        for text in ["é".repeat(1100), format!("a{}", "é".repeat(1100))] {
            let chunks = chunk_message(&text, DISCORD_MAX_LENGTH);
            assert!(chunks.len() > 1);
            for chunk in &chunks {
                assert!(chunk.len() <= DISCORD_MAX_LENGTH);
            }
            assert_eq!(chunks.concat(), text);
        }
    }

    #[test]
    fn multibyte_chars_with_newlines_and_tiny_limit() {
        let text = "日本語\n🎉🎉\nok";
        for max in 1..=text.len() {
            let chunks = chunk_message(text, max);
            assert_eq!(chunks.concat(), text, "max={max}");
        }
    }

    #[test]
    fn ntfy_caption_with_literal_backslash_n_goes_text_first() {
        let plan = plan_ntfy_captions(r"path C:\new\file");
        assert!(plan.text_first);
        assert_eq!(plan.caption_for(0), "");
        assert_eq!(plan.caption_for(1), "");
    }

    #[test]
    fn ntfy_caption_with_real_newline_stays_inline() {
        let plan = plan_ntfy_captions("line one\nline two");
        assert!(!plan.text_first);
        assert_eq!(plan.caption_for(0), "line one\nline two");
        assert_eq!(plan.caption_for(1), "");
    }

    #[test]
    fn discord_chunk_limit_is_2000() {
        assert_eq!(DISCORD_MAX_LENGTH, 2000);
    }

    #[test]
    fn telegram_caption_limit_is_1024() {
        assert_eq!(TELEGRAM_MAX_CAPTION_LENGTH, 1024);
    }

    #[test]
    fn short_caption_goes_on_first_file() {
        let plan = plan_captions("[t]\nhello".into(), "[t]".into(), 1024);
        assert_eq!(
            plan,
            CaptionPlan {
                text_first: false,
                first: "[t]\nhello".into(),
                rest: "[t]".into(),
            }
        );
        assert_eq!(plan.caption_for(0), "[t]\nhello");
        assert_eq!(plan.caption_for(1), "[t]");
        assert_eq!(plan.caption_for(5), "[t]");
    }

    #[test]
    fn caption_at_limit_still_inline() {
        let full = "a".repeat(1024);
        let plan = plan_captions(full.clone(), "[t]".into(), 1024);
        assert!(!plan.text_first);
        assert_eq!(plan.first, full);
    }

    #[test]
    fn long_caption_sent_as_text_first() {
        let plan = plan_captions("a".repeat(1025), "[t]".into(), 1024);
        assert_eq!(
            plan,
            CaptionPlan {
                text_first: true,
                first: "[t]".into(),
                rest: "[t]".into(),
            }
        );
    }

    #[test]
    fn empty_short_caption_for_ntfy() {
        let plan = plan_captions(String::new(), String::new(), NTFY_MAX_LENGTH);
        assert!(!plan.text_first);
        assert_eq!(plan.caption_for(0), "");
        assert_eq!(plan.caption_for(1), "");
    }
}
