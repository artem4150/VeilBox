# VailBox

VailBox is a Windows-only MVP desktop VPN client for VLESS built with Tauri 2, React, TypeScript, and Rust. It uses `xray.exe` as a bundled sidecar and routes traffic through Windows System Proxy mode only.

## Project Structure

```text
.
├─ src/                     React UI
│  ├─ components/
│  ├─ features/
│  ├─ lib/
│  ├─ pages/
│  ├─ store/
│  └─ types/
├─ src-tauri/               Rust / Tauri backend
│  ├─ bin/                  Put xray.exe here
│  ├─ capabilities/
│  ├─ icons/
│  └─ src/
└─ examples/                Sample profile payloads
```

## Architecture Overview

- Frontend
  - React + TypeScript + Vite
  - Zustand for app state
  - Sidebar layout with Dashboard, Profiles, Logs, Settings, and About
  - Polling for status/log freshness plus backend events for connection state updates
- Backend
  - `profile_store`: profile CRUD in local JSON storage
  - `settings_store`: app settings and last selected profile
  - `vless_parser`: strict `vless://` parser with validation and normalization
  - `config_builder`: typed Xray config generation from internal profile model
  - `xray_manager`: spawn/monitor/stop `xray.exe`, wait for ports, reconnect after crash
  - `proxy_manager_windows`: Windows Internet Settings registry update + WinInet refresh
  - `log_manager`: JSONL logs for app/runtime/connection/Xray stdout/stderr
  - `tray_manager`: tray menu for connect, disconnect, show window, and quit

## Supported Features

- Profile list with create, edit, delete, duplicate
- Import from `vless://` URI
- Manual profile creation
- Connect / Disconnect
- Connection states: `Disconnected`, `Connecting`, `Connected`, `Error`
- Runtime logs and last connection logs
- Raw Xray stdout/stderr capture
- Auto reconnect after unexpected Xray exit
- Minimize to tray
- Optional launch at startup
- Last selected profile persistence
- Proxy cleanup on disconnect and launch recovery
- Theme: dark / light / system

## Supported VLESS Modes

- VLESS TCP + Reality
- VLESS WS + TLS
- VLESS gRPC + TLS

## Unsupported Cases

- TUN mode
- Split tunneling
- Kill switch
- Subscriptions
- Updating Xray from network
- Multi-protocol abstraction
- Mobile builds
- Telemetry
- Login, backend sync, or cloud state
- Unsupported VLESS variants such as `kcp`, `quic`, `httpupgrade`, `h2`, or `tcp+tls`

## Threat / Risk Notes

- The app does not implement its own VPN transport. Security depends on the supplied `xray.exe` build and the imported profile.
- System Proxy mode only affects applications that respect Windows proxy settings.
- Startup proxy cleanup clears local loopback proxies that look like stale VailBox sessions. If a user intentionally uses another localhost proxy, review this behavior before shipping.
- Secrets are masked in UI and log output, but profile JSON on disk still contains real credentials because Xray needs them.
- Temporary Xray config is written under app-local runtime storage and deleted on normal disconnect. Crash recovery removes stale files on next launch.

## Local Storage

App data is stored in the Tauri app data directory for `com.vailbox.desktop`.

Files created there:

- `profiles.json`
- `settings.json`
- `runtime/session.json`
- `runtime/xray-active.json`
- `logs/app.jsonl`
- `logs/connection.jsonl`

## How Proxy Cleanup Works

1. On connect, the app starts Xray and waits for the local HTTP proxy port to accept connections.
2. Only after that it enables Windows System Proxy.
3. On disconnect, it disables the system proxy before stopping Xray.
4. On unexpected app/core termination, the next launch checks saved runtime state and clears loopback proxy settings that look like the previous VailBox session.
5. If `autoReconnect` is enabled and the previous session was active, the app can safely attempt to restore the last profile after cleanup.

## Development on Windows

Prerequisites:

- Node.js 20+
- Rust stable toolchain with Windows MSVC target
- Visual Studio Build Tools with Desktop C++ workload
- WebView2 runtime

Place Xray:

1. Download a trusted Windows build of Xray-core.
2. Rename the executable to `xray.exe`.
3. Put it at [src-tauri/bin/xray.exe](./src-tauri/bin/xray.exe).

Install dependencies:

```powershell
npm install
```

Run in development:

```powershell
npm run tauri dev
```

Build release:

```powershell
npm run tauri build
```

## Troubleshooting

- `xray.exe was not found`
  - Put the binary into `src-tauri/bin/xray.exe`.
- `Timed out waiting for the local proxy port`
  - The profile is invalid, the remote server is blocked, or Xray failed before readiness. Check the Logs page.
- `System proxy verification failed`
  - Another application may be modifying Windows proxy settings at the same time.
- The app closes instead of going to tray
  - Enable `Minimize to tray` in Settings.
- Connect from tray does nothing
  - Select an active profile first in the Profiles page.

## Known Limitations

- This repository assumes a Windows environment and does not attempt cross-platform support.
- System Proxy mode does not cover traffic from applications that ignore proxy settings.
- The tray and proxy code target Tauri 2 APIs and should be rechecked against the exact crate versions used in your environment.
- In this workspace I could not run `cargo check` or `npm build` because the Rust toolchain is not installed in the shell environment, so final API-level verification must be done locally after installing the toolchain.

## Version 2 Improvements

- Add TUN mode with explicit driver and privilege handling
- Add subscription management and profile groups
- Add connection latency tests and richer diagnostics
- Add selective auto-reconnect policy and exponential backoff tuning
- Add signed Xray bundle management and upgrade workflow
