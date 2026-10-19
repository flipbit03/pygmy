# pygmy — Development Guide

## Workflow

**Never commit directly to main.** Always create a branch and open a PR.

**After every change**, re-check and update documentation — including `README.md` (which serves as the ad-hoc user-facing docs) and this `CLAUDE.md`. Keep them in sync with the code.

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
├── attachment.rs     # --file validation (exists/readable), per-backend size limits
├── config.rs         # ~/.config/pygmy/config.toml
├── telegram.rs       # Raw reqwest calls to Telegram Bot API (3 endpoints, no SDK)
├── discord.rs        # Raw reqwest calls to Discord webhook API (1 endpoint: JSON or multipart)
├── ntfy.rs           # Raw reqwest calls to ntfy API (POST message / PUT attachment, optional auth)
├── markdown.rs       # pulldown-cmark event filtering → Telegram HTML subset
├── send.rs           # Multi-backend fan-out, topic prefix, message chunking, file captions
├── init.rs           # Guided interactive setup (telegram / discord-webhook)
├── self_update.rs    # Binary or cargo self-update
├── version_check.rs  # GitHub releases version cache (24h TTL)
├── usage.rs          # LLM-friendly compact help
└── pygmy_claude_snippet.md  # CLAUDE.md snippet printed by `pygmy init` (included via include_str!)
```

## Key Design Decisions

**Multi-backend architecture**: pygmy supports multiple notification backends (Telegram, Discord webhook, ntfy). Config has optional sections for each backend with an `enabled` flag. The send flow fans out to all enabled backends — returns success if any succeed, warns on stderr for failures.

**Telegram API**: Only 3 endpoints — `sendMessage`, `sendDocument`, `getUpdates`. Direct `reqwest` calls, no Telegram SDK. Keep it this way.

**Discord Webhook API**: Single POST to the webhook URL with `{ "content": "..." }`. Messages are prefixed with `**[topic-name]**` on a separate line. Discord renders Markdown natively, so raw markdown is sent as-is (no HTML conversion). Message limit is 2000 chars.

**ntfy API**: Single POST to `{server}/{topic}` with raw markdown body. Headers: `Markdown: yes`, `Title: {pygmy-topic}`, and optional `Authorization: Bearer {token}`. Stateless — no topic creation or caching needed. Pygmy's `--topic` maps to the `Title` header. The ntfy topic in config is the "channel" (analogous to a Telegram group). Message limit is 4096 bytes.

**Attachments (`--file`)**: Repeatable `--file <PATH>`. Files are validated up front in `main.rs` (exists, regular file, openable) via `attachment::Attachment::open`; bytes are only read when a backend uploads. With files, the message is optional and is the caption of the first file; later files get the short topic-only caption. With `--file`, stdin is read only on explicit `--stdin` (agents run with stdin as a pipe). `send::plan_captions` decides caption handling: if the full caption is over the backend's limit (Telegram 1024, Discord 2000, ntfy 4096), the text goes through the normal chunked message path first and all files get the short caption. Size limits live in `attachment.rs` (decimal MB, conservative): Telegram 50 MB, Discord 10 MB, ntfy 15 MB. A file over the limit fails that backend before anything is sent to it (no zipping/splitting); the usual fan-out rule applies. reqwest's `multipart` feature is on for this.

- Telegram: `sendDocument` multipart (`chat_id`, `document`, `caption`, `parse_mode=HTML`), caption keeps the `[topic]` prefix.
- Discord: multipart POST to the webhook URL with `payload_json` (`{"content": ...}`) and `files[0]`. One webhook message per file, so the per-message upload limit never adds up across files.
- ntfy: `PUT {server}/{topic}` with the raw bytes as the body; headers `Filename`, `Title` (pygmy topic), optional `Message` (caption) + `Markdown: yes`, optional bearer auth. Non-ASCII or multiline header values are sent as RFC 2047 `=?UTF-8?B?...?=` encoded words (ntfy decodes them).

**Markdown → HTML (Telegram only)**: `pulldown-cmark` event-based parser, filtered to emit only Telegram-supported tags (`<b>`, `<i>`, `<s>`, `<code>`, `<pre>`, `<a>`, `<blockquote>`). Unsupported elements degrade to plain text (headings → bold, lists → unicode bullets). Discord gets raw markdown.

**Topic handling**: For Telegram and Discord, topics are a bold `[topic-name]` prefix on the message. For ntfy, topics map to the `Title` header on the notification. All backends are stateless with respect to topics.

**Config vs cache**: `~/.config/pygmy/` for user-managed config (backend credentials, enabled state). `~/.cache/pygmy/` for auto-managed data (version_check.json). Respects `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`.

**Enable/disable**: Each backend section has an `enabled` field (defaults to `true` for backward compat). `pygmy enable/disable <backend>` toggles this without removing credentials. `pygmy status` shows what's configured.

**Stdin detection**: Positional args take priority. Only read stdin when `--stdin` is passed or when no positional args and stdin is not a terminal. This matters because agents (Claude Code Bash tool) run with stdin as a pipe.

**Version scheme**: Repo always at `0.0.0`. CI/CD patches the real version from the git tag at release time via `sed`. The `binary-release` feature flag switches self-update from `cargo install` to GitHub Releases binary download.

## Backend Gotchas

### Telegram
- Bot needs **admin** on the channel to post messages. Add the bot as an admin when creating the channel.
- `getUpdates` returns `channel_post` (messages in the channel) and `my_chat_member` events (when bot is added as admin) — `init` checks both to discover channels.
- Telegram message limit is 4096 chars. `send.rs` chunks on line boundaries.
- `sendDocument` caption limit is 1024 chars; bot upload limit is 50 MB.

### Discord Webhook
- Webhook URLs look like `https://discord.com/api/webhooks/{id}/{token}` — validated during init.
- No bot setup, no permissions, no admin — just create a webhook in channel settings and paste the URL.
- Discord message limit is 2000 chars. `send.rs` chunks on line boundaries.
- Discord renders Markdown natively, so no conversion is needed.
- Webhook file uploads are limited to 10 MB per file (no boost).

### ntfy
- On the public `ntfy.sh` server, the topic name is effectively a password — anyone who knows it can read/write. Init warns users to pick hard-to-guess names.
- Markdown rendering is web app only — mobile apps show raw text, which is still readable.
- Message limit is 4096 bytes. `send.rs` chunks on line boundaries (same as Telegram).
- Optional bearer token auth for self-hosted instances with ACLs. Public ntfy.sh needs no auth.
- Server URL is configurable, defaults to `https://ntfy.sh` during init.
- Attachments work on ntfy.sh (15 MB default). Self-hosted servers need `attachment-cache-dir` set, otherwise uploads are rejected; their size limit may differ.

## CI/CD

- `ci.yml`: fmt, clippy, tests, cross-platform builds (Linux x86_64/aarch64 musl, macOS aarch64)
- `release.yml`: triggered by GitHub Release publish. Patches version, builds binaries, uploads as release assets.
- `install.sh`: curl-based installer that fetches latest release binary from GitHub.
