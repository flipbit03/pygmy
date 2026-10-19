//! `pygmy usage`: the agent-facing reference (<1000 tokens, plain text).
//! `--help` is the detailed clap output for humans; this is "what can I do?".

use crate::attachment::{self, DISCORD_MAX_FILE_SIZE, NTFY_MAX_FILE_SIZE, TELEGRAM_MAX_FILE_SIZE};
use crate::config::{self, BackendState};
use crate::send::{
    DISCORD_MAX_LENGTH, NTFY_MAX_LENGTH, TELEGRAM_MAX_CAPTION_LENGTH, TELEGRAM_MAX_LENGTH,
};
use crate::version_check;

pub async fn run() {
    let version = version_check::current_version();
    print!("{}", body(version, &config::config_dir_display()));
    print!("{}", backends_footer(config::load_config().ok().as_ref()));

    // Show update hint if the cached latest version is newer.
    if !version_check::is_dev_build()
        && let Some(latest) = version_check::get_cached_version()
        && version_check::is_newer(version, &latest)
    {
        println!();
        println!("Update available: {version} -> {latest}. Run: pygmy self update");
    }
}

/// "50 MB" for whole megabytes, otherwise the precise formatted size.
fn mb(bytes: u64) -> String {
    if bytes.is_multiple_of(attachment::MB) {
        format!("{} MB", bytes / attachment::MB)
    } else {
        attachment::format_size(bytes)
    }
}

fn body(version: &str, config_dir: &str) -> String {
    let tg_file = mb(TELEGRAM_MAX_FILE_SIZE);
    let dc_file = mb(DISCORD_MAX_FILE_SIZE);
    let nt_file = format!("{}*", mb(NTFY_MAX_FILE_SIZE));
    let tg_caption = TELEGRAM_MAX_CAPTION_LENGTH;
    // Discord and ntfy captions share the normal message limit.
    let dc_caption = DISCORD_MAX_LENGTH;
    let nt_caption = NTFY_MAX_LENGTH;

    format!(
        r#"pygmy {version} -- one-way notifications to the user's phone

Sends messages and files to the user via Telegram, Discord and/or ntfy.
The user can't reply through pygmy: it's a ping that brings them back to you.

WHEN TO NOTIFY:
  - The user asked to be pinged / notified / told when you're done
  - A long task finished while the user may be away
  - You're blocked and need the user's input
  - The user asked you to send them a file (report, log, screenshot)
  Don't notify for every small step. Pick one short topic per session
  (e.g. "auth-refactor") and reuse it for every message.

SENDING:
  pygmy --topic <TOPIC> <MESSAGE>...   Send to every enabled backend (--topic is required;
                                       several message args are joined with spaces)
  pygmy --topic <TOPIC> --stdin        Read the message from stdin (use a heredoc for long text)
  pygmy --topic <TOPIC> -- "-3 ..."    A message starting with "-" needs "--" before it
  Without a message arg, piped stdin is read. A message arg plus --stdin is an error.
  Messages are Markdown:
    telegram   converted to HTML (headings -> bold, lists -> bullets, tables -> monospace)
    discord    sent as-is (tables -> code blocks)
    ntfy       sent as-is (rendered in the web app, raw text on mobile); topic = title
  Long text is split on line breaks: telegram {TELEGRAM_MAX_LENGTH}, discord {DISCORD_MAX_LENGTH}, ntfy {NTFY_MAX_LENGTH} bytes per message.
  Exit 0 if any backend succeeded (failures are warnings on stderr); non-zero if all failed.

FILES:
  pygmy --topic <TOPIC> --file <PATH> [CAPTION]   Send a file; --file repeats
    The message is optional and becomes the first file's caption; other files
    get only the topic. With --file, stdin is read only if --stdin is passed.
  Every file is checked before anything is sent: missing, unreadable, a directory
  or empty (0 bytes) is an error and nothing is sent.
  Limits (caption = rendered text in bytes, topic prefix included):
    backend    max file   max caption
    telegram   {tg_file:<10} {tg_caption} bytes
    discord    {dc_file:<10} {dc_caption} bytes
    ntfy       {nt_file:<10} {nt_caption} bytes
  A file over a backend's limit fails that backend; the others still send. No zipping
  or splitting: compress or trim it yourself. A caption over the limit is sent as a
  normal message first, then the files with a topic-only caption. A backend stops at
  its first error, so earlier items (text, file 1) may already have been delivered.
  * ntfy.sh default. Self-hosted ntfy needs attachments enabled and may use another limit.

SETUP (for the user -- init is interactive, so ask them to run it):
  pygmy init telegram|discord-webhook|ntfy   Set up a backend
  pygmy status                               Show configured backends
  pygmy enable|disable <BACKEND>             Toggle a backend, keeping its credentials
  Config: {config_dir}/config.toml

SELF:
  pygmy self update [--check]   Update pygmy (or just check for a new version)
  pygmy --help                  Every flag in detail

EXAMPLES:
  pygmy --topic auth-refactor "Done. PR #47 ready for review."
  pygmy --topic nightly --file ./report.pdf "Nightly report attached"
  pygmy --topic deploy --stdin <<'EOF'
**Deploy blocked**: need the prod DB password to continue.
EOF
"#
    )
}

