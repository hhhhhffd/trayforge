//! These tests pin the current behavior of the web-dev tools.

use serde_json::{json, Map, Value};
use trayforge_core::{run, ToolError, ToolOutput};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn text(id: &str, input: &str, options: Value, reverse: bool) -> Result<String, ToolError> {
    run(id, input, &opts(options), reverse).map(|o| o.as_text().to_string())
}

fn go(id: &str, input: &str, options: Value) -> String {
    text(id, input, options, false).unwrap()
}

fn back(id: &str, input: &str, options: Value) -> String {
    text(id, input, options, true).unwrap()
}

#[test]
fn registry_order() {
    let ids: Vec<_> = trayforge_core::registry()
        .iter()
        .filter(|t| t.info.category == trayforge_core::Category::WebDev)
        .map(|t| t.info.id)
        .collect();
    assert_eq!(
        ids,
        [
            "color-converter",
            "css-minifier",
            "js-minifier",
            "html-minifier",
            "markdown-preview",
            "lorem-ipsum",
            "data-uri",
            "svg-path-analyzer",
            "placeholder-image"
        ]
    );
}

// ---- color converter

#[test]
fn color_hex_default() {
    assert_eq!(
        go("color-converter", "#FF5733", json!({})),
        "HEX: #FF5733\nRGB: rgb(255, 87, 51)\nHSL: hsl(11, 100%, 60%)\nHSV: hsv(11, 80%, 100%)\n\n\
         background-color: #FF5733;\nbackground-color: rgb(255, 87, 51);\nbackground-color: hsl(11, 100%, 60%);"
    );
}

#[test]
fn color_options() {
    let out = go(
        "color-converter",
        "rgb(255, 87, 51)",
        json!({"uppercaseHex": false, "showCss": false}),
    );
    assert_eq!(
        out,
        "HEX: #ff5733\nRGB: rgb(255, 87, 51)\nHSL: hsl(11, 100%, 60%)\nHSV: hsv(11, 80%, 100%)"
    );
    let out = go(
        "color-converter",
        "#f003",
        json!({"includeAlpha": true, "showCss": false}),
    );
    assert_eq!(
        out,
        "HEX: #FF000033\nRGB: rgba(255, 0, 0, 0.2)\nHSL: hsla(0, 100%, 50%, 0.2)\nHSV: hsv(0, 100%, 100%)"
    );
}

#[test]
fn color_hsl_input_and_names() {
    let out = go(
        "color-converter",
        "hsl(9, 100%, 60%)",
        json!({"showCss": false}),
    );
    assert!(out.starts_with("HEX: #FF5233"), "{out}");
    let out = go("color-converter", "white", json!({"showCss": false}));
    assert!(out.starts_with("HEX: #FFFFFF"));
    assert!(go("color-converter", "00ff00", json!({})).starts_with("HEX: #00FF00"));
}

#[test]
fn color_invalid() {
    for bad in ["nope", "#12", "rgb(300,0,0)", "rgb(1,2)", "hsl(1,2,200)"] {
        assert_eq!(
            text("color-converter", bad, json!({}), false)
                .unwrap_err()
                .to_string(),
            "Invalid input: Could not parse color. Try formats like #FF5733, rgb(255,87,51), or hsl(9,100%,60%)",
            "{bad}"
        );
    }
}

// ---- css

#[test]
fn css_minify() {
    let css = "/* c */\nbody {\n  color: #ffffff;\n  margin: 0 auto;\n}\n\na > b , .x:hover {\n  background: url( \"a b.png\" ) ;\n}\n";
    assert_eq!(
        go("css-minifier", css, json!({})),
        "body{color:#fff;margin:0 auto}a>b,.x:hover{background:url(\"a b.png\")}"
    );
}

