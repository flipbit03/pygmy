# pygmy — Development Guide

## Workflow

**Never commit directly to main.** Always create a branch and open a PR.

## Quick Reference

```bash
cargo build                  # Build
cargo test                   # Run tests
cargo clippy -- -D warnings  # Lint (warnings are errors)
cargo fmt --check            # Check formatting
cargo install --path .       # Install locally for testing
```

Before pushing, always verify: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Architecture

Single binary crate. No workspace, no sub-crates.

```
src/
├── main.rs           # Clap CLI entry point
├── config.rs         # ~/.config/pygmy/config.toml
├── telegram.rs       # Raw reqwest calls to Telegram Bot API (2 endpoints, no SDK)
├── discord.rs        # Raw reqwest call to Discord webhook API (1 endpoint)
├── ntfy.rs           # Raw reqwest call to ntfy API (1 endpoint, optional auth)
├── markdown.rs       # pulldown-cmark event filtering → Telegram HTML subset
├── send.rs           # Multi-backend fan-out, topic prefix, message chunking
├── init.rs           # Guided interactive setup (telegram / discord-webhook)
├── self_update.rs    # Binary or cargo self-update
├── version_check.rs  # GitHub releases version cache (24h TTL)
├── usage.rs          # LLM-friendly compact help
└── pygmy_claude_snippet.md  # CLAUDE.md snippet printed by `pygmy init` (included via include_str!)
```

## Key Design Decisions

**Multi-backend architecture**: pygmy supports multiple notification backends (Telegram, Discord webhook, ntfy). Config has optional sections for each backend with an `enabled` flag. The send flow fans out to all enabled backends — returns success if any succeed, warns on stderr for failures.

**Telegram API**: Only 2 endpoints — `sendMessage`, `getUpdates`. Direct `reqwest` calls, no Telegram SDK. Keep it this way.

**Discord Webhook API**: Single POST to the webhook URL with `{ "content": "..." }`. Messages are prefixed with `**[topic-name]**` on a separate line. Discord renders Markdown natively, so raw markdown is sent as-is (no HTML conversion). Message limit is 2000 chars.

**ntfy API**: Single POST to `{server}/{topic}` with raw markdown body. Headers: `Markdown: yes`, `Title: {pygmy-topic}`, and optional `Authorization: Bearer {token}`. Stateless — no topic creation or caching needed. Pygmy's `--topic` maps to the `Title` header. The ntfy topic in config is the "channel" (analogous to a Telegram group). Message limit is 4096 bytes.

**Markdown → HTML (Telegram only)**: `pulldown-cmark` event-based parser, filtered to emit only Telegram-supported tags (`<b>`, `<i>`, `<s>`, `<code>`, `<pre>`, `<a>`, `<blockquote>`). Unsupported elements degrade to plain text (headings → bold, lists → unicode bullets). Discord gets raw markdown.

**Topic handling**: For Telegram and Discord, topics are a bold `[topic-name]` prefix on the message. For ntfy, topics map to the `Title` header on the notification. All backends are stateless with respect to topics.

**Config vs cache**: `~/.config/pygmy/` for user-managed config (backend credentials, enabled state). `~/.cache/pygmy/` for auto-managed data (version_check.json). Respects `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`.

**Enable/disable**: Each backend section has an `enabled` field (defaults to `true` for backward compat). `pygmy enable/disable <backend>` toggles this without removing credentials. `pygmy status` shows what's configured.

**Stdin detection**: Positional args take priority. Only read stdin when `--stdin` is passed or when no positional args and stdin is not a terminal. This matters because agents (Claude Code Bash tool) run with stdin as a pipe.

**Version scheme**: Repo always at `0.0.0`. CI/CD patches the real version from the git tag at release time via `sed`. The `binary-release` feature flag switches self-update from `cargo install` to GitHub Releases binary download.

## Backend Gotchas

### Telegram
- Bot needs **admin** on the channel to post messages. Add the bot as an admin when creating the channel.
- `getUpdates` also returns `my_chat_member` events (when bot is added to a channel) — `init` checks both `message` and `my_chat_member` to discover channels.
- Telegram message limit is 4096 chars. `send.rs` chunks on line boundaries.

### Discord Webhook
- Webhook URLs look like `https://discord.com/api/webhooks/{id}/{token}` — validated during init.
- No bot setup, no permissions, no admin — just create a webhook in channel settings and paste the URL.
- Discord message limit is 2000 chars. `send.rs` chunks on line boundaries.
- Discord renders Markdown natively, so no conversion is needed.

### ntfy
- On the public `ntfy.sh` server, the topic name is effectively a password — anyone who knows it can read/write. Init warns users to pick hard-to-guess names.
- Markdown rendering is web app only — mobile apps show raw text, which is still readable.
- Message limit is 4096 bytes. `send.rs` chunks on line boundaries (same as Telegram).
- Optional bearer token auth for self-hosted instances with ACLs. Public ntfy.sh needs no auth.
- Server URL is configurable, defaults to `https://ntfy.sh` during init.

## CI/CD

- `ci.yml`: fmt, clippy, tests, cross-platform builds (Linux x86_64/aarch64 musl, macOS aarch64)
- `release.yml`: triggered by GitHub Release publish. Patches version, builds binaries, uploads as release assets.
- `install.sh`: curl-based installer that fetches latest release binary from GitHub.