/// Which backends this machine would send to.
fn backends_footer(cfg: Option<&config::Config>) -> String {
    let not_configured = "\nBACKENDS: not configured -- ask the user to run: pygmy init <backend>\n\
                          (telegram, discord-webhook or ntfy). Sending will fail until then.\n";
    let Some(cfg) = cfg else {
        return not_configured.to_string();
    };

    let states = config::backend_states(cfg);
    if states
        .iter()
        .all(|(_, s)| *s == BackendState::NotConfigured)
    {
        return not_configured.to_string();
    }

    let mut out = String::from("\nBACKENDS (this machine):\n");
    for (name, state) in &states {
        let label = match state {
            BackendState::Enabled(_) => "enabled",
            BackendState::Disabled => "disabled",
            BackendState::NotConfigured => "not configured",
        };
        out.push_str(&format!("  {name:<16} {label}\n"));
    }
    if !states
        .iter()
        .any(|(_, s)| matches!(s, BackendState::Enabled(_)))
    {
        out.push_str(
            "  No backend is enabled: sending will fail. Ask the user to run: pygmy enable <backend>\n",
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DiscordWebhookConfig, NtfyConfig, TelegramConfig};

    fn rendered() -> String {
        body("1.2.3", "/home/u/.config/pygmy")
    }

    #[test]
    fn has_all_sections_and_key_flags() {
        let text = rendered();
        for needle in [
            "pygmy 1.2.3",
            "WHEN TO NOTIFY:",
            "SENDING:",
            "FILES:",
            "SETUP",
            "SELF:",
            "EXAMPLES:",
            "--topic",
            "--stdin",
            "--file",
            "pygmy init telegram|discord-webhook|ntfy",
            "pygmy status",
            "pygmy enable|disable",
            "pygmy self update",
            "pygmy --help",
            "/home/u/.config/pygmy/config.toml",
        ] {
            assert!(text.contains(needle), "missing {needle:?}");
        }
    }

    #[test]
    fn limits_come_from_constants() {
        let text = rendered();
        assert!(text.contains(&format!("telegram {TELEGRAM_MAX_LENGTH}, discord {DISCORD_MAX_LENGTH}, ntfy {NTFY_MAX_LENGTH} bytes")));
        let row = |name: &str, file: u64, caption: usize| {
            let line = text
                .lines()
                .find(|l| {
                    l.trim_start().starts_with(name) && l.contains("bytes") && l.contains("MB")
                })
                .unwrap_or_else(|| panic!("no limits row for {name}"));
            let cols: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(cols[1], (file / attachment::MB).to_string(), "{line}");
            assert_eq!(cols[3], caption.to_string(), "{line}");
        };
        row(
            "telegram",
            TELEGRAM_MAX_FILE_SIZE,
            TELEGRAM_MAX_CAPTION_LENGTH,
        );
        row("discord", DISCORD_MAX_FILE_SIZE, DISCORD_MAX_LENGTH);
        row("ntfy", NTFY_MAX_FILE_SIZE, NTFY_MAX_LENGTH);
    }

    #[test]
    fn plain_text_not_markdown() {
        let text = rendered();
        assert!(!text.lines().any(|l| l.starts_with('#')));
        assert!(!text.contains("|---"));
        assert!(!text.contains("```"));
    }

    #[test]
    fn stays_under_token_budget() {
        // ~4 chars per token for English; keep the whole reference under ~1000 tokens.
        let text = rendered() + &backends_footer(None);
        assert!(text.len() < 4000, "usage is {} bytes", text.len());
    }

    #[test]
    fn footer_not_configured() {
        let footer = backends_footer(None);
        assert!(footer.contains("not configured"));
        assert!(footer.contains("pygmy init <backend>"));
        assert!(backends_footer(Some(&Config::default())).contains("pygmy init <backend>"));
    }

    #[test]
    fn footer_lists_states() {
        let cfg = Config {
            telegram: Some(TelegramConfig {
                enabled: true,
                bot_token: "t".into(),
                channel_id: "-100".into(),
            }),
            discord_webhook: Some(DiscordWebhookConfig {
                enabled: false,
                url: "u".into(),
            }),
            ntfy: None,
        };
        let footer = backends_footer(Some(&cfg));
        assert!(footer.contains("telegram         enabled"), "{footer}");
        assert!(footer.contains("discord-webhook  disabled"), "{footer}");
        assert!(
            footer.contains("ntfy             not configured"),
            "{footer}"
        );
        assert!(!footer.contains("No backend is enabled"));
        // Credentials and topics never leak into agent context.
        assert!(!footer.contains("-100"));
    }

    #[test]
    fn footer_warns_when_all_disabled() {
        let cfg = Config {
            telegram: None,
            discord_webhook: None,
            ntfy: Some(NtfyConfig {
                enabled: false,
                server: "https://ntfy.sh".into(),
                topic: "secret".into(),
                token: None,
            }),
        };
        let footer = backends_footer(Some(&cfg));
        assert!(footer.contains("No backend is enabled"), "{footer}");
        assert!(footer.contains("pygmy enable <backend>"));
    }

    #[test]
    fn snippet_points_to_usage() {
        let snippet = include_str!("pygmy_claude_snippet.md");
        assert!(snippet.contains("`pygmy usage`"));
        assert!(snippet.len() < 500);
    }
}
