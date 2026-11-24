// Rust, src/bin/shodan-mcp.rs, rust 2024, цель: MCP-сервер (JSON-RPC 2.0 over stdio) для Shodan
use std::io::{self, BufRead, Write};

use anyhow::Result;
use serde_json::{Value, json};

use shodan_rs::{client, output, parse};

const PROTOCOL_VERSION: &str = "2024-11-05";

fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    eprintln!("[shodan-mcp] listening on stdio");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[shodan-mcp] stdin: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {"code": -32700, "message": format!("parse error: {e}")}
                });
                writeln!(stdout, "{err}")?;
                stdout.flush()?;
                continue;
            }
        };

        if let Some(resp) = handle(msg) {
            writeln!(stdout, "{resp}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

fn handle(msg: Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "shodan-mcp", "version": env!("CARGO_PKG_VERSION")}
        })),
        "notifications/initialized" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => call_tool(&params),
        _ => Err((-32601, format!("method not found: {method}"))),
    };

    let id = id?;
    Some(match result {
        Ok(value) => json!({"jsonrpc": "2.0", "id": id, "result": value}),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "shodan_search",
            "description": "Search Shodan (first page) and return host cards as JSON: ip, port, scheme, title, hostnames, org, country, city, tags, timestamp, banner. Falls back to the facet IP list if the anonymous search limit is hit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Shodan query, e.g. 'ollama' or 'port:11434 product:\"Ollama\"'"},
                    "cookie": {"type": "string", "description": "Optional Shodan session cookie"},
                    "proxy": {"type": "string", "description": "Optional HTTP proxy, e.g. http://127.0.0.1:8080"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "shodan_facet",
            "description": "Return a list of IPs from the Shodan facet endpoint as JSON [{ip, count}]. Works with filters like port:/product: and is not hit by the anonymous search limit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Shodan query, e.g. 'port:11434 product:\"Ollama\"'"},
                    "cookie": {"type": "string", "description": "Optional Shodan session cookie"},
                    "proxy": {"type": "string", "description": "Optional HTTP proxy, e.g. http://127.0.0.1:8080"}
                },
                "required": ["query"]
            }
        }
    ])
}

fn call_tool(params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".to_string()))?;
    if !matches!(name, "shodan_search" | "shodan_facet") {
        return Err((-32602, format!("unknown tool: {name}")));
    }

    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if query.is_empty() {
        return Err((-32602, "missing 'query' argument".to_string()));
    }
    let cookie = args.get("cookie").and_then(Value::as_str);
    let proxy = args.get("proxy").and_then(Value::as_str);

    let result = match name {
        "shodan_search" => run_search(&query, cookie, proxy),
        "shodan_facet" => run_facet(&query, cookie, proxy),
        _ => unreachable!("tool name checked above"),
    };

    Ok(match result {
        Ok(text) => json!({"content": [{"type": "text", "text": text}], "isError": false}),
        Err(e) => json!({
            "content": [{"type": "text", "text": format!("error: {e:#}")}],
            "isError": true
        }),
    })
}

fn run_search(query: &str, cookie: Option<&str>, proxy: Option<&str>) -> anyhow::Result<String> {
    let client = client::build_client(proxy)?;
    let body = client::fetch_page(&client, query, cookie)?;
    if body.contains("Search usage limit reached") {
        let body = client::fetch_facet(&client, query, cookie)?;
        return output::facet_json(&parse::parse_facet(&body));
    }
    output::to_json(&parse::parse_page(&body))
}

fn run_facet(query: &str, cookie: Option<&str>, proxy: Option<&str>) -> anyhow::Result<String> {
    let client = client::build_client(proxy)?;
    let body = client::fetch_facet(&client, query, cookie)?;
    output::facet_json(&parse::parse_facet(&body))
}
