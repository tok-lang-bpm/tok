//! Command line interface of the tok process language: the same commands, options and
//! output as `python -m tok` (version 0.1.1). `sim` and `eval` arrive with tok 0.4.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::diagnostics::{Diag, format_diag};
use tok_core::json::{Json, obj};
use tok_core::{dmn, export, forms, import, printer};

const EXIT_OK: u8 = 0;
const EXIT_ERRORS: u8 = 1;
const EXIT_USAGE: u8 = 2;

const CHEATSHEET: &str = include_str!("../../../python/src/tok/cheatsheet.md");

const HELP: &str =
    "usage: tok [-h] [--version] {check,fmt,bpmn,svg,export,import,sim,eval,cheatsheet} ...

tok: plain-text information-process models (BPMN 2.0, DMN, forms, simulation).

positional arguments:
  {check,fmt,bpmn,svg,export,import,sim,eval,cheatsheet}
    check               check a document and list diagnostics
    fmt                 print the canonical form
    bpmn                write .bpmn next to the file
    svg                 write .svg next to the file
    export              write BPMN, SVG, DMN and form files into a folder
    import              convert a BPMN 2.0 XML file to tok
    sim                 simulate the process and print indicators
    eval                evaluate a decision table: tok eval FILE DECISION name=value …
    cheatsheet          print the one-page language summary

options:
  -h, --help            show this help message and exit
  --version             show program's version number and exit
";

/// Console output like Python's text-mode streams: `\n` becomes the platform line ending.
fn emit(text: &str, to_stderr: bool) {
    use std::io::Write;
    let text = if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.to_string()
    };
    let res = if to_stderr {
        std::io::stderr().write_all(text.as_bytes())
    } else {
        std::io::stdout().write_all(text.as_bytes())
    };
    // a closed pipe (e.g. `tok fmt x | head`) is not an error worth reporting
    let _ = res;
}

macro_rules! out {
    ($($t:tt)*) => { emit(&format!($($t)*), false) };
}

macro_rules! outln {
    ($($t:tt)*) => { emit(&(format!($($t)*) + "\n"), false) };
}

macro_rules! errln {
    ($($t:tt)*) => { emit(&(format!($($t)*) + "\n"), true) };
}

/// BPMN rules from the `tok-bpmn` crate.
struct Rules;

impl BpmnRules for Rules {
    fn validate(&self, xml: &str) -> Vec<BpmnIssue> {
        tok_bpmn::validate(xml)
            .unwrap_or_default()
            .into_iter()
            .map(|i| BpmnIssue {
                level: match i.level {
                    tok_bpmn::Level::Error => "błąd",
                    tok_bpmn::Level::Warning => "ostrzeżenie",
                    tok_bpmn::Level::Note => "uwaga",
                }
                .to_string(),
                code: i.code,
                id: i.id,
                message: i.message,
            })
            .collect()
    }
}

#[derive(Debug, Default)]
struct Args {
    cmd: String,
    file: String,
    lang: String,
    json: bool,
    level: String,
    write: bool,
    check: bool,
    output: Option<String>,
    no_layout: bool,
    no_flow_ids: bool,
}

fn usage(msg: &str) -> ExitCode {
    errln!(
        "usage: tok [-h] [--version] {{check,fmt,bpmn,svg,export,import,sim,eval,cheatsheet}} ..."
    );
    errln!("tok: error: {msg}");
    ExitCode::from(EXIT_USAGE)
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(code) => ExitCode::from(code),
        Err(code) => code,
    }
}

