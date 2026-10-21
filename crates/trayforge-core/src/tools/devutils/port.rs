use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

const LIST_LIMIT: usize = 15;

struct Port {
    number: u16,
    service: &'static str,
    protocol: &'static str,
    category: &'static str,
    description: &'static str,
}

const fn p(
    number: u16,
    service: &'static str,
    protocol: &'static str,
    category: &'static str,
    description: &'static str,
) -> Port {
    Port {
        number,
        service,
        protocol,
        category,
        description,
    }
}

const PORTS: &[Port] = &[
    p(
        80,
        "HTTP",
        "TCP",
        "Web",
        "Hypertext Transfer Protocol (web traffic)",
    ),
    p(
        443,
        "HTTPS",
        "TCP",
        "Web",
        "HTTP over TLS/SSL (secure web traffic)",
    ),
    p(
        8080,
        "HTTP-Alt",
        "TCP",
        "Web",
        "Alternative HTTP port, often used for proxies",
    ),
    p(8443, "HTTPS-Alt", "TCP", "Web", "Alternative HTTPS port"),
    p(
        3000,
        "Dev Server",
        "TCP",
        "Development",
        "Common development server port (Node.js, Rails, etc.)",
    ),
    p(
        5000,
        "Dev Server",
        "TCP",
        "Development",
        "Common development server port (Flask, etc.)",
    ),
    p(
        4200,
        "Angular",
        "TCP",
        "Development",
        "Angular CLI development server",
    ),
    p(
        3001,
        "React Dev",
        "TCP",
        "Development",
        "React development server (alternative)",
    ),
    p(
        8000,
        "Django/Python",
        "TCP",
        "Development",
        "Django/Python development server",
    ),
    p(
        5173,
        "Vite",
        "TCP",
        "Development",
        "Vite development server",
    ),
    p(
        4000,
        "Phoenix",
        "TCP",
        "Development",
        "Phoenix/Elixir development server",
    ),
    p(
        4369,
        "EPMD",
        "TCP",
        "Development",
        "Erlang Port Mapper Daemon",
    ),
    p(9229, "Node Debug", "TCP", "Development", "Node.js debugger"),
    p(
        35729,
        "LiveReload",
        "TCP",
        "Development",
        "LiveReload server",
    ),
    p(
        6006,
        "TensorBoard",
        "TCP",
        "Development",
        "TensorFlow TensorBoard",
    ),
    p(8888, "Jupyter", "TCP", "Development", "Jupyter Notebook"),
    p(3306, "MySQL", "TCP", "Database", "MySQL database server"),
    p(
        5432,
        "PostgreSQL",
        "TCP",
        "Database",
        "PostgreSQL database server",
    ),
    p(
        27017,
        "MongoDB",
        "TCP",
        "Database",
        "MongoDB database server",
    ),
    p(6379, "Redis", "TCP", "Database", "Redis key-value store"),
    p(
        11211,
        "Memcached",
        "TCP/UDP",
        "Database",
        "Memcached caching system",
    ),
    p(
        9200,
        "Elasticsearch",
        "TCP",
        "Database",
        "Elasticsearch REST API",
    ),
    p(9042, "Cassandra", "TCP", "Database", "Apache Cassandra CQL"),
    p(1433, "MSSQL", "TCP", "Database", "Microsoft SQL Server"),
    p(
        1521,
        "Oracle",
        "TCP",
        "Database",
        "Oracle database listener",
    ),
    p(5984, "CouchDB", "TCP", "Database", "CouchDB database"),
    p(8529, "ArangoDB", "TCP", "Database", "ArangoDB database"),
    p(7687, "Neo4j", "TCP", "Database", "Neo4j graph database"),
    p(26257, "CockroachDB", "TCP", "Database", "CockroachDB SQL"),
    p(25, "SMTP", "TCP", "Mail", "Simple Mail Transfer Protocol"),
    p(465, "SMTPS", "TCP", "Mail", "SMTP over SSL"),
    p(
        587,
        "SMTP",
        "TCP",
        "Mail",
        "SMTP submission (with STARTTLS)",
    ),
    p(110, "POP3", "TCP", "Mail", "Post Office Protocol v3"),
    p(995, "POP3S", "TCP", "Mail", "POP3 over SSL"),
    p(
        143,
        "IMAP",
        "TCP",
        "Mail",
        "Internet Message Access Protocol",
    ),
    p(993, "IMAPS", "TCP", "Mail", "IMAP over SSL"),
    p(20, "FTP-Data", "TCP", "File Transfer", "FTP data transfer"),
    p(21, "FTP", "TCP", "File Transfer", "FTP control/command"),
    p(
        22,
        "SSH/SFTP",
        "TCP",
        "Remote Access",
        "Secure Shell and SFTP",
    ),
    p(
        69,
        "TFTP",
        "UDP",
        "File Transfer",
        "Trivial File Transfer Protocol",
    ),
    p(
        115,
        "SFTP",
        "TCP",
        "File Transfer",
        "Simple File Transfer Protocol",
    ),
    p(
        873,
        "rsync",
        "TCP",
        "File Transfer",
        "rsync file synchronization",
    ),
    p(
        2049,
        "NFS",
        "TCP/UDP",
        "File Transfer",
        "Network File System",
    ),
    p(
        445,
        "SMB",
        "TCP",
        "File Transfer",
        "Server Message Block (file sharing)",
    ),
    p(
        139,
        "NetBIOS",
        "TCP",
        "File Transfer",
        "NetBIOS Session Service",
    ),
    p(
        23,
        "Telnet",
        "TCP",
        "Remote Access",
        "Telnet protocol (insecure)",
    ),
    p(
        3389,
        "RDP",
        "TCP",
        "Remote Access",
        "Remote Desktop Protocol",
    ),
    p(
        5900,
        "VNC",
        "TCP",
        "Remote Access",
        "Virtual Network Computing",
    ),
    p(5901, "VNC", "TCP", "Remote Access", "VNC display :1"),
    p(53, "DNS", "TCP/UDP", "Network", "Domain Name System"),
    p(67, "DHCP", "UDP", "Network", "DHCP Server"),
    p(68, "DHCP", "UDP", "Network", "DHCP Client"),
    p(123, "NTP", "UDP", "Network", "Network Time Protocol"),
    p(
        161,
        "SNMP",
        "UDP",
        "Network",
        "Simple Network Management Protocol",
    ),
    p(162, "SNMPTRAP", "UDP", "Network", "SNMP Trap"),
    p(
        389,
        "LDAP",
        "TCP",
        "Network",
        "Lightweight Directory Access Protocol",
    ),
    p(636, "LDAPS", "TCP", "Network", "LDAP over SSL"),
    p(5672, "AMQP", "TCP", "Messaging", "RabbitMQ AMQP"),
    p(15672, "RabbitMQ", "TCP", "Messaging", "RabbitMQ Management"),
    p(9092, "Kafka", "TCP", "Messaging", "Apache Kafka"),
    p(2181, "Zookeeper", "TCP", "Messaging", "Apache Zookeeper"),
    p(1883, "MQTT", "TCP", "Messaging", "MQTT (IoT messaging)"),
    p(8883, "MQTTS", "TCP", "Messaging", "MQTT over SSL"),
    p(6667, "IRC", "TCP", "Messaging", "Internet Relay Chat"),
    p(
        2375,
        "Docker",
        "TCP",
        "DevOps",
        "Docker REST API (insecure)",
    ),
    p(2376, "Docker", "TCP", "DevOps", "Docker REST API (TLS)"),
    p(
        2377,
        "Swarm",
        "TCP",
        "DevOps",
        "Docker Swarm cluster management",
    ),
    p(5000, "Registry", "TCP", "DevOps", "Docker Registry"),
    p(8500, "Consul", "TCP", "DevOps", "HashiCorp Consul"),
    p(8200, "Vault", "TCP", "DevOps", "HashiCorp Vault"),
    p(9000, "Portainer", "TCP", "DevOps", "Portainer UI"),
    p(10250, "Kubelet", "TCP", "DevOps", "Kubernetes Kubelet API"),
    p(6443, "K8s API", "TCP", "DevOps", "Kubernetes API Server"),
    p(
        30000,
        "NodePort",
        "TCP",
        "DevOps",
        "Kubernetes NodePort range start",
    ),
    p(
        32767,
        "NodePort",
        "TCP",
        "DevOps",
        "Kubernetes NodePort range end",
    ),
    p(
        9090,
        "Prometheus",
        "TCP",
        "Monitoring",
        "Prometheus metrics server",
    ),
    p(3100, "Loki", "TCP", "Monitoring", "Grafana Loki"),
    p(
        9093,
        "Alertmanager",
        "TCP",
        "Monitoring",
        "Prometheus Alertmanager",
    ),
    p(
        9100,
        "Node Exporter",
        "TCP",
        "Monitoring",
        "Prometheus Node Exporter",
    ),
    p(8086, "InfluxDB", "TCP", "Monitoring", "InfluxDB HTTP API"),
    p(3030, "Grafana", "TCP", "Monitoring", "Grafana dashboard"),
];

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "port-lookup",
            name: "Port Lookup",
            category: Category::DevUtils,
            description: "Look up common port numbers and their services",
            keywords: vec!["port", "network", "service", "socket"],
            icon: "network",
            input_placeholder: "Enter port number (e.g., 80) or service name (e.g., http)...",
            output_placeholder: "Port information will appear here",
            primary_action: "Lookup",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "category",
                    label: "Category",
                    help: Some("Filter by service category"),
                    kind: OptionKind::Picker {
                        choices: vec![
                            "All",
                            "Web",
                            "Development",
                            "Database",
                            "Mail",
                            "File Transfer",
                            "Remote Access",
                            "Network",
                            "Messaging",
                            "DevOps",
                            "Monitoring",
                        ],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "showCommon",
                    label: "Show common ports",
                    help: Some("Show list of commonly used ports"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(process),
    }
}

