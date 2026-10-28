//! The properties checked by the fuzz targets. The regression tests of the engine
//! (`tok-cli/tests/fuzz_regressions.rs`) include this file and run every input kept in
//! `fuzz/regressions` through the same functions, so an input that once failed is
//! checked on every platform by `cargo test`.
//!
//! Each function takes the raw bytes of one input. It returns normally when the engine
//! behaves; a panic (including a failed assertion of a property) is a finding.

use tok_cli::api;
use tok_core::Diag;
use tok_core::feel3::eval::{Env, Eval, Types};
use tok_core::feel3::parse::{
    Names, camunda_text, free_names, parse_expression, parse_tests, parse_type,
};
use tok_core::feel3::{Map, Value};
use tok_core::json::Json;
use tok_core::{export, feel, feel_eval, printer};

/// Inputs of the FEEL target longer than this are skipped: the size of an expression
/// bounds the work of its evaluation only loosely (`for` over ranges multiplies it).
pub const MAX_FEEL: usize = 512;

/// Documents larger than this are skipped by the targets that lay out, render or
/// simulate a model.
pub const MAX_MODEL: usize = 64 * 1024;

fn text(data: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(data).ok()?;
    // universal newlines, as the command line reads files
    Some(s.replace("\r\n", "\n").replace('\r', "\n"))
}

fn has_errors(diags: &[Diag]) -> bool {
    diags.iter().any(Diag::is_error)
}

