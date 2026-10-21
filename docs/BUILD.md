# Building the Windows shell

Requirements: Windows with rustup (`stable-x86_64-pc-windows-msvc`), VS 2022 Build Tools, WebView2 runtime, Node.js with pnpm.
Build from a local drive (not a `\\wsl.localhost` path).

```powershell
pnpm --dir ui install
pnpm --dir ui run build          # produces ui/dist
cargo build --release -p trayforge     # target\release\trayforge.exe
```

`custom-protocol` is a default feature of `src-tauri`, so the exe always serves `ui/dist`
from embedded assets (`http://tauri.localhost`) and never needs a dev server.
Installers: `cargo tauri build` (needs `cargo install tauri-cli`) produces the NSIS setup and the MSI under `target\release\bundle`; it runs the `ui` build itself via `beforeBuildCommand`.

Development: `cargo tauri dev` starts the Vite server (port 1420) and disables
`custom-protocol` itself.

`cargo clippy -p trayforge --no-deps -- -D warnings` lints the shell only.
`scripts/measure-memory.ps1 -Exe target\release\trayforge.exe` measures the process tree.

UI contract added by the shell: "Settings" in the tray menu emits the `navigate` event with
`"settings"`; when the window does not exist yet it is created with
`window.__TRAYFORGE_INITIAL_ROUTE__ = "settings"` so the UI can read the route on mount.
