use crate::version_check;

pub async fn run() {
    let version = version_check::current_version();
    let config_dir = crate::config::config_dir_display();
    let cache_dir = crate::config::cache_dir_display();

    print!(
        "\
pygmy {version} — notifications from AI agents

USAGE
  pygmy --topic <TOPIC> <MESSAGE>           Send to all enabled backends
  pygmy --topic <TOPIC> --stdin             Read message from stdin
  echo \"...\" | pygmy --topic <TOPIC>        Pipe message from stdin
  pygmy init telegram                       Set up Telegram bot + group
  pygmy init discord-webhook                Set up Discord webhook
  pygmy enable <BACKEND>                    Enable a configured backend
  pygmy disable <BACKEND>                   Disable a configured backend
  pygmy status                              Show configured backends
  pygmy self update                         Update to latest version
  pygmy self update --check                 Check for updates without installing
  pygmy usage                               Show this reference

OPTIONS
  --topic <NAME>   Topic name (Telegram forum topic / Discord message prefix)

BACKENDS
  telegram         Telegram Bot API with forum topics (requires bot + group)
  discord-webhook  Discord webhook (just a URL, messages prefixed with [topic])

MESSAGES
  Messages are parsed as Markdown.
  Telegram: converted to HTML (bold, italic, code, links, blockquotes, lists).
  Discord: sent as-is (Discord renders Markdown natively).
  Long messages are automatically split (4096 for Telegram, 2000 for Discord).

CONFIG
  {config_dir}/config.toml       Backend credentials and enabled/disabled state
  {cache_dir}/topics.toml        Telegram topic name → thread ID cache (auto-managed)

SETUP
  Run `pygmy init telegram` or `pygmy init discord-webhook` for guided setup.
  You can configure both — messages are sent to all enabled backends.
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
