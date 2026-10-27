//! Full-output parity with the Python reference implementation.
//!
//! The fixtures in `tests/fixtures` are produced by `gen_python_expected.py` from
//! the language examples, the course examples (including the ones with deliberate
//! errors) and the BPMN import corpus. Every output is compared byte for byte:
//! `check --json` diagnostics in English and Polish, BPMN XML (without the diagram, since the
//! layout engine of tok 0.4 differs from Python; drawing succeeds exactly when Python draws),
//! DMN, forms, and `import` with and without flow identifiers.

use std::fs;
use std::path::{Path, PathBuf};

use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::json::Json;
use tok_core::{dmn, export, forms, import, printer};

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

fn fixtures(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(sub)
}

fn read(p: &Path) -> String {
    let text = fs::read_to_string(p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
    text.replace("\r\n", "\n")
}

fn inputs(dir: &Path, ext: &str, skip: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .expect("fixtures directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .filter(|p| skip.is_empty() || !p.to_string_lossy().ends_with(skip))
        .collect();
    v.sort();
    v
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .expect("file name")
        .to_string_lossy()
        .to_string()
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

fn compare(failures: &mut Vec<String>, what: String, expected: &str, actual: &str) {
    if expected != actual {
        let line = expected
            .lines()
            .zip(actual.lines())
            .position(|(a, b)| a != b)
            .map(|k| k + 1)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()) + 1);
        failures.push(format!("{what}: first difference at line {line}"));
    }
}

#[test]
fn check_and_exports_match_python() {
    let dir = fixtures("tok");
    let files: Vec<PathBuf> = inputs(&dir, "tok", "")
        .into_iter()
        .filter(|p| !p.to_string_lossy().contains(".rt."))
        .collect();
    assert!(
        files.len() >= 18,
        "fixtures missing; run gen_python_expected.py"
    );
    let mut failures = Vec::new();
    let mut compared = 0;
    for path in &files {
        let name = stem(path);
        let (doc, diags) = tok_core::load(&read(path), "full", Some(&Rules));
        for lang in ["en", "pl"] {
            let actual = Json::Arr(diags.iter().map(|d| d.to_json(lang)).collect()).pretty() + "\n";
            let expected = read(&dir.join(format!("{name}.{lang}.json")));
            compare(
                &mut failures,
                format!("{name} check {lang}"),
                &expected,
                &actual,
            );
            compared += 1;
        }
        if diags.iter().any(|d| d.is_error()) {
            continue;
        }
        let xml = export::export_xml(&doc, None);
        let bpmn = tok_bpmn::layout(&xml).expect("layout");
        // the layout engine of tok 0.4 differs from Python: the diagram is not compared
        compare(
            &mut failures,
            format!("{name}.bpmn (without the diagram)"),
            &strip_di(&read(&dir.join(format!("{name}.bpmn")))),
            &strip_di(&bpmn),
        );
        match tok_bpmn::render_svg(&xml, Some(&format!("{name}.svg"))) {
            Ok(svg) => {
                if !dir.join(format!("{name}.svg")).exists() || !svg.ends_with("</svg>") {
                    failures.push(format!("{name}.svg: drawn, Python reports an error"));
                }
            }
            Err(e) => {
                let expected = read(&dir.join(format!("{name}.svg.err")));
                compare(
                    &mut failures,
                    format!("{name}.svg.err"),
                    &expected,
                    &format!("{e}\n"),
                );
            }
        }
        compared += 2;
        let (rt, rt_warnings) = import::import_bpmn(&bpmn, true).expect("round-trip import");
        compare(
            &mut failures,
            format!("{name}.rt.tok"),
            &read(&dir.join(format!("{name}.rt.tok"))),
            &printer::fmt(&rt),
        );
        let warn: String = rt_warnings.iter().map(|w| format!("{w}\n")).collect();
        compare(
            &mut failures,
            format!("{name}.rt.warn"),
            &read(&dir.join(format!("{name}.rt.warn"))),
            &warn,
        );
        let (rt, _) = import::import_bpmn(&bpmn, false).expect("round-trip import");
        compare(
            &mut failures,
            format!("{name}.rt.noids.tok"),
            &read(&dir.join(format!("{name}.rt.noids.tok"))),
            &printer::fmt(&rt),
        );
        compared += 3;
        for dec in doc.decisions.values() {
            let f = format!("{name}.{}.dmn", dec.base.id());
            compare(
                &mut failures,
                f.clone(),
                &read(&dir.join(&f)),
                &dmn::export_dmn(dec, Some(&doc)),
            );
            compared += 1;
        }
        for form in doc.forms.values() {
            let f = format!("{name}.{}.form", form.base.id());
            compare(
                &mut failures,
                f.clone(),
                &read(&dir.join(&f)),
                &forms::export_form(form),
            );
            compared += 1;
        }
    }
    println!("compared {compared} outputs of {} documents", files.len());
    assert!(
        failures.is_empty(),
        "{} difference(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn import_matches_python() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/import");
    let dir = fixtures("import");
    let files = inputs(&src, "bpmn", "");
    assert!(!files.is_empty(), "import corpus missing");
    let mut failures = Vec::new();
    for path in &files {
        let name = stem(path);
        let xml = String::from_utf8_lossy(&fs::read(path).expect("read bpmn")).to_string();
        let (doc, warnings) = import::import_bpmn(&xml, true).expect("import");
        compare(
            &mut failures,
            format!("{name}.tok"),
            &read(&dir.join(format!("{name}.tok"))),
            &printer::fmt(&doc),
        );
        let warn: String = warnings.iter().map(|w| format!("{w}\n")).collect();
        compare(
            &mut failures,
            format!("{name}.warn"),
            &read(&dir.join(format!("{name}.warn"))),
            &warn,
        );
        let (doc, _) = import::import_bpmn(&xml, false).expect("import");
        let expected = read(&dir.join(format!("{name}.noids.tok")));
        compare(
            &mut failures,
            format!("{name}.noids.tok"),
            &expected,
            &printer::fmt(&doc),
        );
    }
    println!("compared {} import outputs", files.len() * 3);
    assert!(
        failures.is_empty(),
        "{} difference(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
