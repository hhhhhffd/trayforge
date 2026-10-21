use chrono::{
    format::{Item, StrftimeItems},
    DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone, Utc,
};
use chrono_tz::Tz;

use super::{picker, stepper, text_field, toggle};
use crate::{Category, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOutput, ToolResult};

pub fn base_converter_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "base-converter",
            name: "Base Converter",
            category: Category::Text,
            description: "Convert numbers between binary, octal, decimal, and hexadecimal",
            keywords: vec![
                "base", "binary", "octal", "decimal", "hex", "radix", "number",
            ],
            icon: "calculator",
            input_placeholder: "Enter a number (e.g., 42, 0x2A, 0b101010, 0o52)...",
            output_placeholder: "Converted numbers will appear here",
            primary_action: "Convert",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "inputBase",
                    "Input base",
                    "Base of input number",
                    vec![
                        "Auto-detect",
                        "Binary (2)",
                        "Octal (8)",
                        "Decimal (10)",
                        "Hexadecimal (16)",
                        "Custom",
                    ],
                ),
                stepper(
                    "customBase",
                    "Custom base",
                    "Custom base (2-36)",
                    (2, 36, 10),
                ),
                toggle(
                    "showPrefixes",
                    "Show prefixes",
                    "Show 0b, 0o, 0x prefixes",
                    true,
                ),
                toggle(
                    "groupDigits",
                    "Group digits",
                    "Add separators for readability",
                    true,
                ),
                toggle(
                    "uppercaseHex",
                    "Uppercase hex",
                    "Use uppercase letters for hex",
                    true,
                ),
            ],
        },
        process: Some(base_converter_process),
    }
}

fn base_name(base: u32) -> String {
    match base {
        2 => "Binary".into(),
        8 => "Octal".into(),
        10 => "Decimal".into(),
        16 => "Hexadecimal".into(),
        n => format!("Base {n}"),
    }
}

fn base_converter_process(req: &Request) -> ToolResult {
    let cleaned: String = req
        .input
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '_' | ','))
        .collect();
    let (negative, unsigned) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    let lower = unsigned.to_lowercase();
    let prefixed = [("0b", 2), ("0o", 8), ("0x", 16)]
        .into_iter()
        .find(|(p, _)| lower.starts_with(p));
    let selected = req.options.str("inputBase");
    let auto = selected == "Auto-detect";
    let (base, digits) = match (selected, prefixed) {
        ("Auto-detect", Some((p, b))) => (b, &lower[p.len()..]),
        ("Auto-detect", None) if !lower.is_empty() && lower.chars().all(|c| c.is_ascii_digit()) => {
            (10, lower.as_str())
        }
        ("Auto-detect", None) => (16, lower.as_str()),
        (name, prefix) => {
            let base = match name {
                "Binary (2)" => 2,
                "Octal (8)" => 8,
                "Decimal (10)" => 10,
                "Hexadecimal (16)" => 16,
                _ => req.options.i64("customBase").clamp(2, 36) as u32,
            };
            match prefix {
                Some((p, b)) if b == base => (base, &lower[p.len()..]),
                _ => (base, lower.as_str()),
            }
        }
    };
    // `from_str_radix` also accepts a leading '+'.
    if digits.starts_with('+') {
        return Err(ToolError::InvalidInput(format!(
            "Invalid number for base {base}"
        )));
    }
    let value = u128::from_str_radix(digits, base).map_err(|e| match e.kind() {
        std::num::IntErrorKind::PosOverflow => {
            ToolError::InvalidInput("Number is too large".into())
        }
        _ => ToolError::InvalidInput(format!("Invalid number for base {base}")),
    })?;

    let show_prefixes = req.options.bool("showPrefixes");
    let group = req.options.bool("groupDigits");
    let uppercase = req.options.bool("uppercaseHex");
    let sign = if negative && value != 0 { "-" } else { "" };
    let render = |radix: u32, prefix: &str, group_size: usize, separator: char| {
        let mut text = to_radix(value, radix);
        if radix > 10 && uppercase {
            text = text.to_uppercase();
        }
        if group {
            text = group_digits(&text, group_size, separator);
        }
        let prefix = if show_prefixes { prefix } else { "" };
        format!("{sign}{prefix}{text}")
    };

    let mut items = vec![
        OutputItem::new("Decimal (10)", render(10, "", 3, ',')),
        OutputItem::new("Binary (2)", render(2, "0b", 4, ' ')),
        OutputItem::new("Octal (8)", render(8, "0o", 3, ' ')),
        OutputItem::new("Hexadecimal (16)", render(16, "0x", 4, ' ')),
    ];
    if selected == "Custom" {
        let custom = req.options.i64("customBase").clamp(2, 36) as u32;
        if ![2, 8, 10, 16].contains(&custom) {
            items.push(OutputItem::new(
                format!("Base {custom}"),
                render(custom, "", 4, ' '),
            ));
        }
    }
    items.push(OutputItem::new(
        "Bits required",
        (128 - value.leading_zeros()).max(1).to_string(),
    ));
    items.push(OutputItem::new(
        "Detected input base",
        format!("{}{}", base_name(base), if auto { " (auto)" } else { "" }),
    ));
    Ok(ToolOutput::multiple(items))
}

