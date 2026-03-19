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

**Usage:**
```bash
pygmy --topic "<topic>" "<message>"
pygmy --topic "<topic>" --stdin <<'EOF'
<long message>
EOF
```

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
