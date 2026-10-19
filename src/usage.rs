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
  pygmy --topic <TOPIC> --file <PATH> [MSG]
                                            Send a file; MSG (optional) is the caption
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
  --file <PATH>    Attach a file. Repeat for several files. With --file the
                   message is optional; stdin is read only with --stdin.
  --stdin          Read the message from stdin

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

FILES
  Each file is uploaded to every enabled backend; the message becomes the caption
  of the first file. If the caption is too long for the backend (Telegram 1024,
  Discord 2000, ntfy 4096), the text is sent as a normal message before the files.
  Size limits per file: Telegram 50 MB, Discord 10 MB, ntfy 15 MB (ntfy.sh default;
  self-hosted ntfy servers must have attachments enabled).
  A file over a backend's limit fails that backend; the others still send.

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
