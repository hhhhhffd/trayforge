# Design notes

Deliberate behavior choices in the tools, so that output differences are never a surprise.
Everything here is covered by tests in `crates/trayforge-core/tests`.

## Option lists

- **Encoding.** Base64 line wrap: `None`, `64 chars`, `76 chars`; the decoder trims outer
  whitespace, drops CR/LF, auto-detects the URL-safe alphabet and restores padding (inner
  spaces make the input invalid). URL encode modes: `Query Parameter`, `Full URL`,
  `Path Component`; spaces as `%20` or `+` (`+` is read as a space when decoding). HTML entities:
  `Named` (Latin-1 plus common symbols, other characters fall back to numeric), `Decimal`,
  `Hexadecimal`. Unicode escape: `\uXXXX`, `\UXXXXXXXX`, `\xXX` (falls back to `\uXXXX` above
  U+00FF), `U+XXXX`. Text to Binary: Binary/Hexadecimal/Octal/Decimal with Space/Comma/Newline/None
  separators. Morse: `·/−` or `./-`; with spacing off, letters are 1 space and words 3 spaces
  apart; characters without a Morse code are an error. Caesar: rotation 1..25, optional mapping
  view.
- **Hashing.** HMAC output `Hex` or `Base64`. UUID versions v1-v5 and v7; formats `Standard`,
  `Uppercase`, `No Hyphens`, `Braces`, `URN`; v3/v5 take the name from the input and the
  namespace (`DNS`, `URL`, `OID`, `X.500`) from the picker. A non-empty ULID input is decoded into
  timestamp, Unix milliseconds and random component. Random String presets: `Password`,
  `Alphanumeric`, `Letters`, `Numbers`, `Hex`, `Custom` (the checkboxes apply to `Custom` only).
  Nanoid alphabets: default (64), URL safe (64), hex (16), lowercase + digits (36); a non-empty
  custom alphabet overrides the picker and duplicates are dropped; collision odds use the
  birthday bound.
- **Password Hash** prints `Full Hash String` (`$pbkdf2-sha256$rounds$salt$hash`, unpadded
  Base64), hex `Hash`/`Salt`, `Iterations` and `Key Length`; the random salt is 16 bytes.
- **Text.** String Case has 12 formats, String Escape 9 languages, Unix Timestamp 10 time
  zones. Regex Library is a static list of 21 patterns labelled `Category: Name`; when the chosen
  pattern is outside the chosen category, the category's first pattern is used.
- **Web dev.** Placeholder Image formats: `SVG (Data URI)`, `PNG (Data URI)`, `HTML <img> tag`,
  `CSS background`; sizes 1-2000 px; colors accept hex, `rgb()`, `hsl()` and basic names and
  anything else is rejected (this also prevents attribute injection through color strings);
  PNG text uses a built-in 5x7 bitmap font. The `Dimensions` text style renders the current
  size; diagonal lines are drawn when the text style is `None`. Data URI MIME type: auto-detect
  (svg, html, xml, json, js, css, plain) plus 8 fixed types; decode also accepts `url(...)`
  wrappers and unpadded Base64.
- **Dev utils.** QR output `Preview`, `Base64 PNG`, `Data URL` with a 4-module quiet zone and
  integer-scaled modules. Cron next runs and descriptions are computed in UTC and the header says
  so; `@hourly`/`@daily`/`@weekly`/`@monthly`/`@yearly` are accepted; 5 fields, or 6 with the
  seconds toggle. Git Branch Name separators `-`, `_`, `.`; the date is UTC. ASCII Art:
  `Standard` is the FIGlet standard font, the other fonts are a built-in 5x7 bitmap font
  (letters, digits and `. , ! ? - _ : ' / + = ( ) *`; everything else renders as `?`).
  HTTP Status, MIME Type and Port Lookup use built-in tables (codes 100-511, 87 MIME entries,
  81 ports). IP Address operations: `Analyze`, `Convert to Binary`, `Convert to Decimal`,
  `Subnet Calculator`, `Validate Only`; parsing uses the standard library (leading zeros are
  rejected, any valid IPv6 form is accepted).

