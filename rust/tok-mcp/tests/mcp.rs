//! The MCP server driven as a client drives it: JSON-RPC messages in both eras of the
//! protocol, every tool, its argument checks and its errors.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tok_mcp::Server;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn file(rel: &str) -> String {
    root().join(rel).display().to_string()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect("readable")
}

const MODEL: &str = "tok 0.1 \"Zgłoszenie\"\nstart s \"Początek\"\nuser a \"Rozpatrzenie\" duration=\"exp(1h)\"\nend e\nflow s -> a -> e\nsimulate cases=20 seed=3\n";

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

/// A client of the modern era: every request with its metadata.
struct Modern {
    server: Server,
    next: i64,
}

impl Modern {
    fn new() -> Self {
        Modern {
            server: Server::new(),
            next: 0,
        }
    }

    fn raw(&mut self, method: &str, mut params: Value) -> Value {
        self.next += 1;
        if params.get("_meta").is_none() {
            params["_meta"] = meta();
        }
        let line = json!({"jsonrpc": "2.0", "id": self.next, "method": method, "params": params})
            .to_string();
        let r = self.server.handle_text(&line).expect("a response");
        assert_eq!(r["id"], self.next);
        r
    }

    fn ok(&mut self, method: &str, params: Value) -> Value {
        let r = self.raw(method, params);
        assert!(r.get("error").is_none(), "{method}: {r}");
        assert_eq!(r["result"]["resultType"], "complete");
        assert_eq!(
            r["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
            "tok-mcp"
        );
        r["result"].clone()
    }

    /// A successful tool call: its structured content.
    fn tool(&mut self, name: &str, args: Value) -> Value {
        let r = self.ok("tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(r["isError"], false, "{name}: {r}");
        let structured = r["structuredContent"].clone();
        assert!(r["content"][0]["text"].is_string());
        conforms(name, &structured);
        structured
    }

    /// A tool call that fails as a tool error: its message.
    fn tool_error(&mut self, name: &str, args: Value) -> String {
        let r = self.ok("tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(r["isError"], true, "{name}: {r}");
        assert!(r.get("structuredContent").is_none());
        r["content"][0]["text"].as_str().expect("text").to_string()
    }
}

/// The required members of the output schema of a tool are in its structured content.
fn conforms(tool: &str, structured: &Value) {
    let defs = Modern::new().ok("tools/list", json!({}));
    let def = defs["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|d| d["name"] == tool)
        .cloned()
        .expect("defined");
    for r in def["outputSchema"]["required"]
        .as_array()
        .expect("required")
    {
        let key = r.as_str().expect("name");
        assert!(
            structured.get(key).is_some(),
            "{tool}: no {key} in {structured}"
        );
    }
    let props = def["outputSchema"]["properties"]
        .as_object()
        .expect("properties");
    for key in structured.as_object().expect("object").keys() {
        assert!(
            props.contains_key(key),
            "{tool}: {key} not in the output schema"
        );
    }
}

#[test]
fn discover_lists_versions_and_capabilities() {
    let mut c = Modern::new();
    let r = c.ok("server/discover", json!({}));
    assert_eq!(
        r["supportedVersions"],
        json!([
            "2026-07-28",
            "2025-11-25",
            "2025-06-18",
            "2025-03-26",
            "2024-11-05"
        ])
    );
    assert!(r["capabilities"]["tools"].is_object());
    assert!(r["capabilities"]["resources"].is_object());
    assert!(r["instructions"].as_str().expect("text").contains("check"));
    assert_eq!(
        r["_meta"]["io.modelcontextprotocol/serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn modern_requests_need_their_metadata() {
    let mut c = Modern::new();
    let r = c.raw("tools/list", json!({"_meta": {"io.modelcontextprotocol/protocolVersion": "1900-01-01", "io.modelcontextprotocol/clientCapabilities": {}}}));
    assert_eq!(r["error"]["code"], -32022);
    assert_eq!(r["error"]["data"]["requested"], "1900-01-01");
    assert_eq!(r["error"]["data"]["supported"][0], "2026-07-28");
    let r = c.raw(
        "tools/list",
        json!({"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28"}}),
    );
    assert_eq!(r["error"]["code"], -32602);
    // no metadata and no handshake
    let mut s = Server::new();
    let r = s
        .handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        .expect("response");
    assert_eq!(r["error"]["code"], -32602);
    let r = c.raw("prompts/list", json!({}));
    assert_eq!(r["error"]["code"], -32601);
    let r = s.handle_text("{oops").expect("response");
    assert_eq!(r["error"]["code"], -32700);
    // notifications get no response
    assert!(
        s.handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
            .is_none()
    );
}

#[test]
fn legacy_clients_negotiate_with_initialize() {
    for (asked, given) in [
        ("2025-06-18", "2025-06-18"),
        ("2025-11-25", "2025-11-25"),
        ("2024-11-05", "2024-11-05"),
        ("2099-01-01", "2025-11-25"),
    ] {
        let mut s = Server::new();
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": asked, "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}}))
            .expect("response");
        assert_eq!(r["result"]["protocolVersion"], given);
        assert_eq!(r["result"]["serverInfo"]["name"], "tok-mcp");
        assert!(r["result"].get("resultType").is_none());
        assert!(
            s.handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
                .is_none()
        );
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "fmt", "arguments": {"text": "tok 0.1\nstart  s\n"}}}))
            .expect("response");
        let result = &r["result"];
        assert_eq!(result["content"][0]["text"], "tok 0.1\n\nstart s\n");
        // structured content exists from 2025-06-18
        assert_eq!(
            result.get("structuredContent").is_some(),
            given >= "2025-06-18",
            "{given}"
        );
        let r = s
            .handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}))
            .expect("response");
        assert_eq!(r["result"], json!({}));
    }
}

