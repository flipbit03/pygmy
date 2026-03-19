use crate::version_check;

pub async fn run() {
    let version = version_check::current_version();
    let config_dir = crate::config::config_dir_display();

    print!(
        "\
pygmy {version} — notifications from AI agents

USAGE
  pygmy --topic <TOPIC> <MESSAGE>           Send to all enabled backends
  pygmy --topic <TOPIC> --stdin             Read message from stdin
  echo \"...\" | pygmy --topic <TOPIC>        Pipe message from stdin
  pygmy init telegram                       Set up Telegram bot + channel
  pygmy init discord-webhook                Set up Discord webhook
  pygmy init ntfy                           Set up ntfy push notifications
  pygmy enable <BACKEND>                    Enable a configured backend
  pygmy disable <BACKEND>                   Disable a configured backend
  pygmy status                              Show configured backends
  pygmy self update                         Update to latest version
  pygmy self update --check                 Check for updates without installing
  pygmy usage                               Show this reference

OPTIONS
  --topic <NAME>   Topic name (Telegram/Discord message prefix / ntfy title)

BACKENDS
  telegram         Telegram Bot API with channel (requires bot + channel)
  discord-webhook  Discord webhook (just a URL, messages prefixed with [topic])
  ntfy             ntfy push notifications (topic + optional token, title = pygmy topic)

MESSAGES
  Messages are parsed as Markdown.
  Telegram: converted to HTML (bold, italic, code, links, blockquotes, lists).
  Discord: sent as-is (Discord renders Markdown natively).
  ntfy: sent as Markdown (rendered in web app; mobile shows raw text).
  Long messages are automatically split (4096 for Telegram/ntfy, 2000 for Discord).

CONFIG
  {config_dir}/config.toml       Backend credentials and enabled/disabled state

SETUP
  Run `pygmy init telegram`, `pygmy init discord-webhook`, or `pygmy init ntfy` for guided setup.
  You can configure multiple — messages are sent to all enabled backends.
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
