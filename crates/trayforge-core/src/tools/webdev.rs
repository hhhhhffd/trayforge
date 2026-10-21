use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use pulldown_cmark::{html, CowStr, Event, Options as MdOptions, Parser, Tag, TagEnd};

use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

pub fn tools() -> Vec<Tool> {
    vec![
        color_converter(),
        css_minifier(),
        js_minifier(),
        html_minifier(),
        markdown_preview(),
        lorem_ipsum(),
        data_uri(),
        svg_path_analyzer(),
        placeholder_image(),
    ]
}

fn toggle(id: &'static str, label: &'static str, help: &'static str, default: bool) -> ToolOption {
    ToolOption {
        id,
        label,
        help: Some(help),
        kind: OptionKind::Toggle { default },
    }
}

fn indent_option() -> ToolOption {
    ToolOption {
        id: "indent",
        label: "Indent Spaces",
        help: Some("Indentation for beautify mode"),
        kind: OptionKind::Stepper {
            min: 1,
            max: 8,
            default: 2,
        },
    }
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Formats with at most 3 decimals and no trailing zeros.
fn num(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".into()
    } else {
        s.into()
    }
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        if b.is_ascii_alphanumeric() || b"-_.~/:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

// ---------------------------------------------------------------- Color Converter

#[derive(Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: f64,
}

const NAMED_COLORS: [(&str, u32); 17] = [
    ("black", 0x000000),
    ("white", 0xFFFFFF),
    ("red", 0xFF0000),
    ("green", 0x008000),
    ("blue", 0x0000FF),
    ("yellow", 0xFFFF00),
    ("cyan", 0x00FFFF),
    ("magenta", 0xFF00FF),
    ("gray", 0x808080),
    ("grey", 0x808080),
    ("silver", 0xC0C0C0),
    ("orange", 0xFFA500),
    ("purple", 0x800080),
    ("pink", 0xFFC0CB),
    ("brown", 0xA52A2A),
    ("navy", 0x000080),
    ("teal", 0x008080),
];

fn parse_hex(hex: &str) -> Option<Rgba> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let digits: Vec<u8> = match hex.len() {
        3 | 4 => hex
            .chars()
            .map(|c| c.to_digit(16).unwrap() as u8 * 17)
            .collect(),
        6 | 8 => hex
            .as_bytes()
            .chunks(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect(),
        _ => return None,
    };
    Some(Rgba {
        r: digits[0],
        g: digits[1],
        b: digits[2],
        a: digits.get(3).map_or(1.0, |a| f64::from(*a) / 255.0),
    })
}

fn func_args<'a>(s: &'a str, names: &[&str]) -> Option<Vec<&'a str>> {
    names.iter().find_map(|name| {
        let rest = s.strip_prefix(name)?.trim_start();
        let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
        Some(inner.split(',').map(str::trim).collect())
    })
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to8 = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to8(r), to8(g), to8(b))
}

fn parse_alpha(parts: &[&str]) -> Option<f64> {
    match parts.get(3) {
        None => Some(1.0),
        Some(a) => a.parse::<f64>().ok().map(|a| a.clamp(0.0, 1.0)),
    }
}

fn parse_color(input: &str) -> Option<Rgba> {
    let s = input.trim().to_ascii_lowercase();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(p) = func_args(&s, &["rgba", "rgb"]) {
        if p.len() != 3 && p.len() != 4 {
            return None;
        }
        let ch = |v: &str| v.parse::<f64>().ok().filter(|v| (0.0..=255.0).contains(v));
        return Some(Rgba {
            r: ch(p[0])?.round() as u8,
            g: ch(p[1])?.round() as u8,
            b: ch(p[2])?.round() as u8,
            a: parse_alpha(&p)?,
        });
    }
    if let Some(p) = func_args(&s, &["hsla", "hsl"]) {
        if p.len() != 3 && p.len() != 4 {
            return None;
        }
        let pct = |v: &str| {
            v.trim_end_matches('%')
                .parse::<f64>()
                .ok()
                .filter(|v| (0.0..=100.0).contains(v))
        };
        let h = p[0].trim_end_matches("deg").parse::<f64>().ok()?;
        let (r, g, b) = hsl_to_rgb(h, pct(p[1])? / 100.0, pct(p[2])? / 100.0);
        return Some(Rgba {
            r,
            g,
            b,
            a: parse_alpha(&p)?,
        });
    }
    if let Some((_, v)) = NAMED_COLORS.iter().find(|(n, _)| *n == s) {
        return Some(Rgba {
            r: (v >> 16) as u8,
            g: (v >> 8) as u8,
            b: *v as u8,
            a: 1.0,
        });
    }
    // Bare hex is accepted only when it cannot be confused with a word.
    if matches!(s.len(), 6 | 8) {
        return parse_hex(&s);
    }
    None
}

