//! The documentation covers the language and the tools: every diagnostic code, keyword,
//! attribute key and flag, command and option of the command line, function of the C
//! interface, of the Python module `tok.api` and of the R package, tool of the MCP
//! server and method of the language server is documented, and every keyword, attribute
//! key and flag has an example in the conformance cases or the examples. The copies of
//! the reference documents are identical.
//!
//! The lists come from the sources (the tables of `tok-core`, the help of the `tok`
//! program, the headers and source files of the other interfaces), so a construct added
//! without documentation fails this test.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use tok_core::diagnostics::{BPMN_EN, MESSAGES};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        .replace("\r\n", "\n")
}

fn files(dir: &str, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root().join(dir)];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == ext) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The text of the conformance cases and the examples.
fn corpus() -> String {
    let mut text = String::new();
    for p in files("conformance", "tok")
        .into_iter()
        .chain(files("examples", "tok"))
    {
        text.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
        text.push('\n');
    }
    text
}

/// Whether `word` stands in `text` as a word of the language: not inside a longer
/// identifier or attribute key.
fn has_word(text: &str, word: &str) -> bool {
    let part = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(part) && !after.is_some_and(part)
    })
}

/// The string literals of the `pub const NAME: &[&str]` tables of a source file whose name
/// ends with one of `suffixes`.
fn tables(source: &str, suffixes: &[&str]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(i) = rest.find("pub const ") {
        rest = &rest[i + "pub const ".len()..];
        let Some(colon) = rest.find(':') else { break };
        let name = rest[..colon].trim().to_string();
        let Some(body) = rest[colon..].strip_prefix(": &[&str] = &[") else {
            continue;
        };
        let Some(end) = body.find("];") else { break };
        if suffixes.iter().any(|s| name.ends_with(s)) {
            let items = body[..end]
                .split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect();
            out.push((name, items));
        }
    }
    out
}

fn report(what: &str, missing: &[String]) {
    assert!(missing.is_empty(), "{what}:\n  {}", missing.join("\n  "));
}

#[test]
fn diagnostic_codes_are_documented() {
    let doc = read("docs/diagnostics.md");
    let rows: Vec<&str> = doc.lines().filter(|l| l.starts_with("| ")).collect();
    let row = |code: &str| {
        rows.iter()
            .find(|l| l.starts_with(&format!("| {code} |")))
            .copied()
    };
    let mut missing = Vec::new();
    for (code, _, en, pl) in MESSAGES {
        match row(code) {
            None => missing.push(format!("{code}: no row in docs/diagnostics.md")),
            // codes with a detail text show its forms instead of the template
            Some(_) if en.starts_with("{detail") => {}
            Some(r) => {
                let cell = |t: &str| t.replace('|', "\\|");
                if !r.contains(&cell(en)) || !r.contains(&cell(pl)) {
                    missing.push(format!(
                        "{code}: the row does not give the messages '{en}' and '{pl}'"
                    ));
                }
            }
        }
    }
    let rules = read("docs/bpmn-rules.md");
    for (code, en) in BPMN_EN {
        match row(code) {
            None => missing.push(format!("{code}: no row in docs/diagnostics.md")),
            Some(r) if !r.contains(&en.replace('|', "\\|")) => {
                missing.push(format!("{code}: the row does not give the message '{en}'"));
            }
            Some(_) => {}
        }
        if !has_word(&rules, code) {
            missing.push(format!("{code}: not in docs/bpmn-rules.md"));
        }
    }
    // every code the BPMN validator reports has its English text (its codes C… belong to
    // the `--camunda` mode of `bpmn.py`, which `tok check` does not run)
    let is_code = |s: &str, first: &[u8]| {
        let b = s.as_bytes();
        b.len() == 4 && first.contains(&b[0]) && b[1..].iter().all(u8::is_ascii_digit)
    };
    let validator = read("rust/tok-bpmn/src/validate.rs");
    let english: BTreeSet<&str> = BPMN_EN.iter().map(|b| b.0).collect();
    for part in validator.split('"').skip(1).step_by(2) {
        if is_code(part, b"EWL") && !english.contains(part) {
            missing.push(format!(
                "{part}: reported by tok-bpmn without an English text"
            ));
        }
    }
    // the codes of the engine: the catalogue and the diagnostics written with their own
    // text (the executable profile)
    let mut known: BTreeSet<String> = MESSAGES
        .iter()
        .map(|m| m.0.to_string())
        .chain(english.iter().map(|c| c.to_string()))
        .collect();
    for p in files("rust/tok-core/src", "rs") {
        let src = std::fs::read_to_string(&p).unwrap_or_default();
        for part in src.split('"').skip(1).step_by(2) {
            if is_code(part, b"TPRDMFSC") && !known.contains(part) {
                if row(part).is_none() {
                    missing.push(format!(
                        "{part}: reported by tok-core, no row in docs/diagnostics.md"
                    ));
                }
                known.insert(part.to_string());
            }
        }
    }
    // and nothing documented that does not exist
    for r in &rows {
        let code = r[2..].split(' ').next().unwrap_or("");
        if code.len() == 4
            && code.as_bytes()[0].is_ascii_uppercase()
            && code[1..].bytes().all(|b| b.is_ascii_digit())
            && !known.contains(code)
        {
            missing.push(format!("{code}: documented, but not a code of the engine"));
        }
    }
    report("diagnostic codes", &missing);
}

