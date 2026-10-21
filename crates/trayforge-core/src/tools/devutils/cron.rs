use super::time::{civil_from_days, format_datetime, now_unix, weekday, WEEKDAYS};
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Days scanned for upcoming runs; covers a Feb 29 schedule across a century gap.
const SEARCH_DAYS: i64 = 366 * 8;

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "cron-parser",
            name: "Cron Parser",
            category: Category::DevUtils,
            description: "Parse cron expressions and show next execution times",
            keywords: vec!["cron", "schedule", "time", "unix", "crontab"],
            icon: "calendar-clock",
            input_placeholder: "Enter cron expression (e.g., */5 * * * * or 0 9 * * MON-FRI)...",
            output_placeholder: "Human-readable schedule and next runs will appear here",
            primary_action: "Parse",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "count",
                    label: "Show next N runs",
                    help: Some("Number of upcoming execution times"),
                    kind: OptionKind::Stepper {
                        min: 1,
                        max: 20,
                        default: 5,
                    },
                },
                ToolOption {
                    id: "seconds",
                    label: "6-field format (seconds)",
                    help: Some("Include seconds field"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(process),
    }
}

fn process(req: &Request) -> ToolResult {
    let count = req.options.i64("count") as usize;
    parse(req.input, count, req.options.bool("seconds"), now_unix())
}

struct Schedule {
    secs: Vec<u32>,
    mins: Vec<u32>,
    hours: Vec<u32>,
    doms: Vec<u32>,
    months: Vec<u32>,
    dows: Vec<u32>,
    /// `*`/`?` fields: standard cron ORs day-of-month and day-of-week only
    /// when both are restricted.
    dom_any: bool,
    dow_any: bool,
    /// Raw text of minute and hour fields, for step detection in descriptions.
    raw_min: String,
    raw_hour: String,
    raw_sec: String,
}

fn parse(input: &str, count: usize, with_seconds: bool, now: i64) -> ToolResult {
    let input = input.trim();
    let expanded = expand_macro(input, with_seconds)?;
    let fields: Vec<&str> = expanded.split_whitespace().collect();
    let expected = if with_seconds { 6 } else { 5 };
    if fields.len() != expected {
        let names = if with_seconds {
            "second minute hour day-of-month month day-of-week"
        } else {
            "minute hour day-of-month month day-of-week"
        };
        return Err(ToolError::InvalidInput(format!(
            "Invalid cron expression: expected {expected} fields ({names}), got {}",
            fields.len()
        )));
    }
    let schedule = Schedule::parse(&fields, with_seconds)?;

    let mut out = format!(
        "Expression: {input}\nDescription: {}\n\nNext {count} runs (UTC):\n",
        schedule.describe()
    );
    let runs = schedule.next_runs(now, count);
    if runs.is_empty() {
        out.push_str("  No upcoming runs found\n");
    }
    for (i, ts) in runs.iter().enumerate() {
        let day = WEEKDAYS[weekday(ts.div_euclid(86_400))];
        out.push_str(&format!("  {}. {} ({day})\n", i + 1, format_datetime(*ts)));
    }
    Ok(ToolOutput::text(out.trim_end()))
}

fn expand_macro(input: &str, with_seconds: bool) -> Result<String, ToolError> {
    let five = match input.to_ascii_lowercase().as_str() {
        "@yearly" | "@annually" => "0 0 1 1 *",
        "@monthly" => "0 0 1 * *",
        "@weekly" => "0 0 * * 0",
        "@daily" | "@midnight" => "0 0 * * *",
        "@hourly" => "0 * * * *",
        m if m.starts_with('@') => {
            return Err(ToolError::InvalidInput(format!(
                "Unsupported macro '{input}'"
            )))
        }
        _ => return Ok(input.to_string()),
    };
    Ok(if with_seconds {
        format!("0 {five}")
    } else {
        five.to_string()
    })
}

