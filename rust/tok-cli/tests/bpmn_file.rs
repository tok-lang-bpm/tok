//! `tok check`, `tok bpmn` and `tok svg` on BPMN 2.0 XML files.
//!
//! The diagnostics of `tok check FILE.bpmn` must be those of the BPMN validator for the
//! fixtures of `tok-bpmn` (expected outputs of the Python reference, `tok-bpmn/tests`): the
//! rules, with `--steps` the step numbers, with `--camunda` the rules followed by the Camunda
//! 8 profile. Every finding has an English text; layout and picture are written.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tok-bpmn/tests")
}

fn fixtures() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for group in ["semantic", "extra", "camunda"] {
        let dir = tests_dir().join("fixtures").join(group);
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("fixtures")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "bpmn"))
            .collect();
        files.sort();
        out.extend(files.into_iter().map(|p| (group.to_string(), p)));
    }
    out
}

fn expected(group: &str, path: &Path, ext: &str) -> Option<Vec<Value>> {
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let p = tests_dir()
        .join("expected")
        .join(group)
        .join(format!("{stem}{ext}"));
    let text = fs::read_to_string(p).ok()?;
    let v: Value = serde_json::from_str(&text).expect("expected json");
    Some(
        v.as_array()
            .expect("array")
            .iter()
            .map(|i| {
                let sev = match i["poziom"].as_str() {
                    Some("błąd") => "error",
                    Some("ostrzeżenie") => "warning",
                    _ => "note",
                };
                serde_json::json!({"code": i["kod"], "id": i["id"], "severity": sev, "message": i["komunikat"]})
            })
            .collect(),
    )
}

fn tok(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_tok"))
        .args(args)
        .output()
        .expect("run tok");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

fn diagnostics(args: &[&str]) -> Vec<Value> {
    let (_, out) = tok(args);
    let v: Value = serde_json::from_str(&out).unwrap_or_else(|e| panic!("{args:?}: {e}: {out}"));
    v["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|d| serde_json::json!({"code": d["code"], "id": d["id"], "severity": d["severity"], "message": d["message"]}))
        .collect()
}

#[test]
fn check_matches_the_validator() {
    let mut failures = Vec::new();
    let mut compared = 0;
    for (group, path) in fixtures() {
        let file = path.to_string_lossy().to_string();
        let mut cases: Vec<(&str, Vec<&str>, Vec<Value>)> = Vec::new();
        if let Some(v) = expected(&group, &path, ".validate.json") {
            cases.push(("rules", vec![], v.clone()));
            if let Some(c) = expected(&group, &path, ".camunda.json") {
                cases.push(("--camunda", vec!["--camunda"], [v, c].concat()));
            }
        }
        if let Some(k) = expected(&group, &path, ".kroki.json") {
            cases.push(("--steps", vec!["--steps"], k));
        }
        for (name, opts, want) in cases {
            let mut args = vec!["check", file.as_str(), "--json", "--lang", "pl"];
            args.extend(opts);
            let got = diagnostics(&args);
            compared += 1;
            if got != want {
                failures.push(format!(
                    "{group}/{} {name}\n  tok:       {}\n  validator: {}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    serde_json::to_string(&got).unwrap_or_default(),
                    serde_json::to_string(&want).unwrap_or_default()
                ));
            }
        }
    }
    assert!(compared > 30, "only {compared} comparisons");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_finding_has_an_english_text() {
    for (_, path) in fixtures() {
        let file = path.to_string_lossy().to_string();
        let en = diagnostics(&["check", &file, "--json", "--camunda", "--steps"]);
        let pl = diagnostics(&[
            "check",
            &file,
            "--json",
            "--camunda",
            "--steps",
            "--lang",
            "pl",
        ]);
        for (e, p) in en.iter().zip(&pl) {
            assert_ne!(
                e["message"], p["message"],
                "{file}: {} has no English text",
                e["code"]
            );
        }
    }
}

#[test]
fn lines_point_at_the_element() {
    let file = tests_dir().join("fixtures/extra/rich.bpmn");
    let xml = fs::read_to_string(&file).expect("fixture");
    let (_, out) = tok(&["check", &file.to_string_lossy(), "--json"]);
    let v: Value = serde_json::from_str(&out).expect("json");
    for d in v["diagnostics"].as_array().expect("diagnostics") {
        let (Some(id), Some(line)) = (d["id"].as_str(), d["line"].as_u64()) else {
            continue;
        };
        if line == 0 {
            continue;
        }
        let text = xml.lines().nth(line as usize - 1).expect("line");
        assert!(
            text.contains(&format!("id=\"{id}\"")),
            "line {line} does not hold {id}"
        );
    }
}

#[test]
fn exit_status_and_malformed_xml() {
    let dir = std::env::temp_dir().join(format!("tok-bpmn-file-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp");
    let bad = dir.join("bad.bpmn");
    fs::write(&bad, "<definitions").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_tok"))
        .args(["check", &bad.to_string_lossy()])
        .output()
        .expect("run");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not well-formed XML"));
    let rich = tests_dir().join("fixtures/extra/rich.bpmn");
    assert_eq!(
        tok(&["check", &rich.to_string_lossy()]).0,
        1,
        "errors give status 1"
    );
    let clean = tests_dir().join("fixtures/semantic/02-linear.bpmn");
    assert_eq!(tok(&["check", &clean.to_string_lossy()]).0, 0);
}

#[test]
fn layout_and_picture() {
    let dir = std::env::temp_dir().join(format!("tok-bpmn-draw-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp");
    let src = tests_dir().join("fixtures/semantic/03-xor.bpmn");
    let file = dir.join("03-xor.bpmn");
    fs::copy(&src, &file).expect("copy");
    let f = file.to_string_lossy().to_string();
    let (code, out) = tok(&["bpmn", &f]);
    assert_eq!(code, 0, "{out}");
    let laid = fs::read_to_string(&file).expect("laid out");
    assert!(laid.contains("BPMNShape"), "no diagram written");
    let before = diagnostics(&["check", &src.to_string_lossy(), "--json"]);
    let after = diagnostics(&["check", &f, "--json"]);
    assert_eq!(before, after, "the layout changed the findings");
    let (code, out) = tok(&["svg", &f]);
    assert_eq!(code, 0, "{out}");
    let svg = fs::read_to_string(dir.join("03-xor.svg")).expect("svg");
    assert!(svg.starts_with("<svg"));
}