/// The keywords of the language: the statements the language server describes (its list
/// is checked against the parser and the grammar of the VS Code extension).
fn keywords() -> Vec<String> {
    let src = read("rust/tok-lsp/src/keywords.rs");
    let start = src.find("pub const KEYWORDS").unwrap_or(0);
    let end = start + src[start..].find("\n];").unwrap_or(0);
    src[start..end]
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            let t = t
                .strip_prefix("(\"")
                .or_else(|| (t.starts_with('"') && t.ends_with("\",")).then(|| &t[1..]))?;
            let w = &t[..t.find('"')?];
            (!w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase())).then(|| w.to_string())
        })
        .collect()
}

#[test]
fn keywords_attributes_and_flags_are_documented_with_examples() {
    let spec = read("docs/spec.md");
    let cheat = read("docs/cheatsheet.md");
    let corpus = corpus();
    let mut missing = Vec::new();
    let kws = keywords();
    assert!(kws.len() > 50, "the keyword list was not found: {kws:?}");
    for kw in &kws {
        if !has_word(&spec, kw) {
            missing.push(format!("keyword '{kw}': not in docs/spec.md"));
        }
        if !has_word(&cheat, kw) {
            missing.push(format!("keyword '{kw}': not in docs/cheatsheet.md"));
        }
        let used = corpus
            .lines()
            .any(|l| l.trim_start().split(' ').next() == Some(kw.as_str()));
        if !used {
            missing.push(format!(
                "keyword '{kw}': no example in conformance/ or examples/"
            ));
        }
    }
    let model = read("rust/tok-core/src/model.rs");
    let mut keys = BTreeSet::new();
    for (table, items) in tables(&model, &["_KEYS", "_KEYS_06", "_KEYS_07", "_FLAGS"]) {
        for k in items {
            keys.insert((k, table.clone()));
        }
    }
    assert!(keys.len() > 100, "the attribute tables were not found");
    let mut seen = BTreeSet::new();
    for (k, table) in &keys {
        if !seen.insert(k.clone()) {
            continue;
        }
        let assign = format!("{k}=");
        if !spec.contains(&assign) && !spec.contains(&format!("`{k}`")) && !has_word(&spec, k) {
            missing.push(format!("attribute '{k}' ({table}): not in docs/spec.md"));
        }
        if !cheat.contains(&assign) && !has_word(&cheat, k) {
            missing.push(format!(
                "attribute '{k}' ({table}): not in docs/cheatsheet.md"
            ));
        }
        if !corpus.contains(&assign) && !has_word(&corpus, k) {
            missing.push(format!(
                "attribute '{k}' ({table}): no example in conformance/ or examples/"
            ));
        }
    }
    report("keywords and attributes", &missing);
}

fn tok(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_tok"))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("cannot run tok: {e}"));
    String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
}