/// Lexer, parser and checker at every level and conformance class, with the BPMN rules.
pub fn check(data: &[u8]) {
    let Some(text) = text(data) else { return };
    for level in ["basic", "analytic", "full"] {
        let _ = api::check(&text, &format!(r#"{{"level": "{level}"}}"#));
    }
    for class in tok_core::classes::CLASSES {
        let _ = api::check(&text, &format!(r#"{{"class": "{class}", "bpmn": false}}"#));
    }
}

/// What a document means, without its line numbers: the semantic BPMN export, the
/// decision and form files (by name) and the simulation model.
fn meaning(doc: &tok_core::Document) -> Vec<String> {
    let mut out = vec![export::export_xml(doc, None)];
    // the files of the decision requirements graphs, in the order of the graphs' first
    // elements, which the canonical form may change (input data come first)
    let mut files = tok_core::drg::export::decision_files(doc);
    files.sort();
    for (name, content) in files {
        out.push(name);
        out.push(content);
    }
    for form in doc.forms.values() {
        out.push(tok_core::forms::export_form(form));
    }
    out.push(match tok_sim::describe::describe(doc, None, None) {
        Ok(j) => canonical_json(j).compact(),
        Err(e) => e.to_string(),
    });
    out
}

/// The simulation model without what the canonical form may reorder and the simulator
/// does not depend on: the order of attributes and of the entries of `listeners=`, and the
/// order of sequence flows except among the flows that leave one node (a gateway takes the
/// first that holds).
fn canonical_json(j: Json) -> Json {
    match j {
        Json::Obj(fields) => {
            let mut fields: Vec<(String, Json)> = fields
                .into_iter()
                .map(|(k, v)| {
                    let mut v = canonical_json(v);
                    // the canonical form orders the entries of `listeners=` (execution
                    // listeners first), which the export reads in any order
                    if k == "listeners"
                        && let Json::Str(text) = &v
                    {
                        v = Json::Str(printer::normalize_listeners(text));
                    }
                    if k == "flows"
                        && let Json::Arr(items) = &mut v
                    {
                        items.sort_by_key(|f| match f {
                            Json::Obj(m) => m
                                .iter()
                                .find(|(k, _)| k == "source")
                                .map(|(_, s)| s.compact())
                                .unwrap_or_default(),
                            _ => String::new(),
                        });
                    }
                    (k, v)
                })
                .collect();
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            Json::Obj(fields)
        }
        Json::Arr(items) => Json::Arr(items.into_iter().map(canonical_json).collect()),
        other => other,
    }
}

/// The canonical printer: for a valid document, parse → print → parse → print is a
/// fixed point after the first print, and printing does not change what the document
/// means.
pub fn roundtrip(data: &[u8]) {
    let Some(text) = text(data) else { return };
    let (doc, diags) = tok_core::load(&text, "full", None);
    if has_errors(&diags) {
        return;
    }
    let first = printer::fmt(&doc);
    let (again, diags) = tok_core::load(&first, "full", None);
    assert!(
        !has_errors(&diags),
        "the canonical form has errors: {diags:?}\n--- canonical form\n{first}"
    );
    let second = printer::fmt(&again);
    assert_eq!(first, second, "printing the canonical form changes it");
    for (a, b) in meaning(&doc).iter().zip(meaning(&again).iter()) {
        assert!(
            a == b,
            "the canonical form means something else\n--- before\n{a}\n--- after\n{b}\n--- canonical form\n{first}"
        );
    }
}

fn subset_context() -> feel_eval::Context {
    let mut ctx = feel_eval::Context::new();
    let num = |s: &str| {
        tok_core::pydec::Decimal::parse(s).map_or(feel_eval::Value::Null, feel_eval::Value::Num)
    };
    ctx.insert("a".into(), num("1"));
    ctx.insert("b".into(), num("-2.5"));
    ctx.insert("s".into(), feel_eval::Value::Str("tekst".into()));
    ctx.insert("t".into(), feel_eval::Value::Bool(true));
    ctx.insert(
        "l".into(),
        feel_eval::Value::List(vec![num("1"), num("2"), num("3")]),
    );
    ctx
}

fn full_context() -> Map {
    let mut m = Map::new();
    m.insert("a".into(), Value::int(1));
    m.insert("s".into(), Value::str("tekst"));
    m.insert("t".into(), Value::Bool(true));
    m.insert(
        "l".into(),
        Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]),
    );
    m.insert(
        "d".into(),
        tok_core::feel3::eval::temporal_literal("2027-01-15"),
    );
    let mut c = Map::new();
    c.insert("x".into(), Value::int(2));
    c.insert("y z".into(), Value::str("w"));
    m.insert("c".into(), Value::context(c));
    m
}

/// FEEL of languages 0.1–0.6 (the subset, conformance level 2) and of language 0.7
/// (conformance level 3): parsing and evaluation of expressions, unary tests and types.
pub fn feel(data: &[u8]) {
    if data.len() > MAX_FEEL {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // level 2
    let _ = feel::check_expr(text);
    let _ = feel::check_unary(text);
    let _ = feel::detail_pl(text);
    let ctx = subset_context();
    if let Ok(ast) = feel::parse_expr(text) {
        let _ = feel_eval::evaluate(&ast, &ctx);
    }
    if let Ok(tests) = feel::parse_unary_tests(text) {
        let _ = feel_eval::test(&tests, &feel_eval::Value::Str("tekst".into()), &ctx);
    }
    // level 3
    let mut names = Names::builtins();
    for n in ["a", "s", "t", "l", "d", "c", "y z"] {
        names.add(n);
    }
    let types = Types::default();
    let ev = Eval::new(&types);
    let env = Env::root(full_context());
    if let Ok(e) = parse_expression(text, &names) {
        let mut free = Vec::new();
        free_names(&e, &mut free);
        let _ = ev.eval(&e, &env).literal();
    }
    if let Ok(t) = parse_tests(text, &names) {
        let _ = ev.test(&t, &Value::int(1), &env);
    }
    let _ = parse_type(text, &names);
    let _ = camunda_text(text, &names, false);
    let _ = camunda_text(text, &names, true);
}

fn imported(result: &api::Res) {
    let Ok(v) = result else { return };
    let tok_core::json::Json::Obj(fields) = v else {
        panic!("the import result is not an object: {v:?}");
    };
    let Some((_, tok_core::json::Json::Str(text))) = fields.iter().find(|(k, _)| k == "text")
    else {
        panic!("the import result has no text");
    };
    // the import writes the canonical form of a document that reads back
    let (doc, diags) = tok_core::parse(text);
    let syntax: Vec<_> = diags
        .iter()
        .filter(|d| d.is_error() && (d.code.starts_with('T') || d.code.starts_with('P')))
        .collect();
    assert!(
        syntax.is_empty(),
        "the imported document does not parse: {syntax:?}\n--- imported\n{text}"
    );
    if !has_errors(&diags) {
        assert_eq!(
            &printer::fmt(&doc),
            text,
            "the imported text is not canonical"
        );
    }
}

/// BPMN 2.0 XML import (with the layout sidecar of its diagram).
pub fn import_bpmn(data: &[u8]) {
    let Ok(xml) = std::str::from_utf8(data) else {
        return;
    };
    imported(&api::import_file(xml, r#"{"format": "bpmn"}"#));
}

/// DMN XML import.
pub fn import_dmn(data: &[u8]) {
    let Ok(xml) = std::str::from_utf8(data) else {
        return;
    };
    imported(&api::import_file(xml, r#"{"format": "dmn"}"#));
}

/// The models the sidecar target draws; the first byte of an input chooses one.
pub const SIDECAR_MODELS: [&str; 3] = [
    "tok 0.1 \"Wniosek\"\n\
     start s \"Start\"\n\
     user a \"Ocena\"\n\
     boundary t \"Termin\" on=a timer=P1D\n\
     xor g \"Zgoda?\"\n\
     end e1 \"Tak\"\n\
     end e2 \"Nie\"\n\
     end e3 \"Po terminie\"\n\
     data w \"Wniosek\"\n\
     note n \"Uwaga\" on=a\n\
     flow s -> a -> g\n\
     flow g -> e1 \"tak\"\n\
     flow g -> e2 \"nie\"\n\
     flow t -> e3\n\
     reads a w\n",
    "tok 0.1 \"Współpraca\"\n\
     pool p \"Biblioteka\"\n\
     \x20 lane l1 \"Czytelnik\"\n\
     \x20   start s\n\
     \x20   send z \"Zamówienie\" message=\"Z\"\n\
     \x20 lane l2 \"Bibliotekarz\"\n\
     \x20   sub k \"Kompletowanie\"\n\
     \x20     start ks\n\
     \x20     task kt \"Szukanie\"\n\
     \x20     end ke\n\
     \x20     flow ks -> kt -> ke\n\
     \x20   end e\n\
     pool q \"Dostawca\"\n\
     flow s -> z -> k -> e\n\
     message z -> q \"Zamówienie\"\n",
    "tok 0.6 \"Konwersacja\"\n\
     pool a \"A\"\n\
     pool b \"B\"\n\
     pool c \"C\" max=3\n\
     message a -> b \"M1\" id=m1\n\
     message b -> c \"M2\" id=m2\n\
     conversation k \"Rozmowa\" pools=\"a b\" messages=\"m1\"\n\
     conversation k2 \"Druga\" pools=\"b c\" messages=\"m2\"\n\
     choreography ch \"Choreografia\"\n\
     \x20 start s\n\
     \x20 choreo t \"Zamówienie\" from=a to=b message=\"Z\" reply=\"P\"\n\
     \x20 end e\n\
     \x20 flow s -> t -> e\n",
];

/// The layout sidecar: reading it and drawing a model with it (BPMN layout and SVG).
pub fn sidecar(data: &[u8]) {
    let Some((&choice, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let Ok(side) = tok_bpmn::Sidecar::parse(text) else {
        return;
    };
    let _ = side.to_text();
    let model = SIDECAR_MODELS[usize::from(choice) % SIDECAR_MODELS.len()];
    let (doc, diags) = tok_core::load(model, "full", None);
    assert!(!has_errors(&diags), "a sidecar model has errors: {diags:?}");
    let xml = export::export_xml(&doc, None);
    let _ = tok_bpmn::layout_with(&xml, &side);
    let _ = tok_bpmn::render_svg_with(&xml, None, &side);
}

/// Export of a valid document: BPMN with layout, SVG and every file of `tok export`.
pub fn render(data: &[u8]) {
    if data.len() > MAX_MODEL {
        return;
    }
    let Some(text) = text(data) else { return };
    let (_, diags) = tok_core::load(&text, "full", None);
    if has_errors(&diags) {
        return;
    }
    let _ = api::bpmn(&text, "");
    let _ = api::bpmn(&text, r#"{"layout": false}"#);
    let _ = api::svg(&text, r#"{"title": "model.svg"}"#);
    let _ = api::export_files(&text, "");
}

/// The simulator on a small valid document: a few cases, one seed.
pub fn sim(data: &[u8]) {
    if data.len() > MAX_MODEL {
        return;
    }
    let Some(text) = text(data) else { return };
    let (doc, diags) = tok_core::load(&text, "full", None);
    if has_errors(&diags) || doc.nodes.len() > 60 {
        return;
    }
    let _ = api::sim(
        &text,
        r#"{"cases": 3, "seed": 1, "csv": true, "xes": true}"#,
    );
    let _ = api::sim_model(&text, "");
}

/// The property of a target: it panics on a finding.
pub type Property = fn(&[u8]);

/// The targets by name, for the regression tests.
pub const TARGETS: [(&str, Property); 8] = [
    ("check", check),
    ("roundtrip", roundtrip),
    ("feel", feel),
    ("import_bpmn", import_bpmn),
    ("import_dmn", import_dmn),
    ("sidecar", sidecar),
    ("render", render),
    ("sim", sim),
];