#[test]
fn css_minify_options() {
    let css = "/* c */ a { color: #aabbcc; }";
    assert_eq!(
        go(
            "css-minifier",
            css,
            json!({"removeComments": false, "compressColors": false})
        ),
        "/* c */a{color:#aabbcc}"
    );
    // Hex-looking id selectors and calc() spacing are left alone.
    assert_eq!(
        go(
            "css-minifier",
            "#aabbcc { width: calc(1px + 2px); }",
            json!({})
        ),
        "#aabbcc{width:calc(1px + 2px)}"
    );
}

#[test]
fn css_beautify() {
    assert_eq!(
        back(
            "css-minifier",
            "a{color:red;margin:0}@media (min-width:1px){b{top:0}}",
            json!({"indent": 4})
        ),
        "a {\n    color: red;\n    margin: 0;\n}\n\n@media (min-width:1px) {\n    b {\n        top: 0;\n    }\n}"
    );
}

// ---- js

#[test]
fn js_minify() {
    let js = "// hi\nfunction add(a, b) {\n  /* sum */\n  return a + b; // done\n}\nconst s = \"a  // b\";\nlet r = x / 2 / y, q = /a b\\/ /g;\n";
    assert_eq!(
        go("js-minifier", js, json!({})),
        "function add(a,b){return a+b;}\nconst s=\"a  // b\";let r=x/2/y,q=/a b\\/ /g;"
    );
}

#[test]
fn js_minify_keeps_semantics() {
    assert_eq!(go("js-minifier", "a + +b - -c", json!({})), "a+ +b- -c");
    assert_eq!(go("js-minifier", "x = 1\ny = 2", json!({})), "x=1\ny=2");
    assert_eq!(
        go("js-minifier", "const t = `a ${ b }  c`;", json!({})),
        "const t=`a ${ b }  c`;"
    );
    assert_eq!(
        go("js-minifier", "return /re/.test(a)", json!({})),
        "return/re/.test(a)"
    );
    assert_eq!(
        go("js-minifier", "// a\nx", json!({"removeComments": false})),
        "// a\nx"
    );
}

#[test]
fn js_remove_console() {
    let js = "a();\nconsole.log('x', f(1));\nconsole.error(\"y\")\nb();";
    assert_eq!(
        go("js-minifier", js, json!({"removeConsole": true})),
        "a();b();"
    );
    assert!(go("js-minifier", js, json!({})).contains("console.log"));
}

#[test]
fn js_beautify() {
    assert_eq!(
        back(
            "js-minifier",
            "function f(a,b){if(a){return b;}else{return 0;}}for(i=0;i<3;i++){}",
            json!({})
        ),
        "function f(a, b) {\n  if(a) {\n    return b;\n  } else {\n    return 0;\n  }\n}\nfor(i=0;i<3;i++) {}"
    );
}

// ---- html

#[test]
fn html_minify() {
    let html = "<!DOCTYPE html>\n<html>\n  <!-- note -->\n  <body class=\"a  b\"  id=x>\n    <p>Hello   <b>big</b> world</p>\n    <pre>  keep\n  this </pre>\n    <script>\n  var a  =  1;\n</script>\n  </body>\n</html>\n";
    assert_eq!(
        go("html-minifier", html, json!({})),
        "<!DOCTYPE html><html><body class=\"a  b\" id=x><p>Hello <b>big</b> world</p><pre>  keep\n  this </pre><script>\n  var a  =  1;\n</script></body></html>"
    );
}

#[test]
fn html_minify_options() {
    let html = "<p>  a  <!-- c -->  b </p>";
    assert_eq!(
        go("html-minifier", html, json!({"removeComments": false})),
        "<p> a <!-- c --> b </p>"
    );
    assert_eq!(
        go("html-minifier", html, json!({"collapseWhitespace": false})),
        "<p>  a    b </p>"
    );
}

#[test]
fn html_beautify() {
    assert_eq!(
        back(
            "html-minifier",
            "<div><p>Hi</p><br><pre> x </pre></div>",
            json!({"indent": 2})
        ),
        "<div>\n  <p>\n    Hi\n  </p>\n  <br>\n  <pre> x </pre>\n</div>"
    );
}

