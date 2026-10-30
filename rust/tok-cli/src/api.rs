//! The commands of tok as functions on strings: each takes the text of a document (or of
//! an XML file or an expression) and the options as JSON text, and returns the JSON value
//! of a successful call or an [`ApiError`]. The C interface (`tok-capi`), the MCP server
//! (`tok-mcp`), the language server (`tok-lsp`) and the WebAssembly build (`tok-wasm`)
//! call them, so every one of them gives the same results; `docs/c-api.md` documents the
//! options and results.

use serde_json::{Map, Value as JValue};

use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::diagnostics::{Diag, format_diag};
use tok_core::feel_eval::{self as fe, PyValue};
use tok_core::json::{Json, obj};
use tok_core::pydec::Decimal;
use tok_core::pyutil::PyError;
use tok_core::{Document, export, forms, import, printer};
use tok_sim::fallback::{Feel07, Vars, eval_expr};
use tok_sim::{Param, SimResult};

/// A failed call: the `error` member of the result, `{"kind", "message"}` and, for a
/// document with errors, its `diagnostics`.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub kind: &'static str,
    pub message: String,
    pub diagnostics: Option<Json>,
}

impl ApiError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        ApiError {
            kind,
            message: message.into(),
            diagnostics: None,
        }
    }

    fn argument(message: impl Into<String>) -> Self {
        ApiError::new("invalid_argument", message)
    }

    fn to_json(&self) -> Json {
        let mut fields = vec![
            ("kind", Json::Str(self.kind.to_string())),
            ("message", Json::Str(self.message.clone())),
        ];
        if let Some(d) = &self.diagnostics {
            fields.push(("diagnostics", d.clone()));
        }
        obj(fields)
    }
}

pub type Res = Result<Json, ApiError>;

/// The text the C functions return: `{"ok": VALUE}` or `{"error": {...}}`.
pub fn respond(result: &Res) -> String {
    match result {
        Ok(v) => obj(vec![("ok", v.clone())]).compact(),
        Err(e) => obj(vec![("error", e.to_json())]).compact(),
    }
}

/// An exception of the evaluator or the simulator as an error of the interface.
fn py_error(e: &PyError) -> ApiError {
    let kind = match e.kind.as_str() {
        "tok.sim.DistError" => "distribution",
        "tok.dmn.DecisionError" => "decision",
        k if k.starts_with("tok.feel.") => "feel",
        k if k.starts_with("decimal.") => "feel",
        "FileNotFoundError" | "PermissionError" | "IsADirectoryError" | "OSError" => "io",
        _ => "simulation",
    };
    ApiError::new(kind, e.to_string())
}

// ------------------------------------------------------------------------ options

/// The options object: empty text means no options; the keys must be among `allowed`.
pub struct Options {
    map: Map<String, JValue>,
}

impl Options {
    pub fn parse(text: &str, allowed: &[&str]) -> Result<Options, ApiError> {
        if text.trim().is_empty() {
            return Ok(Options { map: Map::new() });
        }
        let v: JValue = serde_json::from_str(text)
            .map_err(|e| ApiError::argument(format!("options are not valid JSON: {e}")))?;
        let JValue::Object(map) = v else {
            return Err(ApiError::argument("options must be a JSON object"));
        };
        if let Some(k) = map.keys().find(|k| !allowed.contains(&k.as_str())) {
            return Err(ApiError::argument(format!(
                "unknown option '{k}'; the options are: {}",
                allowed.join(", ")
            )));
        }
        Ok(Options { map })
    }

    fn get(&self, key: &str) -> Option<&JValue> {
        self.map.get(key).filter(|v| !v.is_null())
    }

