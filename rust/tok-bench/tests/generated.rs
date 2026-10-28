//! The generated models are valid documents: no errors at the level `full`, with the BPMN
//! rules, and their canonical form is a fixed point.

use tok_core::{Diag, printer};

fn assert_valid(name: &str, text: &str) {
    let r = tok_cli::api::check(text, "{}");
    let Ok(tok_core::json::Json::Obj(fields)) = r else {
        panic!("{name}: check failed: {r:?}");
    };
    let errors = fields
        .iter()
        .find(|(k, _)| k == "errors")
        .map(|(_, v)| v.compact());
    assert_eq!(
        errors.as_deref(),
        Some("0"),
        "{name} has errors: {fields:?}"
    );
    let (doc, diags) = tok_core::parse(text);
    assert!(!diags.iter().any(Diag::is_error));
    let canonical = printer::fmt(&doc);
    let (again, _) = tok_core::parse(&canonical);
    assert_eq!(
        printer::fmt(&again),
        canonical,
        "{name}: fmt is not a fixed point"
    );
}

#[test]
fn generated_models_are_valid() {
    assert_valid("process", &tok_bench::process(120));
    assert_valid("collaboration", &tok_bench::collaboration(3, 3, 4));
    assert_valid("nested", &tok_bench::nested(12));
    assert_valid("decision table", &tok_bench::decision_table(50));
    assert_valid("graph", &tok_bench::graph(20));
    assert_valid("feel", &tok_bench::feel_document(50));
}
