//! MCP server of the tok process language, on the stdio transport: newline-delimited
//! JSON-RPC 2.0 messages on standard input and output.
//!
//! The server offers the tools `check`, `fmt`, `export`, `import`, `sim`, `eval` and
//! `reference`, and the language reference as resources `tok://reference/TOPIC`. It speaks
//! both eras of the Model Context Protocol:
//!
//! - **modern** (revision 2026-07-28): every request carries its protocol version and the
//!   client capabilities in `_meta`; there is no handshake, `server/discover` describes the
//!   server, and results carry `resultType`;
//! - **legacy** (revisions 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05): the client
//!   opens with `initialize`, and the negotiated revision holds for the process.
//!
//! The tools call the same functions as the C interface ([`tok_cli::api`]), so they give
//! the results of the `tok` command. `docs/mcp.md` documents the tools and the
//! configuration of MCP clients.

mod tools;

use std::io::{BufRead, Write};

use serde_json::{Value, json};

pub use tools::REFERENCE;

/// Protocol revisions with per-request metadata, newest first.
pub const MODERN_VERSIONS: &[&str] = &["2026-07-28"];
/// Protocol revisions with the `initialize` handshake, newest first.
pub const LEGACY_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
const META_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
const META_SERVER: &str = "io.modelcontextprotocol/serverInfo";

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const UNSUPPORTED_VERSION: i64 = -32022;

const INSTRUCTIONS: &str = "This server checks, formats, exports, imports, simulates and evaluates process models written in tok, a plain-text language for BPMN 2.0 processes with DMN decisions, Camunda forms and simulation parameters. Before writing a model, read the one-page language reference (tool `reference` or resource tok://reference/cheatsheet). Write the .tok text, call `check` and fix every error until there are none, call `fmt` for the canonical form, then `export` for BPMN XML, SVG, DMN and form files. Never write BPMN XML or diagram coordinates by hand: `export` produces them with an automatic layout.";

fn server_info() -> Value {
    json!({"name": "tok-mcp", "title": "tok", "version": env!("CARGO_PKG_VERSION")})
}

fn capabilities() -> Value {
    json!({"tools": {}, "resources": {}})
}

/// A failed request: JSON-RPC code, message and optional data.
struct Failure(i64, String, Option<Value>);

fn error(code: i64, message: impl Into<String>) -> Failure {
    Failure(code, message.into(), None)
}

