//! `tok sim` parity with the Python reference command line.
//!
//! `tok-sim/tests/fixtures/sim/cases-PLATFORM.json` is produced by
//! `tok-sim/tests/gen_python_sim.py`: `python -m tok sim` over the language examples, the
//! conformance cases, the course examples and synthetic models, for several seeds and case
//! counts, in text and JSON, in English and Polish, with CSV outputs and argument errors.
//! This test runs the `tok` binary with the same arguments in the same folder and compares
//! standard output, standard error, the exit code and the CSV files. Random draws that go
//! through the C library's `log`, `exp`, `sin` and `cos` differ in the last bit between
//! platforms, so the test reads the fixture of the platform it runs on. The Rust command
//! also has `--xes`, which Python's usage line and help text get before the comparison.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

const PLATFORM: &str = if cfg!(windows) {
    "windows"
} else if cfg!(target_os = "macos") {
    "macos"
} else {
    "linux"
};

/// The placeholder of a temporary folder in the arguments of the fixtures.
const OUT: &str = "@OUT@";

struct Case {
    args: Vec<String>,
    env_lang: Option<String>,
    stdout: String,
    stderr: String,
    code: i32,
    files: Vec<(String, String)>,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tok-sim")
        .join("tests")
        .join("fixtures")
        .join("sim")
}

/// Python's usage line and help text of `tok sim` with the option `--xes`, which only the
/// Rust command has.
fn with_xes_option(text: &str) -> String {
    text.replace(
        "[--cases-csv CASES_CSV] [--lang",
        "[--cases-csv CASES_CSV] [--xes XES] [--lang",
    )
    .replace(
        "write one row per case (CSV)\n",
        "write one row per case (CSV)\n  --xes XES             write the event log (XES, IEEE 1849-2016)\n",
    )
}

fn load_cases() -> Vec<Case> {
    let p = fixtures().join(format!("cases-{PLATFORM}.json"));
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}; run gen_python_sim.py on this platform",
            p.display()
        )
    });
    let data: Value = serde_json::from_str(&text).expect("fixture is JSON");
    let text_of = |v: &Value| v.as_str().expect("text").to_string();
    data["cases"]
        .as_array()
        .expect("a list of cases")
        .iter()
        .map(|c| Case {
            args: c["args"]
                .as_array()
                .expect("args")
                .iter()
                .map(text_of)
                .collect(),
            env_lang: c.get("env_lang").map(text_of),
            stdout: with_xes_option(&text_of(&c["stdout"])),
            stderr: with_xes_option(&text_of(&c["stderr"])),
            code: c["code"].as_i64().expect("code") as i32,
            files: c
                .get("files")
                .and_then(Value::as_object)
                .map(|m| m.iter().map(|(k, v)| (k.clone(), text_of(v))).collect())
                .unwrap_or_default(),
        })
        .collect()
}

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(case: &Case, dir: &Path) -> Option<String> {
    let tmp = std::env::temp_dir().join(format!(
        "tok-sim-parity-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&tmp).expect("temporary folder");
    let args: Vec<String> = case
        .args
        .iter()
        .map(|a| a.replace(OUT, &tmp.to_string_lossy()))
        .collect();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tok"));
    cmd.args(&args).current_dir(dir).env_remove("TOK_LANG");
    if let Some(lang) = &case.env_lang {
        cmd.env("TOK_LANG", lang);
    }
    let out = cmd.output().expect("run tok");
    let stdout = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n");
    let code = out.status.code().unwrap_or(-1);
    let mut files: Vec<(String, String)> = std::fs::read_dir(&tmp)
        .expect("temporary folder")
        .map(|e| {
            let e = e.expect("entry");
            let text = std::fs::read_to_string(e.path()).expect("CSV file");
            (e.file_name().to_string_lossy().to_string(), text)
        })
        .collect();
    files.sort();
    let _ = std::fs::remove_dir_all(&tmp);
    if stdout == case.stdout && stderr == case.stderr && code == case.code && files == case.files {
        return None;
    }
    let first_diff = |a: &str, b: &str| {
        a.lines()
            .zip(b.lines())
            .find(|(x, y)| x != y)
            .map(|(x, y)| format!("\n    python: {x:?}\n    rust:   {y:?}"))
            .unwrap_or_default()
    };
    Some(format!(
        "tok {}\n  python: code {} stderr {:?}\n  rust:   code {code} stderr {stderr:?}{}{}",
        case.args.join(" "),
        case.code,
        case.stderr,
        first_diff(&case.stdout, &stdout),
        if files == case.files {
            String::new()
        } else {
            format!(
                "\n  CSV files differ: {:?}",
                files.iter().map(|f| &f.0).collect::<Vec<_>>()
            )
        }
    ))
}

#[test]
fn sim_matches_python() {
    let cases = load_cases();
    assert!(cases.len() > 300, "fixtures missing; run gen_python_sim.py");
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
        "{} of {} sim runs differ from Python ({PLATFORM}):\n{}",
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
