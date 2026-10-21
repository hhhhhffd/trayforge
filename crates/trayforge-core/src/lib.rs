//! Platform-independent implementation of all Trayforge tools.

mod tool;
pub mod tools;
pub mod totp;

use std::sync::OnceLock;

pub use tool::*;

/// Every tool in sidebar order: categories in `Category::ALL` order, tools in
/// their registration order inside each category.
pub fn registry() -> &'static [Tool] {
    static REGISTRY: OnceLock<Vec<Tool>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut all = Vec::new();
        all.extend(tools::encoding::tools());
        all.extend(tools::hashing::tools());
        all.extend(tools::formats::tools());
        all.extend(tools::webdev::tools());
        all.extend(tools::text::tools());
        all.extend(tools::devutils::tools());
        all.extend(tools::graphics::tools());
        all
    })
}

pub fn find(id: &str) -> Option<&'static Tool> {
    registry().iter().find(|t| t.info.id == id)
}

/// Single entry point used by the shell: resolves options, rejects empty input
/// where the tool does not allow it and runs the tool.
pub fn run(
    id: &str,
    input: &str,
    raw_options: &serde_json::Map<String, serde_json::Value>,
    reverse: bool,
) -> ToolResult {
    let tool =
        find(id).ok_or_else(|| ToolError::UnsupportedOperation(format!("Unknown tool '{id}'")))?;
    let process = tool.process.ok_or_else(|| {
        ToolError::UnsupportedOperation(format!("'{}' has a dedicated panel", tool.info.name))
    })?;
    if reverse && tool.info.secondary_action.is_none() {
        return Err(ToolError::UnsupportedOperation(format!(
            "'{}' is not bidirectional",
            tool.info.name
        )));
    }
    if !tool.info.allows_empty_input && input.trim().is_empty() {
        return Err(ToolError::EmptyInput);
    }
    let options = Options::resolve(&tool.info, raw_options)?;
    process(&Request {
        input,
        options,
        reverse,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique_and_kebab_case() {
        let mut seen = HashSet::new();
        for tool in registry() {
            let id = tool.info.id;
            assert!(seen.insert(id), "duplicate id {id}");
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "id {id} is not kebab-case"
            );
        }
    }

    #[test]
    fn categories_are_grouped_in_sidebar_order() {
        let order: Vec<usize> = registry()
            .iter()
            .map(|t| {
                Category::ALL
                    .iter()
                    .position(|c| *c == t.info.category)
                    .unwrap()
            })
            .collect();
        assert!(
            order.windows(2).all(|w| w[0] <= w[1]),
            "tools are not grouped by category"
        );
    }

    #[test]
    fn standard_tools_have_a_processor_and_valid_defaults() {
        for tool in registry() {
            assert_eq!(
                tool.process.is_none(),
                tool.info.custom_ui,
                "{}",
                tool.info.id
            );
            for option in &tool.info.options {
                if let OptionKind::Picker { choices, default }
                | OptionKind::Segmented { choices, default } = &option.kind
                {
                    assert!(*default < choices.len(), "{}.{}", tool.info.id, option.id);
                }
            }
        }
    }

    #[test]
    fn out_of_range_numeric_options_are_rejected() {
        for (id, option, value) in [
            ("password-hash", "iterations", -1),
            ("qr-code", "size", 1_000_000),
            ("lorem-ipsum", "count", -1),
            ("cron-parser", "count", 0),
        ] {
            let raw = serde_json::json!({ option: value });
            let result = run(id, "* * * * *", raw.as_object().unwrap(), false);
            assert!(
                matches!(&result, Err(ToolError::InvalidOptions(m)) if m.contains(option)),
                "{id}.{option}: {result:?}"
            );
        }
    }
}
