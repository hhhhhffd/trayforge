use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

const LIST_LIMIT: usize = 15;

/// (extension without dot, MIME type, description)
const MIME_TYPES: &[(&str, &str, &str)] = &[
    ("json", "application/json", "JSON data"),
    ("xml", "application/xml", "XML document"),
    ("pdf", "application/pdf", "PDF document"),
    ("zip", "application/zip", "ZIP archive"),
    ("gz", "application/gzip", "Gzip archive"),
    ("tar", "application/x-tar", "Tar archive"),
    ("rar", "application/vnd.rar", "RAR archive"),
    ("7z", "application/x-7z-compressed", "7-Zip archive"),
    ("js", "application/javascript", "JavaScript"),
    ("mjs", "application/javascript", "JavaScript module"),
    ("wasm", "application/wasm", "WebAssembly"),
    ("doc", "application/msword", "Microsoft Word"),
    (
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "Word document",
    ),
    ("xls", "application/vnd.ms-excel", "Microsoft Excel"),
    (
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "Excel spreadsheet",
    ),
    (
        "ppt",
        "application/vnd.ms-powerpoint",
        "Microsoft PowerPoint",
    ),
    (
        "pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "PowerPoint presentation",
    ),
    ("sql", "application/sql", "SQL query"),
    ("rtf", "application/rtf", "Rich Text Format"),
    ("sh", "application/x-sh", "Shell script"),
    ("jar", "application/java-archive", "Java archive"),
    ("swf", "application/x-shockwave-flash", "Shockwave Flash"),
    ("woff", "font/woff", "Web Open Font Format"),
    ("woff2", "font/woff2", "WOFF 2.0 font"),
    (
        "eot",
        "application/vnd.ms-fontobject",
        "Embedded OpenType font",
    ),
    ("ics", "text/calendar", "iCalendar"),
    ("atom", "application/atom+xml", "Atom feed"),
    ("rss", "application/rss+xml", "RSS feed"),
    ("mp3", "audio/mpeg", "MP3 audio"),
    ("wav", "audio/wav", "WAV audio"),
    ("ogg,oga", "audio/ogg", "Ogg audio"),
    ("m4a", "audio/mp4", "MP4 audio"),
    ("aac", "audio/aac", "AAC audio"),
    ("flac", "audio/flac", "FLAC audio"),
    ("wma", "audio/x-ms-wma", "Windows Media Audio"),
    ("mid,midi", "audio/midi", "MIDI audio"),
    ("weba", "audio/webm", "WebM audio"),
    ("ttf", "font/ttf", "TrueType font"),
    ("otf", "font/otf", "OpenType font"),
    ("jpg,jpeg", "image/jpeg", "JPEG image"),
    ("png", "image/png", "PNG image"),
    ("gif", "image/gif", "GIF image"),
    ("bmp", "image/bmp", "BMP image"),
    ("svg", "image/svg+xml", "SVG image"),
    ("ico", "image/x-icon", "Icon image"),
    ("tif,tiff", "image/tiff", "TIFF image"),
    ("webp", "image/webp", "WebP image"),
    ("avif", "image/avif", "AVIF image"),
    ("heic", "image/heic", "HEIC image"),
    ("heif", "image/heif", "HEIF image"),
    ("psd", "image/vnd.adobe.photoshop", "Photoshop document"),
    ("ai", "application/illustrator", "Adobe Illustrator"),
    ("txt", "text/plain", "Plain text"),
    ("html,htm", "text/html", "HTML document"),
    ("css", "text/css", "CSS stylesheet"),
    ("csv", "text/csv", "CSV file"),
    ("md,markdown", "text/markdown", "Markdown document"),
    ("yaml,yml", "text/yaml", "YAML document"),
    ("toml", "text/toml", "TOML document"),
    ("ini", "text/plain", "INI configuration"),
    ("log", "text/plain", "Log file"),
    ("conf", "text/plain", "Configuration file"),
    ("env", "text/plain", "Environment file"),
    ("swift", "text/x-swift", "Swift source"),
    ("py", "text/x-python", "Python source"),
    ("rb", "text/x-ruby", "Ruby source"),
    ("java", "text/x-java-source", "Java source"),
    ("c", "text/x-c", "C source"),
    ("cpp,cc", "text/x-c++", "C++ source"),
    ("h", "text/x-c", "C header"),
    ("hpp", "text/x-c++", "C++ header"),
    ("go", "text/x-go", "Go source"),
    ("rs", "text/x-rust", "Rust source"),
    ("ts", "text/typescript", "TypeScript source"),
    ("tsx", "text/typescript-jsx", "TypeScript JSX"),
    ("jsx", "text/jsx", "JSX source"),
    ("php", "text/x-php", "PHP source"),
    ("kt", "text/x-kotlin", "Kotlin source"),
    ("scala", "text/x-scala", "Scala source"),
    ("lua", "text/x-lua", "Lua source"),
    ("r", "text/x-r", "R source"),
    ("m", "text/x-objectivec", "Objective-C source"),
    ("mm", "text/x-objectivec++", "Objective-C++ source"),
    ("mp4", "video/mp4", "MP4 video"),
    ("mov", "video/quicktime", "QuickTime video"),
    ("avi", "video/x-msvideo", "AVI video"),
    ("wmv", "video/x-ms-wmv", "Windows Media Video"),
    ("mkv", "video/x-matroska", "Matroska video"),
    ("webm", "video/webm", "WebM video"),
    ("flv", "video/x-flv", "Flash video"),
    ("mpeg,mpg", "video/mpeg", "MPEG video"),
    ("3gp", "video/3gpp", "3GPP video"),
];

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "mime-type",
            name: "MIME Type",
            category: Category::DevUtils,
            description: "Look up MIME types and file extensions",
            keywords: vec!["mime", "type", "extension", "content", "media", "file"],
            icon: "file-type",
            input_placeholder:
                "Enter file extension (e.g., .json) or MIME type (e.g., application/json)...",
            output_placeholder: "MIME type information will appear here",
            primary_action: "Lookup",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![ToolOption {
                id: "category",
                label: "Category filter",
                help: Some("Filter by MIME type category"),
                kind: OptionKind::Picker {
                    choices: vec![
                        "All",
                        "Application",
                        "Audio",
                        "Font",
                        "Image",
                        "Text",
                        "Video",
                    ],
                    default: 0,
                },
            }],
        },
        process: Some(process),
    }
}

