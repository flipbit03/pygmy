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
