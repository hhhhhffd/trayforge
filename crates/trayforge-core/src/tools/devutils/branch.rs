use super::time::{format_date, now_unix};
use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

const ALTERNATIVE_TYPES: [&str; 3] = ["feature/", "bugfix/", "hotfix/"];

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "git-branch-name",
            name: "Git Branch Name",
            category: Category::DevUtils,
            description: "Generate standardized git branch names from descriptions",
            keywords: vec!["git", "branch", "name", "feature", "jira", "issue"],
            icon: "git-branch",
            input_placeholder: "Enter description, issue number, or ticket ID...",
            output_placeholder: "Git branch name will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "branchType",
                    label: "Branch type",
                    help: Some("Branch type prefix"),
                    kind: OptionKind::Picker {
                        choices: vec![
                            "feature/",
                            "bugfix/",
                            "hotfix/",
                            "release/",
                            "chore/",
                            "docs/",
                            "refactor/",
                            "test/",
                            "None",
                        ],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "separator",
                    label: "Word separator",
                    help: Some("Character between words"),
                    kind: OptionKind::Picker {
                        choices: vec!["-", "_", "."],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "lowercase",
                    label: "Lowercase",
                    help: Some("Convert to lowercase"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "includeDate",
                    label: "Include date",
                    help: Some("Add date prefix (YYYY-MM-DD)"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "username",
                    label: "Username prefix",
                    help: Some("Optional username prefix"),
                    kind: OptionKind::TextField {
                        placeholder: "e.g., jsmith",
                        default: "",
                    },
                },
                ToolOption {
                    id: "maxLength",
                    label: "Max length",
                    help: Some("Maximum branch name length"),
                    kind: OptionKind::Stepper {
                        min: 10,
                        max: 100,
                        default: 50,
                    },
                },
            ],
        },
        process: Some(process),
    }
}

fn process(req: &Request) -> ToolResult {
    let separator = req.options.str("separator");
    let lowercase = req.options.bool("lowercase");
    let max_length = req.options.i64("maxLength") as usize;
    let username = slugify(req.options.str("username"), "-", lowercase);
    let date = req
        .options
        .bool("includeDate")
        .then(|| format_date(now_unix()));
    let slug = slugify(req.input, separator, lowercase);
    if slug.is_empty() {
        return Err(ToolError::InvalidInput(
            "Could not generate a branch name from the input".into(),
        ));
    }

    let build = |branch_type: &str| {
        let mut name = String::new();
        if branch_type != "None" {
            name.push_str(branch_type);
        }
        if !username.is_empty() {
            name.push_str(&format!("{username}/"));
        }
        if let Some(date) = &date {
            name.push_str(&format!("{date}{separator}"));
        }
        name.push_str(&slug);
        // Cut on a char boundary and never end on a separator.
        let cut: String = name.chars().take(max_length).collect();
        cut.trim_end_matches(['-', '_', '.', '/']).to_string()
    };

    let selected = req.options.str("branchType");
    let name = build(selected);
    let alternatives: Vec<String> = ALTERNATIVE_TYPES
        .iter()
        .filter(|t| **t != selected)
        .map(|t| build(t))
        .collect();
    Ok(ToolOutput::multiple(vec![
        OutputItem::new("Branch name", name.clone()),
        OutputItem::new("Length", format!("{} characters", name.chars().count())),
        OutputItem::new("Create command", format!("git checkout -b {name}")),
        OutputItem::new("Alternatives", alternatives.join("\n")),
    ]))
}

/// Runs of non-alphanumerics become one separator; leading/trailing ones are dropped.
fn slugify(text: &str, separator: &str, lowercase: bool) -> String {
    let text = if lowercase {
        text.to_lowercase()
    } else {
        text.to_string()
    };
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(separator)
}