#[test]
fn tools_have_exact_schemas() {
    let mut c = Modern::new();
    let r = c.ok("tools/list", json!({}));
    let names: Vec<&str> = r["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert_eq!(
        names,
        vec![
            "check",
            "fmt",
            "export",
            "import",
            "sim",
            "eval",
            "reference"
        ]
    );
    for t in r["tools"].as_array().expect("tools") {
        let s = &t["inputSchema"];
        assert_eq!(s["type"], "object");
        assert_eq!(s["additionalProperties"], false, "{}", t["name"]);
        assert_eq!(t["outputSchema"]["type"], "object");
        assert!(t["description"].as_str().expect("description").len() > 40);
        assert_eq!(t["annotations"]["readOnlyHint"], true);
    }
}

#[test]
fn check_reports_diagnostics() {
    let mut c = Modern::new();
    let r = c.tool("check", json!({"text": MODEL}));
    assert_eq!(r["errors"], 0);
    assert_eq!(r["bpmn_class"], "descriptive");
    let r = c.tool(
        "check",
        json!({"text": "tok 0.1\nstart s\nflow s -> x\n", "lang": "pl"}),
    );
    assert_eq!(r["errors"], 1);
    assert_eq!(r["diagnostics"][0]["code"], "R002");
    assert_eq!(r["diagnostics"][0]["line"], 3);
    assert!(
        r["diagnostics"][0]["message"]
            .as_str()
            .expect("message")
            .starts_with("nieznany")
    );
    let r = c.ok(
        "tools/call",
        json!({"name": "check", "arguments": {"text": "tok 0.1\nstart s\nflow s -> x\n"}}),
    );
    assert!(
        r["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("line 3: error R002")
    );
    let r = c.tool(
        "check",
        json!({"path": file("examples/wypozyczenie.tok"), "class": "descriptive", "level": "full"}),
    );
    // a send task and a message flow with a message belong to the class analytic
    assert_eq!(r["errors"], 2);
    assert_eq!(r["diagnostics"][0]["code"], "R075");
}

#[test]
fn fmt_prints_the_canonical_form_and_refuses_syntax_errors() {
    let mut c = Modern::new();
    let expected = read("conformance/expected/02-linear.tok").replace("\r\n", "\n");
    let r = c.tool(
        "fmt",
        json!({"path": file("conformance/cases/02-linear.tok")}),
    );
    assert_eq!(r["text"], expected);
    let e = c.tool_error("fmt", json!({"text": "tok 0.1\nstart s \"x\n"}));
    assert!(
        e.contains("the document has errors") && e.contains("T001"),
        "{e}"
    );
}

#[test]
fn export_returns_the_files() {
    let mut c = Modern::new();
    let r = c.tool(
        "export",
        json!({"path": file("examples/propozycja-zakupu.tok")}),
    );
    let mut names: Vec<&String> = r["files"].as_object().expect("files").keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "f-akceptacja.form",
            "f-propozycja.form",
            "f-zamowienie.form",
            "propozycja-zakupu.bpmn",
            "propozycja-zakupu.svg",
            "tryb-akceptacji.dmn"
        ]
    );
    assert!(
        r["files"]["propozycja-zakupu.bpmn"]
            .as_str()
            .expect("xml")
            .contains("<bpmndi:BPMNDiagram")
    );
    let r = c.tool(
        "export",
        json!({"text": MODEL, "formats": ["svg"], "name": "x"}),
    );
    let names: Vec<&String> = r["files"].as_object().expect("files").keys().collect();
    assert_eq!(names, vec!["x.svg"]);
    let r = c.tool(
        "export",
        json!({"path": file("conformance/cases/53-drd.tok"), "formats": ["dmn"]}),
    );
    let names: Vec<&String> = r["files"].as_object().expect("files").keys().collect();
    assert!(
        names
            .iter()
            .all(|n| n.ends_with(".dmn") || n.ends_with(".drd.svg")),
        "{names:?}"
    );
    assert!(names.iter().any(|n| n.ends_with(".drd.svg")));
    let e = c.tool_error("export", json!({"text": "tok 0.1\nstart s\nflow s -> x\n"}));
    assert!(e.contains("R002"), "{e}");
}