fn to_radix(mut value: u128, radix: u32) -> String {
    if value == 0 {
        return "0".into();
    }
    let mut digits = Vec::new();
    while value > 0 {
        digits
            .push(char::from_digit((value % radix as u128) as u32, radix).expect("digit < radix"));
        value /= radix as u128;
    }
    digits.iter().rev().collect()
}

fn group_digits(text: &str, size: usize, separator: char) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(size) {
            out.push(separator);
        }
        out.push(*c);
    }
    out
}

const TIMEZONES: [&str; 10] = [
    "Local",
    "UTC",
    "GMT",
    "EST",
    "PST",
    "CET",
    "JST",
    "Europe/London",
    "Europe/Paris",
    "Asia/Tokyo",
];

#[derive(Clone, Copy)]
enum Zone {
    Local,
    Named(Tz),
}

impl Zone {
    fn parse(name: &str) -> Zone {
        let iana = match name {
            "Local" => return Zone::Local,
            "UTC" => "UTC",
            "GMT" => "GMT",
            "EST" => "America/New_York",
            "PST" => "America/Los_Angeles",
            "CET" => "CET",
            "JST" => "Asia/Tokyo",
            other => other,
        };
        Zone::Named(iana.parse().unwrap_or(Tz::UTC))
    }

    fn view(self, instant: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self {
            Zone::Local => instant.with_timezone(&Local).fixed_offset(),
            Zone::Named(tz) => instant.with_timezone(&tz).fixed_offset(),
        }
    }

    fn localize(self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Zone::Local => Local
                .from_local_datetime(&naive)
                .earliest()
                .map(|d| d.to_utc()),
            Zone::Named(tz) => tz
                .from_local_datetime(&naive)
                .earliest()
                .map(|d| d.to_utc()),
        }
    }
}

#[derive(Clone, Copy)]
enum Unit {
    Seconds,
    Millis,
    Micros,
    Nanos,
}

impl Unit {
    fn nanos(self) -> i128 {
        match self {
            Unit::Seconds => 1_000_000_000,
            Unit::Millis => 1_000_000,
            Unit::Micros => 1_000,
            Unit::Nanos => 1,
        }
    }
}

