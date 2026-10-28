//! Command line interface of the tok process language: the same commands, options and
//! output as `python -m tok` (language 0.3), and `tok sim --xes`, which the frozen
//! reference does not have.

use std::path::{Path, PathBuf};

use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::diagnostics::{Diag, format_diag};
use tok_core::json::{Json, obj};
use tok_core::{dmn, export, forms, import, printer};

const EXIT_OK: u8 = 0;
const EXIT_ERRORS: u8 = 1;
const EXIT_USAGE: u8 = 2;

use reference::CHEATSHEET;

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
    ($($t:tt)*) => { $crate::emit(&format!($($t)*), false) };
}

macro_rules! outln {
    ($($t:tt)*) => { $crate::emit(&(format!($($t)*) + "\n"), false) };
}

macro_rules! errln {
    ($($t:tt)*) => { $crate::emit(&(format!($($t)*) + "\n"), true) };
}

pub mod api;
mod argparse;
pub mod reference;

use argparse::{Conv, Opt};

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
    class: Option<String>,
    no_sidecar: bool,
}

fn usage(msg: &str) -> u8 {
    errln!(
        "usage: tok [-h] [--version] {{check,fmt,bpmn,svg,export,import,sim,eval,cheatsheet}} ..."
    );
    errln!("tok: error: {msg}");
    EXIT_USAGE
}

/// Run the command line with the arguments after the program name; returns the exit
/// status. Output goes to the standard output and error streams of the process.
pub fn main_with_args(argv: &[String]) -> u8 {
    match run(argv) {
        Ok(code) | Err(code) => code,
    }
}