fn run(argv: &[String]) -> Result<u8, ExitCode> {
    let Some(cmd) = argv.first() else {
        out!("{HELP}");
        return Ok(EXIT_USAGE);
    };
    match cmd.as_str() {
        "-h" | "--help" => {
            out!("{HELP}");
            return Ok(EXIT_OK);
        }
        "--version" => {
            outln!("tok {}", env!("CARGO_PKG_VERSION"));
            return Ok(EXIT_OK);
        }
        "cheatsheet" => {
            out!("{CHEATSHEET}");
            return Ok(EXIT_OK);
        }
        "check" | "fmt" | "bpmn" | "svg" | "export" | "import" => {}
        "sim" | "eval" => {
            errln!(
                "tok: '{cmd}' is not available in this build (planned for tok 0.4); use the Python implementation"
            );
            return Ok(EXIT_USAGE);
        }
        other => {
            return Err(usage(&format!(
                "argument command: invalid choice: '{other}'"
            )));
        }
    }
    let mut a = Args {
        cmd: cmd.clone(),
        lang: std::env::var("TOK_LANG").unwrap_or_else(|_| "en".into()),
        level: "full".into(),
        ..Default::default()
    };
    let mut positional = Vec::new();
    let mut i = 1;
    while i < argv.len() {
        let arg = argv[i].as_str();
        let value = |i: &mut usize| -> Result<String, ExitCode> {
            *i += 1;
            argv.get(*i)
                .cloned()
                .ok_or_else(|| usage(&format!("argument {arg}: expected one argument")))
        };
        match arg {
            "--lang" if a.cmd != "cheatsheet" => {
                let v = value(&mut i)?;
                if v != "en" && v != "pl" {
                    return Err(usage(&format!(
                        "argument --lang: invalid choice: '{v}' (choose from 'en', 'pl')"
                    )));
                }
                a.lang = v;
            }
            "--json" if a.cmd != "import" => a.json = true,
            "--level" if a.cmd == "check" => {
                let v = value(&mut i)?;
                if !["basic", "analytic", "full"].contains(&v.as_str()) {
                    return Err(usage(&format!(
                        "argument --level: invalid choice: '{v}' (choose from 'basic', 'analytic', 'full')"
                    )));
                }
                a.level = v;
            }
            "-w" | "--write" if a.cmd == "fmt" => a.write = true,
            "--check" if a.cmd == "fmt" => a.check = true,
            "-o" | "--output" if ["bpmn", "svg", "export", "import"].contains(&a.cmd.as_str()) => {
                a.output = Some(value(&mut i)?)
            }
            "--no-layout" if a.cmd == "bpmn" => a.no_layout = true,
            "--no-flow-ids" if a.cmd == "import" => a.no_flow_ids = true,
            "-h" | "--help" => {
                outln!("usage: tok {} [-h] file", a.cmd);
                return Ok(EXIT_OK);
            }
            s if s.starts_with('-') && s.len() > 1 => {
                return Err(usage(&format!("unrecognized arguments: {s}")));
            }
            s => positional.push(s.to_string()),
        }
        i += 1;
    }
    match positional.len() {
        0 => return Err(usage("the following arguments are required: file")),
        1 => a.file = positional.remove(0),
        _ => {
            return Err(usage(&format!(
                "unrecognized arguments: {}",
                positional[1..].join(" ")
            )));
        }
    }
    Ok(match a.cmd.as_str() {
        "check" => cmd_check(&a),
        "fmt" => cmd_fmt(&a),
        "bpmn" => cmd_bpmn(&a),
        "svg" => cmd_svg(&a),
        "export" => cmd_export(&a),
        _ => cmd_import(&a),
    })
}

/// Read a text file like Python's `open(path, encoding="utf-8")` (universal newlines).
fn read(path: &str) -> String {
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).to_string();
            text.replace("\r\n", "\n").replace('\r', "\n")
        }
        Err(e) => {
            errln!("tok: cannot read {path}: {e}");
            std::process::exit(EXIT_USAGE as i32);
        }
    }
}

/// Write like Python's `open(path, "w", encoding="utf-8")`: platform line endings.
fn write_platform(path: &Path, text: &str) -> std::io::Result<()> {
    let text = if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.to_string()
    };
    std::fs::write(path, text)
}

/// Write with `newline="\n"`.
fn write_lf(path: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(path, text)
}

fn load(a: &Args, bpmn: bool) -> (tok_core::Document, Vec<Diag>) {
    let rules = Rules;
    tok_core::load(
        &read(&a.file),
        &a.level,
        if bpmn { Some(&rules) } else { None },
    )
}

