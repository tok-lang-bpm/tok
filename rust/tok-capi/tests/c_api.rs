//! The C interface called as a C program calls it: through the exported functions, with
//! NUL-terminated strings, and freeing every result with `tok_free`.

// calling C functions with raw pointers is what this test is about
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokengine as capi;

type Fn2 = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel))
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
        .replace("\r\n", "\n")
}

/// Call a function of the interface with raw C strings and parse the result.
fn raw(f: Fn2, input: Option<&[u8]>, options: Option<&str>) -> Value {
    let input = input.map(|b| CString::new(b).expect("no NUL in test input"));
    let options = options.map(|o| CString::new(o).expect("no NUL in test options"));
    let ip = input.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    let op = options.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    // SAFETY: both arguments are NULL or NUL-terminated strings alive during the call.
    let out = unsafe { f(ip, op) };
    assert!(!out.is_null());
    // SAFETY: the library returns a NUL-terminated string.
    let text = unsafe { CStr::from_ptr(out) }
        .to_str()
        .expect("result is UTF-8")
        .to_string();
    // SAFETY: `out` was returned by the library and is freed once.
    unsafe { capi::tok_free(out) };
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("result is not JSON ({e}): {text}"))
}

fn call(f: Fn2, input: &str, options: &str) -> Value {
    raw(f, Some(input.as_bytes()), Some(options))
}

fn ok(v: Value) -> Value {
    match v.get("ok") {
        Some(x) => x.clone(),
        None => panic!("expected ok, got {v}"),
    }
}

fn error_kind(v: &Value) -> &str {
    v["error"]["kind"]
        .as_str()
        .unwrap_or_else(|| panic!("expected error, got {v}"))
}

#[test]
fn version_and_abi() {
    // SAFETY: tok_version returns a static NUL-terminated string.
    let v = unsafe { CStr::from_ptr(capi::tok_version()) };
    assert_eq!(v.to_str().ok(), Some(env!("CARGO_PKG_VERSION")));
    assert_eq!(capi::tok_abi_version(), 1);
    // SAFETY: NULL is accepted and ignored.
    unsafe { capi::tok_free(std::ptr::null_mut()) };
}

