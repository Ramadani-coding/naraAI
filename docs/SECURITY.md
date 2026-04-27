# Security

NARA uses a strict local-first policy for v0.1.

- Daemon binds to `127.0.0.1` only.
- Risky tools return an approval request before execution.
- Dangerous command patterns are blocked before approval.
- System folders and credential targets are blocked.
- Event logs are JSONL and redact common secret patterns.
- Commands run in the selected workspace by default.

Risk levels:

- `safe`: read-only workspace operations such as `git_status` and `git_diff`
- `ask`: command execution, writing files, opening apps, and external access
- `block`: registry edits, deleting Windows folders, credential reads, disk formatting, and security-tool disabling