fn run(argv: &[String]) -> Result<u8, u8> {
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
        "eval" => return eval_command(&argv[1..]),
        "sim" => return sim_command(&argv[1..]),
        _ => {}
    }
    let spec = match cmd.as_str() {
        "check" => &CMD_CHECK,
        "fmt" => &CMD_FMT,
        "bpmn" => &CMD_BPMN,
        "svg" => &CMD_SVG,
        "export" => &CMD_EXPORT,
        "import" => &CMD_IMPORT,
        other => {
            return Err(usage(&format!(
                "argument cmd: invalid choice: '{other}' (choose from 'check', 'fmt', 'bpmn', 'svg', 'export', 'import', 'sim', 'eval', 'cheatsheet')"
            )));
        }
    };
    let p = spec.parse(&argv[1..])?;
    let a = Args {
        cmd: cmd.clone(),
        file: p.positional("file").concat(),
        lang: lang_of(&p),
        json: p.flag("json"),
        level: p.value("level").unwrap_or("full").to_string(),
        write: p.flag("write"),
        check: p.flag("check"),
        output: p.value("output").map(str::to_string),
        no_layout: p.flag("no_layout"),
        no_flow_ids: p.flag("no_flow_ids"),
        class: p.value("class").map(str::to_string),
        no_sidecar: p.flag("no_sidecar"),
    };
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
/// A file that cannot be read is reported; the error is the exit status.
fn read(path: &str) -> Result<String, u8> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).to_string();
            Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
        }
        Err(e) => {
            errln!(
                "tok: cannot read {path}: {}",
                tok_sim::result::os_error(&e, path).message
            );
            Err(EXIT_USAGE)
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

fn load(a: &Args, bpmn: bool) -> Result<(tok_core::Document, Vec<Diag>), u8> {
    let rules = Rules;
    Ok(tok_core::load(
        &read(&a.file)?,
        &a.level,
        if bpmn { Some(&rules) } else { None },
    ))
}

fn report(a: &Args, diags: &[Diag]) -> usize {
    report_class(a, diags, None)
}

/// Diagnostics as text or JSON; `class` is the BPMN conformance class of the document
/// (`tok check --json`).
fn report_class(a: &Args, diags: &[Diag], class: Option<&str>) -> usize {
    let errors = diags.iter().filter(|d| d.is_error()).count();
    if a.json {
        let mut fields = vec![
            ("file", Json::Str(a.file.clone())),
            ("errors", Json::Int(errors as i64)),
        ];
        if let Some(c) = class {
            fields.push(("bpmn_class", Json::Str(c.to_string())));
        }
        fields.push((
            "diagnostics",
            Json::Arr(diags.iter().map(|d| d.to_json(&a.lang)).collect()),
        ));
        let out = obj(fields);
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
    let (doc, mut diags) = match load(a, true) {
        Ok(r) => r,
        Err(code) => return code,
    };
    if let Some(class) = &a.class {
        diags.extend(tok_core::classes::check_class(&doc, class));
        tok_core::diagnostics::sort(&mut diags);
    }
    let class = tok_core::classes::bpmn_class(&doc);
    if report_class(a, &diags, Some(class)) > 0 {
        EXIT_ERRORS
    } else {
        EXIT_OK
    }
}

fn cmd_fmt(a: &Args) -> u8 {
    let text = match read(&a.file) {
        Ok(t) => t,
        Err(code) => return code,
    };
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

/// The layout sidecar `FILE.layout` next to the document (`tok import` writes it), unless
/// `--no-sidecar` is given or there is none.
fn sidecar(a: &Args) -> Result<Option<tok_bpmn::Sidecar>, String> {
    if a.no_sidecar {
        return Ok(None);
    }
    let path = format!("{}.layout", a.file);
    match std::fs::read_to_string(&path) {
        Ok(text) => tok_bpmn::Sidecar::parse(&text)
            .map(Some)
            .map_err(|e| format!("{path}: {e}")),
        Err(_) => Ok(None),
    }
}

/// BPMN XML with layout (or without, `--no-layout`), as `tok.io.write_bpmn` writes it; the
/// geometry of the sidecar is kept.
fn bpmn_text(
    doc: &tok_core::Document,
    layout: bool,
    side: Option<&tok_bpmn::Sidecar>,
) -> Result<String, String> {
    let xml = export::export_xml(doc, None);
    if layout {
        return match side {
            Some(s) => tok_bpmn::layout_with(&xml, s),
            None => tok_bpmn::layout(&xml),
        }
        .map_err(|e| e.to_string());
    }
    let mut root = export::export_tree(doc, None);
    root.indent(0);
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n",
        root.to_string_root()
    ))
}

fn svg_text(
    doc: &tok_core::Document,
    path: &Path,
    side: Option<&tok_bpmn::Sidecar>,
) -> Result<String, String> {
    let xml = export::export_xml(doc, None);
    let title = path.file_name().map(|n| n.to_string_lossy().to_string());
    match side {
        Some(s) => tok_bpmn::render_svg_with(&xml, title.as_deref(), s),
        None => tok_bpmn::render_svg(&xml, title.as_deref()),
    }
    .map_err(|e| e.to_string())
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
    let (doc, diags) = match load(a, true) {
        Ok(r) => r,
        Err(code) => return code,
    };
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
    let side = match sidecar(a) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    match bpmn_text(&doc, !a.no_layout, side.as_ref())
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
    let (doc, diags) = match load(a, true) {
        Ok(r) => r,
        Err(code) => return code,
    };
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
    let side = match sidecar(a) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    match svg_text(&doc, Path::new(&path), side.as_ref())
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
    let (doc, diags) = match load(a, true) {
        Ok(r) => r,
        Err(code) => return code,
    };
    if diags.iter().any(Diag::is_error) {
        report(a, &diags);
        return EXIT_ERRORS;
    }
    let has_drg = doc.lang_07() && (!doc.decisions.is_empty() || !doc.bkms.is_empty());
    if !has_drg && let Some(code) = nothing_to_draw(a, &doc) {
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
    // a decision model of language 0.7 without a process has no BPMN file
    let decisions_only = doc.lang_07() && doc.nodes.is_empty() && doc.pools.is_empty();
    let mut written = Vec::new();
    if !decisions_only {
        written.push(bpmn_path.clone());
        written.push(svg_path.clone());
        let side = match sidecar(a) {
            Ok(s) => s,
            Err(e) => return fail(&e),
        };
        let res = bpmn_text(&doc, true, side.as_ref())
            .and_then(|t| write_platform(&bpmn_path, &t).map_err(|e| e.to_string()))
            .and_then(|_| svg_text(&doc, &svg_path, side.as_ref()))
            .and_then(|t| write_platform(&svg_path, &t).map_err(|e| e.to_string()));
        if let Err(e) = res {
            return fail(&e);
        }
    }
    if doc.lang_07() {
        // language 0.7: one DMN 1.5 file and one picture per decision requirements graph
        let pictures = tok_core::drg::export::export_svg(&doc);
        for ((stem, xml), (_, svg)) in tok_core::drg::export::export_dmn15(&doc)
            .into_iter()
            .zip(pictures)
        {
            let p = folder.join(format!("{stem}.dmn"));
            if let Err(e) = write_lf(&p, &xml) {
                return fail(&e.to_string());
            }
            written.push(p);
            let p = folder.join(format!("{stem}.drd.svg"));
            if let Err(e) = write_lf(&p, &svg) {
                return fail(&e.to_string());
            }
            written.push(p);
        }
    } else {
        for dec in doc.decisions.values() {
            let p = folder.join(format!("{}.dmn", dec.base.id()));
            if let Err(e) = write_lf(&p, &dmn::export_dmn(dec, Some(&doc))) {
                return fail(&e.to_string());
            }
            written.push(p);
        }
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
    if a.file.to_lowercase().ends_with(".dmn") || tok_core::drg::dmnimport::is_dmn(&xml) {
        return import_dmn(a, &xml);
    }
    let (doc, warnings, ids) = match import::import_bpmn_mapped(&xml, !a.no_flow_ids) {
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
            if !a.no_layout {
                // the diagram of the file, kept next to the document (`FILE.tok.layout`)
                let map: std::collections::HashMap<String, String> = ids.into_iter().collect();
                let exported = export::export_xml(&doc, None);
                match tok_bpmn::extract_sidecar(&xml, &exported, &map) {
                    Ok(side) if !side.is_empty() => {
                        let path = format!("{o}.layout");
                        if let Err(e) = write_lf(Path::new(&path), &side.to_text()) {
                            return fail(&e.to_string());
                        }
                        outln!("layout: {path}");
                    }
                    Ok(_) => {}
                    Err(e) => return fail(&e.to_string()),
                }
            }
        }
        None => out!("{out}"),
    }
    EXIT_OK
}

/// `tok import FILE.dmn`: a decision model as a document of language 0.7.
fn import_dmn(a: &Args, xml: &str) -> u8 {
    let (doc, warnings) = match tok_core::drg::dmnimport::import_dmn(xml) {
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

// ------------------------------------------------------------------------- eval

const LANGS: &[&str] = &["en", "pl"];

/// Parsers of the commands that read one document, as `cli.build_parser` defines them.
const LEVELS: &[&str] = &["basic", "analytic", "full"];

const CMD_CHECK: argparse::Spec = argparse::Spec {
    name: "check",
    usage: "usage: tok check [-h] [--level {basic,analytic,full}] \
            [--class {descriptive,analytic,common-executable,full}] [--lang {en,pl}] [--json] file",
    help: "usage: tok check [-h] [--level {basic,analytic,full}] \
           [--class {descriptive,analytic,common-executable,full}] [--lang {en,pl}] [--json] file

positional arguments:
  file

options:
  -h, --help            show this help message and exit
  --level {basic,analytic,full}
  --class {descriptive,analytic,common-executable,full}
                        BPMN 2.0.2 conformance class to check against (R075)
  --lang {en,pl}        language of messages (default: en, or TOK_LANG)
  --json                machine-readable output
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("--level", Opt::Value("level", Conv::Choice(LEVELS))),
        (
            "--class",
            Opt::Value("class", Conv::Choice(tok_core::classes::CLASSES)),
        ),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
        ("--json", Opt::Flag("json")),
    ],
    positionals: &[("file", false)],
};

const CMD_FMT: argparse::Spec = argparse::Spec {
    name: "fmt",
    usage: "usage: tok fmt [-h] [-w] [--check] [--lang {en,pl}] [--json] file",
    help: "usage: tok fmt [-h] [-w] [--check] [--lang {en,pl}] [--json] file

positional arguments:
  file

options:
  -h, --help      show this help message and exit
  -w, --write     rewrite the file in place
  --check         only report whether the file is canonical
  --lang {en,pl}  language of messages (default: en, or TOK_LANG)
  --json          machine-readable output
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("-w", Opt::Flag("write")),
        ("--write", Opt::Flag("write")),
        ("--check", Opt::Flag("check")),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
        ("--json", Opt::Flag("json")),
    ],
    positionals: &[("file", false)],
};

const CMD_BPMN: argparse::Spec = argparse::Spec {
    name: "bpmn",
    usage: "usage: tok bpmn [-h] [-o OUTPUT] [--no-layout] [--no-sidecar] [--lang {en,pl}] [--json] file",
    help: "usage: tok bpmn [-h] [-o OUTPUT] [--no-layout] [--no-sidecar] [--lang {en,pl}] [--json] file

positional arguments:
  file

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
  --no-layout           semantic model only, no BPMNDI
  --no-sidecar          ignore the layout sidecar FILE.layout
  --lang {en,pl}        language of messages (default: en, or TOK_LANG)
  --json                machine-readable output
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("-o", Opt::Value("output", Conv::Text)),
        ("--output", Opt::Value("output", Conv::Text)),
        ("--no-layout", Opt::Flag("no_layout")),
        ("--no-sidecar", Opt::Flag("no_sidecar")),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
        ("--json", Opt::Flag("json")),
    ],
    positionals: &[("file", false)],
};

const CMD_SVG: argparse::Spec = argparse::Spec {
    name: "svg",
    usage: "usage: tok svg [-h] [-o OUTPUT] [--no-sidecar] [--lang {en,pl}] [--json] file",
    help: "usage: tok svg [-h] [-o OUTPUT] [--no-sidecar] [--lang {en,pl}] [--json] file

positional arguments:
  file

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
  --no-sidecar          ignore the layout sidecar FILE.layout
  --lang {en,pl}        language of messages (default: en, or TOK_LANG)
  --json                machine-readable output
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("-o", Opt::Value("output", Conv::Text)),
        ("--output", Opt::Value("output", Conv::Text)),
        ("--no-sidecar", Opt::Flag("no_sidecar")),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
        ("--json", Opt::Flag("json")),
    ],
    positionals: &[("file", false)],
};

