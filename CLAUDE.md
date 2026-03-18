# pygmy — Development Guide

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
├── config.rs         # ~/.config/pygmy/config.toml + ~/.cache/pygmy/topics.toml
├── telegram.rs       # Raw reqwest calls to Telegram Bot API (3 endpoints, no SDK)
├── discord.rs        # Raw reqwest call to Discord webhook API (1 endpoint)
├── markdown.rs       # pulldown-cmark event filtering → Telegram HTML subset
├── send.rs           # Multi-backend fan-out, topic resolution, message chunking
├── init.rs           # Guided interactive setup (telegram / discord-webhook)
├── self_update.rs    # Binary or cargo self-update
├── version_check.rs  # GitHub releases version cache (24h TTL)
├── usage.rs          # LLM-friendly compact help
└── pygmy_claude_snippet.md  # CLAUDE.md snippet printed by `pygmy init` (included via include_str!)
```

## Key Design Decisions

**Multi-backend architecture**: pygmy supports multiple notification backends (Telegram, Discord webhook). Config has optional sections for each backend with an `enabled` flag. The send flow fans out to all enabled backends — returns success if any succeed, warns on stderr for failures.

**Telegram API**: Only 3 endpoints — `sendMessage`, `createForumTopic`, `getUpdates`. Direct `reqwest` calls, no Telegram SDK. Keep it this way.

**Discord Webhook API**: Single POST to the webhook URL with `{ "content": "..." }`. Messages are prefixed with `**[topic-name]**` on a separate line. Discord renders Markdown natively, so raw markdown is sent as-is (no HTML conversion). Message limit is 2000 chars.

**Markdown → HTML (Telegram only)**: `pulldown-cmark` event-based parser, filtered to emit only Telegram-supported tags (`<b>`, `<i>`, `<s>`, `<code>`, `<pre>`, `<a>`, `<blockquote>`). Unsupported elements degrade to plain text (headings → bold, lists → unicode bullets). Discord gets raw markdown.

**Topic handling**: For Telegram, topics map to forum threads via `createForumTopic` with caching. For Discord, topics are simply a bold `[topic-name]` prefix on the message.

**Topic cache**: Telegram's `createForumTopic` always creates a new topic even if one with the same name exists (duplicates). The cache in `~/.cache/pygmy/topics.toml` maps topic names to `message_thread_id`. If a cached topic is deleted on Telegram's side, `send.rs` detects the "thread not found" error, evicts the cache entry, recreates the topic, and retries.

**Config vs cache**: `~/.config/pygmy/` for user-managed config (backend credentials, enabled state). `~/.cache/pygmy/` for auto-managed data (topics.toml, version_check.json). Respects `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`.

**Enable/disable**: Each backend section has an `enabled` field (defaults to `true` for backward compat). `pygmy enable/disable <backend>` toggles this without removing credentials. `pygmy status` shows what's configured.

**Stdin detection**: Positional args take priority. Only read stdin when `--stdin` is passed or when no positional args and stdin is not a terminal. This matters because agents (Claude Code Bash tool) run with stdin as a pipe.

**Version scheme**: Repo always at `0.0.0`. CI/CD patches the real version from the git tag at release time via `sed`. The `binary-release` feature flag switches self-update from `cargo install` to GitHub Releases binary download.

## Backend Gotchas

### Telegram
- Bots have **privacy mode** on by default — they only see `/commands` in groups, not regular messages. The `init` flow tells users to send `/start`.
- Bot needs **admin** to create forum topics. Enable Topics in group settings first, then add the bot as admin — it gets the right permissions automatically.
- `getUpdates` also returns `my_chat_member` events (when bot is added to group) — `init` checks both `message` and `my_chat_member` to discover groups.
- Telegram message limit is 4096 chars. `send.rs` chunks on line boundaries.

### Discord Webhook
- Webhook URLs look like `https://discord.com/api/webhooks/{id}/{token}` — validated during init.
- No bot setup, no permissions, no admin — just create a webhook in channel settings and paste the URL.
- Discord message limit is 2000 chars. `send.rs` chunks on line boundaries.
- Discord renders Markdown natively, so no conversion is needed.

## CI/CD

- `ci.yml`: fmt, clippy, tests, cross-platform builds (Linux x86_64/aarch64 musl, macOS aarch64)
- `release.yml`: triggered by GitHub Release publish. Patches version, builds binaries, uploads as release assets.
- `install.sh`: curl-based installer that fetches latest release binary from GitHub.
