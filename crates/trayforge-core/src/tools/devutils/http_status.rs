use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

const LIST_LIMIT: usize = 10;

/// (code, reason phrase, description)
const STATUS_CODES: &[(u16, &str, &str)] = &[
    (
        100,
        "Continue",
        "The server received the request headers and the client should proceed to send the body.",
    ),
    (
        101,
        "Switching Protocols",
        "The server is switching to the protocol requested by the client.",
    ),
    (
        102,
        "Processing",
        "The server has accepted the request but has not completed it yet (WebDAV).",
    ),
    (
        103,
        "Early Hints",
        "Preliminary headers that let the client start preloading resources.",
    ),
    (200, "OK", "The request succeeded."),
    (
        201,
        "Created",
        "The request succeeded and a new resource was created.",
    ),
    (
        202,
        "Accepted",
        "The request was accepted for processing, but processing is not complete.",
    ),
    (
        203,
        "Non-Authoritative Information",
        "The response was modified by a transforming proxy.",
    ),
    (
        204,
        "No Content",
        "The request succeeded and there is no content to send.",
    ),
    (
        205,
        "Reset Content",
        "The request succeeded; the client should reset the document view.",
    ),
    (
        206,
        "Partial Content",
        "The server is delivering only part of the resource (range request).",
    ),
    (
        207,
        "Multi-Status",
        "The response body contains multiple independent status codes (WebDAV).",
    ),
    (
        208,
        "Already Reported",
        "Members of a DAV binding were already enumerated (WebDAV).",
    ),
    (
        226,
        "IM Used",
        "The server fulfilled a GET using instance manipulations.",
    ),
    (
        300,
        "Multiple Choices",
        "The request has more than one possible response.",
    ),
    (
        301,
        "Moved Permanently",
        "The resource has permanently moved to the URL given in the Location header.",
    ),
    (
        302,
        "Found",
        "The resource is temporarily available at a different URL.",
    ),
    (
        303,
        "See Other",
        "The response can be found at another URL using a GET request.",
    ),
    (
        304,
        "Not Modified",
        "The cached version of the resource is still valid.",
    ),
    (
        305,
        "Use Proxy",
        "The resource must be accessed through the given proxy (deprecated).",
    ),
    (
        307,
        "Temporary Redirect",
        "Temporary redirect that keeps the request method and body.",
    ),
    (
        308,
        "Permanent Redirect",
        "Permanent redirect that keeps the request method and body.",
    ),
    (
        400,
        "Bad Request",
        "The server cannot process the request due to a client error such as malformed syntax.",
    ),
    (
        401,
        "Unauthorized",
        "Authentication is required and has failed or has not been provided.",
    ),
    (
        402,
        "Payment Required",
        "Reserved for future use; sometimes used for paywalled APIs.",
    ),
    (
        403,
        "Forbidden",
        "The server understood the request but refuses to authorize it.",
    ),
    (
        404,
        "Not Found",
        "The server cannot find the requested resource.",
    ),
    (
        405,
        "Method Not Allowed",
        "The request method is not supported for the requested resource.",
    ),
    (
        406,
        "Not Acceptable",
        "No representation matches the Accept headers sent by the client.",
    ),
    (
        407,
        "Proxy Authentication Required",
        "Authentication with the proxy is required.",
    ),
    (
        408,
        "Request Timeout",
        "The server timed out waiting for the request.",
    ),
    (
        409,
        "Conflict",
        "The request conflicts with the current state of the resource.",
    ),
    (
        410,
        "Gone",
        "The resource is no longer available and will not be available again.",
    ),
    (
        411,
        "Length Required",
        "The request did not specify a Content-Length the server requires.",
    ),
    (
        412,
        "Precondition Failed",
        "A precondition in the request headers evaluated to false.",
    ),
    (
        413,
        "Payload Too Large",
        "The request body is larger than the server is willing to process.",
    ),
    (
        414,
        "URI Too Long",
        "The request URI is longer than the server is willing to interpret.",
    ),
    (
        415,
        "Unsupported Media Type",
        "The media type of the request body is not supported.",
    ),
    (
        416,
        "Range Not Satisfiable",
        "The requested range cannot be served.",
    ),
    (
        417,
        "Expectation Failed",
        "The expectation in the Expect header cannot be met.",
    ),
    (
        418,
        "I'm a teapot",
        "The server refuses to brew coffee because it is a teapot (RFC 2324).",
    ),
    (
        421,
        "Misdirected Request",
        "The request was directed at a server that cannot produce a response.",
    ),
    (
        422,
        "Unprocessable Entity",
        "The request is well-formed but contains semantic errors.",
    ),
    (423, "Locked", "The resource is locked (WebDAV)."),
    (
        424,
        "Failed Dependency",
        "The request failed because a previous request failed (WebDAV).",
    ),
    (
        425,
        "Too Early",
        "The server is unwilling to risk processing a request that might be replayed.",
    ),
    (
        426,
        "Upgrade Required",
        "The client should switch to a different protocol.",
    ),
    (
        428,
        "Precondition Required",
        "The server requires the request to be conditional.",
    ),
    (
        429,
        "Too Many Requests",
        "The client has sent too many requests in a given time (rate limiting).",
    ),
    (
        431,
        "Request Header Fields Too Large",
        "The request headers are too large.",
    ),
    (
        451,
        "Unavailable For Legal Reasons",
        "The resource is unavailable due to a legal demand.",
    ),
    (
        500,
        "Internal Server Error",
        "The server encountered an unexpected condition.",
    ),
    (
        501,
        "Not Implemented",
        "The server does not support the functionality required to fulfill the request.",
    ),
    (
        502,
        "Bad Gateway",
        "The server, acting as a gateway, received an invalid response from upstream.",
    ),
    (
        503,
        "Service Unavailable",
        "The server is temporarily unable to handle the request (overload or maintenance).",
    ),
    (
        504,
        "Gateway Timeout",
        "The server, acting as a gateway, did not receive a timely response from upstream.",
    ),
    (
        505,
        "HTTP Version Not Supported",
        "The HTTP version used in the request is not supported.",
    ),
    (
        506,
        "Variant Also Negotiates",
        "Transparent content negotiation resulted in a circular reference.",
    ),
    (
        507,
        "Insufficient Storage",
        "The server cannot store the representation needed to complete the request (WebDAV).",
    ),
    (
        508,
        "Loop Detected",
        "The server detected an infinite loop while processing the request (WebDAV).",
    ),
    (
        510,
        "Not Extended",
        "Further extensions to the request are required for the server to fulfill it.",
    ),
    (
        511,
        "Network Authentication Required",
        "The client must authenticate to gain network access.",
    ),
];

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "http-status",
            name: "HTTP Status",
            category: Category::DevUtils,
            description: "Look up HTTP status codes and their meanings",
            keywords: vec!["http", "status", "code", "response", "error"],
            icon: "server",
            input_placeholder: "Enter status code (e.g., 404) or keyword (e.g., not found)...",
            output_placeholder: "HTTP status information will appear here",
            primary_action: "Lookup",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![ToolOption {
                id: "showAll",
                label: "Show all codes",
                help: Some("Show all HTTP status codes"),
                kind: OptionKind::Toggle { default: false },
            }],
        },
        process: Some(process),
    }
}

