mod attachment;
mod config;
mod discord;
mod http;
mod init;
mod markdown;
mod ntfy;
mod self_update;
mod send;
mod telegram;
mod usage;
mod version_check;

use std::io::{IsTerminal, Read};

use clap::{Parser, Subcommand};
use colored::Colorize;

/// pygmy — notifications from AI agents
#[derive(Debug, Parser)]
#[command(name = "pygmy", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Topic name (used as Telegram/Discord message prefix / ntfy notification title).
    #[arg(long, global = true)]
    topic: Option<String>,

    /// Read message from stdin.
    #[arg(long)]
    stdin: bool,

    /// Attach a file (repeatable). With files, the message is optional and used as the caption.
    #[arg(long = "file", value_name = "PATH")]
    files: Vec<std::path::PathBuf>,

    /// Message to send.
    message: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Set up a notification backend.
    Init {
        #[command(subcommand)]
        backend: InitBackend,
    },
    /// Enable a configured backend.
    Enable {
        /// Backend name: telegram, discord-webhook, ntfy
        backend: String,
    },
    /// Disable a configured backend.
    Disable {
        /// Backend name: telegram, discord-webhook, ntfy
        backend: String,
    },
    /// Show configured backends and their status.
    Status,
    /// Print the agent-facing reference: what pygmy can do (plain text, <1k tokens).
    Usage,
    /// Manage pygmy itself (update, etc.).
    #[command(name = "self")]
    SelfCmd(self_update::SelfCmd),
}

#[derive(Debug, Subcommand)]
enum InitBackend {
    /// Set up Telegram bot and channel.
    Telegram,
    /// Set up Discord webhook notifications.
    DiscordWebhook,
    /// Set up ntfy push notifications.
    Ntfy,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Command::Init { backend }) => match backend {
            InitBackend::Telegram => init::run_telegram().await,
            InitBackend::DiscordWebhook => init::run_discord_webhook().await,
            InitBackend::Ntfy => init::run_ntfy().await,
        },
        Some(Command::Enable { backend }) => run_toggle(&backend, true),
        Some(Command::Disable { backend }) => run_toggle(&backend, false),
        Some(Command::Status) => run_status(),
        Some(Command::Usage) => {
            usage::run().await;
            return;
        }
        Some(Command::SelfCmd(cmd)) => self_update::run(cmd).await,
        None => run_send(&cli).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run_toggle(backend: &str, enable: bool) -> anyhow::Result<()> {
    let mut cfg = config::load_config()?;
    let action = if enable { "enabled" } else { "disabled" };

    match backend {
        "telegram" => {
            let tg = cfg.telegram.as_mut().ok_or_else(|| {
                anyhow::anyhow!("Telegram is not configured. Run `pygmy init telegram` first.")
            })?;
            tg.enabled = enable;
        }
        "discord-webhook" => {
            let dw = cfg.discord_webhook.as_mut().ok_or_else(|| {
                anyhow::anyhow!(
                    "Discord webhook is not configured. Run `pygmy init discord-webhook` first."
                )
            })?;
            dw.enabled = enable;
        }
        "ntfy" => {
            let n = cfg.ntfy.as_mut().ok_or_else(|| {
                anyhow::anyhow!("ntfy is not configured. Run `pygmy init ntfy` first.")
            })?;
            n.enabled = enable;
        }
        _ => {
            anyhow::bail!("Unknown backend: {backend}\nAvailable: telegram, discord-webhook, ntfy");
        }
    }

    config::save_config(&cfg)?;
    println!("{} {backend} {action}.", "✓".green());
    Ok(())
}

fn run_status() -> anyhow::Result<()> {
    let cfg = config::load_config();

    let cfg = match cfg {
        Ok(c) => c,
        Err(_) => {
            println!("No backends configured.");
            println!("Run {} to get started.", "pygmy init <backend>".bold());
            return Ok(());
        }
    };

    println!("{}", "pygmy — notification backends".bold());
    println!();

    for (name, state) in config::backend_states(&cfg) {
        match state {
            config::BackendState::Enabled(detail) => {
                println!("  {} {}  ({detail})", "✓".green(), name.bold());
            }
            config::BackendState::Disabled => {
                println!("  {} {}  (disabled)", "✗".red(), name.bold());
            }
            config::BackendState::NotConfigured => {
                println!("  {} {}  (not configured)", "−".dimmed(), name.dimmed());
            }
        }
    }

    Ok(())
}

async fn run_send(cli: &Cli) -> anyhow::Result<()> {
    let topic = cli.topic.as_deref().ok_or_else(|| {
        anyhow::anyhow!("--topic is required.\nUsage: pygmy --topic <NAME> <MESSAGE>")
    })?;

    // Validate files up front, before reading stdin or sending anything.
    let attachments = cli
        .files
        .iter()
        .map(|p| attachment::Attachment::open(p))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let has_files = !attachments.is_empty();

    let message = if !cli.message.is_empty() {
        if cli.stdin {
            anyhow::bail!("Cannot provide both a message argument and --stdin.");
        }
        cli.message.join(" ")
    } else if cli.stdin || (!has_files && !std::io::stdin().is_terminal()) {
        // With --file, stdin is only read when --stdin is explicit: agents run
        // with stdin as a pipe, and the caption is optional.
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else if has_files {
        String::new()
    } else {
        anyhow::bail!("No message provided.\nUsage: pygmy --topic <NAME> \"your message\"");
    };

    if message.trim().is_empty() && !has_files {
        anyhow::bail!("Message is empty.");
    }

    send::send(topic, &message, &attachments).await
}