/// Hue (degrees), HSL saturation and lightness, HSV saturation and value (0..1).
fn hue_sat_light_value(c: Rgba) -> (f64, f64, f64, f64, f64) {
    let (r, g, b) = (
        f64::from(c.r) / 255.0,
        f64::from(c.g) / 255.0,
        f64::from(c.b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let l = (max + min) / 2.0;
    let sl = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    let sv = if max == 0.0 { 0.0 } else { d / max };
    (h, sl, l, sv, max)
}

fn color_converter() -> Tool {
    Tool {
        info: ToolInfo {
            id: "color-converter",
            name: "Color Converter",
            category: Category::WebDev,
            description: "Convert colors between HEX, RGB, HSL, and HSV formats",
            keywords: vec!["color", "hex", "rgb", "hsl", "hsv", "convert"],
            icon: "palette",
            input_placeholder: "Enter color (e.g., #FF5733, rgb(255, 87, 51), hsl(9, 100%, 60%))",
            output_placeholder: "Color conversions will appear here",
            primary_action: "Convert",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                toggle(
                    "uppercaseHex",
                    "Uppercase HEX",
                    "Use uppercase letters in HEX values",
                    true,
                ),
                toggle(
                    "includeAlpha",
                    "Include alpha channel",
                    "Show RGBA/HSLA formats",
                    false,
                ),
                toggle(
                    "showCss",
                    "Show CSS snippets",
                    "Include CSS color declarations",
                    true,
                ),
            ],
        },
        process: Some(color_process),
    }
}

fn color_process(req: &Request) -> ToolResult {
    let c = parse_color(req.input).ok_or_else(|| {
        ToolError::InvalidInput(
            "Could not parse color. Try formats like #FF5733, rgb(255,87,51), or hsl(9,100%,60%)"
                .into(),
        )
    })?;
    let alpha = req.options.bool("includeAlpha");
    let (h, sl, l, sv, v) = hue_sat_light_value(c);
    let pct = |x: f64| (x * 100.0).round();
    let a = num((c.a * 100.0).round() / 100.0);

    let mut hex = format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
    if alpha {
        hex.push_str(&format!("{:02x}", (c.a * 255.0).round() as u8));
    }
    if req.options.bool("uppercaseHex") {
        hex = hex.to_uppercase();
    }
    let (rgb, hsl) = if alpha {
        (
            format!("rgba({}, {}, {}, {a})", c.r, c.g, c.b),
            format!("hsla({}, {}%, {}%, {a})", h.round(), pct(sl), pct(l)),
        )
    } else {
        (
            format!("rgb({}, {}, {})", c.r, c.g, c.b),
            format!("hsl({}, {}%, {}%)", h.round(), pct(sl), pct(l)),
        )
    };
    let hsv = format!("hsv({}, {}%, {}%)", h.round(), pct(sv), pct(v));

    let mut out = format!("HEX: {hex}\nRGB: {rgb}\nHSL: {hsl}\nHSV: {hsv}");
    if req.options.bool("showCss") {
        out.push_str(&format!(
            "\n\nbackground-color: {hex};\nbackground-color: {rgb};\nbackground-color: {hsl};"
        ));
    }
    Ok(ToolOutput::text(out))
}

// ---------------------------------------------------------------- CSS Minifier

fn css_minifier() -> Tool {
    Tool {
        info: ToolInfo {
            id: "css-minifier",
            name: "CSS Minifier",
            category: Category::WebDev,
            description: "Minify or beautify CSS code",
            keywords: vec!["css", "minify", "beautify", "compress", "format"],
            icon: "file-code",
            input_placeholder: "Paste CSS code here...",
            output_placeholder: "Processed CSS will appear here",
            primary_action: "Minify",
            secondary_action: Some("Beautify"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                toggle(
                    "removeComments",
                    "Remove comments",
                    "Strip CSS comments",
                    true,
                ),
                toggle(
                    "compressColors",
                    "Compress colors",
                    "Shorten hex colors (e.g., #ffffff → #fff)",
                    true,
                ),
            ],
        },
        process: Some(css_process),
    }
}

fn css_process(req: &Request) -> ToolResult {
    let strip = req.options.bool("removeComments");
    let out = if req.reverse {
        css_beautify(req.input, req.options.i64("indent") as usize, strip)
    } else {
        css_minify(req.input, strip, req.options.bool("compressColors"))
    };
    Ok(ToolOutput::text(out))
}

/// End index (exclusive) of the quoted string starting at `i`.
fn quoted_end(c: &[char], i: usize) -> usize {
    let mut j = i + 1;
    while j < c.len() && c[j] != c[i] {
        j += if c[j] == '\\' { 2 } else { 1 };
    }
    (j + 1).min(c.len())
}

/// End index (exclusive) of the `/* */` comment starting at `i`.
fn block_comment_end(c: &[char], i: usize) -> usize {
    (i + 2..c.len().saturating_sub(1))
        .find(|&j| c[j] == '*' && c[j + 1] == '/')
        .map_or(c.len(), |j| j + 2)
}

fn css_minify(src: &str, strip: bool, compress: bool) -> String {
    let c: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let (mut i, mut paren, mut space) = (0, 0usize, false);
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            space = true;
            i += 1;
            continue;
        }
        let is_comment = ch == '/' && c.get(i + 1) == Some(&'*');
        let end = match ch {
            _ if is_comment => block_comment_end(&c, i),
            '"' | '\'' => quoted_end(&c, i),
            _ => i + 1,
        };
        if is_comment && strip {
            i = end;
            space = true;
            continue;
        }
        if space {
            let last = out.chars().last();
            let tight_always = matches!(last, None | Some('{' | '}' | ';' | ':' | ',' | '('))
                || out.ends_with("*/")
                || matches!(ch, '{' | '}' | ';' | ',' | ')');
            let tight_selector = paren == 0
                && (matches!(last, Some('>' | '+' | '~')) || matches!(ch, '>' | '+' | '~'));
            if !tight_always && !tight_selector {
                out.push(' ');
            }
            space = false;
        }
        match ch {
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '}' if out.ends_with(';') => {
                out.pop();
            }
            _ => {}
        }
        if ch == '#' && compress {
            let hex: String = c[i + 1..]
                .iter()
                .take_while(|x| x.is_ascii_alphanumeric())
                .collect();
            let b = hex.as_bytes();
            // A color inside a rule body ends with ';' or '}', a selector is followed by '{'.
            let is_value = c[i + 1 + hex.len()..]
                .iter()
                .find(|x| matches!(x, '{' | ';' | '}'))
                .is_some_and(|x| *x != '{');
            if is_value
                && hex.len() == 6
                && hex.chars().all(|x| x.is_ascii_hexdigit())
                && b[0] == b[1]
                && b[2] == b[3]
                && b[4] == b[5]
            {
                out.push('#');
                out.extend([b[0], b[2], b[4]].map(char::from));
                i += 7;
                continue;
            }
        }
        out.extend(&c[i..end]);
        i = end;
    }
    out.trim().to_string()
}

/// Beautifiers stop indenting deeper levels so output stays linear in input size.
const MAX_INDENT_DEPTH: usize = 32;

fn css_beautify(src: &str, indent: usize, strip: bool) -> String {
    let c: Vec<char> = src.chars().collect();
    let pad = |depth: usize| " ".repeat(indent * depth.min(MAX_INDENT_DEPTH));
    let mut out = String::new();
    let mut buf = String::new();
    let (mut i, mut depth, mut paren) = (0, 0usize, 0usize);

    fn flush_statement(out: &mut String, buf: &mut String, pad: &str) {
        let stmt = collapse_ws(buf);
        buf.clear();
        if stmt.is_empty() {
            return;
        }
        match stmt.split_once(':') {
            Some((prop, value)) if !stmt.starts_with('@') => {
                out.push_str(&format!("{pad}{}: {};\n", prop.trim_end(), value.trim()))
            }
            _ => out.push_str(&format!("{pad}{stmt};\n")),
        }
    }

    while i < c.len() {
        let ch = c[i];
        let is_comment = ch == '/' && c.get(i + 1) == Some(&'*');
        let end = match ch {
            _ if is_comment => block_comment_end(&c, i),
            '"' | '\'' => quoted_end(&c, i),
            _ => i + 1,
        };
        let token: String = c[i..end].iter().collect();
        i = end;
        if is_comment {
            if strip {
                continue;
            }
            if buf.trim().is_empty() {
                out.push_str(&format!("{}{token}\n", pad(depth)));
            } else {
                buf.push_str(&token);
            }
            continue;
        }
        match ch {
            '(' => {
                paren += 1;
                buf.push(ch);
            }
            ')' => {
                paren = paren.saturating_sub(1);
                buf.push(ch);
            }
            '{' if paren == 0 => {
                out.push_str(&format!("{}{} {{\n", pad(depth), collapse_ws(&buf)));
                buf.clear();
                depth += 1;
            }
            ';' if paren == 0 => flush_statement(&mut out, &mut buf, &pad(depth)),
            '}' if paren == 0 => {
                flush_statement(&mut out, &mut buf, &pad(depth));
                depth = depth.saturating_sub(1);
                out.push_str(&format!("{}}}\n", pad(depth)));
                if depth == 0 {
                    out.push('\n');
                }
            }
            _ => buf.push_str(&token),
        }
    }
    flush_statement(&mut out, &mut buf, &pad(depth));
    out.trim().to_string()
}

// ---------------------------------------------------------------- JS Minifier