fn category(code: u16) -> &'static str {
    match code / 100 {
        1 => "Informational",
        2 => "Success",
        3 => "Redirection",
        4 => "Client Error",
        _ => "Server Error",
    }
}

fn category_description(class: u16) -> Option<&'static str> {
    Some(match class {
        1 => "1xx codes are informational - the request was received and processing continues.",
        2 => "2xx codes indicate success - the request was received, understood, and accepted.",
        3 => "3xx codes indicate redirection - further action is needed to complete the request.",
        4 => "4xx codes indicate client error - the request contains bad syntax or cannot be fulfilled.",
        5 => "5xx codes indicate server error - the server failed to fulfill a valid request.",
        _ => return None,
    })
}

fn details(&(code, name, description): &(u16, &str, &str)) -> String {
    format!(
        "{code} {name}\n\nCategory:    {}\nDescription: {description}\n\n{}",
        category(code),
        category_description(code / 100).unwrap_or_default()
    )
}

fn process(req: &Request) -> ToolResult {
    let query = req.input.trim().to_lowercase();
    let show_all = req.options.bool("showAll");

    if !show_all {
        if let Ok(code) = query.parse::<u16>() {
            return match STATUS_CODES.iter().find(|s| s.0 == code) {
                Some(status) => Ok(ToolOutput::text(details(status))),
                None => {
                    let hint = category_description(code / 100).unwrap_or_default();
                    Ok(ToolOutput::text(
                        format!("Unknown HTTP status code: {code}\n\n{hint}").trim_end(),
                    ))
                }
            };
        }
        // "4xx" lists a whole class.
        if let Some(class) = query
            .strip_suffix("xx")
            .and_then(|c| c.parse::<u16>().ok())
            .filter(|c| category_description(*c).is_some())
        {
            return Ok(list(
                STATUS_CODES.iter().filter(|s| s.0 / 100 == class).collect(),
                false,
            ));
        }
    }

    let matches: Vec<_> = STATUS_CODES
        .iter()
        .filter(|(code, name, description)| {
            show_all
                || query.is_empty()
                || code.to_string().contains(&query)
                || name.to_lowercase().contains(&query)
                || description.to_lowercase().contains(&query)
        })
        .collect();
    match matches.as_slice() {
        [] => Err(ToolError::InvalidInput(format!(
            "No status codes found matching '{}'",
            req.input.trim()
        ))),
        [only] => Ok(ToolOutput::text(details(only))),
        _ => Ok(list(matches, show_all)),
    }
}

fn list(matches: Vec<&(u16, &str, &str)>, show_all: bool) -> ToolOutput {
    let total = matches.len();
    let shown = if show_all {
        total
    } else {
        total.min(LIST_LIMIT)
    };
    let mut out: Vec<String> = matches[..shown]
        .iter()
        .map(|(code, name, _)| format!("{code}  {name}  ({})", category(*code)))
        .collect();
    if shown < total {
        out.push(format!("\nShowing first {shown} of {total} matches"));
    }
    ToolOutput::text(out.join("\n"))
}
