//! The language server driven as an editor drives it: JSON-RPC messages in, responses
//! and notifications out. Positions are UTF-16 columns, so the documents carry Polish
//! letters (two bytes in UTF-8, one UTF-16 unit) and characters outside the Basic
//! Multilingual Plane (two UTF-16 units) before the identifiers they point at.

use serde_json::{Value, json};
use tok_lsp::Server;

const URI: &str = "file:///nonexistent/folder/wypozyczenie.tok";

/// The model of the tests. Line 9 has `on=akceptacja` after a label with a Polish letter
/// and a character outside the BMP.
const MODEL: &str = "tok 0.1 \"Wypożyczenie\"

pool biblioteka \"Biblioteka uczelniana\"
  lane czytelnik \"Czytelnik\"
    start potrzeba \"Potrzeba książki\"
    user zamowienie \"Złożenie zamówienia\"
  lane bibliotekarz \"Bibliotekarz\"
    user akceptacja \"Akceptacja\"
    boundary termin \"Minęło 7 dni 📅\" on=akceptacja timer=P7D
    end gotowe \"Gotowe\"
    end wygasla \"Wygasła\"

flow potrzeba -> zamowienie -> akceptacja -> gotowe
flow termin -> wygasla
";

struct Client {
    server: Server,
    next: i64,
}

impl Client {
    fn start(params: Value) -> (Client, Value) {
        let mut c = Client {
            server: Server::new(),
            next: 0,
        };
        let r = c.request("initialize", params);
        assert!(c.notify("initialized", json!({})).is_empty());
        (c, r)
    }

    fn new() -> Client {
        Client::start(json!({"capabilities": {}})).0
    }

    fn send(&mut self, message: Value) -> Vec<Value> {
        // every message goes through the JSON text, as on the wire
        self.server.handle_text(&message.to_string())
    }

    /// The whole response of a request.
    fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        let out =
            self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        out.into_iter()
            .find(|m| m["id"] == json!(id))
            .unwrap_or_else(|| panic!("no response to {method}"))
    }

    /// The result of a request that must succeed.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let r = self.call(method, params);
        assert!(r.get("error").is_none(), "{method} failed: {r}");
        r["result"].clone()
    }

    fn notify(&mut self, method: &str, params: Value) -> Vec<Value> {
        self.send(json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    /// Open a document; returns the diagnostics published for it.
    fn open(&mut self, uri: &str, text: &str) -> Vec<Value> {
        let out = self.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": "tok", "version": 1, "text": text}}),
        );
        diagnostics(&out, uri)
    }
}

fn diagnostics(out: &[Value], uri: &str) -> Vec<Value> {
    let m = out
        .iter()
        .find(|m| m["method"] == "textDocument/publishDiagnostics" && m["params"]["uri"] == uri)
        .expect("diagnostics published");
    m["params"]["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn at(line: u32, character: u32) -> Value {
    json!({"textDocument": {"uri": URI}, "position": {"line": line, "character": character}})
}

fn range(l1: u32, c1: u32, l2: u32, c2: u32) -> Value {
    json!({"start": {"line": l1, "character": c1}, "end": {"line": l2, "character": c2}})
}

/// UTF-16 column of the first occurrence of `needle` on line `line` of `text`.
fn col16(text: &str, line: usize, needle: &str) -> u32 {
    let l = text.lines().nth(line).expect("line exists");
    let byte = l.find(needle).expect("needle on line");
    l[..byte].encode_utf16().count() as u32
}

#[test]
fn initialize_announces_the_capabilities() {
    let (mut c, r) = Client::start(json!({"capabilities": {}, "processId": null}));
    let caps = &r["capabilities"];
    assert_eq!(caps["positionEncoding"], "utf-16");
    assert_eq!(caps["textDocumentSync"]["change"], 1);
    for p in [
        "documentFormattingProvider",
        "definitionProvider",
        "referencesProvider",
        "documentSymbolProvider",
        "hoverProvider",
    ] {
        assert_eq!(caps[p], true, "{p}");
    }
    assert_eq!(caps["renameProvider"]["prepareProvider"], true);
    assert_eq!(r["serverInfo"]["name"], "tok-lsp");
    assert_eq!(r["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    let e = c.call("textDocument/unknown", json!({}));
    assert_eq!(e["error"]["code"], -32601);
}

#[test]
fn requests_before_initialize_and_after_shutdown_fail() {
    let mut s = Server::new();
    let out =
        s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "textDocument/hover", "params": {}}));
    assert_eq!(out[0]["error"]["code"], -32002);
    let mut c = Client::new();
    assert_eq!(c.request("shutdown", Value::Null), Value::Null);
    let e = c.call("textDocument/hover", at(0, 0));
    assert_eq!(e["error"]["code"], -32600);
    c.notify("exit", Value::Null);
    assert_eq!(c.server.exit_code(), Some(0));
    let mut d = Client::new();
    d.notify("exit", Value::Null);
    assert_eq!(d.server.exit_code(), Some(1));
}