/// Expands one cron field into its sorted allowed values.
fn parse_field(
    name: &str,
    text: &str,
    min: u32,
    max: u32,
    names: &[&str],
) -> Result<Vec<u32>, ToolError> {
    let fail =
        |why: String| ToolError::InvalidInput(format!("Invalid {name} field '{text}': {why}"));
    let value = |s: &str| -> Result<u32, ToolError> {
        let v = match s.parse::<u32>() {
            Ok(v) => v,
            Err(_) => names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(s))
                .map(|i| i as u32 + min)
                .ok_or_else(|| fail(format!("unknown value '{s}'")))?,
        };
        if (min..=max).contains(&v) {
            Ok(v)
        } else {
            Err(fail(format!("{v} is out of range ({min}-{max})")))
        }
    };

    let mut values = Vec::new();
    for part in text.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((range, step)) => (
                range,
                Some(
                    step.parse::<u32>()
                        .ok()
                        .filter(|s| *s > 0)
                        .ok_or_else(|| fail(format!("invalid step '{step}'")))?,
                ),
            ),
            None => (part, None),
        };
        let (start, end) = if range == "*" || range == "?" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            (value(a)?, value(b)?)
        } else {
            let v = value(range)?;
            (v, if step.is_some() { max } else { v })
        };
        if start > end {
            return Err(fail(format!("range {start}-{end} is reversed")));
        }
        values.extend((start..=end).step_by(step.unwrap_or(1) as usize));
    }
    values.sort_unstable();
    values.dedup();
    Ok(values)
}

impl Schedule {
    fn parse(fields: &[&str], with_seconds: bool) -> Result<Self, ToolError> {
        let (sec, rest) = if with_seconds {
            (fields[0], &fields[1..])
        } else {
            ("0", fields)
        };
        let month_names: Vec<&str> = MONTHS.iter().map(|m| &m[..3]).collect();
        let day_names: Vec<&str> = WEEKDAYS.iter().map(|d| &d[..3]).collect();
        let mut dows = parse_field("day-of-week", rest[4], 0, 7, &day_names)?;
        // 7 is an alias of Sunday.
        for d in &mut dows {
            *d %= 7;
        }
        dows.sort_unstable();
        dows.dedup();
        let any = |f: &str| f.starts_with('*') || f.starts_with('?');
        Ok(Schedule {
            secs: parse_field("second", sec, 0, 59, &[])?,
            mins: parse_field("minute", rest[0], 0, 59, &[])?,
            hours: parse_field("hour", rest[1], 0, 23, &[])?,
            doms: parse_field("day-of-month", rest[2], 1, 31, &[])?,
            months: parse_field("month", rest[3], 1, 12, &month_names)?,
            dows,
            dom_any: any(rest[2]),
            dow_any: any(rest[4]),
            raw_min: rest[0].to_string(),
            raw_hour: rest[1].to_string(),
            raw_sec: sec.to_string(),
        })
    }

    fn day_matches(&self, days: i64) -> bool {
        let (_, month, dom) = civil_from_days(days);
        if !self.months.contains(&month) {
            return false;
        }
        let dom_ok = self.doms.contains(&dom);
        let dow_ok = self.dows.contains(&(weekday(days) as u32));
        if !self.dom_any && !self.dow_any {
            dom_ok || dow_ok
        } else {
            dom_ok && dow_ok
        }
    }

    fn next_runs(&self, now: i64, count: usize) -> Vec<i64> {
        let mut runs = Vec::new();
        let first_day = now.div_euclid(86_400);
        for days in first_day..first_day + SEARCH_DAYS {
            if !self.day_matches(days) {
                continue;
            }
            for &h in &self.hours {
                for &m in &self.mins {
                    for &s in &self.secs {
                        let ts = days * 86_400 + i64::from(h * 3600 + m * 60 + s);
                        if ts > now {
                            runs.push(ts);
                            if runs.len() == count {
                                return runs;
                            }
                        }
                    }
                }
            }
        }
        runs
    }

    fn describe(&self) -> String {
        let time = self.describe_time();
        let all_days = self.dom_any && self.dow_any;
        let mut text = time.clone();
        if all_days && self.months.len() == 12 && time.starts_with("At ") && time.contains(':') {
            text.push_str(" every day");
        }
        if !self.dom_any {
            text.push_str(&format!(" on day {} of the month", join(&self.doms)));
        }
        if !self.dow_any {
            let sep = if self.dom_any { " on " } else { " and on " };
            text.push_str(&format!("{sep}{}", describe_weekdays(&self.dows)));
        }
        if self.months.len() != 12 {
            let names: Vec<&str> = self
                .months
                .iter()
                .map(|m| MONTHS[*m as usize - 1])
                .collect();
            text.push_str(&format!(" in {}", names.join(", ")));
        }
        text
    }

