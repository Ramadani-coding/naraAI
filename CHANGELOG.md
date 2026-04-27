# Changelog

All notable changes to NARA will be documented in this file.

The project follows semantic versioning.

## [0.1.3] - 2026-04-27

### Fixed

- Avoid closing connecting HUD WebSockets during dev cleanup to reduce browser console noise.
- Add a 60-second Codex timeout so prompts cannot leave the HUD stuck in `thinking`.
- Surface `codex.timeout` in the HUD and clear the busy state.

### Added

- Add `POST /api/diagnostics/open-terminal` to open a PowerShell live log terminal.
- Add HUD `Open Logs Terminal` action for mentoring/debugging daemon, Codex, and event-log issues.

## [0.1.2] - 2026-04-27

### Fixed

- Stabilize HUD daemon connection badge by preventing stale WebSocket close events from overriding active connections.
- Avoid unnecessary WebSocket reconnects caused by unstable React effect dependencies in development mode.

## [0.1.1] - 2026-04-27

### Fixed

- Detect Codex CLI correctly on Windows by falling back to `codex.cmd` and `codex.exe`.
- Commit Rust workspace lockfile for reproducible daemon builds.
- Format Rust workspace after the first successful daemon compile.

## [0.1.0] - 2026-04-27

### Added

- Initial NARA MVP scaffold based on `PRD.txt`.
- Tauri + React + TypeScript HUD shell.
- Rust daemon with Axum HTTP API and WebSocket event stream.
- Codex CLI bridge for workspace prompt execution.
- Workspace selection, command approval, git status, and git diff flows.
- Strict policy layer for safe, approval-required, and blocked actions.
- JSONL event logging under `%APPDATA%\NARA\logs`.
- Push-to-talk voice UX fallback through WebView/browser speech APIs.
- Architecture, security, voice, roadmap, and Windows setup documentation.
