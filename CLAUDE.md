# pygmy — Development Guide

## Quick Reference

```bash
cargo build                  # Build
cargo test                   # Run tests (13 unit tests)
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
├── markdown.rs       # pulldown-cmark event filtering → Telegram HTML subset
├── send.rs           # Topic resolution, stale topic recovery, message chunking
├── init.rs           # Guided interactive setup
├── self_update.rs    # Binary or cargo self-update
├── version_check.rs  # GitHub releases version cache (24h TTL)
├── usage.rs          # LLM-friendly compact help
└── pygmy_claude_snippet.md  # CLAUDE.md snippet printed by `pygmy init` (included via include_str!)
```

## Key Design Decisions

**Telegram API**: Only 3 endpoints — `sendMessage`, `createForumTopic`, `getUpdates`. Direct `reqwest` calls, no Telegram SDK. Keep it this way.

**Markdown → HTML**: `pulldown-cmark` event-based parser, filtered to emit only Telegram-supported tags (`<b>`, `<i>`, `<s>`, `<code>`, `<pre>`, `<a>`, `<blockquote>`). Unsupported elements degrade to plain text (headings → bold, lists → unicode bullets).

**Topic cache**: Telegram's `createForumTopic` always creates a new topic even if one with the same name exists (duplicates). The cache in `~/.cache/pygmy/topics.toml` maps topic names to `message_thread_id`. If a cached topic is deleted on Telegram's side, `send.rs` detects the "thread not found" error, evicts the cache entry, recreates the topic, and retries.

**Config vs cache**: `~/.config/pygmy/` for user-managed config (bot token, group ID). `~/.cache/pygmy/` for auto-managed data (topics.toml, version_check.json). Respects `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`.

**Stdin detection**: Positional args take priority. Only read stdin when `--stdin` is passed or when no positional args and stdin is not a terminal. This matters because agents (Claude Code Bash tool) run with stdin as a pipe.

**Version scheme**: Repo always at `0.0.0`. CI/CD patches the real version from the git tag at release time via `sed`. The `binary-release` feature flag switches self-update from `cargo install` to GitHub Releases binary download.

## Telegram Bot Gotchas

- Bots have **privacy mode** on by default — they only see `/commands` in groups, not regular messages. The `init` flow tells users to send `/start`.
- Bot needs **admin** with **"Manage Topics"** permission to create forum topics.
- `getUpdates` also returns `my_chat_member` events (when bot is added to group) — `init` checks both `message` and `my_chat_member` to discover groups.
- Telegram message limit is 4096 chars. `send.rs` chunks on line boundaries.

## CI/CD

- `ci.yml`: fmt, clippy, tests, cross-platform builds (Linux x86_64/aarch64 musl, macOS aarch64)
- `release.yml`: triggered by GitHub Release publish. Patches version, builds binaries, uploads as release assets.
- `install.sh`: curl-based installer that fetches latest release binary from GitHub.