const CMD_EXPORT: argparse::Spec = argparse::Spec {
    name: "export",
    usage: "usage: tok export [-h] [-o OUTPUT] [--no-sidecar] [--lang {en,pl}] [--json] file",
    help: "usage: tok export [-h] [-o OUTPUT] [--no-sidecar] [--lang {en,pl}] [--json] file

positional arguments:
  file

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
                        output folder (default: folder of the file)
  --no-sidecar          ignore the layout sidecar FILE.layout
  --lang {en,pl}        language of messages (default: en, or TOK_LANG)
  --json                machine-readable output
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("-o", Opt::Value("output", Conv::Text)),
        ("--output", Opt::Value("output", Conv::Text)),
        ("--no-sidecar", Opt::Flag("no_sidecar")),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
        ("--json", Opt::Flag("json")),
    ],
    positionals: &[("file", false)],
};

const CMD_IMPORT: argparse::Spec = argparse::Spec {
    name: "import",
    usage: "usage: tok import [-h] [-o OUTPUT] [--no-flow-ids] [--no-layout] [--lang {en,pl}] file",
    help: "usage: tok import [-h] [-o OUTPUT] [--no-flow-ids] [--no-layout] [--lang {en,pl}] file

positional arguments:
  file

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
  --no-flow-ids         drop the identifiers of sequence flows so they print as compact chains
  --no-layout           do not write the diagram into the layout sidecar OUTPUT.layout
  --lang {en,pl}        language of messages (default: en, or TOK_LANG)
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("-o", Opt::Value("output", Conv::Text)),
        ("--output", Opt::Value("output", Conv::Text)),
        ("--no-flow-ids", Opt::Flag("no_flow_ids")),
        ("--no-layout", Opt::Flag("no_layout")),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
    ],
    positionals: &[("file", false)],
};

