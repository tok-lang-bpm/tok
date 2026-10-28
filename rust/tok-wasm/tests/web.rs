//! The WebAssembly module itself, run by `wasm-bindgen-test-runner` under Node.js
//! (`tools/playground.sh --test`, and `tools/check.sh` when the target is installed).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::wasm_bindgen_test;

const MODEL: &str = "tok 0.1 \"Zgłoszenie\"\nstart s \"Początek\"\nuser a \"Rozpatrzenie\" duration=\"exp(1h)\"\nxor g \"Czy zgoda?\"\nend tak\nend nie\nflow s -> a -> g\nflow g -> tak \"tak\" p=0.7\nflow g -> nie \"nie\" p=0.3\nsimulate cases=50 seed=7\n";

fn ok(result: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(result).expect("JSON result");
    assert!(v.get("error").is_none(), "{v}");
    v["ok"].clone()
}

#[wasm_bindgen_test]
fn the_module_checks_formats_and_draws() {
    tok_wasm::use_page_clock();
    assert_eq!(ok(&tok_wasm::check(MODEL, ""))["errors"], 0);
    assert!(tok_wasm::fmt(MODEL, "").contains("flow s -> a -> g"));
    assert!(tok_wasm::svg(MODEL, "").contains("<svg"));
    assert!(tok_wasm::bpmn(MODEL, "").contains("BPMNShape"));
    assert!(tok_wasm::export(MODEL, "{\"name\": \"z\"}").contains("\"z.bpmn\""));
}

#[wasm_bindgen_test]
fn the_module_simulates_and_evaluates() {
    tok_wasm::use_page_clock();
    // the simulator runs on the stack of the module (no threads in WebAssembly)
    let sim = tok_wasm::sim(MODEL, "{\"log\": false, \"cases_table\": false}");
    assert!(sim.contains("\"report\""), "{sim}");
    let doc = "tok 0.7 \"D\"\ninputdata x \"X\" type=number\ndecision d \"D\" requires=\"x\" expr=\"x * 2\"\n";
    let v = tok_wasm::eval(doc, "{\"decision\": \"d\", \"values\": {\"x\": 21}}");
    assert_eq!(ok(&v), serde_json::json!(42));
    // FEEL reads the clock of the page
    let doc = "tok 0.7 \"D\"\ndecision d \"D\" expr=\"today().year >= 2026\"\n";
    assert_eq!(
        ok(&tok_wasm::eval(doc, "{\"decision\": \"d\"}")),
        serde_json::json!(true)
    );
    let bpmn: serde_json::Value =
        serde_json::from_str(&tok_wasm::bpmn(MODEL, "")).expect("JSON result");
    let xml = bpmn["ok"].as_str().expect("BPMN XML");
    let back = tok_wasm::import_file(xml, "{\"layout\": false}");
    assert_eq!(ok(&back)["format"], "bpmn");
}