fn js_minifier() -> Tool {
    Tool {
        info: ToolInfo {
            id: "js-minifier",
            name: "JS Minifier",
            category: Category::WebDev,
            description: "Minify or beautify JavaScript code",
            keywords: vec!["javascript", "js", "minify", "beautify", "compress"],
            icon: "braces",
            input_placeholder: "Paste JavaScript code here...",
            output_placeholder: "Processed JavaScript will appear here",
            primary_action: "Minify",
            secondary_action: Some("Beautify"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                toggle(
                    "removeComments",
                    "Remove comments",
                    "Strip JavaScript comments",
                    true,
                ),
                toggle(
                    "removeConsole",
                    "Remove console.log",
                    "Remove console.log statements",
                    false,
                ),
            ],
        },
        process: Some(js_process),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// Whitespace run; `true` when it contains a newline.
    Ws(bool),
    Comment(String),
    /// String, template or regex literal.
    Lit(String),
    Word(String),
    Punct(char),
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$' || !c.is_ascii()
}

/// End index (exclusive) of the template literal starting at `i`.
fn template_end(c: &[char], i: usize) -> usize {
    let mut j = i + 1;
    while j < c.len() {
        match c[j] {
            '\\' => j += 2,
            '`' => return j + 1,
            '$' if c.get(j + 1) == Some(&'{') => {
                let mut depth = 1;
                j += 2;
                while j < c.len() && depth > 0 {
                    match c[j] {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        '"' | '\'' => j = quoted_end(c, j) - 1,
                        '`' => j = template_end(c, j) - 1,
                        _ => {}
                    }
                    j += 1;
                }
            }
            _ => j += 1,
        }
    }
    c.len()
}

/// End index (exclusive) of a regex literal starting at `i`, if it is terminated on its line.
fn regex_end(c: &[char], i: usize) -> Option<usize> {
    let (mut j, mut class) = (i + 1, false);
    while j < c.len() && c[j] != '\n' {
        match c[j] {
            '\\' => j += 1,
            '[' => class = true,
            ']' => class = false,
            '/' if !class => {
                j += 1;
                while j < c.len() && is_word_char(c[j]) {
                    j += 1;
                }
                return Some(j);
            }
            _ => {}
        }
        j += 1;
    }
    None
}

fn js_tokens(src: &str) -> Vec<Tok> {
    const REGEX_KEYWORDS: [&str; 14] = [
        "return",
        "typeof",
        "instanceof",
        "in",
        "of",
        "new",
        "delete",
        "void",
        "throw",
        "case",
        "do",
        "else",
        "yield",
        "await",
    ];
    let c: Vec<char> = src.chars().collect();
    let mut toks: Vec<Tok> = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        let start = i;
        let tok = if ch.is_whitespace() {
            while i < c.len() && c[i].is_whitespace() {
                i += 1;
            }
            Tok::Ws(c[start..i].contains(&'\n'))
        } else if ch == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            Tok::Comment(c[start..i].iter().collect())
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            i = block_comment_end(&c, i);
            Tok::Comment(c[start..i].iter().collect())
        } else if ch == '"' || ch == '\'' {
            i = quoted_end(&c, i);
            Tok::Lit(c[start..i].iter().collect())
        } else if ch == '`' {
            i = template_end(&c, i);
            Tok::Lit(c[start..i].iter().collect())
        } else if is_word_char(ch) {
            while i < c.len() && is_word_char(c[i]) {
                i += 1;
            }
            Tok::Word(c[start..i].iter().collect())
        } else {
            let prev = toks
                .iter()
                .rev()
                .find(|t| !matches!(t, Tok::Ws(_) | Tok::Comment(_)));
            let regex_allowed = match prev {
                None => true,
                Some(Tok::Punct(p)) => !matches!(p, ')' | ']'),
                Some(Tok::Word(w)) => REGEX_KEYWORDS.contains(&w.as_str()),
                Some(_) => false,
            };
            let regex = if ch == '/' && regex_allowed {
                regex_end(&c, i)
            } else {
                None
            };
            match regex {
                Some(end) => {
                    i = end;
                    Tok::Lit(c[start..i].iter().collect())
                }
                None => {
                    i += 1;
                    Tok::Punct(ch)
                }
            }
        };
        toks.push(tok);
    }
    toks
}

/// Drops `console.<method>(...)` statements; the argument list is matched with balanced parentheses.
fn strip_console(toks: Vec<Tok>) -> Vec<Tok> {
    const METHODS: [&str; 6] = ["log", "error", "warn", "info", "debug", "trace"];
    let skip_ws = |toks: &[Tok], mut i: usize| {
        while matches!(toks.get(i), Some(Tok::Ws(_))) {
            i += 1;
        }
        i
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let is_call = matches!(&toks[i], Tok::Word(w) if w == "console")
            && !matches!(out.last(), Some(Tok::Punct('.')))
            && toks.get(i + 1) == Some(&Tok::Punct('.'))
            && matches!(toks.get(i + 2), Some(Tok::Word(m)) if METHODS.contains(&m.as_str()));
        let open = skip_ws(&toks, i + 3);
        if is_call && toks.get(open) == Some(&Tok::Punct('(')) {
            let (mut depth, mut j) = (0, open);
            while j < toks.len() {
                match toks[j] {
                    Tok::Punct('(') => depth += 1,
                    Tok::Punct(')') => depth -= 1,
                    _ => {}
                }
                j += 1;
                if depth == 0 {
                    break;
                }
            }
            if depth == 0 {
                let semi = skip_ws(&toks, j);
                i = if toks.get(semi) == Some(&Tok::Punct(';')) {
                    semi + 1
                } else {
                    j
                };
                continue;
            }
        }
        out.push(toks[i].clone());
        i += 1;
    }
    out
}

fn js_process(req: &Request) -> ToolResult {
    let mut toks = js_tokens(req.input);
    if req.options.bool("removeConsole") {
        toks = strip_console(toks);
    }
    let strip = req.options.bool("removeComments");
    let out = if req.reverse {
        js_beautify(&toks, strip, req.options.i64("indent") as usize)
    } else {
        js_minify(&toks, strip)
    };
    Ok(ToolOutput::text(out))
}

fn js_minify(toks: &[Tok], strip: bool) -> String {
    let mut out = String::new();
    let (mut ws, mut newline) = (false, false);
    for tok in toks {
        let text = match tok {
            Tok::Ws(nl) => {
                ws = true;
                newline |= nl;
                continue;
            }
            Tok::Comment(_) if strip => {
                ws = true;
                continue;
            }
            Tok::Comment(t) | Tok::Lit(t) | Tok::Word(t) => t.clone(),
            Tok::Punct(p) => p.to_string(),
        };
        let first = text.chars().next().unwrap_or(' ');
        if let (true, Some(last)) = (ws, out.chars().last()) {
            // Newlines are kept unless both sides make them irrelevant, so ASI behaves as before.
            if newline {
                if last != '\n'
                    && !"{([,;:=<>&|?*%^~!".contains(last)
                    && !")]},;:.?=*%<>&|^".contains(first)
                {
                    out.push('\n');
                }
            } else if (is_word_char(last) && is_word_char(first))
                || (last == first && (last == '+' || last == '-'))
                || (last == '/' && (first == '/' || first == '*'))
            {
                out.push(' ');
            }
        }
        ws = false;
        newline = false;
        out.push_str(&text);
        if matches!(tok, Tok::Comment(t) if t.starts_with("//")) {
            out.push('\n');
        }
    }
    out.trim().to_string()
}

