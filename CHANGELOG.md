# Changelog

All notable changes to NARA will be documented in this file.

The project follows semantic versioning.

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