    fn string(&self, key: &str) -> Result<Option<String>, ApiError> {
        match self.get(key) {
            None => Ok(None),
            Some(JValue::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(ApiError::argument(format!(
                "option '{key}' must be a string"
            ))),
        }
    }

    fn string_or(&self, key: &str, default: &str) -> Result<String, ApiError> {
        Ok(self.string(key)?.unwrap_or_else(|| default.to_string()))
    }

    fn choice(&self, key: &str, choices: &[&str], default: &str) -> Result<String, ApiError> {
        let v = self.string_or(key, default)?;
        if !choices.contains(&v.as_str()) {
            return Err(ApiError::argument(format!(
                "option '{key}' must be one of {}, not '{v}'",
                choices.join(", ")
            )));
        }
        Ok(v)
    }

    fn flag(&self, key: &str, default: bool) -> Result<bool, ApiError> {
        match self.get(key) {
            None => Ok(default),
            Some(JValue::Bool(b)) => Ok(*b),
            Some(_) => Err(ApiError::argument(format!(
                "option '{key}' must be a boolean"
            ))),
        }
    }

    fn lang(&self) -> Result<String, ApiError> {
        self.choice("lang", &["en", "pl"], "en")
    }

    /// `cases` or `seed` of the simulation, as the simulator receives it.
    fn param(&self, key: &str) -> Result<Option<Param>, ApiError> {
        match self.get(key) {
            None => Ok(None),
            Some(JValue::Number(n)) => Ok(Some(if n.is_i64() || n.is_u64() {
                Param::Int(n.to_string())
            } else {
                Param::Float(n.as_f64().unwrap_or(f64::NAN))
            })),
            Some(JValue::String(s)) => Ok(Some(Param::Text(s.clone()))),
            Some(_) => Err(ApiError::argument(format!(
                "option '{key}' must be a number or a string"
            ))),
        }
    }

    fn sidecar(&self) -> Result<Option<tok_bpmn::Sidecar>, ApiError> {
        match self.string("sidecar")? {
            None => Ok(None),
            Some(text) => tok_bpmn::Sidecar::parse(&text)
                .map(Some)
                .map_err(|e| ApiError::argument(format!("layout sidecar: {e}"))),
        }
    }
}

// ----------------------------------------------------------------------- documents

/// BPMN rules from the `tok-bpmn` crate (as in the command line).
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

/// Universal newlines, as the command line reads files.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn diagnostics_json(diags: &[Diag], lang: &str) -> Json {
    Json::Arr(diags.iter().map(|d| d.to_json(lang)).collect())
}

/// A document with errors: the errors as the message, every diagnostic as data.
fn document_error(diags: &[Diag], lang: &str) -> ApiError {
    let message = diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| format_diag(d, lang, None))
        .collect::<Vec<_>>()
        .join("\n");
    ApiError {
        kind: "document",
        message,
        diagnostics: Some(diagnostics_json(diags, lang)),
    }
}

fn parse_ok(text: &str, lang: &str) -> Result<Document, ApiError> {
    let (doc, diags) = tok_core::parse(&normalize(text));
    if diags.iter().any(Diag::is_error) {
        return Err(document_error(&diags, lang));
    }
    Ok(doc)
}

fn load_ok(text: &str, lang: &str, bpmn: bool) -> Result<Document, ApiError> {
    let rules = Rules;
    let (doc, diags) = tok_core::load(
        &normalize(text),
        "full",
        if bpmn { Some(&rules) } else { None },
    );
    if diags.iter().any(Diag::is_error) {
        return Err(document_error(&diags, lang));
    }
    Ok(doc)
}

/// A diagram needs at least one flow node or pool.
fn drawable(doc: &Document) -> Result<(), ApiError> {
    if doc.nodes.is_empty() && doc.pools.is_empty() {
        return Err(ApiError::new(
            "render",
            "nothing to draw: the model has no elements",
        ));
    }
    Ok(())
}

fn render_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::new("render", e.to_string())
}

fn bpmn_text(
    doc: &Document,
    layout: bool,
    side: Option<&tok_bpmn::Sidecar>,
) -> Result<String, ApiError> {
    let xml = export::export_xml(doc, None);
    if !layout {
        let mut root = export::export_tree(doc, None);
        root.indent(0);
        return Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n",
            root.to_string_root()
        ));
    }
    match side {
        Some(s) => tok_bpmn::layout_with(&xml, s),
        None => tok_bpmn::layout(&xml),
    }
    .map_err(render_error)
}