#[test]
fn commands_and_options_are_documented() {
    let cli = read("docs/cli.md");
    let help = tok(&["--help"]);
    let line = help
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no list of commands in:\n{help}"));
    let commands: Vec<&str> = line.trim().trim_matches(['{', '}']).split(',').collect();
    assert!(commands.len() >= 9, "commands: {commands:?}");
    let mut missing = Vec::new();
    for cmd in &commands {
        let synopsis = cli
            .lines()
            .find(|l| l.starts_with(&format!("tok {cmd}")))
            .unwrap_or("");
        if synopsis.is_empty() {
            missing.push(format!("tok {cmd}: no synopsis line in docs/cli.md"));
        }
        if !cli.contains(&format!("| `{cmd}` |")) {
            missing.push(format!("tok {cmd}: no row in the table of docs/cli.md"));
        }
        if *cmd == "cheatsheet" {
            continue;
        }
        let usage = tok(&[cmd, "--help"]);
        let first = usage.lines().next().unwrap_or("");
        for opt in first.split(['[', ']', ' ']).filter(|w| w.starts_with('-')) {
            if opt == "-h" {
                continue;
            }
            if !has_word(synopsis, opt) {
                missing.push(format!(
                    "tok {cmd} {opt}: not in the synopsis of docs/cli.md"
                ));
            }
        }
    }
    for opt in ["--version", "--help"] {
        if !cli.contains(opt) {
            missing.push(format!("tok {opt}: not in docs/cli.md"));
        }
    }
    report("command line", &missing);
}

#[test]
fn interfaces_are_documented() {
    let mut missing = Vec::new();
    // the C interface
    let header = read("rust/tok-capi/include/tok.h");
    let capi = read("docs/c-api.md");
    for part in header.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if part.starts_with("tok_") && header.contains(&format!("{part}(")) && !capi.contains(part)
        {
            missing.push(format!("C function {part}: not in docs/c-api.md"));
        }
    }
    // the Python module tok.api
    let api = read("python/src/tok/api.py");
    let python = read("docs/python.md");
    let all = api
        .split("__all__ = [")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .unwrap_or("");
    let names: Vec<&str> = all.split('"').skip(1).step_by(2).collect();
    assert!(!names.is_empty(), "no __all__ in tok/api.py");
    for f in names {
        if !python.contains(&format!("`{f}(")) {
            missing.push(format!(
                "Python function tok.api.{f}: not in docs/python.md"
            ));
        }
    }
    // the R package: every exported function has a help page
    let namespace = read("r/tokr/NAMESPACE");
    let mut aliases = String::new();
    for p in files("r/tokr/man", "Rd") {
        aliases.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
    }
    for l in namespace.lines() {
        if let Some(f) = l.strip_prefix("export(").and_then(|r| r.strip_suffix(')'))
            && !aliases.contains(&format!("\\alias{{{f}}}"))
        {
            missing.push(format!("R function {f}: no help page in r/tokr/man"));
        }
    }
    // the tools of the MCP server
    let tools = read("rust/tok-mcp/src/tools.rs");
    let mcp = read("docs/mcp.md");
    for part in tools.split("\"name\": \"").skip(1) {
        let name = part.split('"').next().unwrap_or("");
        if !mcp.contains(&format!("`{name}`")) {
            missing.push(format!("MCP tool {name}: not in docs/mcp.md"));
        }
    }
    // the methods of the language server
    let server = read("rust/tok-lsp/src/server.rs");
    let lsp = read("docs/lsp.md");
    for part in server.split('"').skip(1).step_by(2) {
        let method = part.starts_with("textDocument/")
            || part.starts_with("workspace/")
            || part.starts_with("tok/");
        if method && !lsp.contains(part) {
            missing.push(format!("LSP method {part}: not in docs/lsp.md"));
        }
    }
    report("interfaces", &missing);
}

#[test]
fn reference_copies_are_identical() {
    let mut missing = Vec::new();
    let pairs = [
        ("docs/cheatsheet.md", "python/src/tok/cheatsheet.md"),
        (
            "docs/cheatsheet.md",
            "skills/tok-modelling/references/cheatsheet.md",
        ),
        ("docs/spec.md", "skills/tok-modelling/references/spec.md"),
        (
            "docs/diagnostics.md",
            "skills/tok-modelling/references/diagnostics.md",
        ),
        ("docs/feel.md", "skills/tok-modelling/references/feel.md"),
        ("docs/cheatsheet.md", "rust/tok-cli/reference/cheatsheet.md"),
        ("docs/spec.md", "rust/tok-cli/reference/spec.md"),
        (
            "docs/diagnostics.md",
            "rust/tok-cli/reference/diagnostics.md",
        ),
        ("docs/feel.md", "rust/tok-cli/reference/feel.md"),
        ("docs/cli.md", "rust/tok-cli/reference/cli.md"),
    ];
    for (a, b) in pairs {
        if read(a) != read(b) {
            missing.push(format!("{b} differs from {a}"));
        }
    }
    report("copies", &missing);
}