#[test]
fn invalid_json_is_a_parse_error() {
    let mut s = Server::new();
    let out = s.handle_text("{not json");
    assert_eq!(out[0]["error"]["code"], -32700);
    assert_eq!(out[0]["id"], Value::Null);
}

#[test]
fn a_correct_model_has_no_diagnostics() {
    let mut c = Client::new();
    assert_eq!(c.open(URI, MODEL), Vec::<Value>::new());
}

#[test]
fn diagnostics_point_at_the_element_in_utf16_columns() {
    let text = MODEL.replace("on=akceptacja", "on=akcept");
    let mut c = Client::new();
    let diags = c.open(URI, &text);
    let r002 = diags
        .iter()
        .find(|d| d["code"] == "R002")
        .expect("R002 reported");
    assert_eq!(r002["severity"], 1);
    assert_eq!(r002["source"], "tok");
    assert_eq!(r002["message"], "unknown activity 'akcept'");
    let start = col16(&text, 8, "akcept");
    // the label before has `ę` and `ł` (two bytes, one unit each) and `📅` (four bytes,
    // two units)
    assert_eq!(
        start,
        text.lines()
            .nth(8)
            .expect("present")
            .find("akcept")
            .expect("present") as u32
            - 4
    );
    assert_eq!(r002["range"], range(8, start, 8, start + 6));
}

#[test]
fn messages_are_polish_for_a_polish_locale_or_option() {
    let text = MODEL.replace("on=akceptacja", "on=akcept");
    let (mut c, _) = Client::start(json!({"capabilities": {}, "locale": "pl-PL"}));
    let d = c.open(URI, &text);
    assert_eq!(d[0]["message"], "nieznany element (czynność): „akcept”");
    let (mut c, _) = Client::start(
        json!({"capabilities": {}, "locale": "en", "initializationOptions": {"lang": "pl"}}),
    );
    let d = c.open(URI, &text);
    assert_eq!(d[0]["message"], "nieznany element (czynność): „akcept”");
    // and back to English through the configuration
    let out = c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"tok": {"lang": "en"}}}),
    );
    assert_eq!(
        diagnostics(&out, URI)[0]["message"],
        "unknown activity 'akcept'"
    );
}

#[test]
fn the_teaching_level_and_class_reach_the_checker() {
    let text = "tok 0.5 \"Pętla\"\nstart s\nservice proba \"Próba\" loop while=\"x < 3\"\nend e\nflow s -> proba -> e\n";
    let (mut c, _) =
        Client::start(json!({"capabilities": {}, "initializationOptions": {"level": "analytic"}}));
    let d = c.open(URI, text);
    assert!(d.iter().any(|d| d["code"] == "R040"), "{d:?}");
    let out = c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"tok": {"level": "full", "class": "descriptive"}}}),
    );
    let d = diagnostics(&out, URI);
    assert!(!d.iter().any(|d| d["code"] == "R040"));
    assert!(d.iter().any(|d| d["code"] == "R075"), "{d:?}");
}