// ---- markdown

#[test]
fn markdown_basic() {
    assert_eq!(
        go("markdown-preview", "# Hi\n\n*a* **b**", json!({})),
        "<h1>Hi</h1>\n<p><em>a</em> <strong>b</strong></p>\n"
    );
}

#[test]
fn markdown_smart_typography() {
    assert_eq!(
        go(
            "markdown-preview",
            "\"a\" --- b -- c",
            json!({"smartTypography": true})
        ),
        "<p>\u{201c}a\u{201d} \u{2014} b \u{2013} c</p>\n"
    );
    assert!(go("markdown-preview", "\"a\"", json!({})).contains("<p>\"a\"</p>"));
}

#[test]
fn markdown_raw_html() {
    let md = "<div>x</div>\n\ntext <b>y</b> [l](javascript:alert(1))";
    let allowed = go("markdown-preview", md, json!({}));
    assert!(allowed.contains("<div>x</div>") && allowed.contains("<b>y</b>"));
    assert!(allowed.contains("href=\"javascript:alert(1)\""));
    let safe = go("markdown-preview", md, json!({"allowRawHtml": false}));
    assert!(!safe.contains("<div>") && !safe.contains("<b>") && !safe.contains("javascript"));
    assert_eq!(safe.matches("raw HTML omitted").count(), 3);
}

#[test]
fn markdown_wrap_document() {
    let out = go("markdown-preview", "hi", json!({"wrapDocument": true}));
    assert!(out.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"));
    assert!(out.ends_with("<p>hi</p>\n</body>\n</html>\n"));
}

// ---- lorem ipsum

#[test]
fn lorem_counts_and_start() {
    let words = go("lorem-ipsum", "", json!({"unit": "Words", "count": 12}));
    assert_eq!(words.split(' ').count(), 12);
    assert!(words.starts_with("lorem ipsum dolor sit amet consectetur adipiscing elit "));
    let words = go(
        "lorem-ipsum",
        "",
        json!({"unit": "Words", "count": 3, "startWithLorem": false, "variant": "Hipster"}),
    );
    assert_eq!(words.split(' ').count(), 3);
    assert!(!words.starts_with("lorem"));

    let sentences = go("lorem-ipsum", "", json!({"unit": "Sentences", "count": 4}));
    assert!(sentences.starts_with("Lorem ipsum dolor sit amet"));
    assert_eq!(sentences.matches('.').count(), 4);

    let paras = go("lorem-ipsum", "", json!({}));
    assert_eq!(paras.split("\n\n").count(), 3);
    assert!(paras.starts_with("Lorem ipsum"));
}

#[test]
fn lorem_wrap_and_no_input() {
    let out = go(
        "lorem-ipsum",
        "ignored",
        json!({"count": 2, "wrapParagraphs": true}),
    );
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines
        .iter()
        .all(|l| l.starts_with("<p>") && l.ends_with("</p>")));
    assert!(text("lorem-ipsum", "", json!({}), false).is_ok());
}

// ---- data uri

#[test]
fn data_uri_encode() {
    assert_eq!(
        go("data-uri", "hello", json!({})),
        "data:text/plain;base64,aGVsbG8="
    );
    assert_eq!(
        go(
            "data-uri",
            "a b&",
            json!({"base64": false, "mimeType": "text/html"})
        ),
        "data:text/html,a%20b%26"
    );
    assert_eq!(
        go(
            "data-uri",
            "x",
            json!({"cssUrl": true, "mimeType": "text/css"})
        ),
        "url('data:text/css;base64,eA==')"
    );
}

