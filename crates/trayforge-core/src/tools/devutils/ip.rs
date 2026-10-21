use std::net::{Ipv4Addr, Ipv6Addr};

use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "ip-address",
            name: "IP Address",
            category: Category::DevUtils,
            description: "Validate, convert, and analyze IP addresses and subnets",
            keywords: vec!["ip", "ipv4", "ipv6", "subnet", "cidr", "network", "mask"],
            icon: "globe",
            input_placeholder: "Enter IP address (e.g., 192.168.1.1 or 192.168.1.0/24)...",
            output_placeholder: "IP address information will appear here",
            primary_action: "Analyze",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "operation",
                    label: "Operation",
                    help: Some("Choose the operation to perform"),
                    kind: OptionKind::Picker {
                        choices: vec![
                            "Analyze",
                            "Convert to Binary",
                            "Convert to Decimal",
                            "Subnet Calculator",
                            "Validate Only",
                        ],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "showBinary",
                    label: "Show binary representation",
                    help: Some("Display IP address in binary format"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "showDecimal",
                    label: "Show decimal representation",
                    help: Some("Display IP address as decimal number"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(process),
    }
}

enum Address {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

fn invalid(message: &str) -> ToolError {
    ToolError::InvalidInput(message.into())
}

/// Parses `address` or `address/prefix`.
fn parse(input: &str) -> Result<(Address, Option<u8>), ToolError> {
    let (text, prefix) = match input.trim().split_once('/') {
        Some((addr, prefix)) => (addr, Some(prefix)),
        None => (input.trim(), None),
    };
    let address = if let Ok(v4) = text.parse::<Ipv4Addr>() {
        Address::V4(v4)
    } else if let Ok(v6) = text.parse::<Ipv6Addr>() {
        Address::V6(v6)
    } else {
        return Err(invalid("Invalid IP address format"));
    };
    let max = match address {
        Address::V4(_) => 32,
        Address::V6(_) => 128,
    };
    let prefix = prefix
        .map(|p| {
            p.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| p.parse::<u8>().ok())
                .flatten()
                .filter(|p| *p <= max)
                .ok_or_else(|| ToolError::InvalidInput(format!("Invalid prefix length (0-{max})")))
        })
        .transpose()?;
    Ok((address, prefix))
}

fn binary_v4(ip: Ipv4Addr) -> String {
    ip.octets()
        .iter()
        .map(|o| format!("{o:08b}"))
        .collect::<Vec<_>>()
        .join(".")
}

fn process(req: &Request) -> ToolResult {
    let input = req.input.trim();
    match req.options.str("operation") {
        "Validate Only" => Ok(ToolOutput::text(match parse(input)?.0 {
            Address::V4(_) => "Valid IPv4 Address",
            Address::V6(_) => "Valid IPv6 Address",
        })),
        "Convert to Binary" => match parse(input) {
            Ok((Address::V4(ip), _)) => Ok(ToolOutput::text(binary_v4(ip))),
            _ => Err(invalid(
                "Cannot convert to binary. Enter a valid IPv4 address.",
            )),
        },
        "Convert to Decimal" => {
            if let Ok(number) = input.parse::<u32>() {
                return Ok(ToolOutput::text(Ipv4Addr::from(number).to_string()));
            }
            match parse(input) {
                Ok((Address::V4(ip), _)) => Ok(ToolOutput::text(u32::from(ip).to_string())),
                _ => Err(invalid("Enter a valid IPv4 address or decimal number")),
            }
        }
        "Subnet Calculator" => match parse(input) {
            Ok((Address::V4(ip), Some(prefix))) => Ok(ToolOutput::multiple(subnet(ip, prefix))),
            _ => Err(invalid("Enter IP in CIDR notation (e.g., 192.168.1.0/24)")),
        },
        _ => analyze(req, input),
    }
}

fn subnet(ip: Ipv4Addr, prefix: u8) -> Vec<OutputItem> {
    let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
    let network = u32::from(ip) & mask;
    let broadcast = network | !mask;
    let total = 1u64 << (32 - prefix);
    let (usable, first, last) = match prefix {
        32 => (1, network, network),
        31 => (2, network, broadcast),
        _ => (total - 2, network + 1, broadcast - 1),
    };
    let v4 = |n: u32| Ipv4Addr::from(n).to_string();
    vec![
        OutputItem::new("CIDR", format!("{ip}/{prefix}")),
        OutputItem::new("Subnet Mask", v4(mask)),
        OutputItem::new("Wildcard Mask", v4(!mask)),
        OutputItem::new("Network Address", v4(network)),
        OutputItem::new("Broadcast Address", v4(broadcast)),
        OutputItem::new("Total Addresses", total.to_string()),
        OutputItem::new("Usable Hosts", usable.to_string()),
        OutputItem::new("First Usable Host", v4(first)),
        OutputItem::new("Last Usable Host", v4(last)),
    ]
}

fn analyze(req: &Request, input: &str) -> ToolResult {
    let (address, prefix) = parse(input)?;
    let show_binary = req.options.bool("showBinary");
    let show_decimal = req.options.bool("showDecimal");
    let mut items = Vec::new();
    match address {
        Address::V4(ip) => {
            let [a, b, ..] = ip.octets();
            let class = match a {
                0..=127 => "A",
                128..=191 => "B",
                192..=223 => "C",
                224..=239 => "D (Multicast)",
                _ => "E (Reserved)",
            };
            let kind = if a == 0 {
                "This Network"
            } else if a == 127 {
                "Loopback"
            } else if ip.is_broadcast() {
                "Broadcast"
            } else if a == 169 && b == 254 {
                "Link-Local (APIPA)"
            } else if a >= 224 {
                "Multicast/Reserved"
            } else if ip.is_private() {
                "Private"
            } else {
                "Public"
            };
            items.push(OutputItem::new("Version", "IPv4"));
            items.push(OutputItem::new("Address", ip.to_string()));
            if show_binary {
                items.push(OutputItem::new("Binary", binary_v4(ip)));
            }
            if show_decimal {
                items.push(OutputItem::new("Decimal", u32::from(ip).to_string()));
            }
            items.push(OutputItem::new("Class", class));
            items.push(OutputItem::new("Type", kind));
            if let Some(prefix) = prefix {
                items.extend(subnet(ip, prefix));
            }
        }
        Address::V6(ip) => {
            let segments = ip.segments();
            let kind = if ip.is_unspecified() {
                "Unspecified (::)"
            } else if ip.is_loopback() {
                "Loopback (::1)"
            } else if segments[0] & 0xffc0 == 0xfe80 {
                "Link-Local"
            } else if segments[0] & 0xfe00 == 0xfc00 {
                "Unique Local"
            } else if ip.is_multicast() {
                "Multicast"
            } else {
                "Global Unicast"
            };
            items.push(OutputItem::new("Version", "IPv6"));
            items.push(OutputItem::new("Address", ip.to_string()));
            items.push(OutputItem::new(
                "Expanded",
                segments
                    .iter()
                    .map(|s| format!("{s:04x}"))
                    .collect::<Vec<_>>()
                    .join(":"),
            ));
            items.push(OutputItem::new("Compressed", ip.to_string()));
            if show_binary {
                items.push(OutputItem::new(
                    "Binary",
                    segments
                        .iter()
                        .map(|s| format!("{s:016b}"))
                        .collect::<Vec<_>>()
                        .join(":"),
                ));
            }
            if show_decimal {
                items.push(OutputItem::new("Decimal", u128::from(ip).to_string()));
            }
            items.push(OutputItem::new("Type", kind));
            if let Some(prefix) = prefix {
                let mask = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
                items.push(OutputItem::new("CIDR", format!("{ip}/{prefix}")));
                items.push(OutputItem::new(
                    "Network Address",
                    Ipv6Addr::from(u128::from(ip) & mask).to_string(),
                ));
            }
        }
    }
    Ok(ToolOutput::multiple(items))
}