fn js_beautify(toks: &[Tok], strip: bool, indent: usize) -> String {
    let next_sig = |from: usize| {
        toks[from..]
            .iter()
            .find(|t| !matches!(t, Tok::Ws(_) | Tok::Comment(_)))
    };
    let mut out = String::new();
    let (mut depth, mut paren, mut space) = (0usize, 0usize, false);
    let mut line_start = true;
    for (i, tok) in toks.iter().enumerate() {
        let text = match tok {
            Tok::Ws(_) => {
                space = true;
                continue;
            }
            Tok::Comment(_) if strip => {
                space = true;
                continue;
            }
            Tok::Comment(t) | Tok::Lit(t) | Tok::Word(t) => t.clone(),
            Tok::Punct(p) => p.to_string(),
        };
        if matches!(tok, Tok::Punct('}')) {
            depth = depth.saturating_sub(1);
            if !line_start && !out.ends_with('{') {
                out.push('\n');
                line_start = true;
            }
        }
        if line_start {
            out.push_str(&" ".repeat(indent * depth.min(MAX_INDENT_DEPTH)));
            line_start = false;
        } else if matches!(tok, Tok::Punct('{'))
            && out.ends_with(|c: char| c == ')' || is_word_char(c))
            || space
                && !matches!(tok, Tok::Punct(')' | ']' | ',' | ';' | '.'))
                && !out.ends_with(['(', '[', '.'])
        {
            out.push(' ');
        }
        space = false;
        out.push_str(&text);
        let newline = match tok {
            Tok::Punct('{') => {
                depth += 1;
                // Keep empty blocks on one line.
                !matches!(next_sig(i + 1), Some(Tok::Punct('}')))
            }
            Tok::Punct('}') => {
                let continues = match next_sig(i + 1) {
                    Some(Tok::Punct(p)) => matches!(p, ')' | ',' | ';' | '.'),
                    Some(Tok::Word(w)) => {
                        ["else", "catch", "finally", "while"].contains(&w.as_str())
                    }
                    _ => false,
                };
                space = continues;
                !continues
            }
            Tok::Punct('(') => {
                paren += 1;
                false
            }
            Tok::Punct(')') => {
                paren = paren.saturating_sub(1);
                false
            }
            Tok::Punct(';') => paren == 0,
            Tok::Punct(',') => {
                space = true;
                false
            }
            Tok::Comment(t) => t.starts_with("//"),
            _ => false,
        };
        if newline {
            out.push('\n');
            line_start = true;
        }
    }
    out.trim().to_string()
}

// ---------------------------------------------------------------- HTML Minifier

fn html_minifier() -> Tool {
    Tool {
        info: ToolInfo {
            id: "html-minifier",
            name: "HTML Minifier",
            category: Category::WebDev,
            description: "Minify or beautify HTML code",
            keywords: vec!["html", "minify", "beautify", "compress", "format"],
            icon: "code-xml",
            input_placeholder: "Paste HTML code here...",
            output_placeholder: "Processed HTML will appear here",
            primary_action: "Minify",
            secondary_action: Some("Beautify"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                toggle(
                    "removeComments",
                    "Remove comments",
                    "Strip HTML comments",
                    true,
                ),
                toggle(
                    "collapseWhitespace",
                    "Collapse whitespace",
                    "Remove extra whitespace",
                    true,
                ),
            ],
        },
        process: Some(html_process),
    }
}

enum Html {
    Tag(String),
    Comment(String),
    Text(String),
    /// Content of script/style/pre/textarea, kept verbatim.
    Raw(String),
}

const HTML_RAW_TAGS: [&str; 4] = ["script", "style", "pre", "textarea"];
const HTML_VOID_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn html_tokens(src: &str) -> Vec<Html> {
    let mut toks = Vec::new();
    let mut rest = src;
    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            let end = rest.find("-->").map_or(rest.len(), |e| e + 3);
            toks.push(Html::Comment(rest[..end].to_string()));
            rest = &rest[end..];
        } else if rest.starts_with('<') {
            // The first '>' outside quotes ends the tag.
            let mut quote = None;
            let end = rest.char_indices().find_map(|(i, c)| match (quote, c) {
                (Some(q), c) if c == q => {
                    quote = None;
                    None
                }
                (None, '"' | '\'') => {
                    quote = Some(c);
                    None
                }
                (None, '>') => Some(i + 1),
                _ => None,
            });
            let Some(end) = end else {
                toks.push(Html::Text(rest.to_string()));
                break;
            };
            let tag = &rest[..end];
            rest = &rest[end..];
            let name = tag_name(tag);
            toks.push(Html::Tag(tag.to_string()));
            if HTML_RAW_TAGS.contains(&name.as_str())
                && !tag.starts_with("</")
                && !tag.ends_with("/>")
            {
                let close = format!("</{name}");
                let end = rest.to_ascii_lowercase().find(&close).unwrap_or(rest.len());
                if end > 0 {
                    toks.push(Html::Raw(rest[..end].to_string()));
                }
                rest = &rest[end..];
            }
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            toks.push(Html::Text(rest[..end].to_string()));
            rest = &rest[end..];
        }
    }
    toks
}

/// Collapses whitespace between attributes while leaving quoted values alone.
fn collapse_tag(tag: &str) -> String {
    let c: Vec<char> = tag.chars().collect();
    let mut out = String::new();
    let (mut quote, mut space) = (None, false);
    for (i, &ch) in c.iter().enumerate() {
        match quote {
            Some(q) => {
                out.push(ch);
                if ch == q {
                    quote = None;
                }
            }
            None if ch.is_whitespace() => space = true,
            None => {
                let closing = ch == '>' || (ch == '/' && c.get(i + 1) == Some(&'>'));
                if space && !closing {
                    out.push(' ');
                }
                space = false;
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                }
                out.push(ch);
            }
        }
    }
    out
}

fn html_process(req: &Request) -> ToolResult {
    let strip = req.options.bool("removeComments");
    let toks = html_tokens(req.input);
    let out = if req.reverse {
        html_beautify(&toks, strip, req.options.i64("indent") as usize)
    } else {
        html_minify(&toks, strip, req.options.bool("collapseWhitespace"))
    };
    Ok(ToolOutput::text(out))
}

fn html_minify(toks: &[Html], strip: bool, collapse: bool) -> String {
    let mut out = String::new();
    for (i, tok) in toks.iter().enumerate() {
        match tok {
            Html::Comment(c) if strip && !c.starts_with("<!--[if") => {}
            Html::Comment(t) | Html::Raw(t) => out.push_str(t),
            Html::Tag(t) if collapse => out.push_str(&collapse_tag(t)),
            Html::Tag(t) => out.push_str(t),
            Html::Text(t) if !collapse => out.push_str(t),
            Html::Text(t) => {
                let words = collapse_ws(t);
                if words.is_empty() {
                    if !t.contains('\n') {
                        out.push(' ');
                    }
                    continue;
                }
                // A newline next to a tag carries no meaning; a plain space does.
                let after_tag = i > 0 && matches!(toks[i - 1], Html::Tag(_));
                let before_tag = matches!(toks.get(i + 1), Some(Html::Tag(_)));
                let lead_ws = &t[..t.len() - t.trim_start().len()];
                let trail_ws = &t[t.trim_end().len()..];
                if !(lead_ws.is_empty() || after_tag && lead_ws.contains('\n')) {
                    out.push(' ');
                }
                out.push_str(&words);
                if !(trail_ws.is_empty() || before_tag && trail_ws.contains('\n')) {
                    out.push(' ');
                }
            }
        }
    }
    out.trim().to_string()
}

