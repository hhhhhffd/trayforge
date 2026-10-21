//! Reference cases (case id in each test name), plus coverage of the options
//! the cases do not exercise.

use serde_json::{json, Map, Value};
use trayforge_core::{find, registry, run, Category, ToolError};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn forward(id: &str, input: &str, options: Value) -> Result<String, ToolError> {
    run(id, input, &opts(options), false).map(|o| o.as_text().to_string())
}

fn reverse(id: &str, input: &str, options: Value) -> Result<String, ToolError> {
    run(id, input, &opts(options), true).map(|o| o.as_text().to_string())
}

#[test]
fn formats_tools_are_registered_in_order() {
    let ids: Vec<_> = registry()
        .iter()
        .filter(|t| t.info.category == Category::Formats)
        .map(|t| t.info.id)
        .collect();
    assert_eq!(
        ids,
        [
            "json-formatter",
            "json-path",
            "json-yaml",
            "yaml-formatter",
            "xml-formatter",
            "xml-json",
            "csv-to-json",
            "json-to-csv",
            "sql-formatter",
            "graphql-formatter",
            "toml-parser"
        ]
    );
    for id in ["json-yaml", "xml-json"] {
        assert!(find(id).unwrap().info.secondary_action.is_some());
    }
}

#[test]
fn json_pretty() {
    let out = forward(
        "json-formatter",
        r#"{"b":2,"a":"Привет","items":[true,null,1.0]}"#,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "{\n  \"b\" : 2,\n  \"a\" : \"Привет\",\n  \"items\" : [\n    true,\n    null,\n    1\n  ]\n}"
    );
    assert_eq!(out.chars().count(), 76);
}

#[test]
fn json_comment() {
    let out = forward(
        "json-formatter",
        "{// comment\n\"url\":\"https://example.test/a//b\",\"x\":1/*ok*/}",
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "{\n  \"url\" : \"https://example.test/a//b\",\n  \"x\" : 1\n}"
    );
    assert_eq!(out.chars().count(), 52);
}

#[test]
fn json_comment_kept_when_not_stripped_is_invalid() {
    assert!(forward(
        "json-formatter",
        "{/*c*/\"x\":1}",
        json!({"stripComments": false})
    )
    .is_err());
}

#[test]
fn json_invalid() {
    let err = forward("json-formatter", r#"{"x":}"#, json!({})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Invalid value around line 1, column 5. Check your input and try again"
    );
}

