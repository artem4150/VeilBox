# VeilBox Third-Party Notices

Last updated: March 29, 2026

VeilBox depends on and may bundle third-party software.  
The list below highlights the main runtime components relevant to end users.

## Runtime Components

- Xray-core  
  License: MPL-2.0  
  Source: https://github.com/XTLS/Xray-core

- AmneziaWG Windows Client  
  License: MIT  
  Source: https://github.com/amnezia-vpn/amneziawg-windows-client

- Wintun  
  Source repository license: GPL-2.0 for repository contents  
  Official note: prebuilt `wintun.dll` binaries from the official distribution are released under separate distribution terms  
  Source: https://github.com/WireGuard/wintun

## Application Framework

- Tauri  
  License: Apache-2.0 OR MIT  
  Source: https://github.com/tauri-apps/tauri

- React  
  License: MIT  
  Source: https://github.com/facebook/react

- Vite  
  License: MIT  
  Source: https://github.com/vitejs/vite

## Dependency Scope

VeilBox also uses additional Rust crates, npm packages, and transitive dependencies required for building and running the application.

For complete dependency trees, see:

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`

Users distributing VeilBox binaries should review bundled dependency licenses before redistribution.

