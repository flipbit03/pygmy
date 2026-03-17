# pygmy

[![CI](https://github.com/flipbit03/pygmy/actions/workflows/ci.yml/badge.svg)](https://github.com/flipbit03/pygmy/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/pygmy)](https://crates.io/crates/pygmy)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`pygmy` lets AI coding agents (Claude Code, etc.) send you Telegram notifications when they finish a task, hit a blocker, or need your attention. You get a ping on your phone and come back when you're ready.

Notification is unidirectional - the agent can't read your replies on Telegram, so it's just a simple "ping" to draw you back to the agent's interface, when needed.

Each agent session gets its own Telegram forum topic, so parallel sessions stay organized.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/flipbit03/pygmy/main/install.sh | sh
```

Or with Cargo:

```bash
cargo install pygmy
```

## Setup

Run `pygmy init` and follow the instructions. It will walk you through creating a Telegram bot, setting up a forum group, and testing the connection. You'll need to do this once. After that, just copy the generated config file (`~/.config/pygmy/config.toml`) to any machine where you run agents and want notifications.

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

Messages are Markdown, automatically converted to Telegram HTML. Topics are created on first use — no manual setup needed.

### What it looks like in Telegram

```
📂 Pygmy Notifications (Telegram Group)
  ├─ 💬 CAD-1234 auth refactor
  │    "Done. PR #47 ready for review."
  │
  ├─ 💬 deploy
  │    "Build failed"
  │
  └─ 💬 investigation
       "I need your attention. Found 3 issues: ..."
```

Each topic is a separate thread with its own notifications.

## Agent integration

After setting everything up, add this to your `CLAUDE.md` (or equivalent agent instructions file). `pygmy init` prints this snippet for you at the end of setup.

~~~markdown
## Notifications (pygmy)

Use `pygmy` to notify me via Telegram. Messages are Markdown, converted to Telegram HTML.

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