fn range(port: u16) -> &'static str {
    match port {
        0..=1023 => "Well-known port (0-1023) - requires root/admin",
        1024..=49151 => "Registered port (1024-49151)",
        _ => "Dynamic/Ephemeral port (49152-65535)",
    }
}

fn process(req: &Request) -> ToolResult {
    let query = req.input.trim().to_lowercase();
    let filter = req.options.str("category");
    let in_category = |port: &Port| filter == "All" || port.category == filter;

    if !req.options.bool("showCommon") {
        if let Ok(number) = query.parse::<u16>() {
            let found: Vec<&Port> = PORTS.iter().filter(|p| p.number == number).collect();
            if found.is_empty() {
                return Ok(ToolOutput::multiple(vec![
                    OutputItem::new("Port", number.to_string()),
                    OutputItem::new("Service", "Unknown/Unregistered"),
                    OutputItem::new("Range", range(number)),
                ]));
            }
            let mut items = vec![OutputItem::new("Port", number.to_string())];
            for port in found {
                items.push(OutputItem::new("Service", port.service));
                items.push(OutputItem::new("Protocol", port.protocol));
                items.push(OutputItem::new("Category", port.category));
                items.push(OutputItem::new("Description", port.description));
            }
            items.push(OutputItem::new("Range", range(number)));
            return Ok(ToolOutput::multiple(items));
        }
    }

    let matches: Vec<&Port> = PORTS
        .iter()
        .filter(|p| in_category(p))
        .filter(|p| {
            req.options.bool("showCommon")
                || query.is_empty()
                || p.service.to_lowercase().contains(&query)
                || p.description.to_lowercase().contains(&query)
                || p.category.to_lowercase() == query
        })
        .collect();
    if matches.is_empty() {
        return Err(ToolError::InvalidInput(format!(
            "No ports found for '{}'",
            req.input.trim()
        )));
    }
    let show_common = req.options.bool("showCommon");
    let limit = if show_common {
        matches.len()
    } else {
        LIST_LIMIT
    };
    let mut lines: Vec<String> = matches
        .iter()
        .take(limit)
        .map(|p| {
            format!(
                "{:<6} {:<14} {:<8} {}",
                p.number, p.service, p.protocol, p.description
            )
        })
        .collect();
    if matches.len() > limit {
        lines.push(format!(
            "\nShowing first {limit} of {} matches",
            matches.len()
        ));
    }
    Ok(ToolOutput::text(lines.join("\n")))
}
