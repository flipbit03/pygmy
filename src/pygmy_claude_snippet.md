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