#[test]
fn json_options() {
    let input = r#"{"b":[1,{"z":"é","a":2.5}],"a":{}}"#;
    assert_eq!(
        forward("json-formatter", input, json!({"mode": "Minify"})).unwrap(),
        r#"{"b":[1,{"z":"é","a":2.5}],"a":{}}"#
    );
    assert_eq!(
        forward(
            "json-formatter",
            input,
            json!({"mode": "Minify", "sortKeys": true, "escapeUnicode": true})
        )
        .unwrap(),
        r#"{"a":{},"b":[1,{"a":2.5,"z":"\u00e9"}]}"#
    );
    assert_eq!(
        forward("json-formatter", "[1,2]", json!({"mode": "Compact"})).unwrap(),
        "[1, 2]"
    );
    assert_eq!(
        forward("json-formatter", "[1]", json!({"indent": 4})).unwrap(),
        "[\n    1\n]"
    );
    assert_eq!(
        forward("json-formatter", r#""😀""#, json!({"escapeUnicode": true})).unwrap(),
        r#""\ud83d\ude00""#
    );
}

#[test]
fn jsonpath_root() {
    let out = forward("json-path", r#"{"items":[{"n":1},{"n":2}]}"#, json!({})).unwrap();
    assert_eq!(
        out,
        "{\n  \"items\" : [\n    {\n      \"n\" : 1\n    },\n    {\n      \"n\" : 2\n    }\n  ]\n}"
    );
    assert_eq!(out.chars().count(), 74);
}

#[test]
fn jsonpath_queries() {
    let doc = r#"{"items":[{"n":1,"s":"a"},{"n":2,"s":"b"}],"k":{"x y":true}}"#;
    let q = |path: &str, extra: Value| {
        let mut options = json!({"path": path, "pretty": false});
        options
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        forward("json-path", doc, options)
    };
    assert_eq!(q("$.items[1].n", json!({})).unwrap(), "2");
    assert_eq!(q("$.items[-1].s", json!({"extract": true})).unwrap(), "b");
    assert_eq!(q("$.items[*].n", json!({})).unwrap(), "[1,2]");
    assert_eq!(q("$.items[*].s", json!({"extract": true})).unwrap(), "a\nb");
    assert_eq!(q("$..n", json!({})).unwrap(), "[1,2]");
    assert_eq!(q("$.k['x y']", json!({})).unwrap(), "true");
    assert_eq!(
        q("$.items[0:1]", json!({})).unwrap(),
        r#"[{"n":1,"s":"a"}]"#
    );
    assert_eq!(
        q("$.nope", json!({})).unwrap_err().to_string(),
        "Processing failed: Failed to execute query: Key 'nope' not found in object"
    );
    assert_eq!(
        q("$.items[5]", json!({})).unwrap_err().to_string(),
        "Processing failed: Failed to execute query: Index 5 out of bounds (array has 2 elements)"
    );
    assert_eq!(
        q("$.items[*].zzz", json!({})).unwrap_err().to_string(),
        "Processing failed: No matches found"
    );
    assert_eq!(
        q("items", json!({})).unwrap_err().to_string(),
        "Invalid input: Invalid JSONPath expression: JSONPath must start with $"
    );
    assert!(q("$.items[", json!({})).is_err());
    assert!(q("$.items[?(@.n)]", json!({})).is_err());
    assert_eq!(
        forward("json-path", "{", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Invalid JSON input"
    );
}

#[test]
fn yaml_basic() {
    let out = forward(
        "json-yaml",
        r#"{"b":2,"a":[true,null,"001"],"unicode":"Привет"}"#,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "\n  b: 2\n  a:\n    - true\n    - null\n    - \"001\"\n  unicode: Привет\n"
    );
    assert_eq!(out.chars().count(), 65);
}

#[test]
fn yaml_from_json_options_and_nesting() {
    assert_eq!(
        forward(
            "json-yaml",
            r#"{"b":[{"x":1,"w":[2]},[3]],"a":{"k":"v: w"}}"#,
            json!({"sortKeys": true, "indent": 4})
        )
        .unwrap(),
        "\n    a:\n        k: \"v: w\"\n    b:\n        - w:\n              - 2\n          x: 1\n        - - 3\n"
    );
}

#[test]
fn yaml_to_json() {
    assert_eq!(
        reverse(
            "json-yaml",
            "b: 2\na:\n  - true\n  - ~\n  - '001'\n",
            json!({})
        )
        .unwrap(),
        "{\n  \"b\" : 2,\n  \"a\" : [\n    true,\n    null,\n    \"001\"\n  ]\n}"
    );
    assert!(reverse("json-yaml", "# only a comment", json!({})).is_err());
    assert!(reverse("json-yaml", "a: [", json!({})).is_err());
}

#[test]
fn yaml_format() {
    // Scalars keep their type and keys keep document order.
    assert_eq!(
        forward(
            "yaml-formatter",
            "b: 2\na: [true, null, \"001\"]\n",
            json!({})
        )
        .unwrap(),
        "b: 2\na:\n- true\n- null\n- '001'\n"
    );
}

#[test]
fn yaml_formatter_options() {
    let input = "z: {w: [1, 2.5], x: hello world}\nlist:\n  - a: 1\n    b: 'yes'\n";
    assert_eq!(
        forward(
            "yaml-formatter",
            input,
            json!({"sortKeys": true, "documentStart": true, "indent": 4})
        )
        .unwrap(),
        "---\nlist:\n- a: 1\n  b: 'yes'\nz:\n    w:\n    - 1\n    - 2.5\n    x: hello world\n"
    );
    assert_eq!(
        forward("yaml-formatter", "a: 1\n---\nb: 2\n", json!({})).unwrap(),
        "a: 1\n---\nb: 2\n"
    );
    assert!(forward("yaml-formatter", "a: [", json!({})).is_err());
}

#[test]
fn xml_basic() {
    let out = forward(
        "xml-json",
        r#"<root a="1"><item>x</item><item>2</item></root>"#,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "{\n  \"root\" : {\n    \"@a\" : \"1\",\n    \"item\" : [\n      \"x\",\n      \"2\"\n    ]\n  }\n}"
    );
    assert_eq!(out.chars().count(), 78);
}

#[test]
fn xml_json_options_and_reverse() {
    assert_eq!(
        forward(
            "xml-json",
            r#"<a k="v">hi <![CDATA[<b>]]><c/></a>"#,
            json!({"attributePrefix": "_", "textKey": "$value", "indent": 1})
        )
        .unwrap(),
        "{\n \"a\" : {\n  \"_k\" : \"v\",\n  \"c\" : \"\",\n  \"$value\" : \"hi<b>\"\n }\n}"
    );
    assert_eq!(
        reverse(
            "xml-json",
            r##"{"root":{"@a":"1","item":["x","2"],"e":null,"#text":"t&u"}}"##,
            json!({})
        )
        .unwrap(),
        "<root a=\"1\">\n  <item>x</item>\n  <item>2</item>\n  <e/>\n  t&amp;u\n</root>"
    );
    assert_eq!(
        reverse("xml-json", "[1,2]", json!({})).unwrap(),
        "<root>1</root>\n<root>2</root>"
    );
    assert!(reverse("xml-json", r#"{"bad name":1}"#, json!({})).is_err());
    assert!(forward("xml-json", "<a><b></a>", json!({})).is_err());
    assert!(forward("xml-json", "<!-- nothing -->", json!({})).is_err());
}

#[test]
fn xml_formatter() {
    let input = "<?xml version=\"1.0\"?><r x=\"1\"><!-- c --><a>t</a><b><![CDATA[<&>]]></b><e/><f></f><g><h>1</h></g></r>";
    assert_eq!(
        forward("xml-formatter", input, json!({})).unwrap(),
        "<?xml version=\"1.0\"?>\n<r x=\"1\">\n  <!-- c -->\n  <a>t</a>\n  <b><![CDATA[<&>]]></b>\n  <e/>\n  <f></f>\n  <g>\n    <h>1</h>\n  </g>\n</r>"
    );
    assert_eq!(
        forward(
            "xml-formatter",
            input,
            json!({"mode": "Minify", "preserveCdata": false})
        )
        .unwrap(),
        "<?xml version=\"1.0\"?><r x=\"1\"><!-- c --><a>t</a><b>&lt;&amp;&gt;</b><e/><f></f><g><h>1</h></g></r>"
    );
    assert_eq!(
        forward(
            "xml-formatter",
            "<a>\n  <b>x</b>\n</a>",
            json!({"indent": 4})
        )
        .unwrap(),
        "<a>\n    <b>x</b>\n</a>"
    );
    assert!(forward("xml-formatter", "<a><b></a>", json!({})).is_err());
    assert!(forward("xml-formatter", "<a>", json!({})).is_err());
    assert!(forward("xml-formatter", "just text", json!({})).is_err());
}

#[test]
fn csv_basic() {
    let out = forward(
        "csv-to-json",
        "name,value,ok\n\"a,b\",001,true\nx,2.5,false",
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "[\n  {\n    \"name\" : \"a,b\",\n    \"ok\" : true,\n    \"value\" : \"001\"\n  },\n  {\n    \"name\" : \"x\",\n    \"ok\" : false,\n    \"value\" : 2.5\n  }\n]"
    );
    assert_eq!(out.chars().count(), 131);
}

#[test]
fn csv_options() {
    assert_eq!(
        forward(
            "csv-to-json",
            "a;b\n 1 ; x \n2",
            json!({"format": "Array of Arrays"})
        )
        .unwrap(),
        "[\n  [\n    \"a\",\n    \"b\"\n  ],\n  [\n    1,\n    \"x\"\n  ],\n  [\n    2\n  ]\n]"
    );
    assert_eq!(
        forward(
            "csv-to-json",
            "1|2\n3|4",
            json!({"delimiter": "Pipe (|)", "hasHeaders": false, "parseNumbers": false, "trim": false})
        )
        .unwrap(),
        "[\n  {\n    \"column1\" : \"1\",\n    \"column2\" : \"2\"\n  },\n  {\n    \"column1\" : \"3\",\n    \"column2\" : \"4\"\n  }\n]"
    );
    assert_eq!(
        forward(
            "csv-to-json",
            "a\tb\n1\t2",
            json!({"delimiter": "Auto-detect"})
        )
        .unwrap(),
        "[\n  {\n    \"a\" : 1,\n    \"b\" : 2\n  }\n]"
    );
    assert_eq!(
        forward(
            "csv-to-json",
            "a,b\n1,2",
            json!({"delimiter": "Semicolon (;)"})
        )
        .unwrap(),
        "[\n  {\n    \"a,b\" : \"1,2\"\n  }\n]"
    );
    assert_eq!(
        forward("csv-to-json", "h1,h2\n1", json!({})).unwrap(),
        "[\n  {\n    \"h1\" : 1,\n    \"h2\" : \"\"\n  }\n]"
    );
}

#[test]
fn json_csv_nested() {
    // Numbers stay numbers (not coerced to booleans).
    let out = forward(
        "json-to-csv",
        r#"[{"name":"a,b","obj":{"x":1},"arr":[1,2]},{"name":"x","obj":{"x":2}}]"#,
        json!({}),
    )
    .unwrap();
    assert_eq!(out, "arr,name,obj.x\n1;2,\"a,b\",1\n,x,2\n");
}

#[test]
fn json_csv_options() {
    let input = r#"[{"a":{"b":{"c":1}},"l":["x",{"k":true}],"n":null}]"#;
    let run_with = |options: Value| forward("json-to-csv", input, options).unwrap();
    assert_eq!(
        run_with(json!({})),
        "a.b,l,n\n\"{\"\"c\"\":1}\",\"x;{\"\"k\"\":true}\",\n"
    );
    assert_eq!(
        run_with(json!({"flatten": "All", "includeHeaders": false, "delimiter": "Tab"})),
        "1\t\"x;{\"\"k\"\":true}\"\t\n"
    );
    assert_eq!(
        run_with(json!({"flatten": "None", "arrays": "JSON string", "delimiter": "Semicolon (;)"})),
        "a;l;n\n\"{\"\"b\"\":{\"\"c\"\":1}}\";\"[\"\"x\"\",{\"\"k\"\":true}]\";\n"
    );
    assert_eq!(
        run_with(json!({"flatten": "All", "arrays": "Index columns"})),
        "a.b.c,l.0,l.1.k,n\n1,x,true,\n"
    );
    assert_eq!(
        forward(
            "json-to-csv",
            r#"{"a":1}"#,
            json!({"delimiter": "Pipe (|)"})
        )
        .unwrap(),
        "a\n1\n"
    );
    assert_eq!(
        forward("json-to-csv", "[1,\"x\"]", json!({})).unwrap(),
        "value\n1\nx\n"
    );
    assert_eq!(
        forward("json-to-csv", "3", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Expected JSON array or object"
    );
}

#[test]
fn sql_basic() {
    let out = forward(
        "sql-formatter",
        "select a,b from users where x=1 and y=2 order by a desc;",
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "SELECT a,\nb\nFROM users\nWHERE x=1\n  AND y=2\nORDER BY a DESC;"
    );
    assert_eq!(out.chars().count(), 59);
}

#[test]
fn sql_options() {
    let input = "SELECT count(a,b), c FROM t LEFT OUTER JOIN u ON t.id = u.id WHERE x BETWEEN 1 AND 2 OR y = 'a b,c'; select 1";
    assert_eq!(
        forward("sql-formatter", input, json!({"indent": 4})).unwrap(),
        "SELECT count(a, b),\nc\nFROM t\nLEFT OUTER JOIN u ON t.id = u.id\nWHERE x BETWEEN 1 AND 2\n    OR y = 'a b,c';\n\nSELECT 1"
    );
    assert_eq!(
        forward(
            "sql-formatter",
            "select a,b from t where x=1 and y=2",
            json!({"keywordCase": "lowercase", "commaFirst": true, "newlineBeforeAnd": false})
        )
        .unwrap(),
        "select a\n, b\nfrom t\nwhere x=1 and y=2"
    );
    assert_eq!(
        forward(
            "sql-formatter",
            "Select (select 1) From t -- note\nwhere a=1",
            json!({"keywordCase": "Preserve"})
        )
        .unwrap(),
        "Select (select 1)\nFrom t -- note\nwhere a=1"
    );
}

#[test]
fn graphql_basic() {
    let out = forward(
        "graphql-formatter",
        "query Q($id:ID!){user(id:$id){id name friends{id}}}",
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "query\nQ($id:ID!) {\n  user(id:$id) {\n    id\n    name\n    friends {\n      id\n    }\n  }\n}"
    );
    assert_eq!(out.chars().count(), 86);
}

#[test]
fn graphql_options() {
    assert_eq!(
        forward(
            "graphql-formatter",
            "{ b, a(x: \"}\") ... on T { z y } # tail\n}",
            json!({"sortFields": true, "indent": 4})
        )
        .unwrap(),
        "{\n    # tail\n    ... on T {\n        y\n        z\n    }\n    a(x: \"}\")\n    b\n}"
    );
    assert!(forward("graphql-formatter", "{ a ", json!({})).is_err());
    assert!(forward("graphql-formatter", "a }", json!({})).is_err());
}

#[test]
fn toml_basic() {
    let out = forward(
        "toml-parser",
        "title = \"test\"\n[server]\nport = 8080\nenabled = true",
        json!({}),
    )
    .unwrap();
    assert_eq!(
        out,
        "title = \"test\"\n\n[server]\nport = 8080\nenabled = true\n"
    );
    assert_eq!(out.chars().count(), 52);
}

#[test]
fn toml_options() {
    let input = "z = 1\na = [1, 2]\n[s.t]\nk = 'v'\n[s]\nn = { x = 1 }\n[[p]]\nq = 1\n[[p]]\nq = 2\n[p.deep]\nd = 3\n";
    assert_eq!(
        forward("toml-parser", input, json!({})).unwrap(),
        "z = 1\na = [1, 2]\n\n[s.t]\nk = \"v\"\n\n[s.n]\nx = 1\n\n[[p]]\nq = 1\n\n[[p]]\nq = 2\n\n[p.deep]\nd = 3\n"
    );
    assert_eq!(
        forward(
            "toml-parser",
            input,
            json!({"sortKeys": true, "inlineTables": true})
        )
        .unwrap(),
        "a = [1, 2]\nz = 1\n\n[[p]]\nq = 1\n\n[[p]]\ndeep = { d = 3 }\nq = 2\n\n[s]\nn = { x = 1 }\nt = { k = \"v\" }\n"
    );
    assert!(forward("toml-parser", "a = ", json!({})).is_err());
    assert_eq!(
        forward("toml-parser", "# nothing", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: (no valid key-value pairs found)"
    );
}