fn html_beautify(toks: &[Html], strip: bool, indent: usize) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    // Inside pre/textarea the content and closing tag stay on the opening tag's line.
    let mut glue = false;
    let line = |out: &mut String, depth: usize, s: &str| {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&" ".repeat(indent * depth.min(MAX_INDENT_DEPTH)));
        out.push_str(s);
    };
    for tok in toks {
        match tok {
            Html::Comment(_) if strip => {}
            Html::Comment(c) => line(&mut out, depth, c),
            Html::Text(t) => {
                let words = collapse_ws(t);
                if !words.is_empty() {
                    line(&mut out, depth, &words);
                }
            }
            Html::Raw(r) if glue => out.push_str(r),
            Html::Raw(r) => {
                for l in r
                    .trim_matches(['\n', '\r'])
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                {
                    line(&mut out, depth, l.trim_end());
                }
            }
            Html::Tag(t) => {
                let closing = t.starts_with("</");
                if glue && closing {
                    out.push_str(t);
                    glue = false;
                    depth = depth.saturating_sub(1);
                    continue;
                }
                if closing {
                    depth = depth.saturating_sub(1);
                }
                line(&mut out, depth, &collapse_tag(t));
                let name = tag_name(t);
                let opens = !closing
                    && !t.starts_with("<!")
                    && !t.ends_with("/>")
                    && !HTML_VOID_TAGS.contains(&name.as_str());
                if opens {
                    depth += 1;
                    glue = name == "pre" || name == "textarea";
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- Markdown Preview

fn markdown_preview() -> Tool {
    Tool {
        info: ToolInfo {
            id: "markdown-preview",
            name: "Markdown Preview",
            category: Category::WebDev,
            description: "Preview Markdown and convert to HTML",
            keywords: vec!["markdown", "md", "html", "preview", "convert"],
            icon: "file-text",
            input_placeholder: "Paste Markdown here...",
            output_placeholder: "HTML output will appear here",
            primary_action: "Convert to HTML",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                toggle(
                    "smartTypography",
                    "Smart Typography",
                    "Convert straight quotes to curly, --- to em dashes, -- to en dashes",
                    false,
                ),
                toggle(
                    "allowRawHtml",
                    "Allow Raw HTML",
                    "Allow raw HTML and unsafe links in the output",
                    true,
                ),
                toggle(
                    "wrapDocument",
                    "Wrap in HTML document",
                    "Add DOCTYPE, html, head, body tags",
                    false,
                ),
            ],
        },
        process: Some(markdown_process),
    }
}

const MARKDOWN_DOC_HEAD: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Markdown Preview</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; max-width: 800px; margin: 0 auto; padding: 20px; }
        code { background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }
        pre { background: #f4f4f4; padding: 16px; border-radius: 6px; overflow-x: auto; }
        pre code { background: none; padding: 0; }
        blockquote { border-left: 4px solid #ddd; margin: 0; padding-left: 16px; color: #666; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background: #f4f4f4; }
    </style>
</head>
<body>
"#;

fn markdown_process(req: &Request) -> ToolResult {
    let mut opts = MdOptions::ENABLE_TABLES | MdOptions::ENABLE_STRIKETHROUGH;
    opts.set(
        MdOptions::ENABLE_SMART_PUNCTUATION,
        req.options.bool("smartTypography"),
    );
    let raw = req.options.bool("allowRawHtml");
    let mut in_html_block = false;
    // Like cmark without "unsafe": raw HTML is replaced and script-like link targets are emptied.
    let events = Parser::new_ext(req.input, opts).filter_map(|e| match e {
        Event::Start(Tag::HtmlBlock) if !raw => {
            in_html_block = true;
            Some(Event::Html("<!-- raw HTML omitted -->\n".into()))
        }
        Event::End(TagEnd::HtmlBlock) if !raw => {
            in_html_block = false;
            None
        }
        Event::Html(_) if !raw && in_html_block => None,
        Event::Html(_) | Event::InlineHtml(_) if !raw => {
            Some(Event::Html("<!-- raw HTML omitted -->".into()))
        }
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) if !raw && is_unsafe_url(&dest_url) => Some(Event::Start(Tag::Link {
            link_type,
            dest_url: CowStr::Borrowed(""),
            title,
            id,
        })),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) if !raw && is_unsafe_url(&dest_url) => Some(Event::Start(Tag::Image {
            link_type,
            dest_url: CowStr::Borrowed(""),
            title,
            id,
        })),
        e => Some(e),
    });
    let mut body = String::new();
    html::push_html(&mut body, events);
    if req.options.bool("wrapDocument") {
        body = format!("{MARKDOWN_DOC_HEAD}{body}</body>\n</html>\n");
    }
    Ok(ToolOutput::text(body))
}

fn is_unsafe_url(url: &str) -> bool {
    let url = url.trim_start().to_ascii_lowercase();
    ["javascript:", "vbscript:", "file:", "data:"]
        .iter()
        .any(|p| url.starts_with(p))
}

// ---------------------------------------------------------------- Lorem Ipsum

fn lorem_ipsum() -> Tool {
    Tool {
        info: ToolInfo {
            id: "lorem-ipsum",
            name: "Lorem Ipsum",
            category: Category::WebDev,
            description: "Generate placeholder text for designs",
            keywords: vec!["lorem", "ipsum", "placeholder", "text", "dummy"],
            icon: "text-align-start",
            input_placeholder: "No input needed - configure options and generate",
            output_placeholder: "Generated text will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "unit",
                    label: "Generate",
                    help: Some("Type of content to generate"),
                    kind: OptionKind::Picker {
                        choices: vec!["Paragraphs", "Sentences", "Words"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "count",
                    label: "Count",
                    help: Some("Number of items to generate"),
                    kind: OptionKind::Stepper {
                        min: 1,
                        max: 100,
                        default: 3,
                    },
                },
                toggle(
                    "startWithLorem",
                    "Start with 'Lorem ipsum...'",
                    "Begin with classic Lorem ipsum text",
                    true,
                ),
                ToolOption {
                    id: "variant",
                    label: "Variant",
                    help: Some("Text style variant"),
                    kind: OptionKind::Picker {
                        choices: vec!["Classic", "Hipster"],
                        default: 0,
                    },
                },
                toggle(
                    "wrapParagraphs",
                    "Wrap paragraphs in <p> tags",
                    "Add HTML paragraph tags",
                    false,
                ),
            ],
        },
        process: Some(lorem_process),
    }
}

const LOREM_OPENING: [&str; 8] = [
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
];

const CLASSIC_WORDS: [&str; 60] = [
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "sed",
    "do",
    "eiusmod",
    "tempor",
    "incididunt",
    "ut",
    "labore",
    "et",
    "dolore",
    "magna",
    "aliqua",
    "enim",
    "ad",
    "minim",
    "veniam",
    "quis",
    "nostrud",
    "exercitation",
    "ullamco",
    "laboris",
    "nisi",
    "aliquip",
    "ex",
    "ea",
    "commodo",
    "consequat",
    "duis",
    "aute",
    "irure",
    "in",
    "reprehenderit",
    "voluptate",
    "velit",
    "esse",
    "cillum",
    "eu",
    "fugiat",
    "nulla",
    "pariatur",
    "excepteur",
    "sint",
    "occaecat",
    "cupidatat",
    "non",
    "proident",
    "sunt",
    "culpa",
    "qui",
    "officia",
    "deserunt",
    "mollit",
    "anim",
];

const HIPSTER_WORDS: [&str; 40] = [
    "artisan",
    "aesthetic",
    "austin",
    "bespoke",
    "brooklyn",
    "craft",
    "beer",
    "cold-pressed",
    "curated",
    "disrupt",
    "direct",
    "trade",
    "flannel",
    "fixie",
    "gastropub",
    "irony",
    "kale",
    "chips",
    "kombucha",
    "letterpress",
    "mustache",
    "organic",
    "pickled",
    "poutine",
    "quinoa",
    "retro",
    "selvage",
    "sriracha",
    "tattooed",
    "thundercats",
    "typewriter",
    "umami",
    "vegan",
    "vinyl",
    "wayfarers",
    "whatever",
    "yolo",
    "locavore",
    "tote",
    "bag",
];

/// xorshift64 seeded from the clock; quality is irrelevant for filler text.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        Rng(nanos | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn lorem_words(rng: &mut Rng, pool: &[&str], n: usize, opening: bool) -> Vec<String> {
    let lead = if opening {
        n.min(LOREM_OPENING.len())
    } else {
        0
    };
    LOREM_OPENING[..lead]
        .iter()
        .map(|w| w.to_string())
        .chain((lead..n).map(|_| pool[rng.below(pool.len())].to_string()))
        .collect()
}

fn lorem_sentence(rng: &mut Rng, pool: &[&str], opening: bool) -> String {
    let n = 6 + rng.below(10);
    let mut s = lorem_words(rng, pool, n, opening).join(" ");
    s[..1].make_ascii_uppercase();
    s.push('.');
    s
}

fn lorem_process(req: &Request) -> ToolResult {
    let count = req.options.i64("count") as usize;
    let opening = req.options.bool("startWithLorem");
    let pool: &[&str] = if req.options.str("variant") == "Hipster" {
        &HIPSTER_WORDS
    } else {
        &CLASSIC_WORDS
    };
    let mut rng = Rng::new();
    let out = match req.options.str("unit") {
        "Words" => lorem_words(&mut rng, pool, count, opening).join(" "),
        "Sentences" => (0..count)
            .map(|i| lorem_sentence(&mut rng, pool, opening && i == 0))
            .collect::<Vec<_>>()
            .join(" "),
        _ => {
            let wrap = req.options.bool("wrapParagraphs");
            let paragraphs: Vec<String> = (0..count)
                .map(|i| {
                    let sentences = 4 + rng.below(4);
                    let text = (0..sentences)
                        .map(|j| lorem_sentence(&mut rng, pool, opening && i == 0 && j == 0))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if wrap {
                        format!("<p>{text}</p>")
                    } else {
                        text
                    }
                })
                .collect();
            paragraphs.join(if wrap { "\n" } else { "\n\n" })
        }
    };
    Ok(ToolOutput::text(out))
}

// ---------------------------------------------------------------- Data URI

const DATA_URI_MIME_TYPES: [&str; 9] = [
    "Auto-detect",
    "text/plain",
    "text/html",
    "text/css",
    "text/javascript",
    "application/json",
    "application/xml",
    "image/svg+xml",
    "image/png",
];

fn data_uri() -> Tool {
    Tool {
        info: ToolInfo {
            id: "data-uri",
            name: "Data URI",
            category: Category::WebDev,
            description: "Convert text or data to Data URI format",
            keywords: vec!["data", "uri", "url", "base64", "inline"],
            icon: "link-2",
            input_placeholder: "Paste text, CSS, JavaScript, or SVG content here...",
            output_placeholder: "Data URI will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "mimeType",
                    label: "MIME Type",
                    help: Some("Content MIME type"),
                    kind: OptionKind::Picker {
                        choices: DATA_URI_MIME_TYPES.to_vec(),
                        default: 0,
                    },
                },
                toggle(
                    "base64",
                    "Base64 encode",
                    "Encode data as Base64 (recommended)",
                    true,
                ),
                toggle(
                    "cssUrl",
                    "CSS url() format",
                    "Wrap in CSS url() function",
                    false,
                ),
            ],
        },
        process: Some(data_uri_process),
    }
}