const EVAL: argparse::Spec = argparse::Spec {
    name: "eval",
    usage: "usage: tok eval [-h] [--lang {en,pl}] file decision [values ...]",
    help: "usage: tok eval [-h] [--lang {en,pl}] file decision [values ...]

positional arguments:
  file
  decision
  values

options:
  -h, --help      show this help message and exit
  --lang {en,pl}  language of messages (default: en, or TOK_LANG)
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
    ],
    positionals: &[("file", false), ("decision", false), ("values", true)],
};

/// The language of messages: `--lang`, else `TOK_LANG`, else English.
fn lang_of(p: &argparse::Parsed) -> String {
    match p.value("lang") {
        Some(l) => l.to_string(),
        None => std::env::var("TOK_LANG").unwrap_or_else(|_| "en".into()),
    }
}

/// A command-line value (`cli._value`): `true`/`false` (any case), a decimal number, or text.
fn cli_value(text: &str) -> tok_core::feel_eval::Value {
    use tok_core::feel_eval::Value;
    let low = text.to_lowercase();
    if low == "true" || low == "false" {
        return Value::Bool(low == "true");
    }
    match tok_core::pydec::Decimal::parse(text) {
        Ok(d) => Value::Num(d),
        Err(_) => Value::Str(text.to_string()),
    }
}