/// `None` unit means auto-detect by the number of integer digits.
fn parse_numeric(text: &str, unit: Option<Unit>) -> Option<DateTime<Utc>> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (int_part, frac_part) = body.split_once('.').unwrap_or((body, ""));
    let all_digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if int_part.is_empty() || int_part.len() > 25 || !all_digits(int_part) || !all_digits(frac_part)
    {
        return None;
    }
    let unit = unit.unwrap_or(match int_part.len() {
        0..=11 => Unit::Seconds,
        12..=14 => Unit::Millis,
        15..=17 => Unit::Micros,
        _ => Unit::Nanos,
    });
    let frac_part = &frac_part[..frac_part.len().min(9)];
    let fraction = if frac_part.is_empty() {
        0
    } else {
        frac_part.parse::<i128>().ok()? * unit.nanos() / 10i128.pow(frac_part.len() as u32)
    };
    let total = int_part.parse::<i128>().ok()? * unit.nanos() + fraction;
    let total = if negative { -total } else { total };
    DateTime::from_timestamp(
        total.div_euclid(1_000_000_000).try_into().ok()?,
        total.rem_euclid(1_000_000_000) as u32,
    )
}

fn parse_date(text: &str, zone: Zone) -> Option<DateTime<Utc>> {
    if let Ok(d) =
        DateTime::parse_from_rfc3339(text).or_else(|_| DateTime::parse_from_rfc2822(text))
    {
        return Some(d.to_utc());
    }
    const DATE_TIME: [&str; 5] = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
    ];
    for format in DATE_TIME {
        if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
            return zone.localize(naive);
        }
    }
    ["%Y-%m-%d", "%Y/%m/%d"]
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(text, f).ok())
        .and_then(|d| zone.localize(d.and_hms_opt(0, 0, 0)?))
}

pub fn unix_timestamp_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "unix-timestamp",
            name: "Unix Timestamp",
            category: Category::Text,
            description: "Convert between Unix timestamps and human-readable dates",
            keywords: vec!["unix", "timestamp", "epoch", "date", "time", "iso"],
            icon: "clock",
            input_placeholder: "Enter timestamp or date — leave empty for current time...",
            output_placeholder: "Converted values will appear here",
            primary_action: "Convert",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                picker(
                    "precision",
                    "Input precision",
                    "Timestamp precision",
                    vec![
                        "Auto-detect",
                        "Seconds",
                        "Milliseconds",
                        "Microseconds",
                        "Nanoseconds",
                    ],
                ),
                picker(
                    "timezone",
                    "Timezone",
                    "Timezone for display",
                    TIMEZONES.to_vec(),
                ),
                toggle(
                    "showRelative",
                    "Show relative time",
                    "Show how long ago/from now",
                    true,
                ),
                toggle(
                    "showAllFormats",
                    "Show all date formats",
                    "Display multiple date formats",
                    true,
                ),
            ],
        },
        process: Some(unix_timestamp_process),
    }
}

fn relative_time(instant: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = (now - instant).num_seconds();
    let magnitude = seconds.abs();
    let (amount, unit) = match magnitude {
        0..=4 => return "just now".into(),
        5..=59 => (magnitude, "second"),
        60..=3599 => (magnitude / 60, "minute"),
        3600..=86_399 => (magnitude / 3600, "hour"),
        86_400..=2_591_999 => (magnitude / 86_400, "day"),
        2_592_000..=31_535_999 => (magnitude / 2_592_000, "month"),
        _ => (magnitude / 31_536_000, "year"),
    };
    let plural = if amount == 1 { "" } else { "s" };
    if seconds >= 0 {
        format!("{amount} {unit}{plural} ago")
    } else {
        format!("in {amount} {unit}{plural}")
    }
}