#[test]
fn import_converts_bpmn_and_dmn() {
    let mut c = Modern::new();
    let r = c.tool(
        "import",
        json!({"path": file("conformance/import/01-wypozyczenie-pierwszy-model.bpmn")}),
    );
    assert_eq!(r["format"], "bpmn");
    assert!(r["text"].as_str().expect("text").starts_with("tok 0."));
    assert!(
        r["layout"]
            .as_str()
            .expect("sidecar")
            .starts_with("tok-layout 1")
    );
    let bpmn = c.tool("export", json!({"text": MODEL, "formats": ["bpmn"]}));
    let xml = bpmn["files"]["model.bpmn"].as_str().expect("xml");
    let r = c.tool(
        "import",
        json!({"xml": xml, "layout": false, "flow_ids": false}),
    );
    assert_eq!(r["layout"], Value::Null);
    assert!(
        r["text"]
            .as_str()
            .expect("text")
            .contains("flow s -> a -> e")
    );
    let dmn = c.tool(
        "export",
        json!({"path": file("conformance/cases/53-drd.tok"), "formats": ["dmn"]}),
    );
    let (_, xml) = dmn["files"]
        .as_object()
        .expect("files")
        .iter()
        .find(|(n, _)| n.ends_with(".dmn"))
        .expect("a dmn file");
    let r = c.tool("import", json!({"xml": xml}));
    assert_eq!(r["format"], "dmn");
    assert!(r["text"].as_str().expect("text").starts_with("tok 0.7"));
    let e = c.tool_error("import", json!({"xml": "<not-bpmn/>"}));
    assert!(e.contains("(import)"), "{e}");
}

#[test]
fn sim_runs_the_simulation() {
    let mut c = Modern::new();
    let a = c.tool("sim", json!({"text": MODEL}));
    let b = c.tool("sim", json!({"text": MODEL}));
    assert_eq!(a, b);
    assert!(a["report"].as_str().expect("report").contains("20"));
    assert!(a.get("log").is_none() && a.get("cases").is_none());
    let r = c.tool(
        "sim",
        json!({"text": MODEL, "cases": 5, "seed": 1, "log": true, "xes": true}),
    );
    assert!(r["log"].as_array().expect("log").len() >= 5);
    assert!(r["xes"].as_str().expect("xes").contains("<log"));
    let e = c.tool_error("sim", json!({"text": MODEL, "cases": 0}));
    assert!(e.contains("at least 1"), "{e}");
}

#[test]
fn eval_evaluates_decisions() {
    let mut c = Modern::new();
    let r = c.tool("eval", json!({"path": file("examples/propozycja-zakupu.tok"), "decision": "tryb-akceptacji", "values": {"rodzaj": "e-book", "cena": 120}}));
    assert_eq!(r["value"], json!({"tryb": "kierownik"}));
    let r = c.tool("eval", json!({"path": file("conformance/cases/53-drd.tok"), "decision": "limit", "values": {"wiek": 15, "konto": "student"}}));
    assert!(r["value"].is_number(), "{r}");
    let e = c.tool_error("eval", json!({"text": MODEL, "decision": "nie-ma"}));
    assert!(e.contains("unknown decision"), "{e}");
}