fn category(mime: &str) -> &'static str {
    match mime.split('/').next() {
        Some("audio") => "Audio",
        Some("font") => "Font",
        Some("image") => "Image",
        Some("text") => "Text",
        Some("video") => "Video",
        _ => "Application",
    }
}

fn extensions(exts: &str) -> String {
    exts.split(',')
        .map(|e| format!(".{e}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn process(req: &Request) -> ToolResult {
    let query = req.input.trim().to_lowercase();
    let filter = req.options.str("category");
    let in_category = |mime: &str| filter == "All" || category(mime) == filter;
    let by_extension = query.trim_start_matches('.');

    // Exact extension or MIME type first, keyword search otherwise.
    let mut matches: Vec<_> = MIME_TYPES
        .iter()
        .filter(|(exts, mime, _)| {
            in_category(mime) && (exts.split(',').any(|e| e == by_extension) || *mime == query)
        })
        .collect();
    if matches.is_empty() {
        matches = MIME_TYPES
            .iter()
            .filter(|(exts, mime, description)| {
                in_category(mime)
                    && (exts.contains(by_extension)
                        || mime.contains(&query)
                        || description.to_lowercase().contains(&query))
            })
            .collect();
    }
    let Some(&&(_, mime, description)) = matches.first() else {
        return Err(ToolError::InvalidInput(format!(
            "No MIME type found for '{}'",
            req.input.trim()
        )));
    };

    // A single match, or entries sharing one MIME type (e.g. .html/.htm), is one result.
    if matches.iter().all(|m| m.1 == mime) {
        let related: Vec<String> = MIME_TYPES
            .iter()
            .filter(|m| m.1 == mime)
            .map(|m| extensions(m.0))
            .collect();
        return Ok(ToolOutput::multiple(vec![
            OutputItem::new("MIME type", mime),
            OutputItem::new("Description", description),
            OutputItem::new("Category", category(mime)),
            OutputItem::new("Content-Type header", format!("Content-Type: {mime}")),
            OutputItem::new("Related extensions", related.join(", ")),
        ]));
    }
    let total = matches.len();
    let mut lines: Vec<String> = matches
        .iter()
        .take(LIST_LIMIT)
        .map(|(exts, mime, description)| format!("{}  {mime}  ({description})", extensions(exts)))
        .collect();
    if total > LIST_LIMIT {
        lines.push(format!("\nShowing first {LIST_LIMIT} of {total} matches"));
    }
    Ok(ToolOutput::text(lines.join("\n")))
}