fn iso8601(dt: &DateTime<FixedOffset>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn unix_timestamp_process(req: &Request) -> ToolResult {
    let zone = Zone::parse(req.options.str("timezone"));
    let text = req.input.trim();
    let now = Utc::now();
    let instant = if text.is_empty() {
        now
    } else {
        let unit = match req.options.str("precision") {
            "Seconds" => Some(Unit::Seconds),
            "Milliseconds" => Some(Unit::Millis),
            "Microseconds" => Some(Unit::Micros),
            "Nanoseconds" => Some(Unit::Nanos),
            _ => None,
        };
        parse_numeric(text, unit)
            .or_else(|| parse_date(text, zone))
            .ok_or_else(|| {
                ToolError::InvalidInput(format!("Could not parse '{text}' as timestamp or date"))
            })?
    };
    let local = zone.view(instant);

    let mut items = vec![
        OutputItem::new("Unix (seconds)", instant.timestamp().to_string()),
        OutputItem::new(
            "Unix (milliseconds)",
            instant.timestamp_millis().to_string(),
        ),
        OutputItem::new("ISO 8601", iso8601(&local)),
    ];
    if req.options.bool("showAllFormats") {
        items.push(OutputItem::new(
            "Standard",
            local.format("%Y-%m-%d %H:%M:%S").to_string(),
        ));
        items.push(OutputItem::new(
            "Long format",
            local.format("%A, %B %-d, %Y at %-I:%M %p").to_string(),
        ));
        items.push(OutputItem::new(
            "US format",
            local.format("%m/%d/%Y %I:%M:%S %p").to_string(),
        ));
        items.push(OutputItem::new(
            "EU format",
            local.format("%d/%m/%Y %H:%M:%S").to_string(),
        ));
        items.push(OutputItem::new(
            "Day of year",
            format!("{} (Week {})", local.ordinal(), local.format("%V")),
        ));
    }
    if req.options.bool("showRelative") {
        items.push(OutputItem::new("Relative", relative_time(instant, now)));
    }
    Ok(ToolOutput::multiple(items))
}

/// Translates a Foundation `DateFormatter` pattern into a chrono strftime format.
fn pattern_to_strftime(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                out.push_str(&chars[i].to_string().replace('%', "%%"));
                i += 1;
            }
            // A doubled quote inside the literal ('') stands for one quote.
            if chars.get(i + 1) == Some(&'\'') {
                out.push('\'');
                i += 1;
            }
            i += 1;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            out.push_str(&c.to_string().replace('%', "%%"));
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|x| **x == c).count();
        i += run;
        out.push_str(match (c, run) {
            ('y', 2) => "%y",
            ('y', _) => "%Y",
            ('M', 1) => "%-m",
            ('M', 2) => "%m",
            ('M', 3) => "%b",
            ('M', _) => "%B",
            ('d', 1) => "%-d",
            ('d', _) => "%d",
            ('D', _) => "%j",
            ('H', 1) => "%-H",
            ('H', _) => "%H",
            ('h', 1) => "%-I",
            ('h', _) => "%I",
            ('m', 1) => "%-M",
            ('m', _) => "%M",
            ('s', 1) => "%-S",
            ('s', _) => "%S",
            ('S', 1..=3) => "%3f",
            ('S', 4..=6) => "%6f",
            ('S', _) => "%9f",
            ('a', _) => "%p",
            ('E', 1..=3) => "%a",
            ('E', _) => "%A",
            ('Z', 1..=3) => "%z",
            ('Z', _) => "%:z",
            ('z', _) => "%Z",
            _ => {
                out.push_str(&c.to_string().repeat(run));
                continue;
            }
        });
    }
    out
}

fn parse_custom(text: &str, format: &str, zone: Zone) -> Option<DateTime<Utc>> {
    if let Ok(d) = DateTime::parse_from_str(text, format) {
        return Some(d.to_utc());
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
        return zone.localize(naive);
    }
    let date = NaiveDate::parse_from_str(text, format).ok()?;
    zone.localize(date.and_hms_opt(0, 0, 0)?)
}

