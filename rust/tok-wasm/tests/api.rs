//! The functions of the module on the host: the JSON results of the C interface, for the
//! examples of the repository.

use std::path::Path;

use serde_json::{Value, json};

fn example(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    std::fs::read_to_string(path).expect("example exists")
}

fn ok(result: &str) -> Value {
    let v: Value = serde_json::from_str(result).expect("JSON result");
    assert!(v.get("error").is_none(), "{v}");
    v["ok"].clone()
}

#[test]
fn every_function_answers_like_the_c_interface() {
    let text = example("propozycja-zakupu.tok");
    let check = ok(&tok_wasm::check(&text, ""));
    assert_eq!(check["errors"], 0);
    let fmt = ok(&tok_wasm::fmt(&text, ""));
    assert_eq!(
        fmt,
        Value::String(
            tok_cli::api::fmt(&text, "")
                .map(|j| j.compact())
                .map(|s| serde_json::from_str::<String>(&s).expect("string"))
                .expect("formatted")
        )
    );
    let bpmn = ok(&tok_wasm::bpmn(&text, ""));
    assert!(bpmn.as_str().expect("xml").contains("bpmndi:BPMNDiagram"));
    let svg = ok(&tok_wasm::svg(&text, r#"{"title": "propozycja.svg"}"#));
    assert!(svg.as_str().expect("svg").contains("<svg"));
    let files = ok(&tok_wasm::export(&text, r#"{"name": "propozycja"}"#));
    assert!(files.get("propozycja.bpmn").is_some() && files.get("tryb-akceptacji.dmn").is_some());
    let back = ok(&tok_wasm::import_file(
        bpmn.as_str().expect("xml"),
        r#"{"layout": false}"#,
    ));
    assert_eq!(back["format"], "bpmn");
    let value = ok(&tok_wasm::eval(
        &text,
        &json!({"decision": "tryb-akceptacji", "values": {"rodzaj": "e-book", "cena": 120}})
            .to_string(),
    ));
    assert_eq!(value, json!({"tryb": "kierownik"}));
    let sim = ok(&tok_wasm::sim(
        &example("kwerenda-symulacja.tok"),
        r#"{"seed": 1, "log": false, "cases_table": false}"#,
    ));
    assert!(sim["report"].as_str().expect("report").contains("seed 1"));
    assert_eq!(tok_wasm::version(), env!("CARGO_PKG_VERSION"));
    assert!(tok_wasm::cheatsheet().contains("flow"));
}

#[test]
fn errors_come_back_as_data() {
    let v: Value = serde_json::from_str(&tok_wasm::svg(
        "tok 0.1\nstart s\nflow s -> x\n",
        r#"{"lang": "pl"}"#,
    ))
    .expect("JSON");
    assert_eq!(v["error"]["kind"], "document");
    assert_eq!(v["error"]["diagnostics"][0]["code"], "R002");
    let v: Value =
        serde_json::from_str(&tok_wasm::check("tok 0.1\n", "{\"colour\": 1}")).expect("JSON");
    assert_eq!(v["error"]["kind"], "invalid_argument");
}
