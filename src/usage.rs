use crate::version_check;

pub async fn run() {
    let version = version_check::current_version();
    let config_dir = crate::config::config_dir_display();
    let cache_dir = crate::config::cache_dir_display();

    print!(
        "\
pygmy {version} — Telegram notifications from AI agents

USAGE
  pygmy --topic <TOPIC> <MESSAGE>         Send a message to a topic
  pygmy --topic <TOPIC> --stdin           Read message from stdin
  echo \"...\" | pygmy --topic <TOPIC>      Pipe message from stdin
  pygmy init                              Guided setup (Telegram bot + group)
  pygmy self update                       Update to latest version
  pygmy self update --check               Check for updates without installing
  pygmy usage                             Show this reference

OPTIONS
  --topic <NAME>   Forum topic name (created automatically on first use)

MESSAGES
  Messages are parsed as Markdown and converted to Telegram HTML.
  Supports: **bold**, *italic*, `code`, ```code blocks```, [links](url),
  ~~strikethrough~~, > blockquotes, headings, and lists.
  Messages over 4096 characters are automatically split.

CONFIG
  {config_dir}/config.toml       Bot token and group ID
  {cache_dir}/topics.toml        Topic name → thread ID cache (auto-managed)

SETUP
  Run `pygmy init` for guided setup. You will need:
  1. A Telegram bot (create via @BotFather)
  2. A Telegram group with Topics enabled
  3. The bot added as admin in the group (with \"Manage Topics\" permission)
"
    );

    // Show update hint if cached version is newer.
    if !version_check::is_dev_build()
        && let Some(latest) = version_check::get_cached_version()
        && version_check::is_newer(version, &latest)
    {
        println!("Update available: {version} → {latest}");
        println!("Run `pygmy self update` to upgrade.");
    }
}
