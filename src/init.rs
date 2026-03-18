use std::io::{self, Write};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{self, DiscordWebhookConfig, NtfyConfig, TelegramConfig};
use crate::discord;
use crate::ntfy;
use crate::telegram;

pub async fn run_telegram() -> Result<()> {
    println!();
    println!("{} — set up Telegram notifications", "pygmy".bold());
    println!();

    println!("{}", "Step 1: Create a Telegram bot".bold());
    println!("1. Open Telegram and message @BotFather");
    println!("2. Send /newbot");
    println!("3. Choose a name (e.g. \"Pygmy Notifications\")");
    println!("4. Choose a username (e.g. \"my_pygmy_bot\")");
    println!("5. Copy the bot token BotFather gives you");
    println!();

    let bot_token = prompt("Paste your bot token")?;
    if bot_token.is_empty() {
        anyhow::bail!("Bot token cannot be empty.");
    }
    println!("{} Bot token saved", "✓".green());
    println!();

    println!("{}", "Step 2: Create a Forum group".bold());
    println!("1. Create a new Telegram group (you can be the only member)");
    println!("2. Go to group settings → Topics → Enable");
    println!(
        "3. Add your bot to the group and make it admin (ensure \"Manage Topics\" is enabled)"
    );
    println!("4. Send /start in the group (important: must start with /)");
    println!();
    prompt("Press Enter once done...")?;

    let updates = telegram::get_updates(&bot_token)
        .await
        .context("Could not reach Telegram API — check your bot token.")?;

    let mut groups: Vec<(i64, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for update in &updates {
        if let Some(msg) = &update.message {
            let chat = &msg.chat;
            if (chat.chat_type == "group" || chat.chat_type == "supergroup") && seen.insert(chat.id)
            {
                let title = chat.title.clone().unwrap_or_else(|| "Untitled".into());
                groups.push((chat.id, title));
            }
        }
        if let Some(member) = &update.my_chat_member {
            let chat = &member.chat;
            if (chat.chat_type == "group" || chat.chat_type == "supergroup") && seen.insert(chat.id)
            {
                let title = chat.title.clone().unwrap_or_else(|| "Untitled".into());
                groups.push((chat.id, title));
            }
        }
    }

    if groups.is_empty() {
        eprintln!(
            "{} getUpdates returned {} update(s), but none contained group info.",
            "Debug:".dimmed(),
            updates.len()
        );
        anyhow::bail!(
            "No groups found. Make sure you:\n\
             1. Added the bot to a group\n\
             2. Sent /start in the group (regular messages are invisible to bots)\n\
             3. The group has Topics enabled\n\
             Then run `pygmy init telegram` again."
        );
    }

    let (group_id, group_title) = if groups.len() == 1 {
        let (id, title) = &groups[0];
        println!("{} Found group: \"{}\" ({})", "✓".green(), title, id);
        (*id, title.clone())
    } else {
        println!("Found these groups:");
        for (i, (id, title)) in groups.iter().enumerate() {
            println!("  {}. \"{}\" ({})", i + 1, title, id);
        }
        println!();
        let choice = prompt("Which group? [1]")?;
        let idx: usize = if choice.is_empty() {
            0
        } else {
            choice
                .parse::<usize>()
                .context("invalid number")?
                .checked_sub(1)
                .context("invalid choice")?
        };
        let (id, title) = groups.get(idx).context("invalid choice")?;
        (*id, title.clone())
    };

    let mut cfg = config::load_config_or_default();
    cfg.telegram = Some(TelegramConfig {
        enabled: true,
        bot_token: bot_token.clone(),
        group_id: group_id.to_string(),
    });
    config::save_config(&cfg)?;
    println!();

    println!("{}", "Step 3: Test".bold());
    print!("Creating test topic and sending message...");
    io::stdout().flush()?;

    let thread_id = telegram::create_forum_topic(&bot_token, &group_id.to_string(), "pygmy-test")
        .await
        .context(
            "Could not create topic. Make sure:\n\
             1. Topics are enabled in group settings\n\
             2. The bot is an admin in the group",
        )?;

    telegram::send_message(
        &bot_token,
        &group_id.to_string(),
        "pygmy is set up and working! 🎉",
        Some(thread_id),
    )
    .await
    .context("Could not send test message")?;

    println!(
        "\r{} Test message delivered! Check \"{}\" in Telegram.",
        "✓".green(),
        group_title
    );
    println!();
    println!("{} Telegram is ready.", "Done.".green().bold());

    print_snippet();

    Ok(())
}

pub async fn run_discord_webhook() -> Result<()> {
    println!();
    println!("{} — set up Discord webhook notifications", "pygmy".bold());
    println!();

    println!("{}", "Step 1: Create a Discord webhook".bold());
    println!("1. Open Discord and go to the channel you want notifications in");
    println!("2. Click the gear icon (Edit Channel) → Integrations → Webhooks");
    println!("3. Click \"New Webhook\", give it a name (e.g. \"pygmy\")");
    println!("4. Click \"Copy Webhook URL\"");
    println!();

    let url = prompt("Paste your webhook URL")?;
    if url.is_empty() {
        anyhow::bail!("Webhook URL cannot be empty.");
    }
    if !url.starts_with("https://discord.com/api/webhooks/")
        && !url.starts_with("https://discordapp.com/api/webhooks/")
    {
        anyhow::bail!(
            "That doesn't look like a Discord webhook URL.\n\
             Expected: https://discord.com/api/webhooks/..."
        );
    }
    println!("{} Webhook URL saved", "✓".green());
    println!();

    println!("{}", "Step 2: Test".bold());
    print!("Sending test message...");
    io::stdout().flush()?;

    discord::send_message(&url, "**[pygmy-test]**\npygmy is set up and working! 🎉")
        .await
        .context("Could not send test message — check your webhook URL.")?;

    println!(
        "\r{} Test message delivered! Check your Discord channel.",
        "✓".green()
    );
    println!();

    let mut cfg = config::load_config_or_default();
    cfg.discord_webhook = Some(DiscordWebhookConfig { enabled: true, url });
    config::save_config(&cfg)?;

    println!("{} Discord webhook is ready.", "Done.".green().bold());

    print_snippet();

    Ok(())
}

pub async fn run_ntfy() -> Result<()> {
    println!();
    println!("{} — set up ntfy push notifications", "pygmy".bold());
    println!();

    println!("{}", "Step 1: Subscribe to a topic".bold());
    println!("1. Install the ntfy app on your phone (Android/iOS) or desktop");
    println!("2. Subscribe to a topic — this will be your notification channel");
    println!("3. Pick a hard-to-guess topic name if using the public ntfy.sh server");
    println!("   (on ntfy.sh, anyone who knows the topic name can read/write to it)");
    println!();

    let server = prompt_with_default("Server URL", "https://ntfy.sh")?;
    if server.is_empty() {
        anyhow::bail!("Server URL cannot be empty.");
    }
    println!("{} Server: {}", "✓".green(), server);
    println!();

    let topic = prompt("ntfy topic name")?;
    if topic.is_empty() {
        anyhow::bail!("Topic name cannot be empty.");
    }
    println!("{} Topic: {}", "✓".green(), topic);
    println!();

    println!("{}", "Step 2: Authentication (optional)".bold());
    println!("If your ntfy server requires authentication, enter a token.");
    println!("The public ntfy.sh server does not require a token.");
    println!();

    let token_input = prompt("Access token (press Enter to skip)")?;
    let token = if token_input.is_empty() {
        println!("{} No token (public access)", "✓".green());
        None
    } else {
        println!("{} Token saved", "✓".green());
        Some(token_input)
    };
    println!();

    let ntfy_config = NtfyConfig {
        enabled: true,
        server: server.clone(),
        topic: topic.clone(),
        token: token.clone(),
    };

    println!("{}", "Step 3: Test".bold());
    print!("Sending test notification...");
    io::stdout().flush()?;

    ntfy::send_message(
        &ntfy_config,
        "pygmy-test",
        "pygmy is set up and working! 🎉",
    )
    .await
    .context("Could not send test notification — check your server URL and topic.")?;

    println!(
        "\r{} Test notification sent! Check your ntfy app.",
        "✓".green()
    );
    println!();

    let mut cfg = config::load_config_or_default();
    cfg.ntfy = Some(ntfy_config);
    config::save_config(&cfg)?;

    println!("{} ntfy is ready.", "Done.".green().bold());

    print_snippet();

    Ok(())
}

fn print_snippet() {
    println!();
    println!("Add the following to your CLAUDE.md or agent instructions:");
    println!();
    println!("---");
    print!("{}", include_str!("pygmy_claude_snippet.md"));
    println!("---");
}

fn prompt(label: &str) -> Result<String> {
    print!("{}: ", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn prompt_with_default(label: &str, default: &str) -> Result<String> {
    print!("{} [{}]: ", label, default);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}