fn detect_mime(text: &str) -> &'static str {
    let t = text.trim_start();
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg")) {
        "image/svg+xml"
    } else if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        "text/html"
    } else if lower.starts_with("<?xml") {
        "application/xml"
    } else if (t.starts_with('{') || t.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(t).is_ok()
    {
        "application/json"
    } else if ["function ", "const ", "let ", "var ", "=>", "console."]
        .iter()
        .any(|k| t.contains(k))
    {
        "text/javascript"
    } else if t.contains('{') && t.contains(':') && t.contains(';') && t.contains('}') {
        "text/css"
    } else {
        "text/plain"
    }
}

fn data_uri_process(req: &Request) -> ToolResult {
    if req.reverse {
        return data_uri_decode(req.input);
    }
    let mime = match req.options.str("mimeType") {
        "Auto-detect" => detect_mime(req.input),
        chosen => chosen,
    };
    let uri = if req.options.bool("base64") {
        format!("data:{mime};base64,{}", STANDARD.encode(req.input))
    } else {
        format!("data:{mime},{}", percent_encode(req.input.as_bytes()))
    };
    Ok(ToolOutput::text(if req.options.bool("cssUrl") {
        format!("url('{uri}')")
    } else {
        uri
    }))
}

fn data_uri_decode(input: &str) -> ToolResult {
    let mut uri = input.trim();
    if let Some(inner) = uri.strip_prefix("url(").and_then(|s| s.strip_suffix(')')) {
        uri = inner.trim().trim_matches(['"', '\'']);
    }
    let invalid = |m: &str| ToolError::InvalidInput(m.into());
    if !uri
        .get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("data:"))
    {
        return Err(invalid("Invalid Data URI format. Must start with 'data:'"));
    }
    let (meta, payload) = uri[5..]
        .split_once(',')
        .ok_or_else(|| invalid("Invalid Data URI format. Missing data separator."))?;
    let bytes = if meta.to_ascii_lowercase().ends_with(";base64") {
        let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
        STANDARD_NO_PAD
            .decode(clean.trim_end_matches('='))
            .map_err(|_| invalid("Invalid Base64 encoding"))?
    } else {
        percent_decode(payload).ok_or_else(|| invalid("Could not decode percent encoding"))?
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| ToolError::ProcessingFailed("Could not decode data as UTF-8 text".into()))?;
    Ok(ToolOutput::text(text))
}

fn percent_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(out)
}

// ---------------------------------------------------------------- SVG Path Analyzer

fn svg_path_analyzer() -> Tool {
    Tool {
        info: ToolInfo {
            id: "svg-path-analyzer",
            name: "SVG Path Analyzer",
            category: Category::WebDev,
            description: "Parse and analyze SVG path commands",
            keywords: vec!["svg", "path", "d", "analyze", "bezier", "curve"],
            icon: "spline",
            input_placeholder: "Paste SVG path data (d attribute) here...",
            output_placeholder: "Path analysis will appear here",
            primary_action: "Analyze",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                toggle(
                    "breakdown",
                    "Show command breakdown",
                    "List all path commands",
                    true,
                ),
                toggle(
                    "simplify",
                    "Simplify path",
                    "Convert relative to absolute commands",
                    false,
                ),
                toggle(
                    "completeSvg",
                    "Generate complete SVG",
                    "Output as complete SVG element",
                    false,
                ),
            ],
        },
        process: Some(svg_path_process),
    }
}

struct Seg {
    cmd: char,
    args: Vec<f64>,
}

fn arity(cmd: char) -> usize {
    match cmd.to_ascii_uppercase() {
        'M' | 'L' | 'T' => 2,
        'H' | 'V' => 1,
        'C' => 6,
        'S' | 'Q' => 4,
        'A' => 7,
        _ => 0,
    }
}

fn command_description(cmd: char) -> &'static str {
    match cmd.to_ascii_uppercase() {
        'M' => "Move to",
        'L' => "Line to",
        'H' => "Horizontal line to",
        'V' => "Vertical line to",
        'C' => "Cubic Bézier curve to",
        'S' => "Smooth cubic Bézier curve to",
        'Q' => "Quadratic Bézier curve to",
        'T' => "Smooth quadratic Bézier curve to",
        'A' => "Elliptical arc to",
        _ => "Close path",
    }
}