## Output layouts

- **JSON Formatter** modes `Pretty`, `Minify`, `Compact`; every syntax error reads
  `Invalid value around line L, column C. Check your input and try again`
  (`Unexpected end of JSON input ...` at end of input).
- **YAML** keeps scalar types and key order (unless Sort keys is on). Strings that would read as
  another type are single-quoted in YAML Formatter and double-quoted in JSON to YAML.
- **JSON to CSV** keeps numbers as numbers; delimiters Comma/Semicolon/Tab/Pipe; flatten
  `None`/`1 level`/`2 levels`/`All`; arrays `Join with ;`/`JSON string`/`Index columns`;
  columns are sorted. **CSV to JSON** can auto-detect the delimiter and types `true`/`false`
  and numbers without leading zeros.
- **JSON Path** supports `.key`, `['key']`, `[n]` (negative allowed), `[*]`, `.*`, `[a:b]` and
  `..`; filters are not supported. A path without wildcard, slice or `..` returns the single
  value, anything else an array.
- **XML to JSON** attribute prefix `@`/`_`/`$`/`attr_`, text key `#text`/`$value`/`_text`/`value`;
  JSON to XML has no declaration and wraps multi-key input in `<root>`. XML Formatter keeps
  text-only elements on one line and preserves declaration, comments, PIs and DOCTYPE.
- **SQL Formatter** puts clauses on new lines and indents `AND`/`OR`; keyword case
  `UPPERCASE`/`lowercase`/`Preserve`. **GraphQL Formatter** puts every whitespace-separated
  token outside parentheses on its own line, keeping `... on T` and `alias: field` joined.
- **TOML Parser** uses the `toml` crate (TOML 1.0); inline tables are re-emitted as sections
  unless Inline simple tables is on. **Markdown Preview** uses pulldown-cmark; with Allow Raw
  HTML off, raw HTML is replaced by `<!-- raw HTML omitted -->` and `javascript:`, `vbscript:`,
  `file:` and `data:` link targets are emptied.
- **JS Minifier** `Remove console.log` matches balanced parentheses and ignores strings, so
  nested calls such as `console.log(f(1))` are removed completely.
- **Color Converter** prints `HEX`/`RGB`/`HSL`/`HSV` lines, `RGBA`/`HSLA` and 8-digit HEX with
  Include alpha, and accepts named colors and bare 6/8-digit hex.
- **Certificate Decoder** uses `x509-parser`; input `Auto-detect`, `PEM` or `Base64 (DER)`;
  dates are UTC; extensions shown: Basic Constraints, Key Usage, Extended Key Usage, Subject
  Alt Names.
- **SVG Path Analyzer** prints a summary (counts, bounding box), a breakdown, the simplified
  absolute path and an optional `<svg>` with a viewBox from the bounding box.
- **JWT Decoder** prints a plain-text report (header, payload, `iat`/`nbf`/`exp` in UTC, time
  until expiration, `aud`). **Semver Comparator** accepts a `v` prefix, a `help` input shows the
  help text, and bumps clear pre-release and build metadata.

## Safety limits

The release build aborts on panics, so hostile input is bounded:

- XML and GraphQL reject nesting deeper than 256 levels; beautifiers and the XML/GraphQL
  printers stop indenting below 32 levels.
- JSON Path fails with `Too many matches` above 100,000 nodes.
- Option values outside a stepper or slider range are rejected.
- `.inf` and `.nan` in YAML to JSON are errors; signed digits (`%+1`, `&#+65;`) are not decoded.
- Placeholder Image sizes are limited to 1-2000 px.

## Known limitations

- Integers beyond `u64` are not preserved by the JSON tools (`serde_json` `arbitrary_precision`
  would change number handling workspace-wide).
- Named HTML entities cover Latin-1 and common symbols only.
- Cron evaluation is UTC only; the core has no OS time zone access.
- JSON Path has no filter expressions.
- Regex Library is a fixed list; it is not user-editable.
- The screen color picker and the taskbar-aware popover placement are Windows only.
