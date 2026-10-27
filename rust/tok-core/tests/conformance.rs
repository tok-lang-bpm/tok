//! Conformance corpus: canonical text, core diagnostics and semantic BPMN XML must equal
//! the expected files produced by the Python reference implementation byte for byte.

use std::fs;
use std::path::PathBuf;

use tok_core::check::{BpmnIssue, BpmnRules};

/// The BPMN rules of `tok-bpmn`, as `tok check` uses them.
struct Rules;

impl BpmnRules for Rules {
    fn validate(&self, xml: &str) -> Vec<BpmnIssue> {
        tok_bpmn::validate(xml)
            .expect("exported XML is well formed")
            .into_iter()
            .map(|i| BpmnIssue {
                level: match i.level {
                    tok_bpmn::Level::Error => "błąd",
                    tok_bpmn::Level::Warning => "ostrzeżenie",
                    tok_bpmn::Level::Note => "uwaga",
                }
                .to_string(),
                code: i.code,
                id: i.id,
                message: i.message,
            })
            .collect()
    }
}

const CORE_CODES: &[&str] = &[
    "T001", "T002", "T003", "P001", "P002", "P003", "P004", "P005", "P006", "P007", "P008", "P009",
    "P010", "R001", "R004", "R020", "D002", "D004",
];

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

#[test]
fn conformance_corpus() {
    let cases = corpus().join("cases");
    let expected = corpus().join("expected");
    let mut names: Vec<_> = fs::read_dir(&cases)
        .expect("cases folder")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".tok"))
        .collect();
    names.sort();
    assert!(!names.is_empty());
    let mut failures = Vec::new();
    for name in &names {
        let stem = &name[..name.len() - 4];
        let text = fs::read_to_string(cases.join(name)).expect("case text");
        let (doc, diags) = tok_core::parse(&text);
        let mut codes: Vec<&str> = diags
            .iter()
            .map(|d| d.code.as_str())
            .filter(|c| CORE_CODES.contains(c))
            .collect();
        codes.sort();
        let diag_text: String = codes.iter().map(|c| format!("{c}\n")).collect();
        let exp_diag = fs::read_to_string(expected.join(format!("{stem}.diag"))).expect("diag");
        if diag_text != exp_diag {
            failures.push(format!(
                "{stem}.diag: got {diag_text:?}, expected {exp_diag:?}"
            ));
        }
        let exp_tok = fs::read_to_string(expected.join(format!("{stem}.tok"))).expect("tok");
        let got_tok = tok_core::fmt(&doc);
        if got_tok != exp_tok {
            failures.push(format!(
                "{stem}.tok differs:\n--- got\n{got_tok}\n--- expected\n{exp_tok}"
            ));
        }
        let levels_path = expected.join(format!("{stem}.levels"));
        if levels_path.exists() {
            let exp = fs::read_to_string(&levels_path).expect("levels");
            let mut got = String::new();
            for level in ["basic", "analytic", "full"] {
                let (mut doc_l, _) = tok_core::parse(&text);
                let mut codes: Vec<String> = tok_core::check(&mut doc_l, level, Some(&Rules))
                    .into_iter()
                    .map(|d| d.code)
                    .collect();
                codes.sort();
                got.push_str(&format!(
                    "{level}: {}
",
                    codes.join(" ")
                ));
            }
            if got != exp {
                failures.push(format!(
                    "{stem}.levels differs:
--- got
{got}--- expected
{exp}"
                ));
            }
        }
        let bpmn_path = expected.join(format!("{stem}.bpmn"));
        if bpmn_path.exists() {
            let exp = fs::read_to_string(&bpmn_path).expect("bpmn");
            let got = tok_core::export::semantic_xml(&doc, None);
            if got != exp {
                failures.push(format!(
                    "{stem}.bpmn differs:\n--- got\n{got}\n--- expected\n{exp}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