fn read_number(c: &[char], i: &mut usize, flag: bool) -> Option<f64> {
    while *i < c.len() && (c[*i].is_whitespace() || c[*i] == ',') {
        *i += 1;
    }
    if flag {
        // Arc flags may be packed without separators ("a1 1 0 011 1").
        let v = match c.get(*i) {
            Some('0') => 0.0,
            Some('1') => 1.0,
            _ => return None,
        };
        *i += 1;
        return Some(v);
    }
    let start = *i;
    if matches!(c.get(*i), Some('+' | '-')) {
        *i += 1;
    }
    let digits = |i: &mut usize| {
        let s = *i;
        while c.get(*i).is_some_and(char::is_ascii_digit) {
            *i += 1;
        }
        *i > s
    };
    let int = digits(i);
    let frac = c.get(*i) == Some(&'.') && {
        *i += 1;
        digits(i)
    };
    if !int && !frac {
        *i = start;
        return None;
    }
    if matches!(c.get(*i), Some('e' | 'E')) {
        let save = *i;
        *i += 1;
        if matches!(c.get(*i), Some('+' | '-')) {
            *i += 1;
        }
        if !digits(i) {
            *i = save;
        }
    }
    c[start..*i].iter().collect::<String>().parse().ok()
}

fn parse_path(d: &str) -> Vec<Seg> {
    let c: Vec<char> = d.chars().collect();
    let mut segs = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let mut cmd = c[i];
        i += 1;
        if !"MmLlHhVvCcSsQqTtAaZz".contains(cmd) {
            continue;
        }
        if arity(cmd) == 0 {
            segs.push(Seg { cmd, args: vec![] });
            continue;
        }
        // Extra coordinate sets after the first repeat the command (M becomes L).
        loop {
            let mut args = Vec::new();
            for k in 0..arity(cmd) {
                let flag = cmd.eq_ignore_ascii_case(&'a') && (k == 3 || k == 4);
                match read_number(&c, &mut i, flag) {
                    Some(n) => args.push(n),
                    None => break,
                }
            }
            if args.len() < arity(cmd) {
                break;
            }
            segs.push(Seg { cmd, args });
            cmd = match cmd {
                'M' => 'L',
                'm' => 'l',
                other => other,
            };
        }
    }
    segs
}

/// Converts every segment to its absolute form and collects all coordinates for the bounding box.
fn absolutize(segs: &[Seg]) -> (Vec<Seg>, Vec<(f64, f64)>) {
    let (mut x, mut y, mut sx, mut sy) = (0.0, 0.0, 0.0, 0.0);
    let mut out = Vec::new();
    let mut points = Vec::new();
    for seg in segs {
        let up = seg.cmd.to_ascii_uppercase();
        let mut args = seg.args.clone();
        let (ox, oy) = if seg.cmd.is_ascii_lowercase() {
            (x, y)
        } else {
            (0.0, 0.0)
        };
        match up {
            'H' => args[0] += ox,
            'V' => args[0] += oy,
            'A' => {
                args[5] += ox;
                args[6] += oy;
            }
            _ => {
                for pair in args.chunks_mut(2) {
                    pair[0] += ox;
                    pair[1] += oy;
                }
            }
        }
        match up {
            'H' => x = args[0],
            'V' => y = args[0],
            'Z' => (x, y) = (sx, sy),
            _ => (x, y) = (args[args.len() - 2], args[args.len() - 1]),
        }
        if up == 'M' {
            (sx, sy) = (x, y);
        }
        match up {
            'Z' => {}
            // Arc radii and flags are not coordinates, so only the endpoint counts.
            'H' | 'V' | 'A' => points.push((x, y)),
            _ => points.extend(args.chunks(2).map(|p| (p[0], p[1]))),
        }
        out.push(Seg { cmd: up, args });
    }
    (out, points)
}

fn path_string(segs: &[Seg]) -> String {
    segs.iter()
        .map(|s| {
            let args: Vec<String> = s.args.iter().map(|a| num(*a)).collect();
            format!("{}{}", s.cmd, args.join(" "))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn describe_segment(seg: &Seg) -> String {
    let a = &seg.args;
    let pair = |i: usize| format!("{:.1}, {:.1}", a[i], a[i + 1]);
    let params = match seg.cmd.to_ascii_uppercase() {
        'Z' => String::new(),
        'H' | 'V' => format!(" {:.1}", a[0]),
        'A' => format!(
            " {} ({:.1} {:.1} {:.1}) {}",
            pair(0),
            a[2],
            a[3],
            a[4],
            pair(5)
        ),
        _ => format!(
            " {}",
            (0..a.len())
                .step_by(2)
                .map(pair)
                .collect::<Vec<_>>()
                .join(" ")
        ),
    };
    format!(
        "{}{params} - {} ({})",
        seg.cmd,
        command_description(seg.cmd),
        if seg.cmd.is_ascii_lowercase() {
            "relative"
        } else {
            "absolute"
        }
    )
}

fn svg_path_process(req: &Request) -> ToolResult {
    // Accept a full `d="..."` attribute or markup containing one.
    let input = req.input;
    let d = input
        .find("d=")
        .and_then(|i| {
            let rest = &input[i + 2..];
            let quote = rest.chars().next().filter(|q| *q == '"' || *q == '\'')?;
            rest[1..].split(quote).next()
        })
        .unwrap_or(input);
    let segs = parse_path(d);
    if segs.is_empty() {
        return Err(ToolError::InvalidInput(
            "No valid path commands found".into(),
        ));
    }
    let (absolute, points) = absolutize(&segs);
    let min_max = |pick: fn(&(f64, f64)) -> f64| {
        points
            .iter()
            .map(pick)
            .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
    };
    let (min_x, max_x) = min_max(|p| p.0);
    let (min_y, max_y) = min_max(|p| p.1);
    let has_box = !points.is_empty();
    let count = |c: char| {
        segs.iter()
            .filter(|s| s.cmd.eq_ignore_ascii_case(&c))
            .count()
    };
    let relative = segs.iter().filter(|s| s.cmd.is_ascii_lowercase()).count();

    let mut summary = format!(
        "Total commands: {}\nSubpaths: {}\nClosed subpaths: {}\nAbsolute commands: {}\nRelative commands: {}",
        segs.len(),
        count('M'),
        count('Z'),
        segs.len() - relative,
        relative
    );
    if has_box {
        summary.push_str(&format!(
            "\nBounding box: ({}, {}) to ({}, {})\nSize: {} × {}",
            num(min_x),
            num(min_y),
            num(max_x),
            num(max_y),
            num(max_x - min_x),
            num(max_y - min_y)
        ));
    }
    let mut sections = vec![summary];
    if req.options.bool("breakdown") {
        let lines: Vec<String> = segs
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, describe_segment(s)))
            .collect();
        sections.push(format!("Command breakdown:\n{}", lines.join("\n")));
    }
    let simplify = req.options.bool("simplify");
    let path = path_string(if simplify { &absolute } else { &segs });
    if simplify {
        sections.push(format!("Simplified path:\n{path}"));
    }
    if req.options.bool("completeSvg") && has_box {
        sections.push(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\"><path d=\"{path}\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1\"/></svg>",
            num(min_x),
            num(min_y),
            num(max_x - min_x),
            num(max_y - min_y)
        ));
    }
    Ok(ToolOutput::text(sections.join("\n\n")))
}

// ---------------------------------------------------------------- Placeholder Image

const PLACEHOLDER_FORMATS: [&str; 4] = [
    "SVG (Data URI)",
    "PNG (Data URI)",
    "HTML <img> tag",
    "CSS background",
];