fn eval_command(argv: &[String]) -> Result<u8, u8> {
    let p = EVAL.parse(argv)?;
    let file = p.positional("file").concat();
    let decision = p.positional("decision").concat();
    let a = Args {
        cmd: "eval".into(),
        file: file.clone(),
        lang: lang_of(&p),
        level: "full".into(),
        ..Default::default()
    };
    let (doc, diags) = load(&a, false)?;
    if diags.iter().any(Diag::is_error) {
        report(&a, &diags);
        return Ok(EXIT_ERRORS);
    }
    let Some(dec) = doc.decisions.get(&decision) else {
        errln!("tok: no decision '{decision}' in {file}");
        return Ok(EXIT_USAGE);
    };
    if doc.lang_07() {
        // language 0.7: the decision with what it requires; the result is its DMN value
        let mut inputs = tok_core::feel3::Map::new();
        for pair in &p.positional("values") {
            let Some((k, v)) = pair.split_once('=') else {
                errln!("tok: expected NAME=VALUE, got '{pair}'");
                return Ok(EXIT_USAGE);
            };
            inputs.insert(k.to_string(), tok_core::drg::values::cli_value(v));
        }
        let (value, problems) = tok_core::drg::eval::evaluate(&doc, &decision, &inputs);
        if !problems.is_empty() {
            for problem in problems {
                errln!("tok: {problem}");
            }
            return Ok(EXIT_ERRORS);
        }
        outln!("{}", tok_core::drg::values::to_json(&value).compact());
        return Ok(EXIT_OK);
    }
    let mut ctx = tok_core::feel_eval::Context::new();
    for pair in &p.positional("values") {
        let Some((k, v)) = pair.split_once('=') else {
            errln!("tok: expected NAME=VALUE, got '{pair}'");
            return Ok(EXIT_USAGE);
        };
        ctx.insert(k.to_string(), cli_value(v));
    }
    match dmn::evaluate_decision(dec, &ctx).and_then(|r| r.to_json()) {
        Ok(result) => {
            outln!("{}", result.compact());
            Ok(EXIT_OK)
        }
        Err(err) if err.kind == "tok.dmn.DecisionError" => {
            errln!("tok: {}", err.message);
            Ok(EXIT_ERRORS)
        }
        Err(err) => {
            // an exception the reference implementation does not catch: its traceback
            // ends with this line
            errln!("{err}");
            Ok(EXIT_ERRORS)
        }
    }
}