#[test]
fn every_layer_is_reported() {
    // a BPMN rule (W/L codes come from the BPMN validator) and a simulation parameter
    let text = "tok 0.1 \"Wybór\"\nstart s\nxor g \"Decyzja\"\ntask a\ntask b\nend e\nflow s -> g\nflow g -> a\nflow g -> b\nflow a -> e\nflow b -> e\nsimulate cases=abc\n";
    let mut c = Client::new();
    let d = c.open(URI, text);
    let codes: Vec<&str> = d.iter().filter_map(|d| d["code"].as_str()).collect();
    assert!(codes.iter().any(|c| c.starts_with('S')), "{codes:?}");
    let mut c = Client::new();
    let ok = "tok 0.1 \"Wybór\"\nstart s\nxor g \"Decyzja\"\ntask a\ntask b\nend e\nflow s -> g\nflow g -> a\nflow g -> b\nflow a -> e\nflow b -> e\n";
    let d = c.open(URI, ok);
    assert!(
        d.iter().any(|d| d["code"]
            .as_str()
            .is_some_and(|c| c.starts_with('L') || c.starts_with('W'))),
        "{d:?}"
    );
}

#[test]
fn changes_and_close_republish() {
    let mut c = Client::new();
    c.open(URI, MODEL);
    let out = c.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 2},
               "contentChanges": [{"text": MODEL.replace("-> wygasla", "-> nikt")}]}),
    );
    let d = diagnostics(&out, URI);
    assert_eq!(d.len(), 1);
    assert_eq!(out[0]["params"]["version"], 2);
    let out = c.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": URI}}),
    );
    assert_eq!(diagnostics(&out, URI), Vec::<Value>::new());
    let e = c.call("textDocument/hover", at(0, 0));
    assert_eq!(e["error"]["code"], -32803);
}

#[test]
fn formatting_prints_the_canonical_form() {
    let messy = "tok 0.1   \"Wypożyczenie\"\npool  b \"Biblioteka\"\n    start s \"Początek 📚\"\n    end e\nflow s->e\n";
    let mut c = Client::new();
    c.open(URI, messy);
    let edits = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {"tabSize": 2, "insertSpaces": true}}),
    );
    let edits = edits.as_array().expect("edits");
    assert_eq!(edits.len(), 1);
    let expected = "tok 0.1 \"Wypożyczenie\"\n\npool b \"Biblioteka\"\n  start s \"Początek 📚\"\n  end e\n\nflow s -> e\n";
    assert_eq!(edits[0]["newText"], expected);
    // the edit replaces the whole document: 6 lines, the last one empty
    assert_eq!(edits[0]["range"], range(0, 0, 5, 0));
    // a formatted document needs no edit
    c.open(URI, expected);
    let edits = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {}}),
    );
    assert_eq!(edits, json!([]));
    // Windows line endings are kept
    c.open(URI, &messy.replace('\n', "\r\n"));
    let edits = c.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {}}),
    );
    assert_eq!(edits[0]["newText"], expected.replace('\n', "\r\n"));
}

#[test]
fn formatting_refuses_a_document_with_syntax_errors() {
    let mut c = Client::new();
    c.open(URI, "tok 0.1\nstart s \"niezamknięty\n");
    let e = c.call(
        "textDocument/formatting",
        json!({"textDocument": {"uri": URI}, "options": {}}),
    );
    assert_eq!(e["error"]["code"], -32803);
    let msg = e["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("syntax errors") && msg.contains("T001"),
        "{msg}"
    );
}

#[test]
fn definition_goes_to_the_declaration() {
    let mut c = Client::new();
    c.open(URI, MODEL);
    // `akceptacja` in `on=akceptacja`, after the emoji
    let col = col16(MODEL, 8, "akceptacja");
    let r = c.request("textDocument/definition", at(8, col + 3));
    assert_eq!(r["uri"], URI);
    assert_eq!(r["range"], range(7, 9, 7, 19));
    // from a flow line, also at the end of the identifier
    let r = c.request("textDocument/definition", at(12, 13));
    assert_eq!(r["range"], range(4, 10, 4, 18));
    // the declaration itself
    let r = c.request("textDocument/definition", at(4, 12));
    assert_eq!(r["range"], range(4, 10, 4, 18));
    // a keyword is not an identifier
    assert_eq!(c.request("textDocument/definition", at(4, 5)), Value::Null);
}

