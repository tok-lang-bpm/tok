//! Round trip of language 0.5 through BPMN: every conformance case of language 0.5 without
//! errors is exported, imported again and exported once more; the semantic BPMN must not
//! change, the import must not warn, and the imported document declares `tok 0.5`.

use std::fs;
use std::path::PathBuf;

fn cases() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .expect("cases folder")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tok"))
        .map(|p| {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            (name, fs::read_to_string(&p).expect("case text"))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn language_05_survives_import() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for (name, text) in cases() {
        let (doc, diags) = tok_core::parse(&text);
        if !doc.lang_05() || diags.iter().any(tok_core::Diag::is_error) {
            continue;
        }
        let mut checked_doc = doc.clone();
        if tok_core::check(&mut checked_doc, "full", None)
            .iter()
            .any(tok_core::Diag::is_error)
        {
            continue;
        }
        checked += 1;
        let x1 = tok_core::export::semantic_xml(&doc, None);
        for keep_ids in [true, false] {
            let (imported, warnings) =
                tok_core::import::import_bpmn(&tok_core::export::export_xml(&doc, None), keep_ids)
                    .expect("import");
            if !warnings.is_empty() {
                failures.push(format!(
                    "{name} (keep ids {keep_ids}): warnings {warnings:?}"
                ));
            }
            if imported.version.as_deref() != Some("0.5") {
                failures.push(format!(
                    "{name} (keep ids {keep_ids}): imported as tok {:?}",
                    imported.version
                ));
            }
            let text2 = tok_core::fmt(&imported);
            let (doc2, diags2) = tok_core::parse(&text2);
            if diags2.iter().any(tok_core::Diag::is_error) {
                failures.push(format!(
                    "{name}: the imported text does not parse:\n{text2}"
                ));
                continue;
            }
            let x2 = tok_core::export::semantic_xml(&doc2, None);
            if x2 != x1 {
                let line = x1
                    .lines()
                    .zip(x2.lines())
                    .position(|(a, b)| a != b)
                    .map_or(0, |k| k + 1);
                failures.push(format!(
                    "{name} (keep ids {keep_ids}): BPMN differs from line {line}:\n{text2}\n--- first\n{x1}\n--- again\n{x2}"
                ));
            }
        }
    }
    assert!(checked >= 9, "only {checked} cases of language 0.5 checked");
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