#[test]
fn arguments_are_checked() {
    let e = raw(capi::tok_fmt, None, None);
    assert_eq!(error_kind(&e), "invalid_argument");
    let e = raw(capi::tok_fmt, Some(b"tok 0.1\n\xff\n"), None);
    assert_eq!(error_kind(&e), "invalid_argument");
    let e = call(capi::tok_fmt, "tok 0.1\n", "[1]");
    assert_eq!(error_kind(&e), "invalid_argument");
    let e = call(capi::tok_fmt, "tok 0.1\n", "{\"langg\": \"pl\"}");
    assert_eq!(error_kind(&e), "invalid_argument");
    assert!(
        e["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("langg")
    );
    let e = call(capi::tok_check, "tok 0.1\n", "{\"level\": \"top\"}");
    assert_eq!(error_kind(&e), "invalid_argument");
    // NULL options are no options
    let v = ok(raw(capi::tok_fmt, Some(b"tok 0.1\nstart s\n"), None));
    assert_eq!(v, json!("tok 0.1\n\nstart s\n"));
}

#[test]
fn check_and_fmt_follow_the_conformance_corpus() {
    for case in ["02-linear", "11-decision", "53-drd", "15-err-unknown"] {
        let text = read(&format!("conformance/cases/{case}.tok"));
        let v = ok(call(capi::tok_check, &text, "{\"bpmn\": false}"));
        let mut codes: Vec<String> = v["diagnostics"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|d| d["code"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        codes.sort();
        let expected_diag = read(&format!("conformance/expected/{case}.diag"));
        for code in expected_diag.lines() {
            assert!(codes.iter().any(|c| c == code), "{case}: {code} missing");
        }
        let fmt = call(capi::tok_fmt, &text, "");
        if v["errors"].as_i64().unwrap_or(0) > 0 && codes.iter().any(|c| c.starts_with('P')) {
            assert_eq!(error_kind(&fmt), "document", "{case}");
            assert!(fmt["error"]["diagnostics"].is_array());
        } else {
            let expected = read(&format!("conformance/expected/{case}.tok"));
            assert_eq!(ok(fmt), json!(expected), "{case}");
        }
    }
    let v = ok(call(
        capi::tok_check,
        "tok 0.1\nprocez x\n",
        "{\"lang\": \"pl\"}",
    ));
    assert_eq!(v["errors"], json!(1));
    assert_eq!(
        v["diagnostics"][0]["message"],
        json!("nieznane słowo kluczowe „procez”")
    );
}

#[test]
fn drawings_and_files() {
    let text = read("examples/propozycja-zakupu.tok");
    let xml = ok(call(capi::tok_bpmn, &text, ""));
    assert!(xml.as_str().unwrap_or("").contains("bpmndi:BPMNDiagram"));
    let plain = ok(call(capi::tok_bpmn, &text, "{\"layout\": false}"));
    assert!(!plain.as_str().unwrap_or("").contains("bpmndi:BPMNDiagram"));
    let svg = ok(call(capi::tok_svg, &text, "{\"title\": \"Propozycja\"}"));
    assert!(
        svg.as_str().unwrap_or("").starts_with("<svg")
            || svg.as_str().unwrap_or("").starts_with("<?xml")
    );
    let files = ok(call(capi::tok_export, &text, "{\"name\": \"p\"}"));
    let names: Vec<&String> = files
        .as_object()
        .map(|m| m.keys().collect())
        .unwrap_or_default();
    assert!(names.iter().any(|n| *n == "p.bpmn"));
    assert!(names.iter().any(|n| *n == "p.svg"));
    assert!(names.iter().any(|n| n.ends_with(".dmn")));
    assert!(names.iter().any(|n| n.ends_with(".form")));
    let e = call(capi::tok_bpmn, "tok 0.1\n", "");
    assert_eq!(error_kind(&e), "render");
    let e = call(capi::tok_bpmn, "tok 0.1\nstart s\nflow s -> x\n", "");
    assert_eq!(error_kind(&e), "document");
}

#[test]
fn import_bpmn_and_dmn() {
    let xml = read("conformance/import/01-wypozyczenie-pierwszy-model.bpmn");
    let v = ok(call(capi::tok_import, &xml, ""));
    assert_eq!(v["format"], json!("bpmn"));
    assert!(v["text"].as_str().unwrap_or("").starts_with("tok "));
    assert!(v["warnings"].is_array());
    assert!(v["layout"].is_string());
    let v = ok(call(capi::tok_import, &xml, "{\"layout\": false}"));
    assert!(v["layout"].is_null());
    let dmn = read("conformance/dmn-tck/compliance-level-3/0001-filter/0001-filter.dmn");
    let v = ok(call(capi::tok_import, &dmn, ""));
    assert_eq!(v["format"], json!("dmn"));
    assert!(v["text"].as_str().unwrap_or("").starts_with("tok 0.7"));
    let e = call(capi::tok_import, "<not xml", "");
    assert_eq!(error_kind(&e), "import");
}

#[test]
fn decisions() {
    let text = read("conformance/cases/11-decision.tok");
    let v = ok(call(
        capi::tok_eval,
        &text,
        "{\"decision\": \"limit\", \"values\": {\"wiek\": 30, \"typ\": \"student\"}}",
    ));
    assert_eq!(v, json!({"limit": 15}));
    let e = call(capi::tok_eval, &text, "{\"decision\": \"brak\"}");
    assert_eq!(error_kind(&e), "unknown_decision");
    let e = call(capi::tok_eval, &text, "{}");
    assert_eq!(error_kind(&e), "invalid_argument");
    let text = read("conformance/cases/61-executable-decisions.tok");
    let v = ok(call(
        capi::tok_eval,
        &text,
        "{\"decision\": \"limit\", \"values\": {\"Wiek czytelnika\": 12, \"wiek\": 12}}",
    ));
    assert_eq!(v, json!(5));
    let v = ok(call(
        capi::tok_eval,
        &text,
        "{\"decision\": \"naleznosc\", \"values\": {\"dni-zwloki\": 4, \"opłata dzienna\": 1.5}}",
    ));
    assert_eq!(v, json!(6));
}

#[test]
fn simulation_matches_the_engine() {
    let text = read("examples/kwerenda-symulacja.tok");
    let v = ok(call(
        capi::tok_sim,
        &text,
        "{\"cases\": 50, \"seed\": 3, \"xes\": true, \"csv\": true}",
    ));
    let (doc, _) = tok_core::load(&text, "full", None);
    let res = tok_sim::simulate(
        &doc,
        Some(tok_sim::Param::Int("50".into())),
        Some(tok_sim::Param::Int("3".into())),
    )
    .expect("simulation runs");
    let kpi: Value = serde_json::from_str(&res.kpi().to_json().compact()).expect("kpi JSON");
    assert_eq!(v["kpi"], kpi);
    assert_eq!(v["report"], json!(res.kpi().format("en")));
    assert_eq!(v["xes"], json!(res.to_xes().expect("xes")));
    assert_eq!(v["log_csv"], json!(res.log_csv().0));
    assert_eq!(v["cases_csv"], json!(res.cases_csv().0));
    let log = v["log"].as_array().expect("log");
    assert_eq!(log.len(), res.outcome.rows.len());
    assert_eq!(log[0]["case"], json!(1));
    assert!(log[0]["enqueue_s"].is_number());
    let cases = v["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 50);
    assert!(cases[0]["cycle_time_s"].is_number());
    let short = ok(call(
        capi::tok_sim,
        &text,
        "{\"cases\": 5, \"log\": false, \"cases_table\": false}",
    ));
    assert!(short.get("log").is_none() && short.get("cases").is_none());
    let e = call(capi::tok_sim, "tok 0.1\nend e\n", "");
    assert_eq!(error_kind(&e), "simulation");
}

#[test]
fn simulation_model() {
    let text = read("examples/kwerenda-symulacja.tok");
    let m = ok(call(capi::tok_sim_model, &text, ""));
    assert_eq!(m["process"], json!("archiwum-process"));
    assert_eq!(m["start_event"], json!("wplyw"));
    assert_eq!(m["cases"], json!(300));
    assert_eq!(m["seed"], json!(7));
    assert_eq!(m["arrival"]["name"], json!("exp"));
    assert_eq!(m["arrival"]["args"], json!([14400.0]));
    assert_eq!(
        m["resources"][0],
        json!({"id": "archiwisci", "label": "Archiwiści", "count": 2})
    );
    let ocena = m["nodes"]
        .as_array()
        .and_then(|n| n.iter().find(|x| x["id"] == json!("ocena")))
        .expect("node ocena");
    assert_eq!(ocena["attrs"]["by"], json!("archiwisci"));
    assert_eq!(ocena["duration"]["name"], json!("lnorm"));
    assert_eq!(ocena["duration"]["args"], json!([1800.0, 600.0]));
    let flows = m["flows"].as_array().expect("flows");
    assert!(
        flows
            .iter()
            .any(|f| f["source"] == json!("mozliwa") && f["p"] == json!(0.85))
    );
    let timers = ok(call(
        capi::tok_sim_model,
        "tok 0.3\nstart s\ncatch a timer=PT2H\ncatch b timer=\"2027-01-05T08:00\"\ncatch c timer=x\nend e\nflow s -> a -> b -> c -> e\n",
        "",
    ));
    let t = |id: &str| {
        timers["nodes"]
            .as_array()
            .and_then(|n| n.iter().find(|x| x["id"] == json!(id)))
            .map(|x| x["timer"].clone())
            .unwrap_or(Value::Null)
    };
    assert_eq!(t("a"), json!({"kind": "duration", "seconds": 7200.0}));
    assert_eq!(t("b"), json!({"kind": "date", "seconds": 86400.0}));
    assert_eq!(t("c")["kind"], json!("invalid"));
    let d = ok(call(capi::tok_sim_dist, "tri(5m, 10m, 20m)", ""));
    assert_eq!(
        d,
        json!({"name": "tri", "kind": "duration", "args": [300.0, 600.0, 1200.0], "labels": []})
    );
    let d = ok(call(
        capi::tok_sim_dist,
        "cat(a=0.5, true=0.25, 3=0.25)",
        "",
    ));
    assert_eq!(d["labels"], json!(["a", true, 3.0]));
    let e = call(capi::tok_sim_dist, "tri(5m)", "");
    assert_eq!(error_kind(&e), "distribution");
}

#[test]
fn simulation_expressions() {
    let v = ok(call(
        capi::tok_sim_eval,
        "koszt > 150 and rodzaj = \"e-book\"",
        "{\"vars\": {\"koszt\": 200.5, \"rodzaj\": \"e-book\"}}",
    ));
    assert_eq!(v, json!({"value": true, "truthy": true}));
    let v = ok(call(
        capi::tok_sim_eval,
        "cena + 10",
        "{\"vars\": {\"cena\": 5.0}}",
    ));
    assert_eq!(v["value"], json!(15));
    let e = call(
        capi::tok_sim_eval,
        "koszt ** 2 > 100",
        "{\"vars\": {\"koszt\": 2.0}}",
    );
    assert_eq!(error_kind(&e), "feel");
    let v = ok(call(
        capi::tok_sim_eval,
        "count([1, 2]) = 2",
        "{\"lang07\": true}",
    ));
    assert_eq!(v["truthy"], json!(true));
}

#[test]
fn command_line() {
    let v = ok(call(capi::tok_cli, "[\"--version\"]", ""));
    assert_eq!(v, json!({"status": 0}));
    let v = ok(call(capi::tok_cli, "[\"check\", \"no-such-file.tok\"]", ""));
    assert_eq!(v, json!({"status": 2}));
    let e = call(capi::tok_cli, "{}", "");
    assert_eq!(error_kind(&e), "invalid_argument");
}

#[test]
fn concurrent_calls() {
    let text = read("examples/kwerenda-symulacja.tok");
    let handles: Vec<_> = (0..4)
        .map(|k| {
            let text = text.clone();
            std::thread::spawn(move || {
                let v = ok(call(
                    capi::tok_sim,
                    &text,
                    &format!("{{\"cases\": 30, \"seed\": {k}, \"log\": false}}"),
                ));
                v["kpi"]["cases"]["started"].as_i64()
            })
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().ok().flatten(), Some(30));
    }
}

/// The C program `tests/c/example.c`, compiled against the shared library when a C
/// compiler (`cc`) is available on a Unix system.
#[cfg(unix)]
#[test]
fn c_program() {
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    if std::process::Command::new(&cc)
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("no C compiler: the C program is not built");
        return;
    }
    // target/<profile>/deps/c_api-HASH → target/<profile>
    let exe = std::env::current_exe().expect("test executable");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target directory")
        .to_path_buf();
    let lib = ["libtokengine.so", "libtokengine.dylib"]
        .iter()
        .map(|n| profile_dir.join(n))
        .find(|p| p.exists());
    let Some(lib) = lib else {
        eprintln!("shared library not built in {}", profile_dir.display());
        return;
    };
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::temp_dir().join(format!("tok-capi-example-{}", std::process::id()));
    let status = std::process::Command::new(&cc)
        .arg(manifest.join("tests/c/example.c"))
        .arg("-I")
        .arg(manifest.join("include"))
        .arg(&lib)
        .arg(format!("-Wl,-rpath,{}", profile_dir.display()))
        .arg("-o")
        .arg(&out)
        .status()
        .expect("C compiler runs");
    assert!(status.success(), "the C program compiles");
    let run = std::process::Command::new(&out)
        .output()
        .expect("C program runs");
    let _ = std::fs::remove_file(&out);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "C program failed: {stdout}");
    assert!(
        stdout.contains("{\"ok\":\"tok 0.1\\n\\nstart s\\n\"}"),
        "{stdout}"
    );
    assert!(stdout.contains("\"kind\":\"document\""), "{stdout}");
}