#[test]
fn references_find_every_use() {
    let mut c = Client::new();
    c.open(URI, MODEL);
    let mut p = at(7, 12);
    p["context"] = json!({"includeDeclaration": true});
    let refs = c.request("textDocument/references", p.clone());
    let lines: Vec<u64> = refs
        .as_array()
        .expect("present")
        .iter()
        .map(|r| r["range"]["start"]["line"].as_u64().expect("present"))
        .collect();
    assert_eq!(lines, vec![7, 8, 12]);
    let col = col16(MODEL, 8, "akceptacja");
    assert_eq!(refs[1]["range"], range(8, col, 8, col + 10));
    p["context"] = json!({"includeDeclaration": false});
    let refs = c.request("textDocument/references", p);
    assert_eq!(refs.as_array().expect("present").len(), 2);
    let hl = c.request("textDocument/documentHighlight", at(12, 33));
    assert_eq!(hl.as_array().expect("present").len(), 3);
    assert_eq!(hl[0]["kind"], 3);
    assert_eq!(hl[1]["kind"], 2);
}

#[test]
fn rename_changes_every_occurrence() {
    let mut c = Client::new();
    c.open(URI, MODEL);
    let prep = c.request("textDocument/prepareRename", at(7, 12));
    assert_eq!(prep["placeholder"], "akceptacja");
    assert_eq!(prep["range"], range(7, 9, 7, 19));
    assert_eq!(
        c.request("textDocument/prepareRename", at(7, 3)),
        Value::Null
    );
    let mut p = at(8, col16(MODEL, 8, "akceptacja") + 1);
    p["newName"] = json!("zatwierdzenie");
    let edit = c.request("textDocument/rename", p.clone());
    let edits = edit["changes"][URI]
        .as_array()
        .expect("edits for the document");
    assert_eq!(edits.len(), 3);
    // apply the edits and check the result
    let mut lines: Vec<String> = MODEL.lines().map(str::to_string).collect();
    let mut sorted = edits.clone();
    sorted.sort_by_key(|e| {
        std::cmp::Reverse((
            e["range"]["start"]["line"].as_u64(),
            e["range"]["start"]["character"].as_u64(),
        ))
    });
    for e in &sorted {
        let l = e["range"]["start"]["line"].as_u64().expect("present") as usize;
        let s = e["range"]["start"]["character"].as_u64().expect("present") as usize;
        let t = e["range"]["end"]["character"].as_u64().expect("present") as usize;
        let units: Vec<u16> = lines[l].encode_utf16().collect();
        let mut out = String::from_utf16(&units[..s]).expect("present");
        out.push_str(e["newText"].as_str().expect("present"));
        out.push_str(&String::from_utf16(&units[t..]).expect("present"));
        lines[l] = out;
    }
    let renamed = lines.join("\n") + "\n";
    assert_eq!(renamed, MODEL.replace("akceptacja", "zatwierdzenie"));
    assert_eq!(c.open(URI, &renamed), Vec::<Value>::new());
    // an invalid identifier and a conflict
    c.open(URI, MODEL);
    p["newName"] = json!("zatwierdzenie wniosku");
    let e = c.call("textDocument/rename", p.clone());
    assert_eq!(e["error"]["code"], -32803);
    assert!(
        e["error"]["message"]
            .as_str()
            .expect("present")
            .contains("not a valid identifier")
    );
    p["newName"] = json!("zatwierdź");
    assert_eq!(
        c.call("textDocument/rename", p.clone())["error"]["code"],
        -32803
    );
    p["newName"] = json!("gotowe");
    let e = c.call("textDocument/rename", p);
    assert_eq!(
        e["error"]["message"],
        "identifier 'gotowe' is already used on line 10"
    );
}

#[test]
fn rename_follows_the_names_of_a_decision_model() {
    let text = "tok 0.7 \"Wypożyczenia\"
inputdata wiek \"Wiek czytelnika\" type=number
inputdata konto \"Konto\" type=string
bkm kara \"Kara\" params=\"dni: number\" type=number expr=\"dni * 0.5\"
decision limit \"Limit\" requires=\"wiek konto\" hit=first
  | wiek: number | konto: string || limit: number |
  | < 18         | -             || 5             |
  | -            | -             || 10            |
decision opis \"Opis\" requires=\"limit kara wiek\" expr=\"if wiek > 60 then \\\"wiek: senior\\\" else string(limit + kara(wiek))\"
";
    let mut c = Client::new();
    assert_eq!(c.open(URI, text), Vec::<Value>::new());
    let mut p = at(1, 11);
    p["newName"] = json!("lata");
    let edit = c.request("textDocument/rename", p);
    let edits = edit["changes"][URI].as_array().expect("present");
    let mut where_: Vec<(u64, u64)> = edits
        .iter()
        .map(|e| {
            (
                e["range"]["start"]["line"].as_u64().expect("present"),
                e["range"]["start"]["character"].as_u64().expect("present"),
            )
        })
        .collect();
    where_.sort();
    // the declaration, `requires` of both decisions, the input column of the table
    // (not its output column `limit`), and the FEEL of `opis` outside its string literal
    let l8 = text.lines().nth(8).expect("present");
    let feel1 = l8.find("if wiek").expect("present") as u64 + 3;
    let feel2 = l8.find("kara(wiek)").expect("present") as u64 + 5;
    let req = l8.find("kara wiek").expect("present") as u64 + 5;
    assert_eq!(
        where_,
        vec![(1, 10), (4, 33), (5, 4), (8, req), (8, feel1), (8, feel2)]
    );
}

