//! `tok eval` parity with the Python reference command line.
//!
//! `tok-core/tests/fixtures/eval/cases.json` is produced by `gen_python_eval.py`: for every
//! decision table of the language examples, the conformance cases, the course examples and a
//! set of synthetic tables, `python -m tok eval` was run over a grid of input values, and over
//! argument errors. This test runs the `tok` binary with the same arguments in the same folder
//! and compares standard output, standard error and the exit code.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

struct Case {
    args: Vec<String>,
    env_lang: Option<String>,
    stdout: String,
    stderr: String,
    code: i32,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tok-core")
        .join("tests")
        .join("fixtures")
        .join("eval")
}

fn load_cases() -> Vec<Case> {
    let p = fixtures().join("cases.json");
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}; run gen_python_eval.py", p.display()));
    let data: Value = serde_json::from_str(&text).expect("cases.json is JSON");
    data.as_array()
        .expect("a list of cases")
        .iter()
        .map(|c| Case {
            args: c["args"]
                .as_array()
                .expect("args")
                .iter()
                .map(|a| a.as_str().expect("argument").to_string())
                .collect(),
            env_lang: c.get("env_lang").and_then(|v| v.as_str()).map(String::from),
            stdout: c["stdout"].as_str().expect("stdout").to_string(),
            stderr: c["stderr"].as_str().expect("stderr").to_string(),
            code: c["code"].as_i64().expect("code") as i32,
        })
        .collect()
}

fn run(case: &Case, dir: &Path) -> Option<String> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tok"));
    cmd.args(&case.args).current_dir(dir).env_remove("TOK_LANG");
    if let Some(lang) = &case.env_lang {
        cmd.env("TOK_LANG", lang);
    }
    let out = cmd.output().expect("run tok");
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
    let code = out.status.code().unwrap_or(-1);
    if stdout == case.stdout && stderr == case.stderr && code == case.code {
        return None;
    }
    Some(format!(
        "tok {}\n  python: code {} stdout {:?} stderr {:?}\n  rust:   code {code} stdout {stdout:?} stderr {stderr:?}",
        case.args.join(" "),
        case.code,
        case.stdout,
        case.stderr
    ))
}

#[test]
fn eval_matches_python() {
    let cases = load_cases();
    assert!(
        cases.len() > 1000,
        "fixtures missing; run gen_python_eval.py"
    );
    let dir = fixtures();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(16);
    let chunk = cases.len().div_ceil(workers);
    let failures: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = cases
            .chunks(chunk)
            .map(|part| {
                let dir = &dir;
                s.spawn(move || part.iter().filter_map(|c| run(c, dir)).collect::<Vec<_>>())
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("worker"))
            .collect()
    });
    assert!(
        failures.is_empty(),
        "{} of {} eval cases differ from Python:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