#[test]
fn the_reference_is_a_tool_and_resources() {
    let mut c = Modern::new();
    let r = c.tool("reference", json!({}));
    assert!(r["text"].as_str().expect("text").contains("tok"));
    let r = c.tool("reference", json!({"topic": "spec"}));
    assert!(
        r["text"]
            .as_str()
            .expect("text")
            .starts_with("# tok — Language Specification")
    );
    let r = c.ok("resources/list", json!({}));
    let uris: Vec<&str> = r["resources"]
        .as_array()
        .expect("list")
        .iter()
        .map(|x| x["uri"].as_str().expect("uri"))
        .collect();
    assert_eq!(
        uris,
        vec![
            "tok://reference/cheatsheet",
            "tok://reference/spec",
            "tok://reference/diagnostics",
            "tok://reference/feel",
            "tok://reference/cli"
        ]
    );
    let r = c.ok(
        "resources/read",
        json!({"uri": "tok://reference/diagnostics"}),
    );
    assert_eq!(r["contents"][0]["mimeType"], "text/markdown");
    assert!(
        r["contents"][0]["text"]
            .as_str()
            .expect("text")
            .starts_with("# Diagnostics")
    );
    let r = c.raw("resources/read", json!({"uri": "tok://reference/nothing"}));
    assert_eq!(r["error"]["code"], -32602);
}

#[test]
fn arguments_are_checked_as_tool_errors() {
    let mut c = Modern::new();
    let e = c.tool_error("check", json!({"txt": MODEL}));
    assert!(e.starts_with("unknown argument 'txt'"), "{e}");
    let e = c.tool_error("check", json!({"text": 3}));
    assert_eq!(e, "argument 'text' must be of type string");
    let e = c.tool_error("check", json!({"text": MODEL, "level": "expert"}));
    assert_eq!(e, "argument 'level' must be one of basic, analytic, full");
    let e = c.tool_error("eval", json!({"text": MODEL}));
    assert_eq!(e, "missing required argument 'decision'");
    let e = c.tool_error("fmt", json!({"text": MODEL, "path": "x.tok"}));
    assert_eq!(e, "give either text or path, not both");
    let e = c.tool_error("fmt", json!({}));
    assert_eq!(e, "give the document as text or its file as path");
    let e = c.tool_error("fmt", json!({"path": file("nonexistent/none.tok")}));
    assert!(e.starts_with("cannot read"), "{e}");
    let e = c.tool_error("export", json!({"text": MODEL, "formats": ["svg", "svg"]}));
    assert!(e.contains("repeats"), "{e}");
    let r = c.raw("tools/call", json!({"name": "draw", "arguments": {}}));
    assert_eq!(r["error"]["code"], -32602);
    assert_eq!(r["error"]["message"], "unknown tool 'draw'");
}

#[test]
fn the_stdio_loop_answers_line_by_line() {
    let lines = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}}).to_string(),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string(),
        String::new(),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "check", "arguments": {"text": "tok 0.1 \"Ćma\"\nstart s\n"}}}).to_string(),
    ];
    let input = lines.join("\n") + "\n";
    let mut out = Vec::new();
    assert_eq!(tok_mcp::run(input.as_bytes(), &mut out), 0);
    let text = String::from_utf8(out).expect("UTF-8");
    let replies: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).expect("one JSON value per line"))
        .collect();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[1]["result"]["structuredContent"]["errors"], 0);
}

#[test]
fn the_program_serves_a_client_over_its_standard_streams() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_tok-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("tok-mcp starts");
    let request = json!({"jsonrpc": "2.0", "id": 7, "method": "server/discover", "params": {"_meta": meta()}});
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(format!("{request}\n").as_bytes())
        .expect("written");
    let out = child.wait_with_output().expect("tok-mcp ends");
    assert!(out.status.success());
    let reply: Value = serde_json::from_slice(&out.stdout).expect("one JSON line");
    assert_eq!(reply["id"], 7);
    assert_eq!(reply["result"]["supportedVersions"][0], "2026-07-28");
}