#[test]
fn data_uri_auto_detect() {
    let mime = |s: &str| {
        go("data-uri", s, json!({"base64": false}))
            .split(',')
            .next()
            .unwrap()
            .to_string()
    };
    assert_eq!(mime("<svg xmlns=\"x\"/>"), "data:image/svg+xml");
    assert_eq!(mime("<!DOCTYPE html><p>"), "data:text/html");
    assert_eq!(mime("{\"a\": 1}"), "data:application/json");
    assert_eq!(mime("const a = 1;"), "data:text/javascript");
    assert_eq!(mime("a { color: red; }"), "data:text/css");
    assert_eq!(mime("plain"), "data:text/plain");
}

#[test]
fn data_uri_decode() {
    assert_eq!(
        back("data-uri", "data:text/plain;base64,aGVsbG8=", json!({})),
        "hello"
    );
    assert_eq!(
        back("data-uri", "data:text/plain;base64,aGVsbG8", json!({})),
        "hello"
    );
    assert_eq!(
        back("data-uri", "data:text/html,a%20b%26", json!({})),
        "a b&"
    );
    assert_eq!(
        back("data-uri", "url('data:text/plain,hi')", json!({})),
        "hi"
    );
}

#[test]
fn data_uri_decode_errors() {
    let err = |s: &str| {
        text("data-uri", s, json!({}), true)
            .unwrap_err()
            .to_string()
    };
    assert_eq!(
        err("hello"),
        "Invalid input: Invalid Data URI format. Must start with 'data:'"
    );
    assert_eq!(
        err("data:text/plain"),
        "Invalid input: Invalid Data URI format. Missing data separator."
    );
    assert_eq!(
        err("data:text/plain,%zz"),
        "Invalid input: Could not decode percent encoding"
    );
    assert_eq!(
        err("data:text/plain;base64,@@@"),
        "Invalid input: Invalid Base64 encoding"
    );
    assert_eq!(
        err("data:application/octet-stream;base64,/w=="),
        "Processing failed: Could not decode data as UTF-8 text"
    );
}

// ---- svg path

#[test]
fn svg_path_analysis() {
    let out = go("svg-path-analyzer", "M10 10 l20 0 v20 z", json!({}));
    assert_eq!(
        out,
        "Total commands: 4\nSubpaths: 1\nClosed subpaths: 1\nAbsolute commands: 1\nRelative commands: 3\n\
         Bounding box: (10, 10) to (30, 30)\nSize: 20 × 20\n\n\
         Command breakdown:\n1. M 10.0, 10.0 - Move to (absolute)\n2. l 20.0, 0.0 - Line to (relative)\n\
         3. v 20.0 - Vertical line to (relative)\n4. z - Close path (relative)"
    );
}

#[test]
fn svg_path_options() {
    let out = go(
        "svg-path-analyzer",
        "d=\"m1,1 2,2 h3 c1 1 2 2 3 3\"",
        json!({"breakdown": false, "simplify": true, "completeSvg": true}),
    );
    assert!(!out.contains("Command breakdown"));
    assert!(
        out.contains("Simplified path:\nM1 1 L3 3 H6 C7 4 8 5 9 6"),
        "{out}"
    );
    assert!(out.contains(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"1 1 8 5\"><path d=\"M1 1 L3 3 H6 C7 4 8 5 9 6\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1\"/></svg>"
    ));
}

#[test]
fn svg_path_numbers_and_arcs() {
    let out = go(
        "svg-path-analyzer",
        "M0,0a5 5 0 1110 0 M-1.5-2e1.5.5",
        json!({"simplify": true}),
    );
    assert!(
        out.contains("Simplified path:\nM0 0 A5 5 0 1 1 10 0 M-1.5 -20 L0.5 0.5"),
        "{out}"
    );
}

#[test]
fn svg_path_invalid() {
    for bad in ["hello", "M", "L 1"] {
        assert_eq!(
            text("svg-path-analyzer", bad, json!({}), false)
                .unwrap_err()
                .to_string(),
            "Invalid input: No valid path commands found",
            "{bad}"
        );
    }
}

