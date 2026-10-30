//! The tools of the server: their definitions (names, descriptions, input and output
//! schemas) and their calls, which go through [`tok_cli::api`] like the C interface.

use serde_json::{Map, Value, json};
use tok_cli::{api, reference};

/// The language reference: topic, title, text.
pub const REFERENCE: &[(&str, &str, &str)] = &[
    (
        "cheatsheet",
        "The tok language on one page",
        reference::CHEATSHEET,
    ),
    (
        "spec",
        "Language specification (normative)",
        reference::SPEC,
    ),
    (
        "diagnostics",
        "Diagnostic codes in English and Polish",
        reference::DIAGNOSTICS,
    ),
    ("feel", "FEEL in tok", reference::FEEL),
    ("cli", "The tok command line", reference::CLI),
];

/// The outcome of a tool call: the structured result, or a message for the model.
pub enum Outcome {
    Done { text: String, structured: Value },
    Failed(String),
}

fn fail(message: impl Into<String>) -> Outcome {
    Outcome::Failed(message.into())
}

fn text_prop() -> Value {
    json!({"type": "string", "description": "The tok document. Give either text or path."})
}

fn path_prop() -> Value {
    json!({"type": "string", "description": "Path of a .tok file, read by the server (relative to its working directory). Give either text or path."})
}

fn lang_prop() -> Value {
    json!({"type": "string", "enum": ["en", "pl"], "default": "en", "description": "Language of diagnostics and reports."})
}

fn diagnostic_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "code": {"type": "string"},
            "severity": {"type": "string", "enum": ["error", "warning", "note"]},
            "line": {"type": "integer"},
            "id": {"type": ["string", "null"]},
            "message": {"type": "string"},
        },
        "required": ["code", "severity", "line", "id", "message"],
    })
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

