//! Conformance gate of the BPMN MIWG reference models (`conformance/miwg`, design 0.5
//! §12): every model is imported with its identifiers, printed as tok, parsed again and
//! exported as semantic BPMN; the semantic facts of the export are compared with those of
//! the reference model (`tests/semantic/mod.rs`).
//!
//! The test fails when
//! - a model does not import, its tok text does not parse or is not in canonical form;
//! - a fact is lost or added that `tests/fixtures/miwg/baseline.tsv` does not list (a fact
//!   that survived before must survive now);
//! - a lost or added fact is not accounted for by an import warning;
//! - the export of the imported text does not import again to the same text.
//!
//! The models of the import corpus (`conformance/import`, drawn in Camunda Modeler) must
//! survive the same round trip without losing anything.
//!
//! After an improvement, store the new baseline (and update `docs/miwg.md`):
//!
//! ```text
//! TOK_BLESS=1 cargo test -p tok-core --test miwg
//! cargo run -p tok-core --example miwg -- --table
//! ```

mod semantic;

use std::fs;
use std::path::PathBuf;

use semantic::RoundTrip;

/// The imported model survives a second round trip: its export, imported and exported
/// again, gives the same semantic BPMN.
fn stable(r: &RoundTrip) -> Option<String> {
    let (doc, _) = tok_core::parse(&r.tok);
    let x1 = tok_core::export::semantic_xml(&doc, None);
    let xml = tok_core::export::export_xml(&doc, None);
    let (again, _) = tok_core::import::import_bpmn(&xml, true).ok()?;
    let (doc2, _) = tok_core::parse(&tok_core::fmt(&again));
    let x2 = tok_core::export::semantic_xml(&doc2, None);
    if x1 == x2 {
        return None;
    }
    let line = x1
        .lines()
        .zip(x2.lines())
        .position(|(a, b)| a != b)
        .map_or(0, |k| k + 1);
    Some(format!(
        "{}: a second round trip changes the BPMN from line {line}",
        r.name
    ))
}

#[test]
fn miwg_reference_models() {
    let models = semantic::models();
    assert_eq!(
        models.len(),
        22,
        "conformance/miwg holds 22 reference models"
    );
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    let mut summary = Vec::new();
    for (name, xml) in &models {
        let r = semantic::round_trip(name, xml);
        for e in &r.errors {
            failures.push(format!("{name}: {e}"));
        }
        for f in &r.silent {
            failures.push(format!(
                "{name}: {} '{}' changed without an import warning",
                f.cat, f.text
            ));
        }
        if let Some(e) = stable(&r) {
            failures.push(e);
        }
        let (total, kept) = r
            .counts
            .iter()
            .filter(|(c, _)| !semantic::BY_DESIGN.contains(c))
            .fold((0, 0), |a, (_, (t, k))| (a.0 + t, a.1 + k));
        summary.push(format!(
            "{name}\t{kept}/{total}\t{} lost\t{} added",
            r.losses().count(),
            r.added.len()
        ));
        lines.extend(semantic::baseline_lines(&r));
    }
    println!("{}", summary.join("\n"));
    let path = semantic::baseline_path();
    if std::env::var_os("TOK_BLESS").is_some() {
        let mut text = String::from(
            "# Facts of the MIWG reference models that the round trip import → tok → BPMN does not\n\
             # keep (lost) or adds (added); written by TOK_BLESS=1 cargo test -p tok-core --test miwg\n",
        );
        for l in &lines {
            text.push_str(l);
            text.push('\n');
        }
        fs::create_dir_all(path.parent().expect("fixtures folder")).expect("fixtures folder");
        fs::write(&path, text).expect("write baseline");
    } else {
        let baseline = semantic::read_baseline(&path);
        for l in &lines {
            if !baseline.contains(l) {
                failures.push(format!("not in the baseline: {}", l.replace('\t', " ")));
            }
        }
        let now: std::collections::BTreeSet<&String> = lines.iter().collect();
        let gone = baseline.iter().filter(|l| !now.contains(l)).count();
        if gone > 0 {
            println!(
                "{gone} fact(s) of the baseline now survive; store the baseline with TOK_BLESS=1"
            );
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn import_corpus_round_trip() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/import");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("conformance/import")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "bpmn"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "import corpus missing");
    let mut failures = Vec::new();
    for p in &files {
        let name = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let xml = String::from_utf8_lossy(&fs::read(p).expect("model")).into_owned();
        let r = semantic::round_trip(&name, &xml);
        failures.extend(r.errors.iter().map(|e| format!("{name}: {e}")));
        for (f, _) in r.losses() {
            failures.push(format!("{name}: lost {} '{}'", f.cat, f.text));
        }
        for (f, _) in &r.added {
            failures.push(format!("{name}: added {} '{}'", f.cat, f.text));
        }
        if let Some(e) = stable(&r) {
            failures.push(e);
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
