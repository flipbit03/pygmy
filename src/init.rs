use std::io::{self, Write};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{self, Config, TelegramConfig};
use crate::telegram;

pub async fn run() -> Result<()> {
    println!();
    println!("{} — Telegram notifications from AI agents", "pygmy".bold());
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
    println!("3. Add your bot to the group and make it admin (ensure \"Manage Topics\" is enabled)");
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
             Then run `pygmy init` again."
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

    let config = Config {
        telegram: TelegramConfig {
            bot_token: bot_token.clone(),
            group_id: group_id.to_string(),
        },
    };
    config::save_config(&config)?;
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
    println!("{} pygmy is ready.", "Done.".green().bold());
    println!();
    println!("Add the following to your CLAUDE.md or agent instructions:");
    println!();
    println!("---");
    print!("{}", include_str!("pygmy_claude_snippet.md"));
    println!("---");

    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    print!("{}: ", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
