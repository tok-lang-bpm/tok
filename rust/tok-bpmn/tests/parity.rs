//! Parity with the Python reference `tok.bpmnlib` (= bpmn.py 1.2).
//!
//! Expected outputs are produced by `tests/gen_expected.py`. Identical to them must be:
//! the diagnostics (codes, levels, element ids and messages of `validate`, `validate_with`
//! and `validate_camunda`), the summary, the semantic part of the laid-out XML (everything
//! except the `BPMNDiagram`) and the drawing of a model that brings its own diagram.
//! The automatic layout itself and the drawing of a model laid out by it are no longer
//! compared with Python (the layout engine of tok 0.4 differs from `bpmn.py`); they are
//! covered by `tests/golden.rs` and `tests/layout_quality.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tok_bpmn::{Issue, ValidateOptions};

fn issues_json(v: &[Issue]) -> Value {
    Value::Array(
        v.iter()
            .map(|i| {
                json!({
                    "poziom": i.level.as_pl(),
                    "kod": i.code,
                    "id": i.id,
                    "komunikat": i.message,
                })
            })
            .collect(),
    )
}

fn first_diff(a: &str, b: &str) -> String {
    for (n, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  rust:   {}\n  python: {}", n + 1, x, y);
        }
    }
    format!(
        "lengths differ: rust {} lines, python {} lines",
        a.lines().count(),
        b.lines().count()
    )
}

/// The document without its `BPMNDiagram` element (and the whitespace before it).
fn strip_di(xml: &str) -> String {
    let Some(start) = xml.find(":BPMNDiagram ").and_then(|k| xml[..k].rfind('<')) else {
        return xml.to_string();
    };
    let prefix = &xml[start + 1..xml[start..].find(':').map_or(start, |k| start + k)];
    let close = format!("</{prefix}:BPMNDiagram>");
    let Some(end) = xml[start..].find(&close).map(|k| start + k + close.len()) else {
        return xml.to_string();
    };
    let head = xml[..start].trim_end_matches([' ', '\t', '\n', '\r']);
    format!("{head}{}", &xml[end..])
}

fn fixtures() -> Vec<(String, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut out = Vec::new();
    let mut groups: Vec<PathBuf> = fs::read_dir(&root)
        .expect("fixtures")
        .flatten()
        .map(|e| e.path())
        .collect();
    groups.sort();
    for g in groups {
        let mut files: Vec<PathBuf> = fs::read_dir(&g)
            .expect("group")
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "bpmn"))
            .collect();
        files.sort();
        let group = g
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        for f in files {
            out.push((group.clone(), f));
        }
    }
    out
}

fn expected(group: &str, stem: &str, ext: &str) -> Option<String> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("expected")
        .join(group)
        .join(format!("{stem}{ext}"));
    fs::read_to_string(p).ok()
}

#[test]
fn parity_with_python_reference() {
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0;
    for (group, path) in fixtures() {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let xml = fs::read_to_string(&path).expect("read fixture");
        let tag = format!("{group}/{stem}");
        let folder = path.parent().expect("parent");

        let cmp_json = |name: &str, got: Value, failures: &mut Vec<String>| {
            let want: Value =
                serde_json::from_str(&expected(&group, &stem, name).expect("expected file"))
                    .expect("json");
            if got != want {
                failures.push(format!(
                    "{tag} {name}\n  rust:   {}\n  python: {}",
                    serde_json::to_string(&got).unwrap_or_default(),
                    serde_json::to_string(&want).unwrap_or_default()
                ));
            }
        };
        cmp_json(
            ".validate.json",
            issues_json(&tok_bpmn::validate(&xml).expect("validate")),
            &mut failures,
        );
        let kroki = ValidateOptions {
            kroki: true,
            opis_kroki: None,
        };
        cmp_json(
            ".kroki.json",
            issues_json(&tok_bpmn::validate_with(&xml, &kroki).expect("kroki")),
            &mut failures,
        );
        cmp_json(
            ".camunda.json",
            issues_json(&tok_bpmn::validate_camunda(&xml, folder).expect("camunda")),
            &mut failures,
        );

        let opis = path.with_extension("opis.md");
        if opis.exists() {
            let steps = tok_bpmn::read_opis(&fs::read_to_string(&opis).expect("opis"));
            let opts = ValidateOptions {
                kroki: true,
                opis_kroki: Some(steps),
            };
            cmp_json(
                ".opis.json",
                issues_json(&tok_bpmn::validate_with(&xml, &opts).expect("opis")),
                &mut failures,
            );
        }
        let summary = tok_bpmn::summarize(&xml).expect("summary");
        if Some(summary.clone()) != expected(&group, &stem, ".summary.txt") {
            failures.push(format!("{tag} summary: {summary}"));
        }

        let laid = strip_di(&tok_bpmn::layout(&xml).expect("layout"));
        let want = strip_di(&expected(&group, &stem, ".layout.bpmn").expect("expected layout"));
        if laid != want {
            failures.push(format!(
                "{tag} layout (semantic part) {}",
                first_diff(&laid, &want)
            ));
        }

        // a model with its own diagram is drawn as it is, exactly as by Python
        let own_diagram = xml.contains(":BPMNShape") || xml.contains("<BPMNShape");
        match tok_bpmn::render_svg(&xml, Some("model.bpmn")) {
            Ok(svg) => {
                let want = expected(&group, &stem, ".svg");
                if want.is_none() {
                    failures.push(format!("{tag} svg drawn, Python reports an error"));
                } else if own_diagram && Some(&svg) != want.as_ref() {
                    failures.push(format!(
                        "{tag} svg {}",
                        first_diff(&svg, &want.unwrap_or_default())
                    ));
                }
            }
            Err(e) => {
                if Some(e.to_string()) != expected(&group, &stem, ".svg.err") {
                    failures.push(format!("{tag} svg error: {e}"));
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 30, "only {checked} fixtures found");
    assert_eq!(
        strip_di("<a>\n  <b/>\n  <x:BPMNDiagram id=\"d\">\n    <x:c/>\n  </x:BPMNDiagram>\n</a>"),
        "<a>\n  <b/>\n</a>"
    );
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
