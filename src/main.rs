mod config;
mod init;
mod markdown;
mod self_update;
mod send;
mod telegram;
mod usage;
mod version_check;

use std::io::{IsTerminal, Read};

use clap::{Parser, Subcommand};

/// pygmy — Telegram notifications from AI agents
#[derive(Debug, Parser)]
#[command(name = "pygmy", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Forum topic name (auto-created on first use).
    #[arg(long, global = true)]
    topic: Option<String>,

    /// Read message from stdin.
    #[arg(long)]
    stdin: bool,

    /// Message to send.
    message: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Guided setup for Telegram bot and group.
    Init,
    /// Print a compact LLM-friendly command reference.
    Usage,
    /// Manage pygmy itself (update, etc.).
    #[command(name = "self")]
    SelfCmd(self_update::SelfCmd),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Command::Init) => init::run().await,
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

async fn run_send(cli: &Cli) -> anyhow::Result<()> {
    let topic = cli.topic.as_deref().ok_or_else(|| {
        anyhow::anyhow!("--topic is required.\nUsage: pygmy --topic <NAME> <MESSAGE>")
    })?;

    let message = if !cli.message.is_empty() {
        if cli.stdin {
            anyhow::bail!("Cannot provide both a message argument and --stdin.");
        }
        cli.message.join(" ")
    } else if cli.stdin || !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        anyhow::bail!("No message provided.\nUsage: pygmy --topic <NAME> \"your message\"");
    };

    if message.trim().is_empty() {
        anyhow::bail!("Message is empty.");
    }

    send::send(topic, &message).await
}
