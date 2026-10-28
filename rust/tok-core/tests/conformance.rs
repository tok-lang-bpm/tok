//! Conformance corpus: canonical text, core diagnostics, semantic BPMN XML, the codes of
//! `tok check` at every level and, for documents of language 0.7, the DMN files must equal
//! the expected files byte for byte.
//!
//! The expected files of cases written for language 0.1 and 0.3 (01–32) are produced by
//! the Python reference implementation (`conformance/generate.py`). Those of every other
//! case (33 and following: a newer language, or a header the Python generator skips) are
//! produced by this engine:
//!
//! ```text
//! TOK_BLESS=1 cargo test -p tok-core --test conformance
//! ```
//!
//! writes them (and never touches the files of 01–32); review the differences before
//! committing.

use std::fs;
use std::path::{Path, PathBuf};

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
    "P010", "R001", "R004", "R020", "D002", "D004", "D008", "D009",
];

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// Whether `conformance/generate.py` (the pure-Python implementation) writes the expected
/// files of a case: the second word of its first statement line is `0.1` or `0.3`, or it
/// has no header. Every other case, also one whose header is malformed (`tok v1.0`),
/// belongs to this engine.
fn python_generates(text: &str) -> bool {
    let first = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let words: Vec<&str> = first.split_whitespace().collect();
    match words.as_slice() {
        ["tok", version, ..] => matches!(*version, "0.1" | "0.3"),
        _ => true,
    }
}

/// Expected outputs of a case of language 0.5 or newer, as `generate.py` writes them for
/// the older cases.
fn bless(expected: &Path, stem: &str, text: &str, codes: &str) {
    let (doc, diags) = tok_core::parse(text);
    write(&expected.join(format!("{stem}.tok")), &tok_core::fmt(&doc));
    write(&expected.join(format!("{stem}.diag")), codes);
    let bpmn_path = expected.join(format!("{stem}.bpmn"));
    let levels_path = expected.join(format!("{stem}.levels"));
    if diags.iter().any(tok_core::Diag::is_error) {
        for p in [&bpmn_path, &levels_path] {
            if p.exists() {
                fs::remove_file(p).expect("remove a stale expected file");
            }
        }
        return;
    }
    write(&bpmn_path, &tok_core::export::semantic_xml(&doc, None));
    write(&levels_path, &levels(text));
    for (file, xml) in dmn_files(&doc) {
        write(&expected.join(format!("{stem}.{file}")), &xml);
    }
}

/// The DMN files of a document of language 0.7 (one per decision requirements graph).
fn dmn_files(doc: &tok_core::Document) -> Vec<(String, String)> {
    if !doc.lang_07() {
        return Vec::new();
    }
    tok_core::drg::export::export_dmn15(doc)
        .into_iter()
        .map(|(stem, xml)| (format!("{stem}.dmn"), xml))
        .collect()
}

/// `basic: …`, `analytic: …` and `full: …` lines with the sorted codes of `tok check`.
fn levels(text: &str) -> String {
    let mut got = String::new();
    for level in ["basic", "analytic", "full"] {
        let (mut doc_l, _) = tok_core::parse(text);
        let mut codes: Vec<String> = tok_core::check(&mut doc_l, level, Some(&Rules))
            .into_iter()
            .map(|d| d.code)
            .collect();
        codes.sort();
        got.push_str(&format!("{level}: {}\n", codes.join(" ")));
    }
    got
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
        if std::env::var_os("TOK_BLESS").is_some() && !python_generates(&text) {
            bless(&expected, stem, &text, &diag_text);
        }
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
            let got = levels(&text);
            if got != exp {
                failures.push(format!(
                    "{stem}.levels differs:
--- got
{got}--- expected
{exp}"
                ));
            }
        }
        if !diags.iter().any(tok_core::Diag::is_error) {
            for (file, got) in dmn_files(&doc) {
                let path = expected.join(format!("{stem}.{file}"));
                match fs::read_to_string(&path) {
                    Ok(exp) if exp == got => {}
                    Ok(_) => failures.push(format!("{stem}.{file} differs")),
                    Err(_) => failures.push(format!("{stem}.{file} is missing")),
                }
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
