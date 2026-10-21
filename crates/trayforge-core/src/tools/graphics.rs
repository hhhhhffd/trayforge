//! All Graphics tools have dedicated panels; the core only describes them.

use crate::{Category, Tool, ToolInfo};

pub fn tools() -> Vec<Tool> {
    vec![
        panel(
            "tailwind-colors",
            "Tailwind Colors",
            "Browse the complete Tailwind CSS v3 color palette",
            vec!["tailwind", "css", "color", "palette"],
            "palette",
        ),
        panel(
            "color-palettes",
            "Color Palettes",
            "Browse and copy curated color palettes",
            vec!["color", "palette", "swatch", "scheme"],
            "swatch-book",
        ),
        panel(
            "svg-editor",
            "SVG Editor",
            "Edit SVG markup with a live preview",
            vec!["svg", "vector", "editor", "preview"],
            "pen-tool",
        ),
        panel(
            "screen-color-picker",
            "Screen Color Picker",
            "Pick any color from anywhere on your screen",
            vec!["color", "picker", "eyedropper", "screen", "pixel"],
            "pipette",
        ),
    ]
}

fn panel(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    keywords: Vec<&'static str>,
    icon: &'static str,
) -> Tool {
    Tool {
        info: ToolInfo {
            id,
            name,
            category: Category::Graphics,
            description,
            keywords,
            icon,
            input_placeholder: "",
            output_placeholder: "",
            primary_action: "",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: true,
            options: Vec::new(),
        },
        process: None,
    }
}