// -------------------------------------------------------------------------- sim

const SIM_USAGE: &str = "usage: tok sim [-h] [--cases CASES] [--seed SEED] [--log LOG] \
                         [--cases-csv CASES_CSV] [--xes XES] [--lang {en,pl}] [--json] file";

const SIM: argparse::Spec = argparse::Spec {
    name: "sim",
    usage: SIM_USAGE,
    help: "usage: tok sim [-h] [--cases CASES] [--seed SEED] [--log LOG] \
           [--cases-csv CASES_CSV] [--xes XES] [--lang {en,pl}] [--json] file

positional arguments:
  file

options:
  -h, --help            show this help message and exit
  --cases CASES
  --seed SEED
  --log LOG             write the event log (CSV)
  --cases-csv CASES_CSV
                        write one row per case (CSV)
  --xes XES             write the event log (XES, IEEE 1849-2016)
  --lang {en,pl}        language of messages (default: en, or TOK_LANG)
  --json                machine-readable output
",
    options: &[
        ("-h", Opt::Help),
        ("--help", Opt::Help),
        ("--cases", Opt::Value("cases", Conv::Int)),
        ("--seed", Opt::Value("seed", Conv::Int)),
        ("--log", Opt::Value("log", Conv::Text)),
        ("--cases-csv", Opt::Value("cases_csv", Conv::Text)),
        ("--xes", Opt::Value("xes", Conv::Text)),
        ("--lang", Opt::Value("lang", Conv::Choice(LANGS))),
        ("--json", Opt::Flag("json")),
    ],
    positionals: &[("file", false)],
};

/// `tok sim FILE`: simulate the process and print the indicators (`cli.cmd_sim`).
fn sim_command(argv: &[String]) -> Result<u8, u8> {
    let p = SIM.parse(argv)?;
    let a = Args {
        cmd: "sim".into(),
        file: p.positional("file").concat(),
        lang: lang_of(&p),
        json: p.flag("json"),
        level: "full".into(),
        ..Default::default()
    };
    let (doc, diags) = load(&a, true)?;
    if diags.iter().any(Diag::is_error) {
        report(&a, &diags);
        return Ok(EXIT_ERRORS);
    }
    let param = |dest: &str| p.value(dest).map(|v| tok_sim::Param::Int(v.to_string()));
    let res = tok_sim::simulate(&doc, param("cases"), param("seed")).and_then(|result| {
        let log = p.value("log").filter(|v| !v.is_empty());
        let cases_csv = p.value("cases_csv").filter(|v| !v.is_empty());
        if log.is_some() || cases_csv.is_some() {
            result.write_csv(p.value("log"), p.value("cases_csv"))?;
        }
        // not in the reference command line: the event log as XES
        if let Some(path) = p.value("xes").filter(|v| !v.is_empty()) {
            result.write_xes(path)?;
        }
        if a.json {
            Ok(result.to_json(false)?
                + "
")
        } else {
            Ok(result.kpi().format(&a.lang))
        }
    });
    match res {
        Ok(text) => {
            out!("{text}");
            Ok(EXIT_OK)
        }
        Err(err) => {
            // an exception the reference implementation does not catch: its traceback
            // ends with this line
            errln!("{err}");
            Ok(EXIT_ERRORS)
        }
    }
}
