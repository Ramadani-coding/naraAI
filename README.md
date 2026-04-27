# NARA

NARA is a Windows-first AI desktop assistant prototype based on `PRD.txt`.
It combines a futuristic HUD, a local daemon, Codex CLI integration, safe command approvals, workspace awareness, git review helpers, local event logs, and push-to-talk voice UX.

Current version: `0.1.1`

## What Is Included

- Tauri + React + TypeScript HUD in `apps/nara-hud`
- Rust daemon scaffold in `crates/nara-daemon`
- Shared protocol, policy, tools, Codex bridge, and store crates
- Local API on `127.0.0.1:44731`
- WebSocket event stream on `/events`
- Workspace selector flow
- Text prompt flow to Codex CLI
- Command approval flow
- Git status and git diff endpoints
- JSONL event logging under `%APPDATA%\NARA\logs`
- Browser/WebView speech recognition and speech synthesis hooks for MVP voice UX

## Prerequisites

- Windows 10 22H2 or newer
- WebView2 Runtime
- Node.js 22+
- Rust toolchain with Cargo
- Git
- Codex CLI logged in and available on `PATH`

This machine currently has Node, Git, and Codex CLI available. Rust/Cargo must be installed before building the daemon or Tauri shell.

## Development

Install dependencies:

```powershell
npm.cmd install
```

Run the HUD only:

```powershell
npm.cmd run dev:hud
```

Run the daemon after installing Rust:

```powershell
cargo run -p nara-daemon
```

Run both together:

```powershell
npm.cmd run dev
```

Run the Tauri HUD after installing Rust:

```powershell
npm.cmd run tauri:dev --workspace apps/nara-hud
```

## API Snapshot

- `GET /api/status`
- `POST /api/workspace`
- `POST /api/sessions/:id/prompt`
- `POST /api/tools/command/request`
- `POST /api/approvals/:id/decision`
- `GET /api/git/status`
- `GET /api/git/diff`
- `GET /events`

## MVP Notes

Wake word detection is intentionally not enabled in v0.1. Push-to-talk voice is implemented in the HUD with the browser/WebView speech APIs where available. The daemon exposes voice event placeholders so the native voice pipeline can replace the HUD fallback later.
