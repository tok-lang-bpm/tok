//! Command line interface of the tok process language: the same commands, options and
//! output as `python -m tok` (language 0.3). `sim` arrives with tok 0.4.

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
        "eval" => return eval_command(&argv[1..]),
        "sim" => {
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

// ------------------------------------------------------------------------- eval

const EVAL_USAGE: &str = "usage: tok eval [-h] [--lang {en,pl}] file decision [values ...]";

const EVAL_HELP: &str = "usage: tok eval [-h] [--lang {en,pl}] file decision [values ...]

positional arguments:
  file
  decision
  values

options:
  -h, --help      show this help message and exit
  --lang {en,pl}  language of messages (default: en, or TOK_LANG)
";

/// An error of the `eval` subcommand's argument parser.
fn eval_usage(msg: &str) -> ExitCode {
    errln!("{EVAL_USAGE}");
    errln!("tok eval: error: {msg}");
    ExitCode::from(EXIT_USAGE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvalOpt {
    Help,
    Lang,
}

/// Option strings of `tok eval`, in declaration order.
const EVAL_OPTIONS: [(&str, EvalOpt); 3] = [
    ("-h", EvalOpt::Help),
    ("--help", EvalOpt::Help),
    ("--lang", EvalOpt::Lang),
];

fn eval_option(s: &str) -> Option<EvalOpt> {
    EVAL_OPTIONS.iter().find(|o| o.0 == s).map(|o| o.1)
}

/// An argument that looks like an option: (action, option string, separator before the
/// explicit argument: `=`, empty for `-hX`, or none; explicit argument).
type OptTuple = (
    Option<EvalOpt>,
    String,
    Option<&'static str>,
    Option<String>,
);

/// argparse `_negative_number_matcher`: `^-\d+$|^-\d*\.\d+$`.
fn negative_number(s: &str) -> bool {
    let Some(body) = s.strip_prefix('-') else {
        return false;
    };
    let digit = |c: char| tok_core::pyutil::decimal_value(c).is_some();
    if !body.is_empty() && body.chars().all(digit) {
        return true;
    }
    match body.split_once('.') {
        Some((a, b)) => a.chars().all(digit) && !b.is_empty() && b.chars().all(digit),
        None => false,
    }
}

/// argparse `_parse_optional` for the `eval` subparser; `Err` is an ambiguity error.
fn eval_parse_optional(arg: &str) -> Result<Option<OptTuple>, String> {
    if !arg.starts_with('-') {
        return Ok(None);
    }
    if let Some(o) = eval_option(arg) {
        return Ok(Some((Some(o), arg.to_string(), None, None)));
    }
    if arg.chars().count() == 1 {
        return Ok(None);
    }
    if let Some((opt, val)) = arg.split_once('=')
        && let Some(o) = eval_option(opt)
    {
        return Ok(Some((
            Some(o),
            opt.to_string(),
            Some("="),
            Some(val.to_string()),
        )));
    }
    let mut tuples: Vec<OptTuple> = Vec::new();
    if arg.starts_with("--") {
        let (prefix, sep, explicit) = match arg.split_once('=') {
            Some((p, e)) => (p, Some("="), Some(e.to_string())),
            None => (arg, None, None),
        };
        for (s, o) in EVAL_OPTIONS {
            if s.starts_with(prefix) {
                tuples.push((Some(o), s.to_string(), sep, explicit.clone()));
            }
        }
    } else {
        let short: String = arg.chars().take(2).collect();
        let short_explicit: String = arg.chars().skip(2).collect();
        for (s, o) in EVAL_OPTIONS {
            if s == short {
                tuples.push((
                    Some(o),
                    s.to_string(),
                    Some(""),
                    Some(short_explicit.clone()),
                ));
            } else if s.starts_with(arg) {
                tuples.push((Some(o), s.to_string(), None, None));
            }
        }
    }
    if tuples.len() > 1 {
        let names: Vec<&str> = tuples.iter().map(|t| t.1.as_str()).collect();
        return Err(format!(
            "ambiguous option: {arg} could match {}",
            names.join(", ")
        ));
    }
    if let Some(t) = tuples.pop() {
        return Ok(Some(t));
    }
    if negative_number(arg) || arg.contains(' ') {
        return Ok(None);
    }
    Ok(Some((None, arg.to_string(), None, None)))
}

/// Parsed arguments of `tok eval`.
struct EvalArgs {
    file: String,
    decision: String,
    values: Vec<String>,
    lang: String,
}

/// Greedy match of the remaining positionals (`false`: exactly one argument, `true`: any
/// number) against the argument pattern: the argument counts of the longest prefix of the
/// positionals that matches (argparse `_match_arguments_partial`).
fn match_positionals(kinds: &[bool], pattern: &[char]) -> Vec<usize> {
    for n in (1..=kinds.len()).rev() {
        let mut counts = Vec::new();
        let mut i = 0;
        let mut ok = true;
        for many in &kinds[..n] {
            let start = i;
            if *many {
                while i < pattern.len() && (pattern[i] == 'A' || pattern[i] == '-') {
                    i += 1;
                }
            } else {
                while i < pattern.len() && pattern[i] == '-' {
                    i += 1;
                }
                if i < pattern.len() && pattern[i] == 'A' {
                    i += 1;
                } else {
                    ok = false;
                    break;
                }
                while i < pattern.len() && pattern[i] == '-' {
                    i += 1;
                }
            }
            counts.push(i - start);
        }
        if ok {
            return counts;
        }
    }
    Vec::new()
}

/// State of the `eval` argument parser.
struct EvalParser<'a> {
    argv: &'a [String],
    pattern: Vec<char>,
    /// remaining positionals: (name, takes any number of arguments)
    positionals: Vec<(&'static str, bool)>,
    taken: Vec<(&'static str, Vec<String>)>,
}

impl EvalParser<'_> {
    /// argparse `consume_positionals`: the index after the consumed arguments.
    fn consume_positionals(&mut self, start: usize) -> usize {
        let kinds: Vec<bool> = self.positionals.iter().map(|p| p.1).collect();
        let counts = match_positionals(&kinds, &self.pattern[start..]);
        let mut i = start;
        for (p, n) in self.positionals.iter().zip(&counts) {
            let mut args: Vec<String> = self.argv[i..i + n].to_vec();
            if let Some(k) = args.iter().position(|a| a == "--") {
                args.remove(k);
            }
            self.taken.push((p.0, args));
            i += n;
        }
        self.positionals.drain(..counts.len());
        i
    }

    fn get(&self, name: &str) -> Vec<String> {
        self.taken
            .iter()
            .find(|t| t.0 == name)
            .map(|t| t.1.clone())
            .unwrap_or_default()
    }
}

fn ignored_explicit(name: &str, arg: &str) -> ExitCode {
    eval_usage(&format!(
        "argument {name}: ignored explicit argument {}",
        tok_core::pyutil::repr_str(arg)
    ))
}

/// Arguments of `tok eval FILE DECISION name=value …`, parsed like Python's argparse
/// (including its handling of options between the positional arguments). `Err` carries
/// the exit code after a usage error or the help text.
fn parse_eval_args(argv: &[String]) -> Result<EvalArgs, ExitCode> {
    let mut pattern: Vec<char> = Vec::new();
    let mut options: Vec<Option<OptTuple>> = Vec::new();
    let mut rest_positional = false;
    for arg in argv {
        if rest_positional {
            pattern.push('A');
            options.push(None);
        } else if arg == "--" {
            pattern.push('-');
            options.push(None);
            rest_positional = true;
        } else {
            if arg.starts_with("--=") {
                // the main parser classifies the arguments first and cannot resolve this prefix
                return Err(usage(&format!(
                    "ambiguous option: {arg} could match --help, --version"
                )));
            }
            match eval_parse_optional(arg).map_err(|m| eval_usage(&m))? {
                Some(t) => {
                    pattern.push('O');
                    options.push(Some(t));
                }
                None => {
                    pattern.push('A');
                    options.push(None);
                }
            }
        }
    }
    let mut p = EvalParser {
        argv,
        pattern,
        positionals: vec![("file", false), ("decision", false), ("values", true)],
        taken: Vec::new(),
    };
    let mut extras: Vec<String> = Vec::new();
    let mut lang: Option<String> = None;
    let option_indices: Vec<usize> = (0..p.pattern.len())
        .filter(|i| p.pattern[*i] == 'O')
        .collect();
    let max_option = option_indices.last().copied();
    let mut start = 0usize;
    while max_option.is_some_and(|m| start <= m) {
        let next_option = option_indices
            .iter()
            .copied()
            .find(|i| *i >= start)
            .unwrap_or(p.pattern.len());
        if start != next_option {
            let end = p.consume_positionals(start);
            if end > start {
                start = end;
                continue;
            }
            start = end;
        }
        if p.pattern.get(start) != Some(&'O') {
            extras.extend(argv[start..next_option].iter().cloned());
            start = next_option;
        }
        // consume_optional
        let Some(Some((action, mut option_string, mut sep, mut explicit))) =
            options.get(start).cloned()
        else {
            break;
        };
        let Some(mut action) = action else {
            extras.push(argv[start].clone());
            start += 1;
            continue;
        };
        let mut actions: Vec<(EvalOpt, Option<String>)> = Vec::new();
        let stop;
        loop {
            let name = match action {
                EvalOpt::Help => "-h/--help",
                EvalOpt::Lang => "--lang",
            };
            if let Some(exp) = explicit.clone() {
                if action == EvalOpt::Lang {
                    actions.push((action, Some(exp)));
                    stop = start + 1;
                    break;
                }
                // a single-dash flag with an explicit argument: `-hX` reads X as further flags
                if option_string.chars().nth(1) != Some('-') && !exp.is_empty() {
                    if sep == Some("=") || exp.starts_with('-') {
                        return Err(ignored_explicit(name, &exp));
                    }
                    actions.push((action, None));
                    let next = format!("-{}", exp.chars().next().unwrap_or('-'));
                    let rest: String = exp.chars().skip(1).collect();
                    if let Some(o) = eval_option(&next) {
                        action = o;
                        option_string = next;
                        if rest.is_empty() {
                            sep = None;
                            explicit = None;
                        } else if let Some(r) = rest.strip_prefix('=') {
                            sep = Some("=");
                            explicit = Some(r.to_string());
                        } else {
                            sep = Some("");
                            explicit = Some(rest);
                        }
                        continue;
                    }
                    extras.push(format!("-{exp}"));
                    stop = start + 1;
                    break;
                }
                return Err(ignored_explicit(name, &exp));
            }
            match action {
                EvalOpt::Help => {
                    actions.push((action, None));
                    stop = start + 1;
                }
                EvalOpt::Lang => {
                    if p.pattern.get(start + 1) != Some(&'A') {
                        return Err(eval_usage(&format!(
                            "argument {name}: expected one argument"
                        )));
                    }
                    actions.push((action, Some(argv[start + 1].clone())));
                    stop = start + 2;
                }
            }
            break;
        }
        for (act, value) in actions {
            match act {
                EvalOpt::Help => {
                    out!("{EVAL_HELP}");
                    return Err(ExitCode::from(EXIT_OK));
                }
                EvalOpt::Lang => {
                    let v = value.unwrap_or_default();
                    if v != "en" && v != "pl" {
                        return Err(eval_usage(&format!(
                            "argument --lang: invalid choice: {} (choose from 'en', 'pl')",
                            tok_core::pyutil::repr_str(&v)
                        )));
                    }
                    lang = Some(v);
                }
            }
        }
        start = stop;
    }
    let stop = p.consume_positionals(start);
    extras.extend(argv[stop..].iter().cloned());
    if !p.positionals.is_empty() {
        let names: Vec<&str> = p.positionals.iter().map(|q| q.0).collect();
        return Err(eval_usage(&format!(
            "the following arguments are required: {}",
            names.join(", ")
        )));
    }
    if !extras.is_empty() {
        return Err(usage(&format!(
            "unrecognized arguments: {}",
            extras.join(" ")
        )));
    }
    Ok(EvalArgs {
        file: p.get("file").concat(),
        decision: p.get("decision").concat(),
        values: p.get("values"),
        lang: lang.unwrap_or_else(|| std::env::var("TOK_LANG").unwrap_or_else(|_| "en".into())),
    })
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

fn eval_command(argv: &[String]) -> Result<u8, ExitCode> {
    let e = parse_eval_args(argv)?;
    let a = Args {
        cmd: "eval".into(),
        file: e.file.clone(),
        lang: e.lang.clone(),
        level: "full".into(),
        ..Default::default()
    };
    let (doc, diags) = load(&a, false);
    if diags.iter().any(Diag::is_error) {
        report(&a, &diags);
        return Ok(EXIT_ERRORS);
    }
    let Some(dec) = doc.decisions.get(&e.decision) else {
        errln!("tok: no decision '{}' in {}", e.decision, e.file);
        return Ok(EXIT_USAGE);
    };
    let mut ctx = tok_core::feel_eval::Context::new();
    for pair in &e.values {
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
