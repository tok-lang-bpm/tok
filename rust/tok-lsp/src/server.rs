//! The state of the server and the handling of each message.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Value, json};
use tok_core::diagnostics::{BPMN_EN, Diag, MESSAGES, Severity};
use tok_core::lexer::Kind;
use tok_core::parser::is_id;
use tok_core::{classes, printer};

use crate::analysis::{Analysis, Settings, normalize};
use crate::index::{self, REF_KEYS};
use crate::keywords;
use crate::text::position;

// JSON-RPC and LSP error codes
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const SERVER_NOT_INITIALIZED: i64 = -32002;
const REQUEST_FAILED: i64 = -32803;

/// An open document and its latest analysis.
struct Doc {
    text: String,
    version: Value,
    analysis: Analysis,
}

/// The language server: open documents and settings. Feed it messages with
/// [`Server::handle`]; it returns the responses and notifications to send.
pub struct Server {
    docs: HashMap<String, Doc>,
    settings: Settings,
    initialized: bool,
    shut_down: bool,
    exit: Option<i32>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

/// A failed request: a JSON-RPC error code and a message.
struct Failure(i64, String);

type Reply = Result<Value, Failure>;

fn failed(message: impl Into<String>) -> Failure {
    Failure(REQUEST_FAILED, message.into())
}

fn invalid(message: impl Into<String>) -> Failure {
    Failure(INVALID_PARAMS, message.into())
}

fn response(id: &Value, reply: Reply) -> Value {
    match reply {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(Failure(code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    }
}

fn notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

fn uri_of(params: &Value) -> Option<&str> {
    params.get("textDocument")?.get("uri")?.as_str()
}

/// The local path of a `file:` URI (percent-decoded; `file:///c%3A/x` is `c:/x`).
pub fn file_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len() + 1
            && let Some(hex) = rest.get(i + 1..i + 3)
            && let Ok(b) = u8::from_str_radix(hex, 16)
        {
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let path = String::from_utf8(out).ok()?;
    // `/c:/x` on Windows
    let b = path.as_bytes();
    let path = if b.len() > 2 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        path[1..].to_string()
    } else {
        path
    };
    Some(PathBuf::from(path))
}

/// The explanation of a diagnostic code: its layer, severity and message templates.
fn explain(code: &str) -> Option<String> {
    if let Some((_, sev, en, pl)) = MESSAGES.iter().find(|e| e.0 == code) {
        return Some(format!(
            "**{code}** ({}) — {}\n\n{en}\n\n*pl:* {pl}",
            sev.as_str(),
            keywords::layer(code)
        ));
    }
    BPMN_EN
        .iter()
        .find(|e| e.0 == code)
        .map(|(_, en)| format!("**{code}** — {}\n\n{en}", keywords::layer(code)))
}

/// The derived identifier `id` of a flow (`f-SOURCE-TARGET`) or message flow
/// (`m-SOURCE-TARGET`), with `-2`, `-3`… on repeated pairs (spec §6.1), after renaming
/// `old` to `new`; `None` when `id` is not such an identifier or does not involve `old`.
fn rederive(doc: &tok_core::Document, id: &str, old: &str, new: &str) -> Option<String> {
    let ends = doc
        .flows
        .iter()
        .find(|f| !f.explicit_id && f.base.id() == id)
        .map(|f| ("f", &f.source, &f.target))
        .or_else(|| {
            doc.messages
                .iter()
                .find(|m| !m.explicit_id && m.base.id() == id)
                .map(|m| ("m", &m.source, &m.target))
        })?;
    let (prefix, source, target) = ends;
    if source != old && target != old {
        return None;
    }
    let suffix = id.strip_prefix(&format!("{prefix}-{source}-{target}"))?;
    let swap = |s: &str| {
        if s == old {
            new.to_string()
        } else {
            s.to_string()
        }
    };
    Some(format!(
        "{prefix}-{}-{}{suffix}",
        swap(source),
        swap(target)
    ))
}

fn looks_like_code(word: &str) -> bool {
    let b = word.as_bytes();
    b.len() == 4 && b"TPRDCFSMEWL".contains(&b[0]) && b[1..].iter().all(u8::is_ascii_digit)
}

impl Server {
    pub fn new() -> Self {
        Server {
            docs: HashMap::new(),
            settings: Settings::default(),
            initialized: false,
            shut_down: false,
            exit: None,
        }
    }

    /// The exit status once the client has sent `exit`.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit
    }

    /// Whether the client has sent `shutdown`.
    pub fn is_shut_down(&self) -> bool {
        self.shut_down
    }

    /// Handle the JSON text of one message.
    pub fn handle_text(&mut self, body: &str) -> Vec<Value> {
        match serde_json::from_str::<Value>(body) {
            Ok(message) => self.handle(&message),
            Err(e) => vec![json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {"code": PARSE_ERROR, "message": format!("invalid JSON: {e}")},
            })],
        }
    }

    /// Handle one message; returns the messages to send to the client.
    pub fn handle(&mut self, message: &Value) -> Vec<Value> {
        let method = message.get("method").and_then(Value::as_str);
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        match (method, message.get("id")) {
            (Some(method), Some(id)) => self.request(id, method, &params),
            (Some(method), None) => self.notification(method, &params),
            // a response to a request of the server: the server sends none
            _ => Vec::new(),
        }
    }

    fn request(&mut self, id: &Value, method: &str, params: &Value) -> Vec<Value> {
        if method == "initialize" {
            return vec![response(id, Ok(self.initialize(params)))];
        }
        if !self.initialized {
            return vec![response(
                id,
                Err(Failure(
                    SERVER_NOT_INITIALIZED,
                    "the server is not initialized".into(),
                )),
            )];
        }
        if self.shut_down {
            return vec![response(
                id,
                Err(Failure(INVALID_REQUEST, "the server is shut down".into())),
            )];
        }
        let reply = match method {
            "shutdown" => {
                self.shut_down = true;
                Ok(Value::Null)
            }
            "textDocument/formatting" => self.formatting(params),
            "textDocument/definition" => self.definition(params),
            "textDocument/references" => self.references(params),
            "textDocument/documentHighlight" => self.highlight(params),
            "textDocument/prepareRename" => self.prepare_rename(params),
            "textDocument/rename" => self.rename(params),
            "textDocument/documentSymbol" => self.document_symbols(params),
            "textDocument/hover" => self.hover(params),
            "textDocument/completion" => self.completion(params),
            "tok/check" => self.check(params),
            "tok/svg" | "tok/bpmn" | "tok/export" => self.projection(method, params),
            _ => Err(Failure(
                METHOD_NOT_FOUND,
                format!("unknown method '{method}'"),
            )),
        };
        vec![response(id, reply)]
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<Value> {
        match method {
            "exit" => {
                self.exit = Some(if self.shut_down { 0 } else { 1 });
                Vec::new()
            }
            _ if !self.initialized => Vec::new(),
            "textDocument/didOpen" => {
                let item = params.get("textDocument");
                let uri = item.and_then(|i| i.get("uri")).and_then(Value::as_str);
                let text = item.and_then(|i| i.get("text")).and_then(Value::as_str);
                let version = item.and_then(|i| i.get("version")).cloned();
                match (uri, text) {
                    (Some(uri), Some(text)) => {
                        self.update(uri, text.to_string(), version.unwrap_or(Value::Null))
                    }
                    _ => Vec::new(),
                }
            }
            "textDocument/didChange" => {
                let Some(uri) = uri_of(params) else {
                    return Vec::new();
                };
                let version = params
                    .get("textDocument")
                    .and_then(|t| t.get("version"))
                    .cloned()
                    .unwrap_or(Value::Null);
                // full synchronization: the last change holds the whole text
                let text = params
                    .get("contentChanges")
                    .and_then(Value::as_array)
                    .and_then(|c| c.last())
                    .and_then(|c| c.get("text"))
                    .and_then(Value::as_str);
                match text {
                    Some(text) => self.update(uri, text.to_string(), version),
                    None => Vec::new(),
                }
            }
            "textDocument/didClose" => {
                let Some(uri) = uri_of(params) else {
                    return Vec::new();
                };
                self.docs.remove(uri);
                vec![notification(
                    "textDocument/publishDiagnostics",
                    json!({"uri": uri, "diagnostics": []}),
                )]
            }
            "workspace/didChangeConfiguration" => {
                let settings = params.get("settings").cloned().unwrap_or(Value::Null);
                let section = settings.get("tok").cloned().unwrap_or(settings);
                if self.configure(&section) {
                    self.reanalyse()
                } else {
                    Vec::new()
                }
            }
            // `initialized`, `didSave`, `$/cancelRequest`, `$/setTrace`: nothing to do
            _ => Vec::new(),
        }
    }

    /// Apply the settings `lang`, `level` and `class` of an object; returns whether they
    /// changed. Values that are not valid are ignored.
    fn configure(&mut self, v: &Value) -> bool {
        let before = self.settings.clone();
        if let Some(lang) = v.get("lang").and_then(Value::as_str)
            && matches!(lang, "en" | "pl")
        {
            self.settings.lang = lang.to_string();
        }
        if let Some(level) = v.get("level").and_then(Value::as_str)
            && matches!(level, "basic" | "analytic" | "full")
        {
            self.settings.level = level.to_string();
        }
        match v.get("class") {
            Some(Value::String(c)) if classes::CLASSES.contains(&c.as_str()) => {
                self.settings.class = Some(c.clone());
            }
            Some(Value::String(c)) if c.is_empty() || c == "none" => self.settings.class = None,
            Some(Value::Null) => self.settings.class = None,
            _ => {}
        }
        before != self.settings
    }

    fn initialize(&mut self, params: &Value) -> Value {
        self.initialized = true;
        if params
            .get("locale")
            .and_then(Value::as_str)
            .is_some_and(|l| l == "pl" || l.starts_with("pl-") || l.starts_with("pl_"))
        {
            self.settings.lang = "pl".into();
        }
        if let Some(options) = params.get("initializationOptions") {
            self.configure(options);
        }
        json!({
            "capabilities": {
                "positionEncoding": "utf-16",
                "textDocumentSync": {"openClose": true, "change": 1},
                "documentFormattingProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "documentHighlightProvider": true,
                "renameProvider": {"prepareProvider": true},
                "documentSymbolProvider": true,
                "hoverProvider": true,
                "completionProvider": {"triggerCharacters": ["=", ">"]},
            },
            "serverInfo": {"name": "tok-lsp", "version": env!("CARGO_PKG_VERSION")},
        })
    }

    fn publish(&self, uri: &str) -> Vec<Value> {
        let Some(doc) = self.docs.get(uri) else {
            return Vec::new();
        };
        let mut params = json!({
            "uri": uri,
            "diagnostics": doc.analysis.lsp_diagnostics(&self.settings.lang),
        });
        if doc.version.is_i64() || doc.version.is_u64() {
            params["version"] = doc.version.clone();
        }
        vec![notification("textDocument/publishDiagnostics", params)]
    }

    fn update(&mut self, uri: &str, text: String, version: Value) -> Vec<Value> {
        let analysis = Analysis::new(&text, &self.settings);
        self.docs.insert(
            uri.to_string(),
            Doc {
                text,
                version,
                analysis,
            },
        );
        self.publish(uri)
    }

    fn reanalyse(&mut self) -> Vec<Value> {
        let mut uris: Vec<String> = self.docs.keys().cloned().collect();
        uris.sort();
        let mut out = Vec::new();
        for uri in uris {
            if let Some(doc) = self.docs.get_mut(&uri) {
                doc.analysis = Analysis::new(&doc.text, &self.settings);
            }
            out.extend(self.publish(&uri));
        }
        out
    }

    fn doc(&self, params: &Value) -> Result<(&str, &Doc), Failure> {
        let uri = uri_of(params).ok_or_else(|| invalid("missing textDocument.uri"))?;
        let (uri, doc) = self
            .docs
            .get_key_value(uri)
            .ok_or_else(|| failed(format!("the document {uri} is not open")))?;
        Ok((uri.as_str(), doc))
    }

    /// The document and the identifier at the position of a request.
    fn target<'a>(
        &'a self,
        params: &Value,
    ) -> Result<(&'a str, &'a Doc, Option<&'a index::Occurrence>), Failure> {
        let (uri, doc) = self.doc(params)?;
        let (line, units) = position(params).ok_or_else(|| invalid("missing position"))?;
        let col = doc.analysis.text.column(line, units);
        Ok((uri, doc, index::at(&doc.analysis.occurrences, line, col)))
    }

    fn formatting(&self, params: &Value) -> Reply {
        let (_, doc) = self.doc(params)?;
        let normalized = normalize(&doc.text);
        let (parsed, diags) = tok_core::parse(&normalized);
        let errors: Vec<&Diag> = diags.iter().filter(|d| d.is_error()).collect();
        if let Some(first) = errors.first() {
            let lang = self.settings.lang.as_str();
            let what = if lang == "pl" {
                format!(
                    "tok: dokument ma błędy składni ({}); popraw je przed formatowaniem. {}",
                    errors.len(),
                    tok_core::diagnostics::format_diag(first, lang, None)
                )
            } else {
                format!(
                    "tok: the document has syntax errors ({}); fix them before formatting. {}",
                    errors.len(),
                    tok_core::diagnostics::format_diag(first, lang, None)
                )
            };
            return Err(failed(what));
        }
        let mut formatted = printer::fmt(&parsed);
        if doc.text.contains("\r\n") {
            formatted = formatted.replace('\n', "\r\n");
        }
        let current = doc.text.strip_prefix('\u{feff}').unwrap_or(&doc.text);
        if formatted == current {
            return Ok(json!([]));
        }
        let text = &doc.analysis.text;
        let last = text.len() - 1;
        Ok(json!([{
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": last, "character": text.end(last)},
            },
            "newText": formatted,
        }]))
    }

    fn location(uri: &str, doc: &Doc, o: &index::Occurrence) -> Value {
        json!({"uri": uri, "range": doc.analysis.occ_range(o)})
    }

    fn definition(&self, params: &Value) -> Reply {
        let (uri, doc, occ) = self.target(params)?;
        let Some(occ) = occ else {
            return Ok(Value::Null);
        };
        Ok(doc
            .analysis
            .of(&occ.id)
            .find(|o| o.def)
            .map(|d| Self::location(uri, doc, d))
            .unwrap_or(Value::Null))
    }

    fn references(&self, params: &Value) -> Reply {
        let (uri, doc, occ) = self.target(params)?;
        let Some(occ) = occ else {
            return Ok(json!([]));
        };
        let with_decl = params
            .get("context")
            .and_then(|c| c.get("includeDeclaration"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        Ok(Value::Array(
            doc.analysis
                .of(&occ.id)
                .filter(|o| with_decl || !o.def)
                .map(|o| Self::location(uri, doc, o))
                .collect(),
        ))
    }

    fn highlight(&self, params: &Value) -> Reply {
        let (_, doc, occ) = self.target(params)?;
        let Some(occ) = occ else {
            return Ok(json!([]));
        };
        Ok(Value::Array(
            doc.analysis
                .of(&occ.id)
                .map(|o| {
                    json!({"range": doc.analysis.occ_range(o), "kind": if o.def { 3 } else { 2 }})
                })
                .collect(),
        ))
    }

    fn prepare_rename(&self, params: &Value) -> Reply {
        let (_, doc, occ) = self.target(params)?;
        let Some(occ) = occ else {
            return Ok(Value::Null);
        };
        if !doc.analysis.of(&occ.id).any(|o| o.def) {
            // a derived identifier (`f-a-b`) is not written in the document
            return Ok(Value::Null);
        }
        Ok(json!({"range": doc.analysis.occ_range(occ), "placeholder": occ.id}))
    }

    fn rename(&self, params: &Value) -> Reply {
        let (uri, doc, occ) = self.target(params)?;
        let new = params
            .get("newName")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("missing newName"))?;
        let Some(occ) = occ else {
            return Err(failed("no identifier at this position"));
        };
        if !is_id(new) {
            return Err(failed(format!(
                "'{new}' is not a valid identifier: an ASCII letter or '_', then ASCII letters, \
                 digits, '_' and '-'"
            )));
        }
        if new == occ.id {
            return Ok(json!({"changes": {}}));
        }
        if let Some(other) = doc.analysis.doc.ids.get(new) {
            return Err(failed(format!(
                "identifier '{new}' is already used on line {}",
                other.line
            )));
        }
        if !doc.analysis.of(&occ.id).any(|o| o.def) {
            return Err(failed(format!(
                "'{}' is derived and not written in the document",
                occ.id
            )));
        }
        let a = &doc.analysis;
        let mut edits: Vec<Value> = a
            .of(&occ.id)
            .map(|o| json!({"range": a.occ_range(o), "newText": new}))
            .collect();
        // derived identifiers of flows and message flows (`f-a-b`, `m-a-b`) that other
        // statements name change with the names of their ends
        for o in &a.occurrences {
            if let Some(derived) = rederive(&a.doc, &o.id, &occ.id, new) {
                edits.push(json!({"range": a.occ_range(o), "newText": derived}));
            }
        }
        Ok(json!({"changes": {uri: edits}}))
    }

    fn document_symbols(&self, params: &Value) -> Reply {
        let (_, doc) = self.doc(params)?;
        let a = &doc.analysis;
        Ok(Value::Array(
            a.symbols().iter().map(|s| a.symbol_json(s)).collect(),
        ))
    }

    fn hover(&self, params: &Value) -> Reply {
        let (_, doc) = self.doc(params)?;
        let (line, units) = position(params).ok_or_else(|| invalid("missing position"))?;
        let a = &doc.analysis;
        let col = a.text.column(line, units);
        let mut sections: Vec<String> = Vec::new();
        let mut range = None;
        if let Some(o) = index::at(&a.occurrences, line, col) {
            if let Some((kind, label)) = index::describe(&a.doc, &o.id) {
                let label = label.map(|l| format!(" \"{l}\"")).unwrap_or_default();
                let mut s = format!("**{kind}** `{}`{label}", o.id);
                if let Some(d) = keywords::describe(&kind) {
                    s.push_str(&format!("\n\n{d}"));
                }
                if let Some(entry) = a.doc.ids.get(&o.id) {
                    s.push_str(&format!("\n\ndeclared on line {}", entry.line));
                }
                sections.push(s);
            }
            range = Some(a.occ_range(o));
        } else if let Some(l) = a.line(line)
            && let Some(&(s, e)) = l.spans.first()
            && s <= col
            && col <= e
            && let Some(t) = l.tokens.first()
            && t.kind == Kind::Word
            && let Some(d) = keywords::describe(&t.value)
        {
            sections.push(format!("**{}** — {d}", t.value));
            range = Some(a.text.range(line, s, e));
        }
        // a diagnostic code written in a comment or elsewhere
        if sections.is_empty() {
            let chars = a.text.chars(line);
            let c = col.min(chars.len());
            let start = (0..c)
                .rev()
                .take_while(|&k| chars[k].is_ascii_alphanumeric())
                .last()
                .unwrap_or(c);
            let end = (c..chars.len())
                .take_while(|&k| chars[k].is_ascii_alphanumeric())
                .last()
                .map_or(c, |k| k + 1);
            let word: String = chars[start..end].iter().collect();
            if looks_like_code(&word)
                && let Some(e) = explain(&word)
            {
                sections.push(e);
                range = Some(a.text.range(line, start, end));
            }
        }
        // the diagnostics at the position
        for d in &a.diags {
            let r = a.diag_range(d);
            let inside = |p: &Value| -> Option<bool> {
                let (sl, sc) = (
                    p["start"]["line"].as_u64()?,
                    p["start"]["character"].as_u64()?,
                );
                let (el, ec) = (p["end"]["line"].as_u64()?, p["end"]["character"].as_u64()?);
                let pos = (line as u64, u64::from(units));
                Some((sl, sc) <= pos && pos <= (el, ec))
            };
            if inside(&r) == Some(true) {
                let sev = match d.severity() {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Note => "note",
                };
                let mut s = format!("**{}** {sev}: {}", d.code, d.message(&self.settings.lang));
                if let Some(e) = explain(&d.code) {
                    s.push_str(&format!("\n\n{e}"));
                }
                sections.push(s);
            }
        }
        if sections.is_empty() {
            return Ok(Value::Null);
        }
        let mut result = json!({
            "contents": {"kind": "markdown", "value": sections.join("\n\n---\n\n")},
        });
        if let Some(r) = range {
            result["range"] = r;
        }
        Ok(result)
    }

    fn completion(&self, params: &Value) -> Reply {
        let (_, doc) = self.doc(params)?;
        let (line, units) = position(params).ok_or_else(|| invalid("missing position"))?;
        let a = &doc.analysis;
        let col = a.text.column(line, units);
        let chars = a.text.chars(line);
        let before: String = chars[..col.min(chars.len())].iter().collect();
        let trimmed = before.trim_start();
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        let at_word_end = !before.ends_with(' ');
        let keyword_items = || -> Vec<Value> {
            keywords::KEYWORDS
                .iter()
                .map(|(k, d)| json!({"label": k, "kind": 14, "detail": d}))
                .collect()
        };
        // the first word of a statement
        if words.is_empty() || (words.len() == 1 && at_word_end) {
            return Ok(json!({"isIncomplete": false, "items": keyword_items()}));
        }
        let keyword = words[0];
        let ids = |filter: &dyn Fn(&str) -> bool| -> Vec<Value> {
            a.doc
                .ids
                .iter()
                .filter(|(id, e)| filter(&e.kind) && !id.starts_with("f-"))
                .map(|(id, e)| {
                    let label = index::describe(&a.doc, id).and_then(|(_, l)| l);
                    let detail = match label {
                        Some(l) => format!("{} \"{l}\"", e.kind),
                        None => e.kind.clone(),
                    };
                    json!({"label": id, "kind": 18, "detail": detail})
                })
                .collect()
        };
        let node = |k: &str| a.doc.nodes.values().any(|n| n.base.kind == k);
        // after `KEY=`
        if let Some(last) = trimmed.rsplit(' ').next()
            && let Some((key, _)) = last.split_once('=')
            && !last.contains('"')
        {
            let items = match key {
                "on" | "activity" | "compensate" => ids(&|k| node(k)),
                "form" => ids(&|k| k == "form"),
                "decision" => ids(&|k| k == "decision"),
                "by" => ids(&|k| k == "resource"),
                "task" => ids(&|k| k == "global"),
                "operation" => ids(&|k| k == "operation"),
                "from" | "to" => ids(&|k| k == "pool"),
                "invoke" => ids(&|k| k == "bkm"),
                "choreography" | "collaboration" => ids(&|k| k == "choreography"),
                "type" if keyword != "field" => ids(&|k| k == "item"),
                "type" => [
                    "textfield",
                    "textarea",
                    "number",
                    "checkbox",
                    "select",
                    "radio",
                    "datetime",
                ]
                .iter()
                .map(|t| json!({"label": t, "kind": 20}))
                .collect(),
                "hit" => {
                    let hits = if a.doc.lang_07() {
                        tok_core::model::HIT_POLICIES_07
                    } else {
                        tok_core::model::HIT_POLICIES
                    };
                    hits.iter()
                        .map(|t| json!({"label": t, "kind": 20}))
                        .collect()
                }
                k if REF_KEYS.contains(&k) => ids(&|_| true),
                _ => Vec::new(),
            };
            return Ok(json!({"isIncomplete": false, "items": items}));
        }
        // node names of flows and associations
        let arrow = words.last().is_some_and(|w| w.ends_with("->"))
            || (words.len() >= 2 && words[words.len() - 2].ends_with("->") && at_word_end);
        let names_position = match keyword {
            "flow" | "message" => arrow || words.len() == 1 || (words.len() == 2 && at_word_end),
            "reads" | "writes" => words.len() <= 3 && !(words.len() == 3 && !at_word_end),
            _ => false,
        };
        if names_position {
            let items = match keyword {
                "flow" => ids(&|k| node(k)),
                "message" => ids(&|k| node(k) || k == "pool"),
                _ => ids(&|k| {
                    node(k) || ["data", "store", "input", "output", "property"].contains(&k)
                }),
            };
            return Ok(json!({"isIncomplete": false, "items": items}));
        }
        // attribute keys and flags after the identifier
        if words.len() >= 2 {
            let items: Vec<Value> = keywords::attributes(keyword)
                .into_iter()
                .map(|k| {
                    if keywords::is_flag(keyword, k) {
                        json!({"label": k, "kind": 10, "detail": "flag"})
                    } else {
                        json!({"label": format!("{k}="), "kind": 10, "insertText": format!("{k}=")})
                    }
                })
                .collect();
            return Ok(json!({"isIncomplete": false, "items": items}));
        }
        Ok(json!({"isIncomplete": false, "items": []}))
    }

    fn check(&self, params: &Value) -> Reply {
        let (_, doc) = self.doc(params)?;
        let a = &doc.analysis;
        let count = |s: Severity| a.diags.iter().filter(|d| d.severity() == s).count();
        Ok(json!({
            "errors": count(Severity::Error),
            "warnings": count(Severity::Warning),
            "notes": count(Severity::Note),
            "bpmn_class": classes::bpmn_class(&a.doc),
        }))
    }

    /// `tok/svg`, `tok/bpmn` and `tok/export`: the projections of `tok svg`, `tok bpmn` and
    /// `tok export`, with the layout sidecar next to the file when there is one.
    fn projection(&self, method: &str, params: &Value) -> Reply {
        let (uri, doc) = self.doc(params)?;
        let path = file_path(uri);
        let stem = path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .or_else(|| {
                let name = uri.rsplit('/').next().unwrap_or("model");
                Some(name.split('.').next().unwrap_or("model").to_string())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "model".to_string());
        let sidecar = path.and_then(|p| {
            let mut side = p.into_os_string();
            side.push(".layout");
            std::fs::read_to_string(PathBuf::from(side)).ok()
        });
        let mut options = json!({"lang": self.settings.lang});
        if let Some(s) = sidecar {
            options["sidecar"] = Value::String(s);
        }
        let api = match method {
            "tok/svg" => {
                options["title"] = Value::String(format!("{stem}.svg"));
                tok_cli::api::svg
            }
            "tok/bpmn" => tok_cli::api::bpmn,
            _ => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| stem.clone());
                options["name"] = Value::String(name);
                tok_cli::api::export_files
            }
        };
        let result = api(&doc.text, &options.to_string()).map_err(|e| failed(e.message))?;
        let value: Value = serde_json::from_str(&result.compact())
            .map_err(|e| failed(format!("internal error: {e}")))?;
        Ok(match method {
            "tok/svg" => json!({"svg": value}),
            "tok/bpmn" => json!({"bpmn": value}),
            _ => json!({"files": value}),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_become_paths() {
        assert_eq!(
            file_path("file:///c%3A/models/x/model%20a.tok"),
            Some(PathBuf::from("c:/models/x/model a.tok"))
        );
        assert_eq!(
            file_path("file:///home/x/mo%C5%BCe.tok"),
            Some(PathBuf::from("/home/x/może.tok"))
        );
        assert_eq!(file_path("untitled:Untitled-1"), None);
    }

    #[test]
    fn codes_are_explained() {
        assert!(explain("R002").is_some_and(|e| e.contains("unknown")));
        assert!(explain("E008").is_some());
        assert!(explain("X999").is_none());
        assert!(looks_like_code("C100"));
        assert!(!looks_like_code("C10"));
    }
}