#[test]
fn document_symbols_give_the_outline() {
    let mut c = Client::new();
    c.open(URI, MODEL);
    let s = c.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": URI}}),
    );
    let pool = &s[0];
    assert_eq!(pool["name"], "biblioteka");
    assert_eq!(pool["kind"], 2);
    assert_eq!(pool["detail"], "pool \"Biblioteka uczelniana\"");
    assert_eq!(pool["range"]["start"]["line"], 2);
    assert_eq!(pool["range"]["end"]["line"], 10);
    let lanes = pool["children"].as_array().expect("present");
    assert_eq!(lanes.len(), 2);
    assert_eq!(lanes[1]["name"], "bibliotekarz");
    let nodes: Vec<&str> = lanes[1]["children"]
        .as_array()
        .expect("present")
        .iter()
        .map(|n| n["name"].as_str().expect("present"))
        .collect();
    assert_eq!(nodes, vec!["akceptacja", "termin", "gotowe", "wygasla"]);
    assert_eq!(lanes[1]["children"][1]["kind"], 24);
    assert_eq!(
        lanes[1]["children"][1]["selectionRange"],
        range(8, 13, 8, 19)
    );
    assert_eq!(s.as_array().expect("present").len(), 1);
}

#[test]
fn hover_shows_elements_keywords_and_diagnostic_codes() {
    let text = MODEL.replace("on=akceptacja", "on=akcept") + "# R002 means an unknown reference\n";
    let mut c = Client::new();
    c.open(URI, &text);
    let h = c.request("textDocument/hover", at(5, 10));
    let v = h["contents"]["value"].as_str().expect("present");
    assert!(
        v.starts_with("**user** `zamowienie` \"Złożenie zamówienia\""),
        "{v}"
    );
    assert!(
        v.contains("user task") && v.contains("declared on line 6"),
        "{v}"
    );
    assert_eq!(h["contents"]["kind"], "markdown");
    let h = c.request("textDocument/hover", at(8, 6));
    assert!(
        h["contents"]["value"]
            .as_str()
            .expect("present")
            .starts_with("**boundary** — boundary event")
    );
    let col = col16(&text, 8, "akcept");
    let h = c.request("textDocument/hover", at(8, col + 2));
    let v = h["contents"]["value"].as_str().expect("present");
    assert!(
        v.contains("**R002** error: unknown activity 'akcept'"),
        "{v}"
    );
    assert!(v.contains("references and structure"), "{v}");
    let h = c.request("textDocument/hover", at(14, 3));
    assert!(
        h["contents"]["value"]
            .as_str()
            .expect("present")
            .starts_with("**R002** (error)")
    );
    assert_eq!(c.request("textDocument/hover", at(1, 0)), Value::Null);
}