fn report(a: &Args, diags: &[Diag]) -> usize {
    let errors = diags.iter().filter(|d| d.is_error()).count();
    if a.json {
        let out = obj(vec![
            ("file", Json::Str(a.file.clone())),
            ("errors", Json::Int(errors as i64)),
            (
                "diagnostics",
                Json::Arr(diags.iter().map(|d| d.to_json(&a.lang)).collect()),
            ),
        ]);
        outln!("{}", out.pretty());
    } else {
        for d in diags {
            outln!("{}", format_diag(d, &a.lang, Some(&a.file)));
        }
        let warnings = diags
            .iter()
            .filter(|d| d.severity() == tok_core::Severity::Warning)
            .count();
        let notes = diags.len() - errors - warnings;
        if a.lang == "pl" {
            outln!(
                "{}: błędy {errors}, ostrzeżenia {warnings}, uwagi {notes}",
                a.file
            );
        } else {
            outln!(
                "{}: {errors} error(s), {warnings} warning(s), {notes} note(s)",
                a.file
            );
        }
    }
    errors
}

fn cmd_check(a: &Args) -> u8 {
    let (_, diags) = load(a, true);
    if report(a, &diags) > 0 {
        EXIT_ERRORS
    } else {
        EXIT_OK
    }
}

fn cmd_fmt(a: &Args) -> u8 {
    let text = read(&a.file);
    let (doc, diags) = tok_core::parse(&text);
    if diags.iter().any(Diag::is_error) {
        report(a, &diags);
        return EXIT_ERRORS;
    }
    let out = printer::fmt(&doc);
    if a.check {
        if out != text {
            outln!("{}: not in canonical form", a.file);
            return EXIT_ERRORS;
        }
        return EXIT_OK;
    }
    if a.write {
        if let Err(e) = write_lf(Path::new(&a.file), &out) {
            errln!("tok: cannot write {}: {e}", a.file);
            return EXIT_USAGE;
        }
    } else {
        out!("{out}");
    }
    EXIT_OK
}

/// `os.path.splitext(file)[0] + ext`
fn with_ext(file: &str, ext: &str) -> String {
    let p = Path::new(file);
    match (p.extension(), p.file_name()) {
        (Some(e), Some(name))
            if !name.to_string_lossy().starts_with('.')
                || name.to_string_lossy().matches('.').count() > 1 =>
        {
            let s = file.to_string();
            let cut = s.len() - e.len() - 1;
            format!("{}{ext}", &s[..cut])
        }
        _ => format!("{file}{ext}"),
    }
}

/// BPMN XML with layout (or without, `--no-layout`), as `tok.io.write_bpmn` writes it.
fn bpmn_text(doc: &tok_core::Document, layout: bool) -> Result<String, String> {
    let xml = export::export_xml(doc, None);
    if layout {
        return tok_bpmn::layout(&xml).map_err(|e| e.to_string());
    }
    let mut root = export::export_tree(doc, None);
    root.indent(0);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n",
        root.to_string_root()
    ))
}

fn svg_text(doc: &tok_core::Document, path: &Path) -> Result<String, String> {
    let xml = export::export_xml(doc, None);
    let title = path.file_name().map(|n| n.to_string_lossy().to_string());
    tok_bpmn::render_svg(&xml, title.as_deref()).map_err(|e| e.to_string())
}

fn fail(msg: &str) -> u8 {
    errln!("tok: {msg}");
    EXIT_ERRORS
}

/// A diagram needs at least one flow node or pool.
fn nothing_to_draw(a: &Args, doc: &tok_core::model::Document) -> Option<u8> {
    if doc.nodes.is_empty() && doc.pools.is_empty() {
        let msg = if a.lang == "pl" {
            "nie ma czego narysować: model nie zawiera elementów"
        } else {
            "nothing to draw: the model has no elements"
        };
        return Some(fail(msg));
    }
    None
}

fn cmd_bpmn(a: &Args) -> u8 {
    let (doc, diags) = load(a, true);
    if diags.iter().any(Diag::is_error) {
        report(a, &diags);
        return EXIT_ERRORS;
    }
    if let Some(code) = nothing_to_draw(a, &doc) {
        return code;
    }
    let path = a
        .output
        .clone()
        .unwrap_or_else(|| with_ext(&a.file, ".bpmn"));
    match bpmn_text(&doc, !a.no_layout)
        .and_then(|t| write_platform(Path::new(&path), &t).map_err(|e| e.to_string()))
    {
        Ok(()) => {
            outln!("BPMN: {path}");
            EXIT_OK
        }
        Err(e) => fail(&e),
    }
}

