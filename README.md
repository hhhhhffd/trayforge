<p align="center"><img src="assets/icon-256.png" alt="Trayforge icon" width="128" height="128"></p>

<h1 align="center">Trayforge</h1>

<p align="center"><b>64 developer tools in your Windows tray: offline, instant, private.</b></p>

<p align="center">
  <a href="https://github.com/hhhhhffd/trayforge/releases">Download</a> ·
  <a href="https://hhhhhffd.github.io/trayforge/">Website</a> ·
  <a href="docs/ARCHITECTURE.md">Architecture</a> ·
  <a href="docs/BUILD.md">Build</a>
</p>

Trayforge puts a developer toolbox one hotkey away. Press `Ctrl+Alt+D`, paste something, get
it encoded, hashed, formatted or converted, and get back to work.

## Highlights

- **64 tools** in 7 categories: encoders, hashes, formatters, converters, generators, a regex
  tester, an HTTP client, a TOTP generator, a screen color picker and more.
- **Tiny when idle.** About 13 MB of RAM while sitting in the tray: no window and no WebView2
  process exist until you open Trayforge, and the window is destroyed again when you close it.
- **Instant.** All tools run in a native Rust core, with no round trips and no server.
- **Private.** No telemetry, no accounts, no network access in any tool except the HTTP Client.
- **Tray popover or standalone window.** The popover opens next to the taskbar, remembers its
  size, can be dragged, and steps aside for an auto-hidden taskbar.
- **Keyboard first.** Global hotkey, quick switcher, search and a shortcut for every action.
- **Customizable.** Theme, accent color, glass effect, favorites, recent tools, category and tool
  order.

## Screenshots

![Dashboard](site/assets/screenshot-dashboard.png)
![A tool](site/assets/screenshot-tool.png)
![Settings](site/assets/screenshot-settings.png)

## Tools

### Encoding (8)
Base64, URL Encode / Decode, HTML Entity, JWT Decoder, Unicode Escape, Text to Binary/Hex/Octal/Decimal, Morse Code, Caesar Cipher

### Hashing (8)
Hash Generator, HMAC Generator, Password Hash, Checksum, UUID Generator, ULID Generator, Random String, Nanoid

### Formats (11)
JSON Formatter, JSON Path Query, JSON ↔ YAML, YAML Formatter, XML Formatter, XML ↔ JSON, CSV → JSON, JSON → CSV, SQL Formatter, GraphQL Formatter, TOML Parser

### Web Dev (9)
Color Converter, CSS Minifier, JS Minifier, HTML Minifier, Markdown Preview, Lorem Ipsum, Data URI, SVG Path Analyzer, Placeholder Image

### Text (12)
String Case, Regex Tester, Regex Library, String Diff, Line Sorter, Text Statistics, String Escape, Whitespace Tool, Base Converter, Unix Timestamp, Epoch Batch Converter, Slug Generator

### Dev Utils (12)
QR Code, Cron Parser, Git Branch Name, Semver Comparator, ASCII Art, HTTP Status, MIME Type, Port Lookup, IP Address, Certificate Decoder, HTTP Client, TOTP Generator

### Graphics (4)
Tailwind Colors, Color Palettes, SVG Editor, Screen Color Picker

Behavior choices, option lists and safety limits are documented in
[docs/DESIGN_NOTES.md](docs/DESIGN_NOTES.md).

## Install

Download the latest build from the [Releases](https://github.com/hhhhhffd/trayforge/releases)
page:

- **Setup (`.exe`, NSIS):** per-user install, no administrator rights needed.
- **Installer (`.msi`):** for managed deployments.
- **Portable (`trayforge.exe`):** run it from anywhere; requires the WebView2 runtime, which is
  included with Windows 10/11.

Builds are not code-signed yet, so Windows SmartScreen may show "Windows protected your PC".
Click **More info**, then **Run anyway**.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Toggle Trayforge (global, rebindable) | `Ctrl+Alt+D` |
| Search tools | `Ctrl+K` |
| Quick Switch | `Ctrl+P` |
| Process | `Ctrl+Enter` |
| Copy output | `Ctrl+Shift+C` |
| Clear all | `Ctrl+L` |
| Swap input/output | `Ctrl+Shift+S` |
| Toggle favorite | `Ctrl+B` |
| Screen color picker | `Ctrl+Shift+P` |
| Category 1-7 | `Ctrl+1` ... `Ctrl+7` |
| Settings | `Ctrl+,` |
| Close window | `Esc` |

## Build from source

Requires Rust (MSVC toolchain), Node.js with pnpm, and the WebView2 runtime. The core crate
builds and tests on any OS; the shell builds on Windows.

```powershell
cd ui; pnpm install; pnpm build; cd ..
cargo build --release -p trayforge
```

See [docs/BUILD.md](docs/BUILD.md) for installers, development mode and measuring memory.

## Architecture

- `crates/trayforge-core`: pure Rust tool implementations and contract, tested on any OS.
- `src-tauri`: Tauri 2 shell with the tray, global hotkey, on-demand window and IPC commands.
- `ui`: React + Vite + TypeScript frontend rendered in WebView2.

Details in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Privacy

Trayforge has no telemetry, no analytics and no update pings. The only tool that touches the
network is the HTTP Client, and it sends only the requests you write. Settings are stored
locally in your user profile.

## License

[MIT](LICENSE) © 2026 hhhhhffd
