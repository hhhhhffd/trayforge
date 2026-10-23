# Microsoft Store listing (en-us)

Text for Partner Center > Submission > Store listings. Screenshots are in `screenshots/`
(2000x1240, rendered at 2x).

## Product name
Trayforge

## Short description
64 offline developer tools in your system tray: format JSON, decode JWTs, hash, encode, convert and generate, one hotkey away.

## Description
Stop opening random websites to decode a JWT, format JSON or generate a UUID. Trayforge puts 64 developer tools one hotkey away, right in your Windows system tray.

Press Ctrl+Alt+D, paste something, and get it encoded, hashed, formatted or converted. Then get back to work.

EVERYTHING RUNS LOCALLY
Every tool runs on your computer in a native Rust core. Your data never goes to someone else's server. No telemetry, no accounts, no ads.

TINY WHEN IDLE
Trayforge sits in the tray using about 12 MB of memory. The window is created when you open it and destroyed when you close it.

64 TOOLS IN 7 CATEGORIES
- Encoding: Base64, URL, HTML entities, JWT decoder, Unicode escape, binary/hex/octal, Morse, Caesar cipher
- Hashing: MD5 and SHA hashes, HMAC, password hashes, checksums, UUID, ULID, Nano ID, random strings
- Formats: JSON formatter and path query, JSON/YAML/XML/CSV converters, TOML, SQL and GraphQL formatters
- Web dev: color converter, CSS/JS/HTML minifiers, Markdown preview, data URIs, SVG paths, placeholder images
- Text: string case, regex tester and library, diff, line sorter, statistics, Unix timestamps, number bases, slugs
- Dev utils: HTTP client, TOTP generator, QR codes, cron parser, semver, certificate decoder, HTTP status, MIME types
- Graphics: screen color picker, Tailwind colors, color palettes, SVG editor

BUILT FOR THE KEYBOARD
A global hotkey, a quick switcher (Ctrl+P), search (Ctrl+K) and a shortcut for every action.

REMEMBERS WHAT YOU WERE DOING
Close the window mid-task and reopen it: the same tool and the same input are back.

Trayforge is free and open source under the MIT license: https://github.com/hhhhhffd/trayforge

## What's new in this version
Trayforge 1.0: first Microsoft Store release. Launch at login now uses Windows startup apps, so you can also manage it in Settings > Apps > Startup.

## Product features (one per line, up to 20)
64 developer tools in 7 categories
Opens from the tray or with a global hotkey (Ctrl+Alt+D)
Every tool runs locally on your PC; only the HTTP Client goes online
About 12 MB of memory while idle in the tray
JSON, YAML, XML, CSV, TOML, SQL and GraphQL formatters and converters
JWT decoder, Base64, URL and HTML encoders
MD5 and SHA hashes, HMAC, UUID and ULID generators
Regex tester with a pattern library
HTTP client for quick API requests
TOTP generator for testing 2FA, with QR import
QR code generator, cron parser, certificate decoder
Screen color picker and color converter
Quick switcher, search and keyboard shortcuts for every action
Tray popover or standalone window
Remembers your tool and input when you reopen the window
Light and dark themes with Acrylic and Mica effects
Free and open source (MIT)

## Screenshot captions
1-dashboard.png: All 64 tools grouped by category
2-json.png: Format, validate and minify JSON
3-jwt.png: Decode and inspect JSON Web Tokens
4-hash.png: Every common hash at once
5-quickswitch.png: Jump to any tool with Ctrl+P
6-settings.png: Launch at login, hotkey, appearance and more

## Search terms (up to 7)
devtools
JSON formatter
JWT decoder
base64
hash generator
uuid generator
regex tester

## Other fields
- Category: Developer tools
- Privacy policy URL: https://github.com/hhhhhffd/trayforge/blob/main/PRIVACY.md
- Website: https://hhhhhffd.github.io/trayforge/
- Support contact: https://github.com/hhhhhffd/trayforge/issues
- Copyright: © 2026 hhhhhffd
- Additional system requirements: Microsoft Edge WebView2 Runtime (included in Windows 11 and current Windows 10)
- Pricing: free
- Age ratings questionnaire: no user interaction, no shared location, no purchases, no objectionable content

## Restricted capability justification (runFullTrust)
Trayforge is a desktop (Win32) app built with Tauri and packaged as MSIX. It needs runFullTrust to run as a regular desktop process: it shows a system tray icon, registers a global hotkey (Ctrl+Alt+D) to open its window, creates a topmost screen overlay for its color picker, and renders its UI with Microsoft Edge WebView2. It does not install drivers or services and needs no administrator rights.