/// The MCP server: the legacy revision negotiated by `initialize`, if any. Feed it
/// messages with [`Server::handle`].
#[derive(Debug, Default)]
pub struct Server {
    legacy: Option<String>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle one line of JSON; returns the response, or `None` for a notification.
    pub fn handle_text(&mut self, line: &str) -> Option<Value> {
        match serde_json::from_str::<Value>(line) {
            Ok(message) => self.handle(&message),
            Err(e) => Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {"code": PARSE_ERROR, "message": format!("invalid JSON: {e}")},
            })),
        }
    }

    /// Handle one message; returns the response, or `None` for a notification or a
    /// response.
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        let id = message.get("id")?;
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // a response to a request of the server: the server sends none
            return None;
        };
        if id.is_null() {
            return Some(respond(
                id,
                Err(error(INVALID_REQUEST, "the request id must not be null")),
            ));
        }
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        Some(respond(id, self.request(method, &params)))
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<Value, Failure> {
        if method == "initialize" {
            return Ok(self.initialize(params));
        }
        let meta = params.get("_meta");
        let version = meta.and_then(|m| m.get(META_VERSION));
        let modern = match version {
            Some(Value::String(v)) if MODERN_VERSIONS.contains(&v.as_str()) => {
                if meta.and_then(|m| m.get(META_CAPABILITIES)).is_none() {
                    return Err(error(
                        INVALID_PARAMS,
                        format!("_meta must include {META_CAPABILITIES}"),
                    ));
                }
                true
            }
            Some(v) => {
                let supported: Vec<&str> = MODERN_VERSIONS
                    .iter()
                    .chain(LEGACY_VERSIONS)
                    .copied()
                    .collect();
                return Err(Failure(
                    UNSUPPORTED_VERSION,
                    "Unsupported protocol version".into(),
                    Some(json!({"supported": supported, "requested": v})),
                ));
            }
            None if self.legacy.is_some() || method == "ping" => false,
            None => {
                return Err(error(
                    INVALID_PARAMS,
                    format!(
                        "the request has no {META_VERSION} in _meta; send it with every request \
                         (protocol {}) or open the session with initialize",
                        MODERN_VERSIONS[0]
                    ),
                ));
            }
        };
        let mut result = match method {
            "server/discover" => json!({
                "supportedVersions": MODERN_VERSIONS.iter().chain(LEGACY_VERSIONS).collect::<Vec<_>>(),
                "capabilities": capabilities(),
                "instructions": INSTRUCTIONS,
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": tools::definitions()}),
            "tools/call" => self.call_tool(params, modern)?,
            "resources/list" => json!({"resources": resources()}),
            "resources/templates/list" => json!({"resourceTemplates": []}),
            "resources/read" => read_resource(params)?,
            _ => {
                return Err(error(
                    METHOD_NOT_FOUND,
                    format!("unknown method '{method}'"),
                ));
            }
        };
        if modern && let Value::Object(map) = &mut result {
            map.insert("resultType".into(), json!("complete"));
            map.insert("_meta".into(), json!({META_SERVER: server_info()}));
        }
        Ok(result)
    }

    fn initialize(&mut self, params: &Value) -> Value {
        let requested = params.get("protocolVersion").and_then(Value::as_str);
        let version = requested
            .filter(|v| LEGACY_VERSIONS.contains(v))
            .unwrap_or(LEGACY_VERSIONS[0])
            .to_string();
        self.legacy = Some(version.clone());
        json!({
            "protocolVersion": version,
            "capabilities": capabilities(),
            "serverInfo": server_info(),
            "instructions": INSTRUCTIONS,
        })
    }

    fn call_tool(&self, params: &Value, modern: bool) -> Result<Value, Failure> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| error(INVALID_PARAMS, "tools/call needs the name of a tool"))?;
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);
        let outcome = tools::run(name, &args).map_err(|e| error(INVALID_PARAMS, e))?;
        // structured content arrived with revision 2025-06-18
        let structured_ok = modern || self.legacy.as_deref().is_some_and(|v| v >= "2025-06-18");
        Ok(match outcome {
            tools::Outcome::Done { text, structured } => {
                let mut r = json!({"content": [{"type": "text", "text": text}], "isError": false});
                if structured_ok {
                    r["structuredContent"] = structured;
                }
                r
            }
            tools::Outcome::Failed(message) => {
                json!({"content": [{"type": "text", "text": message}], "isError": true})
            }
        })
    }
}

fn respond(id: &Value, result: Result<Value, Failure>) -> Value {
    match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(Failure(code, message, data)) => {
            let mut e = json!({"code": code, "message": message});
            if let Some(d) = data {
                e["data"] = d;
            }
            json!({"jsonrpc": "2.0", "id": id, "error": e})
        }
    }
}

fn resources() -> Vec<Value> {
    REFERENCE
        .iter()
        .map(|(topic, title, text)| {
            json!({
                "uri": format!("tok://reference/{topic}"),
                "name": topic,
                "title": title,
                "mimeType": "text/markdown",
                "size": text.len(),
            })
        })
        .collect()
}

fn read_resource(params: &Value) -> Result<Value, Failure> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| error(INVALID_PARAMS, "resources/read needs a uri"))?;
    let topic = uri.strip_prefix("tok://reference/").unwrap_or("");
    let (_, _, text) = REFERENCE
        .iter()
        .find(|(t, _, _)| *t == topic)
        .ok_or_else(|| {
            Failure(
                INVALID_PARAMS,
                format!("unknown resource '{uri}'"),
                Some(json!({"uri": uri})),
            )
        })?;
    Ok(json!({"contents": [{"uri": uri, "mimeType": "text/markdown", "text": text}]}))
}

/// Serve one client: read newline-delimited messages from `input` until its end and write
/// one line per response to `output`. Returns the exit status.
pub fn run(input: impl BufRead, mut output: impl Write) -> i32 {
    let mut server = Server::new();
    for line in input.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("tok-mcp: {e}");
                return 1;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = server.handle_text(&line) {
            // serde_json escapes line breaks, so a message is always one line
            if writeln!(output, "{reply}")
                .and_then(|()| output.flush())
                .is_err()
            {
                return 1;
            }
        }
    }
    0
}