pub fn epoch_batch_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "epoch-batch-converter",
            name: "Epoch Batch Converter",
            category: Category::Text,
            description: "Convert multiple timestamps or dates in batch",
            keywords: vec!["epoch", "batch", "timestamp", "convert", "dates", "bulk"],
            icon: "calendar-clock",
            input_placeholder: "Enter timestamps or dates (one per line):\n1708963200\n2024-02-26T12:00:00Z\n1708963200000",
            output_placeholder: "Converted timestamps will appear here",
            primary_action: "To Dates",
            secondary_action: Some("To Timestamps"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "inputFormat",
                    "Input Format",
                    "Format of input timestamps",
                    vec![
                        "Auto-detect",
                        "Unix Seconds",
                        "Unix Milliseconds",
                        "ISO 8601",
                        "RFC 2822",
                        "Custom",
                    ],
                ),
                picker(
                    "outputFormat",
                    "Output Format",
                    "Format for output timestamps",
                    vec![
                        "ISO 8601",
                        "RFC 2822",
                        "Unix Seconds",
                        "Unix Milliseconds",
                        "Custom",
                        "Relative",
                    ],
                ),
                text_field(
                    "customFormat",
                    "Custom Format",
                    "Custom date format (when Custom is selected)",
                    "yyyy-MM-dd HH:mm:ss",
                    "yyyy-MM-dd HH:mm:ss",
                ),
                picker("timezone", "Timezone", "Timezone for conversion", TIMEZONES.to_vec()),
                toggle(
                    "includeOriginal",
                    "Include original value",
                    "Show original value alongside converted",
                    true,
                ),
                toggle(
                    "skipInvalid",
                    "Skip invalid lines",
                    "Skip lines that cannot be parsed",
                    true,
                ),
            ],
        },
        process: Some(epoch_batch_process),
    }
}

fn epoch_batch_process(req: &Request) -> ToolResult {
    let zone = Zone::parse(req.options.str("timezone"));
    let input_format = req.options.str("inputFormat");
    let output_format = req.options.str("outputFormat");
    let custom = pattern_to_strftime(req.options.str("customFormat"));
    if StrftimeItems::new(&custom).any(|item| matches!(item, Item::Error)) {
        return Err(ToolError::InvalidOptions(
            "Invalid custom date format".into(),
        ));
    }
    let include_original = req.options.bool("includeOriginal");
    let skip_invalid = req.options.bool("skipInvalid");
    let now = Utc::now();

    let parse = |text: &str| match input_format {
        "Unix Seconds" => parse_numeric(text, Some(Unit::Seconds)),
        "Unix Milliseconds" => parse_numeric(text, Some(Unit::Millis)),
        "ISO 8601" => DateTime::parse_from_rfc3339(text).ok().map(|d| d.to_utc()),
        "RFC 2822" => DateTime::parse_from_rfc2822(text).ok().map(|d| d.to_utc()),
        "Custom" => parse_custom(text, &custom, zone),
        _ => parse_numeric(text, None).or_else(|| parse_date(text, zone)),
    };
    let format = |instant: DateTime<Utc>| {
        let local = zone.view(instant);
        match output_format {
            "RFC 2822" => local.to_rfc2822(),
            "Unix Seconds" => instant.timestamp().to_string(),
            "Unix Milliseconds" => instant.timestamp_millis().to_string(),
            "Custom" => local.format(&custom).to_string(),
            "Relative" => relative_time(instant, now),
            _ => iso8601(&local),
        }
    };

    let mut out = Vec::new();
    for line in req.input.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let converted = if req.reverse {
            // To Timestamps: dates in, Unix timestamps out.
            parse(line).map(|d| match output_format {
                "Unix Milliseconds" => d.timestamp_millis().to_string(),
                _ => d.timestamp().to_string(),
            })
        } else {
            parse(line).map(format)
        };
        match converted {
            Some(value) if include_original => out.push(format!("{line} → {value}")),
            Some(value) => out.push(value),
            None if skip_invalid => {}
            None if include_original => out.push(format!("{line} → Invalid format")),
            None => out.push("Invalid format".into()),
        }
    }
    if out.is_empty() {
        return Err(ToolError::ProcessingFailed(
            "No valid timestamps found".into(),
        ));
    }
    Ok(ToolOutput::text(out.join("\n")))
}