fn svg_text(
    doc: &Document,
    title: Option<&str>,
    side: Option<&tok_bpmn::Sidecar>,
) -> Result<String, ApiError> {
    let xml = export::export_xml(doc, None);
    match side {
        Some(s) => tok_bpmn::render_svg_with(&xml, title, s),
        None => tok_bpmn::render_svg(&xml, title),
    }
    .map_err(render_error)
}

/// `tok check`: `{"errors", "bpmn_class", "diagnostics"}`.
pub fn check(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang", "level", "bpmn", "class"])?;
    let lang = o.lang()?;
    let level = o.choice("level", &["basic", "analytic", "full"], "full")?;
    let bpmn = o.flag("bpmn", true)?;
    let class = match o.string("class")? {
        Some(c) if !tok_core::classes::CLASSES.contains(&c.as_str()) => {
            return Err(ApiError::argument(format!(
                "option 'class' must be one of {}, not '{c}'",
                tok_core::classes::CLASSES.join(", ")
            )));
        }
        c => c,
    };
    let rules = Rules;
    let (doc, mut diags) = tok_core::load(
        &normalize(text),
        &level,
        if bpmn { Some(&rules) } else { None },
    );
    if let Some(c) = &class {
        diags.extend(tok_core::classes::check_class(&doc, c));
        tok_core::diagnostics::sort(&mut diags);
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    Ok(obj(vec![
        ("errors", Json::Int(errors as i64)),
        (
            "bpmn_class",
            Json::Str(tok_core::classes::bpmn_class(&doc).to_string()),
        ),
        ("diagnostics", diagnostics_json(&diags, &lang)),
    ]))
}

/// `tok check FILE.bpmn`: the findings of the BPMN validator for BPMN 2.0 XML,
/// `{"errors", "diagnostics"}`. Options: `lang`, `steps` (names start with step numbers),
/// `steps_from` (the text of a process description whose table lists the steps), `camunda`
/// (the Camunda 8 profile) and `folder` (where its forms and decisions are; default `.`).
pub fn check_bpmn(xml: &str, options: &str) -> Res {
    let o = Options::parse(
        options,
        &["lang", "steps", "steps_from", "camunda", "folder"],
    )?;
    let lang = o.lang()?;
    let steps_from = o
        .string("steps_from")?
        .map(|t| tok_bpmn::read_opis(&normalize(&t)));
    let folder = o.string("folder")?.unwrap_or_else(|| ".".to_string());
    let camunda = o.flag("camunda", false)?;
    let diags = crate::bpmn_file_diags(
        &normalize(xml),
        o.flag("steps", false)?,
        steps_from,
        camunda.then_some(std::path::Path::new(&folder)),
    )
    .map_err(|e| match (&e, lang.as_str()) {
        (tok_bpmn::Error::Xml(m), "en") => {
            ApiError::new("xml", format!("not well-formed XML: {m}"))
        }
        _ => ApiError::new("xml", e.to_string()),
    })?;
    let errors = diags.iter().filter(|d| d.is_error()).count();
    Ok(obj(vec![
        ("errors", Json::Int(errors as i64)),
        ("diagnostics", diagnostics_json(&diags, &lang)),
    ]))
}

/// `tok fmt`: the canonical text.
pub fn fmt(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang"])?;
    Ok(Json::Str(printer::fmt(&parse_ok(text, &o.lang()?)?)))
}

/// `tok bpmn`: BPMN 2.0 XML with diagram layout.
pub fn bpmn(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang", "layout", "sidecar"])?;
    let doc = load_ok(text, &o.lang()?, true)?;
    drawable(&doc)?;
    let side = o.sidecar()?;
    bpmn_text(&doc, o.flag("layout", true)?, side.as_ref()).map(Json::Str)
}

/// `tok svg`: the picture of the diagram.
pub fn svg(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang", "title", "sidecar"])?;
    let doc = load_ok(text, &o.lang()?, true)?;
    drawable(&doc)?;
    let side = o.sidecar()?;
    svg_text(&doc, o.string("title")?.as_deref(), side.as_ref()).map(Json::Str)
}

/// `tok export`: every file, as an object file name → content.
pub fn export_files(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang", "name", "sidecar"])?;
    let doc = load_ok(text, &o.lang()?, true)?;
    let name = o.string_or("name", "model")?;
    let has_drg = doc.lang_07() && (!doc.decisions.is_empty() || !doc.bkms.is_empty());
    if !has_drg {
        drawable(&doc)?;
    }
    let side = o.sidecar()?;
    let mut files = Vec::new();
    // a decision model of language 0.7 without a process has no BPMN file
    let decisions_only = doc.lang_07() && doc.nodes.is_empty() && doc.pools.is_empty();
    if !decisions_only {
        files.push((
            format!("{name}.bpmn"),
            Json::Str(bpmn_text(&doc, true, side.as_ref())?),
        ));
        let svg_name = format!("{name}.svg");
        let svg = svg_text(&doc, Some(&svg_name), side.as_ref())?;
        files.push((svg_name, Json::Str(svg)));
    }
    for (file, content) in tok_core::drg::export::decision_files(&doc) {
        files.push((file, Json::Str(content)));
    }
    for form in doc.forms.values() {
        files.push((
            format!("{}.form", form.base.id()),
            Json::Str(forms::export_form_of(&doc, form)),
        ));
    }
    Ok(Json::Obj(files))
}

/// `tok import`: `{"format", "text", "warnings", "layout"}` for a BPMN or DMN file.
pub fn import_file(xml: &str, options: &str) -> Res {
    let o = Options::parse(options, &["format", "flow_ids", "layout"])?;
    let format = o.choice("format", &["auto", "bpmn", "dmn"], "auto")?;
    let dmn = match format.as_str() {
        "dmn" => true,
        "bpmn" => false,
        _ => tok_core::drg::dmnimport::is_dmn(xml),
    };
    let warnings = |w: Vec<String>| Json::Arr(w.into_iter().map(Json::Str).collect());
    if dmn {
        let (doc, w) = tok_core::drg::dmnimport::import_dmn(xml)
            .map_err(|e| ApiError::new("import", e.to_string()))?;
        return Ok(obj(vec![
            ("format", Json::Str("dmn".into())),
            ("text", Json::Str(printer::fmt(&doc))),
            ("warnings", warnings(w)),
            ("layout", Json::Null),
        ]));
    }
    let (doc, w, ids) = import::import_bpmn_mapped(xml, o.flag("flow_ids", true)?)
        .map_err(|e| ApiError::new("import", e.to_string()))?;
    let layout = if o.flag("layout", true)? {
        // the diagram of the file, as `tok import -o` keeps it next to the document
        let map: std::collections::HashMap<String, String> = ids.into_iter().collect();
        let exported = export::export_xml(&doc, None);
        let side = tok_bpmn::extract_sidecar(xml, &exported, &map)
            .map_err(|e| ApiError::new("import", e.to_string()))?;
        if side.is_empty() {
            Json::Null
        } else {
            Json::Str(side.to_text())
        }
    } else {
        Json::Null
    };
    Ok(obj(vec![
        ("format", Json::Str("bpmn".into())),
        ("text", Json::Str(printer::fmt(&doc))),
        ("warnings", warnings(w)),
        ("layout", layout),
    ]))
}

// ----------------------------------------------------------------------- decisions

/// A JSON value as a value of the FEEL subset (documents of languages 0.1–0.6).
fn subset_value(v: &JValue) -> Result<fe::Value, ApiError> {
    Ok(match v {
        JValue::Null => fe::Value::Null,
        JValue::Bool(b) => fe::Value::Bool(*b),
        JValue::Number(n) => fe::Value::Num(
            Decimal::parse(&n.to_string()).map_err(|e| ApiError::argument(e.to_string()))?,
        ),
        JValue::String(s) => fe::Value::Str(s.clone()),
        JValue::Array(items) => {
            fe::Value::List(items.iter().map(subset_value).collect::<Result<_, _>>()?)
        }
        JValue::Object(m) => {
            let mut ctx = fe::Context::new();
            for (k, x) in m {
                ctx.insert(k.clone(), subset_value(x)?);
            }
            fe::Value::Context(ctx)
        }
    })
}

/// `tok eval`: the value of a decision for the input values of `values`.
pub fn eval(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang", "decision", "values"])?;
    let lang = o.lang()?;
    let Some(decision) = o.string("decision")? else {
        return Err(ApiError::argument("option 'decision' is required"));
    };
    let values = match o.get("values") {
        None => Map::new(),
        Some(JValue::Object(m)) => m.clone(),
        Some(_) => return Err(ApiError::argument("option 'values' must be an object")),
    };
    let doc = load_ok(text, &lang, false)?;
    let Some(dec) = doc.decisions.get(&decision) else {
        let known: Vec<&str> = doc.decisions.keys().map(String::as_str).collect();
        return Err(ApiError::new(
            "unknown_decision",
            format!(
                "unknown decision '{decision}'; the document has: {}",
                known.join(", ")
            ),
        ));
    };
    if doc.lang_07() {
        // language 0.7: the decision with what it requires; the result is its DMN value
        let mut inputs = tok_core::feel3::Map::new();
        for (k, v) in &values {
            let value = tok_core::drg::values::parse_json(&v.to_string()).ok_or_else(|| {
                ApiError::argument(format!("value of '{k}' cannot be read as FEEL"))
            })?;
            inputs.insert(k.clone(), value);
        }
        let (value, problems) = tok_core::drg::eval::evaluate(&doc, &decision, &inputs);
        if !problems.is_empty() {
            return Err(ApiError::new("decision", problems.join("\n")));
        }
        return Ok(tok_core::drg::values::to_json(&value));
    }
    let mut ctx = fe::Context::new();
    for (k, v) in &values {
        ctx.insert(k.clone(), subset_value(v)?);
    }
    tok_core::dmn::evaluate_decision(dec, &ctx)
        .and_then(|r| r.to_json())
        .map_err(|e| py_error(&e))
}

// ---------------------------------------------------------------------- simulation

fn opt_float(x: Option<f64>) -> Json {
    x.map(Json::Float).unwrap_or(Json::Null)
}

fn sim_json(res: &SimResult, o: &Options, lang: &str) -> Res {
    let err = |e: PyError| py_error(&e);
    let kpi = res.kpi();
    let mut out = vec![
        ("kpi", kpi.to_json()),
        ("report", Json::Str(kpi.format(lang))),
    ];
    if o.flag("log", true)? {
        let rows = res.log().map_err(err)?;
        let log = rows
            .iter()
            .zip(&res.outcome.rows)
            .map(|(r, raw)| {
                let Json::Obj(mut fields) = r.to_json() else {
                    return Json::Null;
                };
                fields.push(("enqueue_s".into(), Json::Float(raw.enqueue)));
                fields.push(("start_s".into(), opt_float(raw.start)));
                fields.push(("end_s".into(), opt_float(raw.end)));
                Json::Obj(fields)
            })
            .collect();
        out.push(("log", Json::Arr(log)));
    }
    if o.flag("cases_table", true)? {
        let rows = res.cases().map_err(err)?;
        let mut cases = Vec::with_capacity(rows.len());
        for (c, raw) in rows.iter().zip(&res.outcome.cases) {
            let Json::Obj(mut fields) = c.to_json().map_err(err)? else {
                continue;
            };
            fields.push(("start_s".into(), Json::Float(raw.start)));
            fields.push(("end_s".into(), opt_float(raw.end)));
            cases.push(Json::Obj(fields));
        }
        out.push(("cases", Json::Arr(cases)));
    }
    if o.flag("csv", false)? {
        let (log, r) = res.log_csv();
        r.map_err(err)?;
        let (cases, r) = res.cases_csv();
        r.map_err(err)?;
        out.push(("log_csv", Json::Str(log)));
        out.push(("cases_csv", Json::Str(cases)));
    }
    if o.flag("xes", false)? {
        out.push(("xes", Json::Str(res.to_xes().map_err(err)?)));
    }
    Ok(obj(out))
}

/// `tok sim`: indicators, report, event log, cases, and on request CSV and XES.
pub fn sim(text: &str, options: &str) -> Res {
    let o = Options::parse(
        options,
        &["lang", "cases", "seed", "log", "cases_table", "csv", "xes"],
    )?;
    let lang = o.lang()?;
    let doc = load_ok(text, &lang, true)?;
    let res =
        tok_sim::simulate(&doc, o.param("cases")?, o.param("seed")?).map_err(|e| py_error(&e))?;
    sim_json(&res, &o, &lang)
}

/// The simulation model of a document (`tok_sim::describe`).
pub fn sim_model(text: &str, options: &str) -> Res {
    let o = Options::parse(options, &["lang", "cases", "seed"])?;
    let doc = load_ok(text, &o.lang()?, true)?;
    tok_sim::describe::describe(&doc, o.param("cases")?, o.param("seed")?).map_err(|e| py_error(&e))
}

/// A distribution of the simulation statements, parsed.
pub fn sim_dist(text: &str, options: &str) -> Res {
    Options::parse(options, &[])?;
    tok_sim::describe::describe_dist(text).map_err(|e| py_error(&e))
}

/// A JSON value as a case variable of the simulator.
fn plain_value(v: &JValue) -> PyValue {
    match v {
        JValue::Null => PyValue::None,
        JValue::Bool(b) => PyValue::Bool(*b),
        JValue::Number(n) if n.is_i64() || n.is_u64() => PyValue::Int(n.to_string()),
        JValue::Number(n) => PyValue::Float(n.as_f64().unwrap_or(f64::NAN)),
        JValue::String(s) => PyValue::Str(s.clone()),
        JValue::Array(items) => PyValue::List(items.iter().map(plain_value).collect()),
        JValue::Object(m) => {
            PyValue::Dict(m.iter().map(|(k, x)| (k.clone(), plain_value(x))).collect())
        }
    }
}

/// An expression of the simulation (a condition of a flow, for example) evaluated as the
/// simulator evaluates it: `{"value", "truthy"}`.
pub fn sim_eval(expr: &str, options: &str) -> Res {
    let o = Options::parse(options, &["vars", "lang07"])?;
    let mut vars = Vars::new();
    match o.get("vars") {
        None => {}
        Some(JValue::Object(m)) => {
            for (k, v) in m {
                vars.insert(k.clone(), plain_value(v));
            }
        }
        Some(_) => return Err(ApiError::argument("option 'vars' must be an object")),
    }
    let _feel = Feel07::set(o.flag("lang07", false)?);
    let v = eval_expr(expr, &vars).map_err(|e| ApiError::new("feel", e.message))?;
    let truthy = tok_sim::pyval::truthy(&v);
    let value = v.to_json().map_err(|e| py_error(&e))?;
    Ok(obj(vec![("value", value), ("truthy", Json::Bool(truthy))]))
}

/// The `tok` command line with the arguments of a JSON array (`["check", "model.tok"]`);
/// it writes to the standard streams of the process and returns `{"status"}`.
pub fn cli(args: &str, options: &str) -> Res {
    Options::parse(options, &[])?;
    let v: JValue = serde_json::from_str(args)
        .map_err(|e| ApiError::argument(format!("arguments are not valid JSON: {e}")))?;
    let JValue::Array(items) = v else {
        return Err(ApiError::argument(
            "arguments must be a JSON array of strings",
        ));
    };
    let mut argv = Vec::with_capacity(items.len());
    for item in items {
        let JValue::String(s) = item else {
            return Err(ApiError::argument(
                "arguments must be a JSON array of strings",
            ));
        };
        argv.push(s);
    }
    let status = crate::main_with_args(&argv);
    Ok(obj(vec![("status", Json::Int(i64::from(status)))]))
}
