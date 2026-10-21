use std::cmp::Ordering;

use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

const HELP: &str = " SEMANTIC VERSIONING HELP
Format: MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]

 VERSION COMPONENTS
MAJOR   Breaking changes (incompatible API changes)
MINOR   New features (backwards compatible)
PATCH   Bug fixes (backwards compatible)
PRERELEASE  Optional pre-release identifier (e.g., alpha.1)
BUILD   Optional build metadata (e.g., +build.123)

 EXAMPLES
1.0.0           Stable release
2.1.3           Major 2, minor 1, patch 3
1.0.0-alpha     Alpha pre-release
1.0.0-beta.2    Beta 2 pre-release
1.0.0-rc.1      Release candidate 1
1.0.0+build.1   With build metadata
1.0.0-alpha+001 Pre-release with build

 PRECEDENCE
1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta
< 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11
< 1.0.0-rc.1 < 1.0.0";

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "semver-comparator",
            name: "Semver Comparator",
            category: Category::DevUtils,
            description: "Parse, compare, and bump semantic versions",
            keywords: vec![
                "semver", "version", "semantic", "compare", "bump", "major", "minor", "patch",
            ],
            icon: "circle-arrow-up",
            input_placeholder: "Enter version(s) to parse/compare (e.g., 1.2.3, 2.0.0-beta.1)...",
            output_placeholder: "Version information will appear here",
            primary_action: "Analyze",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "action",
                    label: "Action",
                    help: Some("What to do with the version(s)"),
                    kind: OptionKind::Picker {
                        choices: vec![
                            "Parse & Info",
                            "Compare",
                            "Bump Major",
                            "Bump Minor",
                            "Bump Patch",
                        ],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "includePrerelease",
                    label: "Include pre-release in comparisons",
                    help: Some("Consider pre-release identifiers when comparing"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(process),
    }
}

#[derive(Clone)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<String>,
    build: String,
}

impl Version {
    fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
        let (rest, build) = text.split_once('+').unwrap_or((text, ""));
        let (core, pre) = rest.split_once('-').unwrap_or((rest, ""));
        let identifier =
            |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        let numeric = |s: &str| {
            !s.is_empty()
                && s.chars().all(|c| c.is_ascii_digit())
                && (s == "0" || !s.starts_with('0'))
        };

        let mut parts = core.split('.');
        let mut number = || parts.next().filter(|p| numeric(p))?.parse::<u64>().ok();
        let (major, minor, patch) = (number()?, number()?, number()?);
        if parts.next().is_some() {
            return None;
        }
        let pre: Vec<String> = if rest.contains('-') {
            pre.split('.').map(str::to_string).collect()
        } else {
            Vec::new()
        };
        if !pre
            .iter()
            .all(|p| identifier(p) && (!p.chars().all(|c| c.is_ascii_digit()) || numeric(p)))
        {
            return None;
        }
        if text.contains('+') && !build.split('.').all(identifier) {
            return None;
        }
        Some(Version {
            major,
            minor,
            patch,
            pre,
            build: build.to_string(),
        })
    }

    fn cmp(&self, other: &Version, include_prerelease: bool) -> Ordering {
        let core =
            (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core != Ordering::Equal || !include_prerelease {
            return core;
        }
        match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => compare_identifiers(&self.pre, &other.pre),
        }
    }

    fn bumped(&self, action: &str) -> Version {
        let mut next = Version {
            pre: Vec::new(),
            build: String::new(),
            ..self.clone()
        };
        match action {
            "Bump Major" => (next.major, next.minor, next.patch) = (self.major + 1, 0, 0),
            "Bump Minor" => (next.minor, next.patch) = (self.minor + 1, 0),
            _ => next.patch += 1,
        }
        next
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        if !self.build.is_empty() {
            write!(f, "+{}", self.build)?;
        }
        Ok(())
    }
}

fn compare_identifiers(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

fn process(req: &Request) -> ToolResult {
    if req.input.trim().eq_ignore_ascii_case("help") {
        return Ok(ToolOutput::text(HELP));
    }
    let versions = req
        .input
        .split(['\n', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            Version::parse(s).ok_or_else(|| {
                ToolError::InvalidInput(format!(
                    "Invalid version: {s}\n   Must follow format: MAJOR.MINOR.PATCH[-prerelease][+build]"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let action = req.options.str("action");
    let include_pre = req.options.bool("includePrerelease");
    let text = match action {
        "Compare" => compare(versions, include_pre)?,
        "Parse & Info" => versions.iter().map(info).collect::<Vec<_>>().join("\n\n"),
        _ => {
            let label = action.trim_start_matches("Bump ").to_uppercase();
            let lines: Vec<String> = versions
                .iter()
                .map(|v| format!("{v} → {}", v.bumped(action)))
                .collect();
            format!(" VERSION BUMP ({label}) \n{}", lines.join("\n"))
        }
    };
    Ok(ToolOutput::text(text))
}

fn info(v: &Version) -> String {
    let pre = if v.pre.is_empty() {
        "None".to_string()
    } else {
        v.pre.join(".")
    };
    let build = if v.build.is_empty() { "None" } else { &v.build };
    let stability = if v.pre.is_empty() {
        "Stable"
    } else {
        "Pre-release"
    };
    format!(
        " VERSION ANALYSIS \nVersion:     {v}\nMajor:       {}\nMinor:       {}\nPatch:       {}\nPre-release: {pre}\nBuild:       {build}\nStability:   {stability}",
        v.major, v.minor, v.patch
    )
}

fn compare(mut versions: Vec<Version>, include_pre: bool) -> Result<String, ToolError> {
    if versions.len() < 2 {
        return Err(ToolError::InvalidInput(
            "Enter at least 2 versions to compare (separated by newlines or commas)".into(),
        ));
    }
    let difference = |a: &Version, b: &Version| {
        format!(
            "  Major versions: {}\n  Minor versions: {}\n  Patch versions: {}",
            a.major.abs_diff(b.major),
            a.minor.abs_diff(b.minor),
            a.patch.abs_diff(b.patch)
        )
    };

    if let [a, b] = &versions[..] {
        let relation = match a.cmp(b, include_pre) {
            Ordering::Greater => format!("{a} is newer than {b}"),
            Ordering::Less => format!("{a} is older than {b}"),
            Ordering::Equal => format!("{a} equals {b}"),
        };
        return Ok(format!(
            " VERSION COMPARISON \n{relation}\nDifference:\n{}",
            difference(a, b)
        ));
    }

    versions.sort_by(|a, b| a.cmp(b, include_pre));
    let sorted: Vec<String> = versions
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{}. {v}", i + 1))
        .collect();
    let comparisons: Vec<String> = versions
        .windows(2)
        .map(|w| {
            let op = if w[0].cmp(&w[1], include_pre) == Ordering::Equal {
                "="
            } else {
                "<"
            };
            format!("{} {op} {}", w[0], w[1])
        })
        .collect();
    let (oldest, latest) = (&versions[0], &versions[versions.len() - 1]);
    Ok(format!(
        " SORTED (ASCENDING) \n{}\n\n COMPARISONS \n{}\n\n RANGE \nOldest: {oldest}\nLatest: {latest}\nDifference:\n{}",
        sorted.join("\n"),
        comparisons.join("\n"),
        difference(oldest, latest)
    ))
}
