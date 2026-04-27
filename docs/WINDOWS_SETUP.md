# Windows Setup

Install:

- WebView2 Runtime
- Node.js 22+
- Rust via rustup
- Git for Windows
- Codex CLI

Check tools:

```powershell
node --version
npm.cmd --version
cargo --version
git --version
codex.cmd --version
```

Run daemon:

```powershell
cargo run -p nara-daemon
```

Run HUD:

```powershell
npm.cmd install
npm.cmd run dev:hud
```

Open `http://127.0.0.1:5173` while developing the HUD.