#[test]
fn odd_input_never_panics() {
    let inputs = [
        "\"",
        "'",
        "`${",
        "/*",
        "/",
        "a/",
        "<",
        "<a",
        "<a href=\"",
        "<!--",
        "{",
        "}",
        "(",
        ")",
        "#",
        "#abcdef",
        "url(",
        "data:",
        "d=\"",
        "M",
        "a1",
        "é👋",
        "%",
        "\\",
        "${`",
        "<script>",
    ];
    for id in [
        "color-converter",
        "css-minifier",
        "js-minifier",
        "html-minifier",
        "markdown-preview",
        "data-uri",
        "svg-path-analyzer",
    ] {
        for input in inputs {
            for reverse in [false, true] {
                let _ = run(id, input, &Map::new(), reverse);
            }
        }
    }
}

// ---- placeholder image

fn placeholder(input: &str, options: Value) -> ToolOutput {
    run("placeholder-image", input, &opts(options), false).unwrap()
}

#[test]
fn placeholder_svg_default() {
    let out = placeholder("", json!({}));
    let items = out.items();
    assert_eq!(items.len(), 2);
    assert!(items[0].value.starts_with("data:image/svg+xml,%3Csvg"));
    assert!(items[1].is_image && items[1].value == items[0].value);
    assert!(
        items[0].value.contains("400%C3%97300"),
        "{}",
        items[0].value
    );
    assert!(items[0].value.contains("%23CCCCCC") && items[0].value.contains("%23666666"));
}

#[test]
fn placeholder_formats() {
    let html = placeholder(
        "",
        json!({"format": "HTML <img> tag", "width": 10, "height": 20}),
    );
    assert!(html.items()[0]
        .value
        .starts_with("<img src=\"data:image/svg+xml,"));
    assert!(html.items()[0]
        .value
        .ends_with("\" alt=\"Placeholder\" width=\"10\" height=\"20\">"));
    let css = placeholder("", json!({"format": "CSS background"}));
    assert!(css.items()[0]
        .value
        .starts_with("background-image: url('data:image/svg+xml,"));
    assert!(css.items()[0].value.ends_with("');"));
}

#[test]
fn placeholder_png() {
    use base64::Engine;
    let out = placeholder(
        "Hi",
        json!({"format": "PNG (Data URI)", "width": 64, "height": 32, "textStyle": "Custom text", "border": true,
               "background": "white", "textColor": "rgb(0,0,0)"}),
    );
    let uri = &out.items()[0].value;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(uri.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 64);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 32);
    assert!(out.items()[1].is_image);
}

#[test]
fn placeholder_text_styles_and_escaping() {
    let none = placeholder("", json!({"textStyle": "None", "format": "CSS background"}));
    assert!(none.items()[0].value.contains("%3Cline"));
    assert!(!none.items()[0].value.contains("%3Ctext"));
    let custom = placeholder("<&>", json!({"textStyle": "Custom text", "border": true}));
    let v = &custom.items()[0].value;
    assert!(v.contains("%26lt%3B%26amp%3B%26gt%3B"), "{v}");
    assert!(v.contains("stroke-width%3D%222%22"));
}

#[test]
fn placeholder_errors() {
    let err = text(
        "placeholder-image",
        "",
        json!({"background": "\"><x"}),
        false,
    )
    .unwrap_err();
    assert!(matches!(err, ToolError::InvalidInput(_)));
    // Extreme sizes must not panic.
    for (w, h) in [(1, 1), (1, 2000), (2000, 1)] {
        for fmt in ["SVG (Data URI)", "PNG (Data URI)"] {
            placeholder(
                "x",
                json!({"width": w, "height": h, "format": fmt, "border": true}),
            );
            placeholder(
                "",
                json!({"width": w, "height": h, "format": fmt, "textStyle": "None"}),
            );
        }
    }
}