#[test]
fn completion_offers_keywords_identifiers_and_attributes() {
    let text = MODEL.to_string()
        + "flow zamowienie -> \n    user nowe \"Nowe\" fo\n    boundary b on=\n  \n";
    let mut c = Client::new();
    c.open(URI, &text);
    let labels = |r: &Value| -> Vec<String> {
        r["items"]
            .as_array()
            .expect("present")
            .iter()
            .map(|i| i["label"].as_str().expect("present").to_string())
            .collect()
    };
    let r = c.request("textDocument/completion", at(17, 2));
    let l = labels(&r);
    assert!(l.contains(&"boundary".to_string()) && l.contains(&"inputdata".to_string()));
    let r = c.request("textDocument/completion", at(14, 19));
    let l = labels(&r);
    assert!(
        l.contains(&"gotowe".to_string()) && !l.contains(&"czytelnik".to_string()),
        "{l:?}"
    );
    let r = c.request("textDocument/completion", at(16, 18));
    let l = labels(&r);
    assert!(
        l.contains(&"akceptacja".to_string()) && !l.contains(&"biblioteka".to_string()),
        "{l:?}"
    );
    let r = c.request("textDocument/completion", at(15, 23));
    let l = labels(&r);
    assert!(
        l.contains(&"form=".to_string()) && l.contains(&"parallel".to_string()),
        "{l:?}"
    );
}

#[test]
fn extension_requests_give_the_projections() {
    let mut c = Client::new();
    c.open(URI, MODEL);
    let svg = c.request("tok/svg", json!({"textDocument": {"uri": URI}}));
    let svg = svg["svg"].as_str().expect("present");
    assert!(
        svg.starts_with("<?xml") || svg.starts_with("<svg"),
        "{}",
        &svg[..40]
    );
    assert!(svg.contains("Złożenie"));
    let bpmn = c.request("tok/bpmn", json!({"textDocument": {"uri": URI}}));
    assert!(
        bpmn["bpmn"]
            .as_str()
            .expect("present")
            .contains("<bpmndi:BPMNDiagram")
    );
    let files = c.request("tok/export", json!({"textDocument": {"uri": URI}}));
    let names: Vec<&String> = files["files"]
        .as_object()
        .expect("present")
        .keys()
        .collect();
    assert_eq!(names, vec!["wypozyczenie.bpmn", "wypozyczenie.svg"]);
    let check = c.request("tok/check", json!({"textDocument": {"uri": URI}}));
    assert_eq!(check["errors"], 0);
    assert_eq!(check["bpmn_class"], "analytic");
    // a document with errors gives the error of the engine
    c.open(URI, &MODEL.replace("on=akceptacja", "on=akcept"));
    let e = c.call("tok/svg", json!({"textDocument": {"uri": URI}}));
    assert_eq!(e["error"]["code"], -32803);
    assert!(
        e["error"]["message"]
            .as_str()
            .expect("present")
            .contains("R002")
    );
}

#[test]
fn the_stdio_loop_frames_messages() {
    let messages = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}),
        json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {"uri": URI, "languageId": "tok", "version": 1, "text": "tok 0.1 \"Zażółć\"\nstart s\n"}}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}),
        json!({"jsonrpc": "2.0", "method": "exit"}),
    ];
    let mut input = Vec::new();
    for m in &messages {
        let body = m.to_string();
        input.extend_from_slice(format!("Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{body}", body.len()).as_bytes());
    }
    let mut output = Vec::new();
    let code = tok_lsp::run(&input[..], &mut output);
    assert_eq!(code, 0);
    let mut reader = &output[..];
    let mut replies = Vec::new();
    while let Some(body) = tok_lsp::read_message(&mut reader).expect("framed output") {
        replies.push(serde_json::from_str::<Value>(&body).expect("JSON"));
    }
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["id"], 1);
    assert_eq!(replies[1]["method"], "textDocument/publishDiagnostics");
    assert_eq!(replies[2]["id"], 2);
    // the input ends without exit: status 1
    let mut out = Vec::new();
    assert_eq!(tok_lsp::run(&b""[..], &mut out), 1);
}

#[test]
fn the_program_serves_an_editor_over_its_standard_streams() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_tok-lsp"))
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("tok-lsp starts");
    let mut input = Vec::new();
    for m in [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}),
        json!({"jsonrpc": "2.0", "method": "exit"}),
    ] {
        let body = m.to_string();
        input.extend_from_slice(format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes());
    }
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&input)
        .expect("written");
    let out = child.wait_with_output().expect("tok-lsp ends");
    assert_eq!(out.status.code(), Some(0));
    let mut reader = &out.stdout[..];
    let first = tok_lsp::read_message(&mut reader)
        .expect("framed")
        .expect("a message");
    let v: Value = serde_json::from_str(&first).expect("JSON");
    assert_eq!(v["result"]["serverInfo"]["name"], "tok-lsp");
}
