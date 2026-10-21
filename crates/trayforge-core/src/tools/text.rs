mod lines;
mod numbers;
mod regexes;
mod strings;

use crate::{OptionKind, Tool, ToolOption};

pub fn tools() -> Vec<Tool> {
    vec![
        strings::string_case_tool(),
        regexes::regex_tester_tool(),
        regexes::regex_library_tool(),
        lines::string_diff_tool(),
        lines::line_sorter_tool(),
        lines::text_statistics_tool(),
        strings::string_escape_tool(),
        lines::whitespace_tool(),
        numbers::base_converter_tool(),
        numbers::unix_timestamp_tool(),
        numbers::epoch_batch_tool(),
        strings::slug_generator_tool(),
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

fn picker(
    id: &'static str,
    label: &'static str,
    help: &'static str,
    choices: Vec<&'static str>,
) -> ToolOption {
    ToolOption {
        id,
        label,
        help: Some(help),
        kind: OptionKind::Picker {
            choices,
            default: 0,
        },
    }
}

fn text_field(
    id: &'static str,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
    default: &'static str,
) -> ToolOption {
    ToolOption {
        id,
        label,
        help: Some(help),
        kind: OptionKind::TextField {
            placeholder,
            default,
        },
    }
}

fn stepper(
    id: &'static str,
    label: &'static str,
    help: &'static str,
    (min, max, default): (i64, i64, i64),
) -> ToolOption {
    ToolOption {
        id,
        label,
        help: Some(help),
        kind: OptionKind::Stepper { min, max, default },
    }
}