    fn describe_time(&self) -> String {
        let step = |raw: &str| raw.strip_prefix("*/").and_then(|n| n.parse::<u32>().ok());
        let every_minute = self.mins.len() == 60 && self.hours.len() == 24;
        let minutes = if every_minute {
            "Every minute".to_string()
        } else if let (Some(n), 24) = (step(&self.raw_min), self.hours.len()) {
            format!("Every {n} minutes")
        } else if let (&[m], 24) = (&self.mins[..], self.hours.len()) {
            match m {
                0 => "Every hour".to_string(),
                _ => format!("At minute {m} past every hour"),
            }
        } else if let (&[m], Some(n)) = (&self.mins[..], step(&self.raw_hour)) {
            match m {
                0 => format!("Every {n} hours"),
                _ => format!("At minute {m} past every {n} hours"),
            }
        } else if self.mins.len() * self.hours.len() <= 6 {
            let times: Vec<String> = self
                .hours
                .iter()
                .flat_map(|h| self.mins.iter().map(move |m| format!("{h:02}:{m:02}")))
                .collect();
            format!("At {}", times.join(", "))
        } else {
            format!("At minute {} past hour {}", self.raw_min, self.raw_hour)
        };

        let seconds = match (&self.secs[..], step(&self.raw_sec)) {
            ([0], _) => return minutes,
            (s, _) if s.len() == 60 => "Every second".to_string(),
            (_, Some(n)) => format!("Every {n} seconds"),
            ([s], _) => format!("At second {s}"),
            _ => format!("At seconds {}", self.raw_sec),
        };
        if every_minute {
            seconds
        } else {
            let mut chars = minutes.chars();
            let lowered: String = chars
                .next()
                .map(|c| c.to_lowercase().chain(chars).collect())
                .unwrap_or_default();
            format!("{seconds}, {lowered}")
        }
    }
}

fn join(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_weekdays(dows: &[u32]) -> String {
    let contiguous = dows.windows(2).all(|w| w[1] == w[0] + 1);
    if dows.len() >= 3 && contiguous {
        format!(
            "{} through {}",
            WEEKDAYS[dows[0] as usize],
            WEEKDAYS[dows[dows.len() - 1] as usize]
        )
    } else {
        dows.iter()
            .map(|d| WEEKDAYS[*d as usize])
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::super::time::days_from_civil;
    use super::*;

    // 2026-10-21 12:34:56 UTC, a Wednesday.
    fn now() -> i64 {
        days_from_civil(2026, 10, 21) * 86_400 + 12 * 3600 + 34 * 60 + 56
    }

    fn output(expr: &str, count: usize, seconds: bool) -> String {
        parse(expr, count, seconds, now())
            .unwrap()
            .as_text()
            .to_string()
    }

    #[test]
    fn every_five_minutes() {
        let out = output("*/5 * * * *", 2, false);
        assert!(out.contains("Description: Every 5 minutes"));
        assert!(out.contains("1. 2026-10-21 12:35:00 (Wednesday)"));
        assert!(out.contains("2. 2026-10-21 12:40:00 (Wednesday)"));
    }

    #[test]
    fn weekday_mornings() {
        let out = output("0 9 * * MON-FRI", 3, false);
        assert!(out.contains("At 09:00 on Monday through Friday"));
        assert!(out.contains("1. 2026-10-22 09:00:00 (Thursday)"));
        assert!(out.contains("3. 2026-10-26 09:00:00 (Monday)"));
    }

    #[test]
    fn day_of_month_or_day_of_week() {
        let out = output("0 0 13 * 5", 2, false);
        assert!(out.contains("1. 2026-10-23 00:00:00 (Friday)"));
        assert!(out.contains("2. 2026-10-30 00:00:00 (Friday)"));
    }

    #[test]
    fn leap_day() {
        let out = output("0 0 29 2 *", 1, false);
        assert!(out.contains("1. 2028-02-29 00:00:00 (Tuesday)"));
    }

    #[test]
    fn six_fields() {
        let out = output("30 */10 * * * *", 1, true);
        assert!(out.contains("1. 2026-10-21 12:40:30 (Wednesday)"));
    }

    #[test]
    fn macros_and_sunday_alias() {
        assert!(output("@daily", 1, false).contains("2026-10-22 00:00:00"));
        assert!(output("0 0 * * 7", 1, false).contains("2026-10-25 00:00:00 (Sunday)"));
    }

    #[test]
    fn rejects_bad_expressions() {
        for bad in [
            "* * * *",
            "60 * * * *",
            "*/0 * * * *",
            "5-1 * * * *",
            "@reboot",
            "a * * * *",
        ] {
            assert!(
                matches!(parse(bad, 1, false, now()), Err(ToolError::InvalidInput(_))),
                "{bad}"
            );
        }
    }
}
