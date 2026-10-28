//! Rename over the conformance corpus and the examples: renaming an identifier and
//! checking the result again must give the same diagnostic codes, so every reference the
//! checker resolves is found and nothing else is changed.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tok_lsp::Server;

const URI: &str = "file:///nonexistent/case.tok";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn documents() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for dir in ["conformance/cases", "examples"] {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(root().join(dir))
            .expect("folder exists")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "tok"))
            .collect();
        paths.sort();
        for p in paths {
            let text = std::fs::read_to_string(&p)
                .expect("readable")
                .replace("\r\n", "\n");
            out.push((p.display().to_string(), text));
        }
    }
    out
}

struct Session {
    server: Server,
    next: i64,
    /// Errors among the diagnostics of the last document opened.
    errors: usize,
}

impl Session {
    fn new() -> Self {
        let mut s = Session {
            server: Server::new(),
            next: 0,
            errors: 0,
        };
        s.call("initialize", json!({"capabilities": {}}));
        s
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        let out = self
            .server
            .handle(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        out.into_iter()
            .find(|m| m["id"] == json!(id))
            .expect("a response")
    }

    fn codes(&mut self, text: &str) -> Vec<String> {
        let out = self.server.handle(&json!({"jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": URI, "languageId": "tok", "version": 1, "text": text}}}));
        let diags = out[0]["params"]["diagnostics"]
            .as_array()
            .expect("diagnostics");
        self.errors = diags.iter().filter(|d| d["severity"] == 1).count();
        let mut codes: Vec<String> = diags
            .iter()
            .map(|d| d["code"].as_str().unwrap_or_default().to_string())
            .collect();
        codes.sort();
        codes
    }
}

/// Apply the edits of one document (UTF-16 ranges within single lines).
fn apply(text: &str, edits: &[Value]) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let mut edits = edits.to_vec();
    edits.sort_by_key(|e| {
        std::cmp::Reverse((
            e["range"]["start"]["line"].as_u64(),
            e["range"]["start"]["character"].as_u64(),
        ))
    });
    for e in &edits {
        let l = e["range"]["start"]["line"].as_u64().unwrap_or(0) as usize;
        let s = e["range"]["start"]["character"].as_u64().unwrap_or(0) as usize;
        let t = e["range"]["end"]["character"].as_u64().unwrap_or(0) as usize;
        let units: Vec<u16> = lines[l].encode_utf16().collect();
        let mut out = String::from_utf16(&units[..s]).expect("valid UTF-16 boundary");
        out.push_str(e["newText"].as_str().unwrap_or_default());
        out.push_str(&String::from_utf16(&units[t..]).expect("valid UTF-16 boundary"));
        lines[l] = out;
    }
    lines.join("\n")
}

#[test]
fn renaming_keeps_the_diagnostics() {
    let mut renamed = 0;
    for (name, text) in documents() {
        let mut s = Session::new();
        let before = s.codes(&text);
        if s.errors > 0 {
            // a document with errors may use a name ambiguously; rename keeps valid
            // documents valid
            continue;
        }
        // the declarations of the document, from the outline
        let symbols = s.call(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": URI}}),
        );
        let mut stack: Vec<Value> = symbols["result"].as_array().cloned().unwrap_or_default();
        let mut targets = Vec::new();
        while let Some(sym) = stack.pop() {
            targets.push(sym["selectionRange"]["start"].clone());
            stack.extend(sym["children"].as_array().cloned().unwrap_or_default());
        }
        for start in targets {
            let params = json!({"textDocument": {"uri": URI}, "position": start});
            let prep = s.call("textDocument/prepareRename", params.clone());
            let Some(old) = prep["result"]["placeholder"].as_str().map(str::to_string) else {
                continue;
            };
            let mut p = params.clone();
            p["newName"] = json!("zz-renamed");
            let r = s.call("textDocument/rename", p);
            let Some(edits) = r["result"]["changes"][URI].as_array().cloned() else {
                panic!("{name}: rename of {old} failed: {r}");
            };
            let changed = apply(&text, &edits);
            let after = s.codes(&changed);
            assert_eq!(before, after, "{name}: rename of {old}\n{changed}");
            // the document is back for the next rename
            s.codes(&text);
            renamed += 1;
        }
    }
    assert!(renamed > 400, "only {renamed} renames");
}