/// The definitions of `tools/list`, in a fixed order.
pub fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "check",
            "title": "Check a tok model",
            "description": "Check a tok document and list its diagnostics (code, severity, line, element, message), as `tok check --json`. Every layer is checked: tokens, statements, references, BPMN 2.0 rules, the Camunda 8 profile of executable documents, decisions, FEEL and simulation parameters. A document with errors is a successful call with errors > 0; fix the errors and check again. Codes are explained in the reference topic 'diagnostics'.",
            "inputSchema": object(json!({
                "text": text_prop(),
                "path": path_prop(),
                "lang": lang_prop(),
                "level": {"type": "string", "enum": ["basic", "analytic", "full"], "default": "full", "description": "Teaching level: the elements the document may use (spec §13)."},
                "class": {"type": "string", "enum": ["descriptive", "analytic", "common-executable", "full"], "description": "BPMN 2.0.2 conformance class to check against; elements outside it are R075."},
                "bpmn": {"type": "boolean", "default": true, "description": "Also check the BPMN 2.0 rules of the exported model (E, W, L codes)."},
            }), &[]),
            "outputSchema": object(json!({
                "errors": {"type": "integer"},
                "bpmn_class": {"type": "string"},
                "diagnostics": {"type": "array", "items": diagnostic_schema()},
            }), &["errors", "bpmn_class", "diagnostics"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "check_bpmn",
            "title": "Check a BPMN file",
            "description": "Check BPMN 2.0 XML, from tok or from any modelling tool, with the BPMN rules of the tok checker, as `tok check FILE.bpmn --json`: every finding has a code, a severity, the line and the id of its element and a message. steps requires every element name to start with the step number of a process description (K01); description, the text of that description, reports steps without an element and elements without a step (K02, K03); camunda adds the Camunda 8 profile of executable models (C codes; with path, forms and decisions are looked for in the file's folder). XML with errors is a successful call with errors > 0. Codes are explained in the reference topic 'diagnostics'.",
            "inputSchema": object(json!({
                "xml": {"type": "string", "description": "The BPMN 2.0 XML. Give either xml or path."},
                "path": {"type": "string", "description": "Path of a .bpmn file, read by the server (relative to its working directory). Give either xml or path."},
                "steps": {"type": "boolean", "default": false, "description": "Element names start with step numbers (K01)."},
                "description": {"type": "string", "description": "The process description whose table lists the steps (its first column holds the step numbers); checks that every step is in the model (K02, K03)."},
                "camunda": {"type": "boolean", "default": false, "description": "Also check the Camunda 8 profile of executable models (C codes)."},
                "lang": lang_prop(),
            }), &[]),
            "outputSchema": object(json!({
                "errors": {"type": "integer"},
                "diagnostics": {"type": "array", "items": diagnostic_schema()},
            }), &["errors", "diagnostics"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "fmt",
            "title": "Format a tok model",
            "description": "Print a tok document in its canonical form, as `tok fmt`. The document must have no syntax errors. The result is the text only; write it to the file yourself.",
            "inputSchema": object(json!({
                "text": text_prop(),
                "path": path_prop(),
                "lang": lang_prop(),
            }), &[]),
            "outputSchema": object(json!({"text": {"type": "string"}}), &["text"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "export",
            "title": "Export a tok model",
            "description": "Export a checked tok document and return the contents of the files `tok export` writes: BPMN 2.0 XML with diagram layout (NAME.bpmn), its SVG picture (NAME.svg), DMN files of the decisions (with NAME.drd.svg pictures in languages 0.7 and 1.0) and Camunda form files (ID.form). With path, the layout sidecar FILE.tok.layout next to the file is used. A document with errors is a tool error listing them.",
            "inputSchema": object(json!({
                "text": text_prop(),
                "path": path_prop(),
                "formats": {"type": "array", "items": {"type": "string", "enum": ["bpmn", "svg", "dmn", "forms"]}, "uniqueItems": true, "description": "The kinds of file to return; all when left out. 'dmn' includes the decision diagrams (.drd.svg)."},
                "name": {"type": "string", "description": "Stem of the BPMN and SVG file names; default: the stem of path, or 'model'."},
                "sidecar": {"type": "boolean", "default": true, "description": "Use the layout sidecar FILE.tok.layout next to path."},
                "lang": lang_prop(),
            }), &[]),
            "outputSchema": object(json!({
                "files": {"type": "object", "additionalProperties": {"type": "string"}, "description": "File name → content."},
            }), &["files"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "import",
            "title": "Import BPMN or DMN",
            "description": "Convert a BPMN 2.0 XML or DMN 1.1–1.5 file to a tok document, as `tok import`. Elements tok does not keep are listed in warnings. For BPMN the diagram of the file is returned as a layout sidecar: save it as FILE.tok.layout next to the document to keep the drawing.",
            "inputSchema": object(json!({
                "xml": {"type": "string", "description": "The BPMN or DMN XML. Give either xml or path."},
                "path": {"type": "string", "description": "Path of a .bpmn or .dmn file, read by the server. Give either xml or path."},
                "format": {"type": "string", "enum": ["auto", "bpmn", "dmn"], "default": "auto"},
                "flow_ids": {"type": "boolean", "default": true, "description": "Keep the identifiers of sequence flows (false prints compact chains)."},
                "layout": {"type": "boolean", "default": true, "description": "Return the layout sidecar of a BPMN file."},
            }), &[]),
            "outputSchema": object(json!({
                "format": {"type": "string", "enum": ["bpmn", "dmn"]},
                "text": {"type": "string"},
                "warnings": {"type": "array", "items": {"type": "string"}},
                "layout": {"type": ["string", "null"]},
            }), &["format", "text", "warnings", "layout"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "sim",
            "title": "Simulate a tok model",
            "description": "Run the discrete-event simulation of a tok document, as `tok sim`: indicators (cases per end event, cycle times, waiting and processing times, resource utilization) and the report text. The same seed gives the same run. The event log and the case table are large; ask for them only when needed.",
            "inputSchema": object(json!({
                "text": text_prop(),
                "path": path_prop(),
                "cases": {"type": "integer", "minimum": 1, "description": "Number of cases; overrides `simulate cases=`."},
                "seed": {"type": "integer", "description": "Random seed; overrides `simulate seed=`."},
                "log": {"type": "boolean", "default": false, "description": "Include the event log (one object per activity instance)."},
                "cases_table": {"type": "boolean", "default": false, "description": "Include one object per case."},
                "csv": {"type": "boolean", "default": false, "description": "Include the event log and the case table as CSV text."},
                "xes": {"type": "boolean", "default": false, "description": "Include the event log as XES."},
                "lang": lang_prop(),
            }), &[]),
            "outputSchema": object(json!({
                "kpi": {"type": "object"},
                "report": {"type": "string"},
                "log": {"type": "array"},
                "cases": {"type": "array"},
                "log_csv": {"type": "string"},
                "cases_csv": {"type": "string"},
                "xes": {"type": "string"},
            }), &["kpi", "report"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "eval",
            "title": "Evaluate a decision",
            "description": "Evaluate a decision of a tok document for input values, as `tok eval`. In languages 0.7 and 1.0 the decision is evaluated with everything it requires and values are given by the FEEL names of input data; in earlier languages the decision table alone is evaluated over its input columns. Values are JSON (numbers, text, booleans, null, lists, objects as contexts).",
            "inputSchema": object(json!({
                "text": text_prop(),
                "path": path_prop(),
                "decision": {"type": "string", "description": "Identifier of the decision."},
                "values": {"type": "object", "description": "Input name → value.", "additionalProperties": true},
                "lang": lang_prop(),
            }), &["decision"]),
            "outputSchema": object(json!({
                "value": {"description": "The value of the decision as JSON."},
            }), &["value"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
        json!({
            "name": "reference",
            "title": "tok language reference",
            "description": "The reference documentation of the tok language: 'cheatsheet' (the whole language on one page; read it before writing a model), 'spec' (the normative specification), 'diagnostics' (every diagnostic code), 'feel' (expressions), 'cli' (the tok command). The same texts are resources tok://reference/TOPIC.",
            "inputSchema": object(json!({
                "topic": {"type": "string", "enum": ["cheatsheet", "spec", "diagnostics", "feel", "cli"], "default": "cheatsheet"},
            }), &[]),
            "outputSchema": object(json!({"text": {"type": "string"}}), &["text"]),
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false},
        }),
    ]
}

/// The arguments of a call, checked against the input schema of the tool.
struct Args<'a> {
    map: &'a Map<String, Value>,
}

impl<'a> Args<'a> {
    fn new(tool: &str, args: &'a Value, empty: &'a Map<String, Value>) -> Result<Self, String> {
        let map = match args {
            Value::Object(m) => m,
            Value::Null => empty,
            _ => return Err("arguments must be an object".into()),
        };
        let definition = definitions()
            .into_iter()
            .find(|d| d["name"] == tool)
            .ok_or_else(|| format!("unknown tool '{tool}'"))?;
        let schema = &definition["inputSchema"];
        let props = schema["properties"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        for (key, value) in map {
            let Some(p) = props.get(key) else {
                let known: Vec<&String> = props.keys().collect();
                return Err(format!(
                    "unknown argument '{key}'; the arguments of {tool} are: {}",
                    known
                        .iter()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            };
            check_type(key, value, p)?;
        }
        for r in schema["required"].as_array().into_iter().flatten() {
            if let Some(r) = r.as_str()
                && !map.contains_key(r)
            {
                return Err(format!("missing required argument '{r}'"));
            }
        }
        Ok(Args { map })
    }

    fn str(&self, key: &str) -> Option<&'a str> {
        self.map.get(key).and_then(Value::as_str)
    }

    fn bool(&self, key: &str, default: bool) -> bool {
        self.map
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    }

    /// The document: `text`, or the file at `path` (`input` names the text argument).
    fn source(&self, input: &str) -> Result<(String, Option<String>), String> {
        match (self.str(input), self.str("path")) {
            (Some(_), Some(_)) => Err(format!("give either {input} or path, not both")),
            (Some(t), None) => Ok((t.to_string(), None)),
            (None, Some(p)) => std::fs::read_to_string(p)
                .map(|t| (t, Some(p.to_string())))
                .map_err(|e| format!("cannot read {p}: {e}")),
            (None, None) => Err(format!("give the document as {input} or its file as path")),
        }
    }
}

/// A value against the type, `enum` and `minimum` of a property schema.
fn check_type(key: &str, value: &Value, p: &Value) -> Result<(), String> {
    let ok = match p.get("type").and_then(Value::as_str) {
        Some("string") => value.is_string(),
        Some("boolean") => value.is_boolean(),
        Some("integer") => value.is_i64() || value.is_u64(),
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        _ => true,
    };
    if !ok {
        return Err(format!(
            "argument '{key}' must be of type {}",
            p["type"].as_str().unwrap_or("?")
        ));
    }
    if let Some(choices) = p.get("enum").and_then(Value::as_array)
        && !choices.contains(value)
    {
        return Err(format!(
            "argument '{key}' must be one of {}",
            choices
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(min) = p.get("minimum").and_then(Value::as_i64)
        && value.as_i64().is_some_and(|v| v < min)
    {
        return Err(format!("argument '{key}' must be at least {min}"));
    }
    if let (Some(items), Some(list)) = (p.get("items"), value.as_array()) {
        for item in list {
            check_type(key, item, items)?;
        }
        if p.get("uniqueItems") == Some(&Value::Bool(true)) {
            for (i, a) in list.iter().enumerate() {
                if list[..i].contains(a) {
                    return Err(format!("argument '{key}' repeats {a}"));
                }
            }
        }
    }
    Ok(())
}

/// The result of a function of [`api`] as a JSON value, or the message of its error.
fn call(f: fn(&str, &str) -> api::Res, input: &str, options: &Value) -> Result<Value, String> {
    match f(input, &options.to_string()) {
        Ok(v) => serde_json::from_str(&v.compact()).map_err(|e| format!("internal error: {e}")),
        Err(e) => Err(match &e.diagnostics {
            Some(d) if e.kind == "document" => format!(
                "the document has errors:\n{}\n\ndiagnostics: {}",
                e.message,
                d.compact()
            ),
            _ => format!("{} ({})", e.message, e.kind),
        }),
    }
}

/// Run a tool. `Err` is a protocol error (an unknown tool); a failure of the tool itself is
/// [`Outcome::Failed`].
pub fn run(tool: &str, args: &Value) -> Result<Outcome, String> {
    if !definitions().iter().any(|d| d["name"] == tool) {
        return Err(format!("unknown tool '{tool}'"));
    }
    let empty = Map::new();
    let a = match Args::new(tool, args, &empty) {
        Ok(a) => a,
        Err(e) => return Ok(fail(e)),
    };
    Ok(match tool {
        "check" => check(&a),
        "check_bpmn" => check_bpmn(&a),
        "fmt" => fmt(&a),
        "export" => export(&a),
        "import" => import(&a),
        "sim" => sim(&a),
        "eval" => eval(&a),
        _ => reference(&a),
    })
}

fn lang(a: &Args) -> &'static str {
    if a.str("lang") == Some("pl") {
        "pl"
    } else {
        "en"
    }
}

fn check(a: &Args) -> Outcome {
    let (text, _) = match a.source("text") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let mut options = json!({"lang": lang(a), "bpmn": a.bool("bpmn", true)});
    for key in ["level", "class"] {
        if let Some(v) = a.str(key) {
            options[key] = json!(v);
        }
    }
    match call(api::check, &text, &options) {
        Ok(v) => {
            let mut lines: Vec<String> = v["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|d| {
                    format!(
                        "line {}: {} {}: {}",
                        d["line"],
                        d["severity"].as_str().unwrap_or(""),
                        d["code"].as_str().unwrap_or(""),
                        d["message"].as_str().unwrap_or("")
                    )
                })
                .collect();
            let count = lines.len();
            lines.push(format!(
                "{} error(s), {} diagnostic(s); BPMN conformance class: {}",
                v["errors"],
                count,
                v["bpmn_class"].as_str().unwrap_or("")
            ));
            Outcome::Done {
                text: lines.join("\n"),
                structured: v,
            }
        }
        Err(e) => fail(e),
    }
}

fn check_bpmn(a: &Args) -> Outcome {
    let (xml, path) = match a.source("xml") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let folder = path
        .as_deref()
        .and_then(|p| std::path::Path::new(p).parent())
        .map(|p| p.to_string_lossy().to_string())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| ".".to_string());
    let mut options = json!({
        "lang": lang(a),
        "steps": a.bool("steps", false),
        "camunda": a.bool("camunda", false),
        "folder": folder,
    });
    if let Some(d) = a.str("description") {
        options["steps_from"] = json!(d);
    }
    match call(api::check_bpmn, &xml, &options) {
        Ok(v) => {
            let mut lines: Vec<String> = v["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|d| {
                    format!(
                        "line {}: {} {}: {}",
                        d["line"],
                        d["severity"].as_str().unwrap_or(""),
                        d["code"].as_str().unwrap_or(""),
                        d["message"].as_str().unwrap_or("")
                    )
                })
                .collect();
            let count = lines.len();
            lines.push(format!("{} error(s), {} diagnostic(s)", v["errors"], count));
            Outcome::Done {
                text: lines.join("\n"),
                structured: v,
            }
        }
        Err(e) => fail(e),
    }
}

fn fmt(a: &Args) -> Outcome {
    let (text, _) = match a.source("text") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    match call(api::fmt, &text, &json!({"lang": lang(a)})) {
        Ok(Value::String(t)) => Outcome::Done {
            text: t.clone(),
            structured: json!({"text": t}),
        },
        Ok(other) => fail(format!("internal error: unexpected result {other}")),
        Err(e) => fail(e),
    }
}

/// The kind of an exported file for `formats`.
fn file_kind(name: &str) -> &'static str {
    if name.ends_with(".drd.svg") || name.ends_with(".dmn") {
        "dmn"
    } else if name.ends_with(".svg") {
        "svg"
    } else if name.ends_with(".bpmn") {
        "bpmn"
    } else {
        "forms"
    }
}

fn export(a: &Args) -> Outcome {
    let (text, path) = match a.source("text") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let stem = path
        .as_deref()
        .and_then(|p| std::path::Path::new(p).file_stem())
        .map(|s| s.to_string_lossy().to_string());
    let name = a
        .str("name")
        .map(str::to_string)
        .or(stem)
        .unwrap_or_else(|| "model".into());
    let mut options = json!({"lang": lang(a), "name": name});
    if a.bool("sidecar", true)
        && let Some(p) = &path
        && let Ok(side) = std::fs::read_to_string(format!("{p}.layout"))
    {
        options["sidecar"] = json!(side);
    }
    let wanted: Option<Vec<&str>> = a
        .map
        .get("formats")
        .and_then(Value::as_array)
        .map(|f| f.iter().filter_map(Value::as_str).collect());
    match call(api::export_files, &text, &options) {
        Ok(Value::Object(files)) => {
            let files: Map<String, Value> = files
                .into_iter()
                .filter(|(n, _)| wanted.as_ref().is_none_or(|w| w.contains(&file_kind(n))))
                .collect();
            let structured = json!({"files": files});
            Outcome::Done {
                text: structured.to_string(),
                structured,
            }
        }
        Ok(other) => fail(format!("internal error: unexpected result {other}")),
        Err(e) => fail(e),
    }
}

fn import(a: &Args) -> Outcome {
    let (xml, _) = match a.source("xml") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let options = json!({
        "format": a.str("format").unwrap_or("auto"),
        "flow_ids": a.bool("flow_ids", true),
        "layout": a.bool("layout", true),
    });
    match call(api::import_file, &xml, &options) {
        Ok(v) => {
            let mut text = v["text"].as_str().unwrap_or("").to_string();
            let warnings: Vec<&str> = v["warnings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if !warnings.is_empty() {
                text.push_str("\n# not kept by the import:\n");
                for w in warnings {
                    text.push_str(&format!("# {w}\n"));
                }
            }
            Outcome::Done {
                text,
                structured: v,
            }
        }
        Err(e) => fail(e),
    }
}

fn sim(a: &Args) -> Outcome {
    let (text, _) = match a.source("text") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let mut options = json!({
        "lang": lang(a),
        "log": a.bool("log", false),
        "cases_table": a.bool("cases_table", false),
        "csv": a.bool("csv", false),
        "xes": a.bool("xes", false),
    });
    for key in ["cases", "seed"] {
        if let Some(v) = a.map.get(key) {
            options[key] = v.clone();
        }
    }
    match call(api::sim, &text, &options) {
        Ok(v) => Outcome::Done {
            text: v["report"].as_str().unwrap_or("").to_string(),
            structured: v,
        },
        Err(e) => fail(e),
    }
}

fn eval(a: &Args) -> Outcome {
    let (text, _) = match a.source("text") {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let options = json!({
        "lang": lang(a),
        "decision": a.str("decision").unwrap_or(""),
        "values": a.map.get("values").cloned().unwrap_or_else(|| json!({})),
    });
    match call(api::eval, &text, &options) {
        Ok(v) => Outcome::Done {
            text: v.to_string(),
            structured: json!({"value": v}),
        },
        Err(e) => fail(e),
    }
}

fn reference(a: &Args) -> Outcome {
    let topic = a.str("topic").unwrap_or("cheatsheet");
    match REFERENCE.iter().find(|(t, _, _)| *t == topic) {
        Some((_, _, text)) => Outcome::Done {
            text: (*text).to_string(),
            structured: json!({"text": text}),
        },
        None => fail(format!("unknown topic '{topic}'")),
    }
}
