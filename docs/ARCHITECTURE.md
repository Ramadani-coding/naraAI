# Architecture

NARA is split into a desktop HUD and a local daemon.

```text
apps/nara-hud              React/Tauri HUD
crates/nara-daemon         Localhost API and WebSocket event stream
crates/nara-protocol       Shared JSON payloads
crates/nara-codex          Codex CLI subprocess bridge
crates/nara-tools          Workspace-aware tool runtime
crates/nara-policy         SAFE / ASK / BLOCK decision layer
crates/nara-store          Config and JSONL event logs
```

The HUD talks to the daemon through `http://127.0.0.1:44731` and `ws://127.0.0.1:44731/events`.

The daemon owns:

- workspace state
- Codex availability checks
- Codex prompt execution
- approval requests and decisions
- tool execution
- event logging

The HUD owns:

- visual status
- prompt input
- push-to-talk MVP voice UX
- text-to-speech playback
- approval presentation
- timeline rendering
