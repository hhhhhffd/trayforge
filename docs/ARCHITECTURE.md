# Trayforge architecture

Trayforge is a Windows tray app: a small Rust process that owns the tray icon and the
global hotkey, and creates a WebView2 window on demand to host the React UI.

## Layout

| Path | Role |
| --- | --- |
| `crates/trayforge-core` | Pure Rust: tool contract, 58 standard tools, TOTP logic. No UI or OS deps; tested on any OS. |
| `src-tauri` | Tauri 2 shell: tray, global hotkey, lazy window, settings file, IPC commands, HTTP client, screen color picker. |
| `ui` | React + Vite + TypeScript frontend rendered in WebView2. |
| `docs/DESIGN_NOTES.md` | Deliberate behavior choices, output layouts, safety limits and known limitations. |

## Memory model

Idle target ≤ 50 MB for the whole process tree (goal 20 MB).

- On startup only the Rust process runs: tray icon + global hotkey. No window, no WebView2.
- Hotkey / tray click creates the `main` window on demand.
- Closing or hiding the window calls `destroy()`, so WebView2 processes exit.
- The app keeps running after the last window closes (`ExitRequested` is prevented); it exits only via tray menu "Quit".
- No timers or polling while idle.

## Tool contract (`trayforge-core`)

See `crates/trayforge-core/src/tool.rs`. `ToolInfo` is serialized to the UI with camelCase keys.
Option values travel as JSON: toggle → bool, picker/segmented → the selected choice string,
slider → number, stepper → integer, text field / color → string. Missing values fall back to
declared defaults; unknown ids or wrong types are rejected (`InvalidOptions`).

`ToolOutput` JSON: `{"type":"text","value":"…","metadata":{…}}` or
`{"type":"multiple","items":[{"label","value","isWarning","isImage"}]}`.
`ToolError` JSON: `{"kind":"invalidInput","message":"…"}` (`emptyInput`/`timeout` have no message).

## IPC commands (`src-tauri` → `ui`)

| Command | Args | Returns |
| --- | --- | --- |
| `list_tools` | — | `ToolInfo[]` in sidebar order |
| `process_tool` | `{ toolId, input, options, reverse }` | `ToolOutput`; rejects with `{ kind, message, display }` where `display` is the user-facing string |
| `get_settings` | — | settings JSON object (`{}` on first run) |
| `save_settings` | `{ settings }` | `null`; shell re-registers the global hotkey from `settings.globalHotkey` |
| `http_request` | `{ request: { method, url, headers: [[k,v]], body: string \| null } }` | `{ status, statusText, headers: [[k,v]], body, durationMs, sizeBytes }` |
| `totp_generate` | `{ secret, algorithm: "SHA1"\|"SHA256"\|"SHA512", digits, period }` | `{ code, remainingSeconds }` |
| `totp_parse_uri` | `{ uri }` | `{ account, issuer, secret, algorithm, digits, period }` |
| `decode_qr_image` | `{ imageBase64 }` (PNG/JPEG bytes) | decoded text |
| `pick_screen_color` | — | `"#RRGGBB"` (also copied to clipboard by the UI) |
| `read_text_file` / `write_text_file` | `{ path }` / `{ path, contents }` | file text / `null` |
| `hide_window` | — | destroys the main window |
| `set_launch_at_login` | `{ enabled }` | `null` |
| `reset_window_state` | — | `null`; forgets the resized popover size and re-places an open popover |

Events from the shell: `navigate` (`"settings"`) and `closing` (the window is destroyed
120 ms later; the UI fades out and flushes settings).

The popover opens next to the taskbar at its tray end (edge detected from the
`Shell_TrayWnd`/`Shell_SecondaryTrayWnd` windows), 12 px from it, at the size it was last
resized to (`popover-state.json`). While it is open, a 25 ms poll moves it aside when an
auto-hidden taskbar slides out over it and back when the taskbar hides; the thread ends
with the window. Headers marked `data-drag` move the window.

File dialogs use `@tauri-apps/plugin-dialog`; clipboard uses `navigator.clipboard`.
Drag & drop uses HTML5 events (window `dragDropEnabled: false`).

## Tool ids

Standard tools (process via `process_tool`), sidebar order:

- Encoding: `base64`, `url-encode`, `html-entity`, `jwt-decoder`, `unicode-escape`, `text-to-binary`, `morse-code`, `caesar-cipher`
- Hashing: `hash-generator`, `hmac-generator`, `password-hash`, `checksum`, `uuid-generator`, `ulid-generator`, `random-string`, `nanoid`
- Formats: `json-formatter`, `json-path`, `json-yaml`, `yaml-formatter`, `xml-formatter`, `xml-json`, `csv-to-json`, `json-to-csv`, `sql-formatter`, `graphql-formatter`, `toml-parser`
- Web Dev: `color-converter`, `css-minifier`, `js-minifier`, `html-minifier`, `markdown-preview`, `lorem-ipsum`, `data-uri`, `svg-path-analyzer`, `placeholder-image`
- Text: `string-case`, `regex-tester`, `regex-library`, `string-diff`, `line-sorter`, `text-statistics`, `string-escape`, `whitespace-tool`, `base-converter`, `unix-timestamp`, `epoch-batch-converter`, `slug-generator`
- Dev Utils: `qr-code`, `cron-parser`, `git-branch-name`, `semver-comparator`, `ascii-art`, `http-status`, `mime-type`, `port-lookup`, `ip-address`, `certificate-decoder`

Custom-UI tools (`custom_ui: true`, no processor, dedicated panel in `ui`):
`http-client`, `totp-generator` (Dev Utils), `tailwind-colors`, `color-palettes`, `svg-editor`, `screen-color-picker` (Graphics).

## Keyboard shortcuts

| Action | Windows |
| --- | --- |
| Toggle Trayforge (global, rebindable) | `Ctrl+Alt+D` |
| Search tools | `Ctrl+K` |
| Quick Switch | `Ctrl+P` |
| Process | `Ctrl+Enter` |
| Copy output | `Ctrl+Shift+C` |
| Clear all | `Ctrl+L` |
| Swap input/output | `Ctrl+Shift+S` (`Ctrl+S` would collide with save conventions) |
| Toggle favorite | `Ctrl+B` |
| Screen color picker | `Ctrl+Shift+P` |
| Category 1…7 | `Ctrl+1` … `Ctrl+7` |
| Settings | `Ctrl+,` |
| Close window | `Esc` (when no overlay is open) |