fn placeholder_image() -> Tool {
    Tool {
        info: ToolInfo {
            id: "placeholder-image",
            name: "Placeholder Image",
            category: Category::WebDev,
            description: "Generate placeholder images as data URIs",
            keywords: vec!["placeholder", "image", "dummy", "svg", "png", "data uri"],
            icon: "image",
            input_placeholder: "Enter text to display on image (optional)...",
            output_placeholder: "Data URI will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "width",
                    label: "Width (px)",
                    help: Some("Image width in pixels"),
                    kind: OptionKind::Stepper {
                        min: 1,
                        max: 2000,
                        default: 400,
                    },
                },
                ToolOption {
                    id: "height",
                    label: "Height (px)",
                    help: Some("Image height in pixels"),
                    kind: OptionKind::Stepper {
                        min: 1,
                        max: 2000,
                        default: 300,
                    },
                },
                ToolOption {
                    id: "format",
                    label: "Format",
                    help: Some("Output format"),
                    kind: OptionKind::Picker {
                        choices: PLACEHOLDER_FORMATS.to_vec(),
                        default: 0,
                    },
                },
                ToolOption {
                    id: "background",
                    label: "Background Color",
                    help: Some("Background color (hex, rgb, or named color)"),
                    kind: OptionKind::TextField {
                        placeholder: "#CCCCCC",
                        default: "#CCCCCC",
                    },
                },
                ToolOption {
                    id: "textColor",
                    label: "Text Color",
                    help: Some("Text color (hex, rgb, or named color)"),
                    kind: OptionKind::TextField {
                        placeholder: "#666666",
                        default: "#666666",
                    },
                },
                ToolOption {
                    id: "textStyle",
                    label: "Text Style",
                    help: Some("What text to display on the image"),
                    // "Dimensions" renders the current size, e.g. "400×300".
                    kind: OptionKind::Picker {
                        choices: vec!["Dimensions", "Custom text", "None"],
                        default: 0,
                    },
                },
                toggle(
                    "border",
                    "Show border",
                    "Add a border around the image",
                    false,
                ),
            ],
        },
        process: Some(placeholder_process),
    }
}

/// 5x7 glyph rows (bit 4 = leftmost column).
fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        ' ' => [0; 7],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ',' => [0, 0, 0, 0, 0b01100, 0b00100, 0b01000],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '×' => [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001],
        // Anything outside the embedded set renders as '?'.
        _ => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0, 0b00100],
    }
}

/// Everything that decides how a placeholder looks, shared by the SVG and PNG renderers.
struct PlaceholderStyle {
    w: usize,
    h: usize,
    bg: Rgba,
    fg: Rgba,
    text: Option<String>,
    cross: bool,
    border: bool,
}

fn placeholder_svg(s: &PlaceholderStyle) -> String {
    let (w, h) = (s.w, s.h);
    let hex = |c: Rgba| format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b);
    let fg = hex(s.fg);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        hex(s.bg)
    );
    if s.cross {
        svg.push_str(&format!(
            "<line x1=\"0\" y1=\"0\" x2=\"{w}\" y2=\"{h}\" stroke=\"{fg}\" stroke-width=\"1\" opacity=\"0.3\"/><line x1=\"{w}\" y1=\"0\" x2=\"0\" y2=\"{h}\" stroke=\"{fg}\" stroke-width=\"1\" opacity=\"0.3\"/>"
        ));
    }
    if let Some(text) = &s.text {
        let len = text.chars().count().max(1) as f64;
        let size = (h as f64 * 0.2).min(w as f64 * 0.9 / (len * 0.6)).max(8.0);
        svg.push_str(&format!(
            "<text x=\"50%\" y=\"50%\" dominant-baseline=\"middle\" text-anchor=\"middle\" font-family=\"system-ui, -apple-system, sans-serif\" font-size=\"{size:.0}\" fill=\"{fg}\">{}</text>",
            xml_escape(text)
        ));
    }
    if s.border {
        svg.push_str(&format!(
            "<rect x=\"1\" y=\"1\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{fg}\" stroke-width=\"2\"/>",
            w.saturating_sub(2),
            h.saturating_sub(2)
        ));
    }
    svg.push_str("</svg>");
    svg
}

fn placeholder_png(s: &PlaceholderStyle) -> Result<Vec<u8>, ToolError> {
    let (w, h, fg) = (s.w, s.h, s.fg);
    let mut px: Vec<u8> = [s.bg.r, s.bg.g, s.bg.b].repeat(w * h);
    let mut put = |x: usize, y: usize, alpha: f64| {
        if x < w && y < h {
            let o = (y * w + x) * 3;
            for (k, v) in [fg.r, fg.g, fg.b].into_iter().enumerate() {
                px[o + k] =
                    (f64::from(px[o + k]) * (1.0 - alpha) + f64::from(v) * alpha).round() as u8;
            }
        }
    };
    if s.cross {
        for x in 0..w {
            let y = x * (h - 1) / (w - 1).max(1);
            put(x, y, 0.3);
            put(x, h - 1 - y, 0.3);
        }
    }
    if let Some(text) = &s.text {
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len().max(1);
        let scale = ((w * 9 / 10) / (n * 6)).min(h * 2 / 10 / 7).max(1);
        let x0 = w.saturating_sub((n * 6 - 1) * scale) / 2;
        let y0 = h.saturating_sub(7 * scale) / 2;
        for (ci, ch) in chars.iter().enumerate() {
            for (row, bits) in glyph(*ch).into_iter().enumerate() {
                for col in 0..5 {
                    if bits >> (4 - col) & 1 == 1 {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                put(x0 + (ci * 6 + col) * scale + dx, y0 + row * scale + dy, 1.0);
                            }
                        }
                    }
                }
            }
        }
    }
    if s.border {
        for x in 0..w {
            for y in [0, 1, h.wrapping_sub(2), h.wrapping_sub(1)] {
                put(x, y, 1.0);
            }
        }
        for y in 0..h {
            for x in [0, 1, w.wrapping_sub(2), w.wrapping_sub(1)] {
                put(x, y, 1.0);
            }
        }
    }
    let failed = |_| ToolError::ProcessingFailed("Failed to generate PNG image".into());
    let mut buf = Vec::new();
    let mut encoder = png::Encoder::new(&mut buf, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&px))
        .map_err(failed)?;
    Ok(buf)
}

fn placeholder_process(req: &Request) -> ToolResult {
    let (w, h) = (
        req.options.i64("width") as usize,
        req.options.i64("height") as usize,
    );
    let color = |id: &str| {
        parse_color(req.options.str(id)).ok_or_else(|| {
            ToolError::InvalidInput(format!("Invalid color '{}'", req.options.str(id)))
        })
    };
    let style_name = req.options.str("textStyle");
    let custom = collapse_ws(req.input);
    let style = PlaceholderStyle {
        w,
        h,
        bg: color("background")?,
        fg: color("textColor")?,
        text: match style_name {
            "None" => None,
            "Custom text" if !custom.is_empty() => Some(custom),
            _ => Some(format!("{w}×{h}")),
        },
        cross: style_name == "None",
        border: req.options.bool("border"),
    };
    let svg_uri = format!(
        "data:image/svg+xml,{}",
        percent_encode(placeholder_svg(&style).as_bytes())
    );
    let (label, value, preview) = match req.options.str("format") {
        "SVG (Data URI)" => ("Data URI", svg_uri.clone(), svg_uri),
        "PNG (Data URI)" => {
            let uri = format!(
                "data:image/png;base64,{}",
                STANDARD.encode(placeholder_png(&style)?)
            );
            ("Data URI", uri.clone(), uri)
        }
        "HTML <img> tag" => (
            "HTML",
            format!("<img src=\"{svg_uri}\" alt=\"Placeholder\" width=\"{w}\" height=\"{h}\">"),
            svg_uri,
        ),
        "CSS background" => (
            "CSS",
            format!("background-image: url('{svg_uri}');"),
            svg_uri,
        ),
        other => return Err(ToolError::InvalidInput(format!("Unknown format: {other}"))),
    };
    Ok(ToolOutput::multiple(vec![
        OutputItem::new(label, value),
        OutputItem::image("Preview", preview),
    ]))
}
