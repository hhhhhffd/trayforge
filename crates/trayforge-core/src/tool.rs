//! Shared tool contract: `Tool`, `ToolOption`, `ToolOutput` and `ToolError`.

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Category {
    Encoding,
    Hashing,
    Formats,
    #[serde(rename = "Web Dev")]
    WebDev,
    Text,
    #[serde(rename = "Dev Utils")]
    DevUtils,
    Graphics,
}

impl Category {
    /// Sidebar order.
    pub const ALL: [Category; 7] = [
        Category::Encoding,
        Category::Hashing,
        Category::Formats,
        Category::WebDev,
        Category::Text,
        Category::DevUtils,
        Category::Graphics,
    ];
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OptionKind {
    Toggle {
        default: bool,
    },
    /// Dropdown. The selected value travels as the option string itself.
    Picker {
        choices: Vec<&'static str>,
        default: usize,
    },
    /// Radio buttons / segmented control. Same value semantics as `Picker`.
    Segmented {
        choices: Vec<&'static str>,
        default: usize,
    },
    Slider {
        min: f64,
        max: f64,
        default: f64,
        step: f64,
    },
    #[serde(rename_all = "camelCase")]
    TextField {
        placeholder: &'static str,
        default: &'static str,
    },
    Stepper {
        min: i64,
        max: i64,
        default: i64,
    },
    /// Color as `#RRGGBB`.
    Color {
        default: &'static str,
    },
}

impl OptionKind {
    pub fn default_value(&self) -> Value {
        match self {
            OptionKind::Toggle { default } => Value::Bool(*default),
            OptionKind::Picker { choices, default }
            | OptionKind::Segmented { choices, default } => {
                Value::String(choices[*default].to_string())
            }
            OptionKind::Slider { default, .. } => Value::from(*default),
            OptionKind::TextField { default, .. } => Value::String(default.to_string()),
            OptionKind::Stepper { default, .. } => Value::from(*default),
            OptionKind::Color { default } => Value::String(default.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolOption {
    pub id: &'static str,
    pub label: &'static str,
    pub help: Option<&'static str>,
    pub kind: OptionKind,
}

/// Static description of a tool, sent to the UI as-is.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    /// Stable kebab-case id used by the UI, settings and IPC.
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    pub keywords: Vec<&'static str>,
    /// lucide icon name in kebab-case (https://lucide.dev/icons).
    pub icon: &'static str,
    pub input_placeholder: &'static str,
    pub output_placeholder: &'static str,
    pub primary_action: &'static str,
    /// Action name of the reversed direction, e.g. "Decode". `Some` means the
    /// tool is bidirectional and the UI shows a mode switch and Swap.
    pub secondary_action: Option<&'static str>,
    pub supports_file_input: bool,
    pub allows_empty_input: bool,
    /// Rendered by a dedicated UI panel instead of the standard editor.
    pub custom_ui: bool,
    pub options: Vec<ToolOption>,
}

/// Option values resolved against a tool's declared defaults.
#[derive(Debug, Clone, Default)]
pub struct Options(BTreeMap<String, Value>);

impl Options {
    /// Starts from every declared default and overlays the values sent by the UI.
    /// Unknown ids and values of the wrong type are rejected so UI bugs surface.
    pub fn resolve(
        info: &ToolInfo,
        raw: &serde_json::Map<String, Value>,
    ) -> Result<Self, ToolError> {
        let mut values: BTreeMap<String, Value> = info
            .options
            .iter()
            .map(|o| (o.id.to_string(), o.kind.default_value()))
            .collect();
        for (id, value) in raw {
            let option = info
                .options
                .iter()
                .find(|o| o.id == id)
                .ok_or_else(|| ToolError::InvalidOptions(format!("Unknown option '{id}'")))?;
            let valid = match &option.kind {
                OptionKind::Toggle { .. } => value.is_boolean(),
                OptionKind::Picker { choices, .. } | OptionKind::Segmented { choices, .. } => {
                    value.as_str().is_some_and(|v| choices.contains(&v))
                }
                OptionKind::Slider { min, max, .. } => value
                    .as_f64()
                    .is_some_and(|v| v.is_finite() && (*min..=*max).contains(&v)),
                OptionKind::Stepper { min, max, .. } => {
                    value.as_i64().is_some_and(|v| (*min..=*max).contains(&v))
                }
                OptionKind::TextField { .. } | OptionKind::Color { .. } => value.is_string(),
            };
            if !valid {
                let range = match &option.kind {
                    OptionKind::Slider { min, max, .. } => format!(" (expected {min}..={max})"),
                    OptionKind::Stepper { min, max, .. } => format!(" (expected {min}..={max})"),
                    _ => String::new(),
                };
                return Err(ToolError::InvalidOptions(format!(
                    "Invalid value for option '{id}': {value}{range}"
                )));
            }
            values.insert(id.clone(), value.clone());
        }
        Ok(Options(values))
    }

    fn get(&self, id: &str) -> &Value {
        self.0
            .get(id)
            .unwrap_or_else(|| panic!("option '{id}' is not declared by this tool"))
    }

    pub fn bool(&self, id: &str) -> bool {
        self.get(id).as_bool().expect("toggle option")
    }

    /// Selected choice of a picker/segmented option, a text field or a color.
    pub fn str(&self, id: &str) -> &str {
        self.get(id).as_str().expect("string option")
    }

    pub fn f64(&self, id: &str) -> f64 {
        self.get(id).as_f64().expect("numeric option")
    }

    pub fn i64(&self, id: &str) -> i64 {
        self.get(id).as_i64().expect("integer option")
    }
}

#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub input: &'a str,
    pub options: Options,
    /// True when the secondary action (e.g. Decode) was requested.
    pub reverse: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputItem {
    pub label: String,
    pub value: String,
    pub is_warning: bool,
    /// `value` is an image data URI (e.g. `data:image/png;base64,...`).
    pub is_image: bool,
}

impl OutputItem {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        OutputItem {
            label: label.into(),
            value: value.into(),
            is_warning: false,
            is_image: false,
        }
    }

    pub fn warning(label: impl Into<String>, value: impl Into<String>) -> Self {
        OutputItem {
            is_warning: true,
            ..OutputItem::new(label, value)
        }
    }

    pub fn image(label: impl Into<String>, data_uri: impl Into<String>) -> Self {
        OutputItem {
            is_image: true,
            ..OutputItem::new(label, data_uri)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolOutput {
    Text {
        value: String,
        metadata: BTreeMap<String, String>,
    },
    Multiple {
        items: Vec<OutputItem>,
    },
}

impl ToolOutput {
    pub fn text(value: impl Into<String>) -> Self {
        ToolOutput::Text {
            value: value.into(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn multiple(items: Vec<OutputItem>) -> Self {
        ToolOutput::Multiple { items }
    }

    /// Text value of a `Text` output; panics otherwise. Intended for tests.
    pub fn as_text(&self) -> &str {
        match self {
            ToolOutput::Text { value, .. } => value,
            ToolOutput::Multiple { .. } => panic!("expected text output, got {self:?}"),
        }
    }

    /// Items of a `Multiple` output; panics otherwise. Intended for tests.
    pub fn items(&self) -> &[OutputItem] {
        match self {
            ToolOutput::Multiple { items } => items,
            ToolOutput::Text { .. } => panic!("expected multiple outputs, got {self:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum ToolError {
    InvalidInput(String),
    ProcessingFailed(String),
    InvalidOptions(String),
    UnsupportedOperation(String),
    EmptyInput,
    Timeout,
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::InvalidInput(m) => write!(f, "Invalid input: {m}"),
            ToolError::ProcessingFailed(m) => write!(f, "Processing failed: {m}"),
            ToolError::InvalidOptions(m) => write!(f, "Invalid options: {m}"),
            ToolError::UnsupportedOperation(m) => write!(f, "Unsupported operation: {m}"),
            ToolError::EmptyInput => write!(f, "Input is empty"),
            ToolError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

impl std::error::Error for ToolError {}

pub type ToolResult = Result<ToolOutput, ToolError>;

pub struct Tool {
    pub info: ToolInfo,
    /// `None` for `custom_ui` tools, which are driven by dedicated commands.
    pub process: Option<fn(&Request) -> ToolResult>,
}
