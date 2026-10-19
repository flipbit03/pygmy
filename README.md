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

### Sending files

```bash
# Send a file with a caption
pygmy --topic "nightly" --file ./report.pdf "Here's the report"

# Several files, no caption
pygmy --topic "nightly" --file ./build.log --file ./screenshot.png
```

`--file` can be repeated. When at least one file is given, the message is optional and becomes the caption of the first file (other files get just the `[topic]` prefix). Files are checked up front: a missing or unreadable path fails before anything is sent. With `--file`, stdin is only read when `--stdin` is passed explicitly.

| Backend  | Max file size | Caption limit | How it's sent |
|----------|---------------|---------------|---------------|
| Telegram | 50 MB         | 1024 chars    | `sendDocument` |
| Discord  | 10 MB         | 2000 chars    | webhook multipart upload, one message per file |
| ntfy     | 15 MB*        | 4096 bytes    | `PUT` attachment |

\* ntfy.sh default. Self-hosted ntfy servers need [attachments enabled](https://docs.ntfy.sh/config/#attachments), and may use a different limit.

If the caption is longer than the backend's limit, the text is sent as a normal message first, then the files with a short `[topic]` caption. A file over a backend's size limit fails that backend with an error naming the size and the limit; the other backends still send (pygmy succeeds if any backend succeeds).

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

After setup, add this to your `CLAUDE.md` (or equivalent agent instructions file). `pygmy init` prints this snippet for you.

~~~markdown
## Notifications (pygmy)

Use `pygmy` to notify me. Messages are Markdown, sent to all enabled backends.

**When to use:**
- When I say "ping me", "notify me", or "let me know when done"
- When completing a long-running task while I might be away
- When you hit a blocker that requires my input
- When I ask you to send me a file (report, log, screenshot, etc.)

**Usage:**
```bash
pygmy --topic "<topic>" "<message>"
pygmy --topic "<topic>" --stdin <<'EOF'
<long message>
EOF
pygmy --topic "<topic>" --file ./report.pdf "<optional caption>"
```

`--file` can be repeated. Max file size: 10 MB (Discord), 15 MB (ntfy), 50 MB (Telegram).

Pick a short, descriptive topic name at the start of the session and reuse it for all messages.
Don't notify for every small step — only meaningful milestones or blockers (or when asked to).
~~~

## Self-update

```bash
pygmy self update          # update to latest
pygmy self update --check  # just check
```

## License

MIT
