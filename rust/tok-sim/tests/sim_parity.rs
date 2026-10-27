//! Parity of the simulator library with `sim.py`.
//!
//! The fixtures in `tests/fixtures/sim` are produced by `gen_python_sim.py` (see there):
//! runs of `python -m tok sim`, one file per platform. This test takes the runs that use
//! only `--cases`, `--seed`, `--json` and `--lang` on a document without errors, runs
//! [`tok_sim::simulate`] on the same document and compares the indicators report or the
//! JSON text with Python's standard output, and the uncaught exceptions with the last line
//! of Python's traceback. The command line itself, CSV files and argument errors are
//! compared by `tok-cli/tests/sim_parity.rs`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use tok_sim::{Param, simulate};

const PLATFORM: &str = if cfg!(windows) {
    "windows"
} else if cfg!(target_os = "macos") {
    "macos"
} else {
    "linux"
};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sim")
}

/// Options of a run, or `None` when the run needs the command line.
struct Run {
    file: String,
    cases: Option<String>,
    seed: Option<String>,
    json: bool,
    lang: String,
}

fn run_of(case: &Value) -> Option<Run> {
    if case.get("env_lang").is_some() || case.get("files").is_some() {
        return None;
    }
    let args: Vec<&str> = case["args"]
        .as_array()?
        .iter()
        .map(|a| a.as_str().unwrap_or(""))
        .collect();
    if args.first() != Some(&"sim") || args.len() < 2 || args[1].starts_with('-') {
        return None;
    }
    let mut run = Run {
        file: args[1].to_string(),
        cases: None,
        seed: None,
        json: false,
        lang: "en".into(),
    };
    let mut i = 2;
    while i < args.len() {
        match (args[i], args.get(i + 1)) {
            ("--cases", Some(v)) if v.parse::<i64>().is_ok() => run.cases = Some(v.to_string()),
            ("--seed", Some(v)) if v.parse::<i64>().is_ok() => run.seed = Some(v.to_string()),
            ("--lang", Some(v)) if *v == "en" || *v == "pl" => run.lang = v.to_string(),
            ("--json", _) => {
                run.json = true;
                i += 1;
                continue;
            }
            _ => return None,
        }
        i += 2;
    }
    Some(run)
}

#[test]
fn simulator_matches_python() {
    let p = fixtures().join(format!("cases-{PLATFORM}.json"));
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}; run gen_python_sim.py on this platform",
            p.display()
        )
    });
    let data: Value = serde_json::from_str(&text).expect("fixture is JSON");
    let mut compared = 0;
    let mut failures = Vec::new();
    for case in data["cases"].as_array().expect("a list of cases") {
        let Some(run) = run_of(case) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(fixtures().join(&run.file)) else {
            continue;
        };
        let source = source.replace("\r\n", "\n");
        let (doc, diags) = tok_core::load(&source, "full", None);
        if diags.iter().any(|d| d.is_error()) {
            continue;
        }
        let code = case["code"].as_i64().unwrap_or(-1);
        let stdout = case["stdout"].as_str().unwrap_or("");
        let stderr = case["stderr"].as_str().unwrap_or("");
        if code == 1 && stderr.is_empty() {
            // errors of the BPMN rules, which only the command line applies
            continue;
        }
        let got = simulate(
            &doc,
            run.cases.clone().map(Param::Int),
            run.seed.clone().map(Param::Int),
        )
        .and_then(|r| {
            if run.json {
                Ok(r.to_json(false)? + "\n")
            } else {
                Ok(r.kpi().format(&run.lang))
            }
        });
        let (got_out, got_err, got_code) = match got {
            Ok(out) => (out, String::new(), 0),
            Err(e) => (String::new(), format!("{e}\n"), 1),
        };
        if (got_out.as_str(), got_err.as_str(), got_code) != (stdout, stderr, code) {
            failures.push(format!(
                "{}: python code {code} {stderr:?}, rust code {got_code} {got_err:?}",
                case["args"]
            ));
        }
        compared += 1;
    }
    assert!(compared > 300, "only {compared} runs compared");
    assert!(
        failures.is_empty(),
        "{} of {compared} runs differ from Python ({PLATFORM}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