fn cmd_svg(a: &Args) -> u8 {
    let (doc, diags) = load(a, true);
    if diags.iter().any(Diag::is_error) {
        report(a, &diags);
        return EXIT_ERRORS;
    }
    if let Some(code) = nothing_to_draw(a, &doc) {
        return code;
    }
    let path = a
        .output
        .clone()
        .unwrap_or_else(|| with_ext(&a.file, ".svg"));
    match svg_text(&doc, Path::new(&path))
        .and_then(|t| write_platform(Path::new(&path), &t).map_err(|e| e.to_string()))
    {
        Ok(()) => {
            outln!("SVG: {path}");
            EXIT_OK
        }
        Err(e) => fail(&e),
    }
}

/// `os.path.normpath`
fn normpath(p: &Path) -> String {
    let sep = std::path::MAIN_SEPARATOR;
    let s = p
        .to_string_lossy()
        .replace(if sep == '\\' { '/' } else { '\\' }, &sep.to_string());
    let absolute = s.starts_with(sep);
    let (prefix, rest) = match s.find(':') {
        Some(1) if cfg!(windows) => (s[..2].to_string(), s[2..].to_string()),
        _ => (String::new(), s.clone()),
    };
    let rooted = rest.starts_with(sep);
    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split(sep) {
        match part {
            "" | "." => {}
            ".." if parts.last().map(|l| *l != "..").unwrap_or(false) => {
                parts.pop();
            }
            ".." if rooted => {}
            other => parts.push(other),
        }
    }
    let body = parts.join(&sep.to_string());
    let lead = if rooted || absolute {
        sep.to_string()
    } else {
        String::new()
    };
    let out = format!("{prefix}{lead}{body}");
    if out.is_empty() { ".".into() } else { out }
}

fn cmd_export(a: &Args) -> u8 {
    let (doc, diags) = load(a, true);
    if diags.iter().any(Diag::is_error) {
        report(a, &diags);
        return EXIT_ERRORS;
    }
    if let Some(code) = nothing_to_draw(a, &doc) {
        return code;
    }
    let folder = match &a.output {
        Some(o) => PathBuf::from(o),
        None => std::path::absolute(&a.file)
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    if let Err(e) = std::fs::create_dir_all(&folder) {
        return fail(&e.to_string());
    }
    let stem = Path::new(&a.file)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let base = Path::new(&a.file)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = if base.starts_with('.') && base.matches('.').count() == 1 {
        base
    } else {
        stem
    };
    let bpmn_path = folder.join(format!("{stem}.bpmn"));
    let svg_path = folder.join(format!("{stem}.svg"));
    let mut written = vec![bpmn_path.clone(), svg_path.clone()];
    let res = bpmn_text(&doc, true)
        .and_then(|t| write_platform(&bpmn_path, &t).map_err(|e| e.to_string()))
        .and_then(|_| svg_text(&doc, &svg_path))
        .and_then(|t| write_platform(&svg_path, &t).map_err(|e| e.to_string()));
    if let Err(e) = res {
        return fail(&e);
    }
    for dec in doc.decisions.values() {
        let p = folder.join(format!("{}.dmn", dec.base.id()));
        if let Err(e) = write_lf(&p, &dmn::export_dmn(dec, Some(&doc))) {
            return fail(&e.to_string());
        }
        written.push(p);
    }
    for form in doc.forms.values() {
        let p = folder.join(format!("{}.form", form.base.id()));
        if let Err(e) = write_lf(&p, &forms::export_form(form)) {
            return fail(&e.to_string());
        }
        written.push(p);
    }
    for p in written {
        outln!("{}", normpath(&p));
    }
    EXIT_OK
}

fn cmd_import(a: &Args) -> u8 {
    let xml = match std::fs::read(&a.file) {
        Ok(b) => String::from_utf8_lossy(&b).to_string(),
        Err(e) => {
            errln!("tok: cannot read {}: {e}", a.file);
            return EXIT_USAGE;
        }
    };
    let (doc, warnings) = match import::import_bpmn(&xml, !a.no_flow_ids) {
        Ok(r) => r,
        Err(e) => return fail(&e.to_string()),
    };
    for w in warnings {
        errln!("{}: {w}", a.file);
    }
    let out = printer::fmt(&doc);
    match &a.output {
        Some(o) => {
            if let Err(e) = write_lf(Path::new(o), &out) {
                return fail(&e.to_string());
            }
            outln!("tok: {o}");
        }
        None => out!("{out}"),
    }
    EXIT_OK
}
