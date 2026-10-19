# pygmy

*/ˈpɪŋ.miː/*

[![CI](https://github.com/flipbit03/pygmy/actions/workflows/ci.yml/badge.svg)](https://github.com/flipbit03/pygmy/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/pygmy)](https://crates.io/crates/pygmy)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`pygmy` lets AI coding agents (Claude Code, etc.) send you notifications when they finish a task, hit a blocker, or need your attention. You get a ping on your phone/desktop and come back when you're ready.

Supports **Telegram** (channel), **Discord** (webhooks), and **[ntfy](https://ntfy.sh)** (push notifications). Configure one or more — messages are sent to all enabled backends.

Notification is unidirectional — the agent can't read your replies, so it's just a simple "ping" to draw you back to the agent's interface.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/flipbit03/pygmy/main/install.sh | sh
```

Or with Cargo:

```bash
cargo install pygmy
```

## Setup

Run `pygmy init <backend>` and follow the instructions:

```bash
pygmy init telegram          # Telegram bot + channel
pygmy init discord-webhook   # Discord webhook
pygmy init ntfy              # ntfy push notifications (simplest)
```

You can set up both. After setup, copy the config file (`~/.config/pygmy/config.toml`) to any machine where you run agents.

### Managing backends

```bash
pygmy status                   # Show what's configured and enabled
pygmy disable telegram         # Temporarily disable a backend
pygmy enable telegram          # Re-enable it
```

## Usage

```bash
# Send a notification to a topic
pygmy --topic "CAD-1234 auth refactor" "Done. PR #47 ready for review."

# Pipe from stdin
echo "Build failed" | pygmy --topic "deploy"

# Multiline with heredoc
pygmy --topic "investigation" --stdin <<'EOF'
**Found 3 issues:**
- Missing index on `users.email`
- N+1 in the dashboard query
- Stale cache in Redis
EOF
```

Messages are Markdown. Telegram messages are converted to HTML; Discord and ntfy messages are sent as-is (both render Markdown natively).

A message that starts with `-` must come after `--` (e.g. `pygmy --topic ci -- "-3 tests failing"`), otherwise it is parsed as a flag. Long messages are split on line breaks to fit each backend (Telegram 4096, Discord 2000, ntfy 4096 bytes of rendered text).

### Sending files

```bash
# Send a file with a caption
pygmy --topic "nightly" --file ./report.pdf "Here's the report"

# Several files, no caption
pygmy --topic "nightly" --file ./build.log --file ./screenshot.png
```

`--file` can be repeated. When at least one file is given, the message is optional and becomes the caption of the first file (other files get just the `[topic]` prefix; on ntfy they get no message, so ntfy shows its default text). Files are checked up front: a missing, unreadable or empty (0-byte) path fails before anything is sent. With `--file`, stdin is only read when `--stdin` is passed explicitly.

| Backend  | Max file size | Caption limit | How it's sent |
|----------|---------------|---------------|---------------|
| Telegram | 50 MB         | 1024 bytes    | `sendDocument` |
| Discord  | 10 MB         | 2000 bytes    | webhook multipart upload, one message per file |
| ntfy     | 15 MB*        | 4096 bytes    | `PUT` attachment |

\* ntfy.sh default. Self-hosted ntfy servers need [attachments enabled](https://docs.ntfy.sh/config/#attachments), and may use a different limit.

Caption and message limits are counted in bytes of the rendered text (after the `[topic]` prefix and, for Telegram, the HTML conversion). Telegram and Discord count characters, so this is conservative: non-ASCII text is split a bit earlier than strictly needed. Sizes are decimal (1 MB = 1,000,000 bytes).

If the caption is longer than the backend's limit, the text is sent as a normal message first, then the files with a short `[topic]` caption. On ntfy, a caption containing a literal backslash-`n` (e.g. a Windows path) is also sent as a normal message first, because ntfy would turn `\n` in an attachment's message into a newline. A file over a backend's size limit fails that backend with an error naming the size and the limit; the other backends still send (pygmy succeeds if any backend succeeds).

Each backend sends its items in order (text, then file 1, file 2, ...) and stops at the first error. A failure partway through (say, file 2 of 3) reports that backend as failed, even though the earlier items were already delivered.

### What it looks like

**Telegram** — messages are prefixed with the topic name:
```
📢 Pygmy Notifications (Telegram Channel)
  ┊ [CAD-1234 auth refactor]
  ┊ Done. PR #47 ready for review.
  ┊
  ┊ [deploy]
  ┊ Build failed
```

**Discord** — messages are prefixed with the topic name:
```
#pygmy-notifications (Discord Channel)
  ┊ **[CAD-1234 auth refactor]**
  ┊ Done. PR #47 ready for review.
  ┊
  ┊ **[deploy]**
  ┊ Build failed
```

**ntfy** — topic name appears as the notification title:
```
📱 ntfy (your-pygmy-topic)
  ┌─────────────────────────────────────┐
  │ CAD-1234 auth refactor              │
  │ Done. PR #47 ready for review.      │
  ├─────────────────────────────────────┤
  │ deploy                              │
  │ Build failed                        │
  └─────────────────────────────────────┘
```

## Agent integration

After setup, add this to your `CLAUDE.md`/`AGENTS.md` (or equivalent agent instructions file). `pygmy init` prints it for you.

```markdown
## Notifications (pygmy)

Use the `pygmy` CLI to notify me (messages and files go to my phone) when I ask
to be pinged, when a long task finishes while I'm away, or when you're blocked on
my input. Run `pygmy usage` before the first use for the full reference.
```

That's it — `pygmy usage` gives your agent the full reference (when to notify, sending, files and limits, which backends are set up on this machine) in under 1,000 tokens. `pygmy --help` has the detailed flag reference for humans.

## Self-update

```bash
pygmy self update          # update to latest
pygmy self update --check  # just check
```

## License

MIT
