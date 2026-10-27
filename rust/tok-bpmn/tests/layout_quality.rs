//! Quality of the automatic layout over the corpus (see `quality/mod.rs` for the metrics).
//!
//! For every model the metrics must be no worse than those stored in
//! `tests/layout/metrics.tsv` and than those of the layout of tok 0.3 (the port of `bpmn.py`,
//! `tests/layout/baseline-0.3.tsv`; documents of language 0.5, which tok 0.3 could not
//! draw, have only stored metrics). The laid-out XML must report the same BPMN errors and
//! warnings as its input and have diagram interchange for every element (no `D01`, `D02`).
//!
//! The table is printed with `cargo test -p tok-bpmn --test layout_quality -- --nocapture`.
//! After an improvement, store the new metrics:
//!
//! ```text
//! TOK_BLESS=1 cargo test -p tok-bpmn --test layout_quality
//! ```
//!
//! Models of the course repository (`../tok-course/przyklady`) are measured when it is checked
//! out next to this one; their stored lines are kept when it is not.

mod quality;

use std::fs;
use std::path::PathBuf;

use quality::{Metrics, NAMES};

fn table_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("layout")
        .join(name)
}

/// Errors and warnings (level and code) without the diagram codes `D…`, sorted.
fn codes(xml: &str) -> (Vec<String>, Vec<String>) {
    let issues = tok_bpmn::validate(xml).expect("validate");
    let mut other: Vec<String> = Vec::new();
    let mut di: Vec<String> = Vec::new();
    for i in issues {
        if i.level == tok_bpmn::Level::Note {
            continue;
        }
        if i.code.starts_with('D') {
            di.push(i.code);
        } else {
            other.push(format!("{} {}", i.level.as_pl(), i.code));
        }
    }
    other.sort();
    (other, di)
}

#[test]
fn layout_quality_is_not_worse() {
    let stored_text = fs::read_to_string(table_path("metrics.tsv")).unwrap_or_default();
    let stored = quality::read_table(&stored_text);
    let baseline = quality::read_table(
        &fs::read_to_string(table_path("baseline-0.3.tsv")).expect("baseline table"),
    );
    let corpus = quality::corpus();
    assert!(
        corpus.iter().filter(|c| c.in_repo).count() >= 50,
        "corpus incomplete"
    );
    let mut rows: Vec<(String, Metrics)> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for c in &corpus {
        let (laid, svg) = quality::run(&c.xml);
        let m = quality::measure(&laid, &svg);
        let (want, _) = codes(&c.xml);
        let (got, di) = codes(&laid);
        if got != want {
            failures.push(format!(
                "{}: errors and warnings changed by the layout: {want:?} -> {got:?}",
                c.name
            ));
        }
        // flows with broken references cannot be drawn (as in tok 0.3)
        if !di.is_empty() && !(c.name == "extra/rich" && di == ["D02"]) {
            failures.push(format!("{}: diagram codes after layout: {di:?}", c.name));
        }
        let bless = std::env::var_os("TOK_BLESS").is_some();
        for (what, table) in [("stored", &stored), ("0.3", &baseline)] {
            if bless && what == "stored" {
                continue;
            }
            if what == "0.3" && c.lang_05 {
                continue;
            }
            let Some(b) = table.get(&c.name) else {
                if what == "0.3" || !stored_text.is_empty() {
                    failures.push(format!("{}: no {what} metrics", c.name));
                }
                continue;
            };
            for k in 0..NAMES.len() {
                if m[k] > b[k] {
                    failures.push(format!(
                        "{}: {} is {} (the {what} layout: {})",
                        c.name, NAMES[k], m[k], b[k]
                    ));
                }
            }
        }
        rows.push((c.name.clone(), m));
    }
    let mut total: Metrics = [0; 10];
    let mut total0: Metrics = [0; 10];
    for (name, m) in &rows {
        for k in 0..m.len() {
            total[k] += m[k];
            total0[k] += baseline.get(name).map_or(m[k], |b| b[k]);
        }
    }
    println!("{}", quality::table(&rows));
    println!("total now\t{total:?}\ntotal 0.3\t{total0:?}");
    if std::env::var_os("TOK_BLESS").is_some() {
        // keep the lines of models that are not available here
        let measured: Vec<&String> = rows.iter().map(|(n, _)| n).collect();
        let mut all = rows.clone();
        let mut kept: Vec<(String, Metrics)> = stored
            .iter()
            .filter(|(n, _)| !measured.contains(n))
            .map(|(n, m)| (n.clone(), *m))
            .collect();
        kept.sort();
        all.extend(kept);
        fs::write(table_path("metrics.tsv"), quality::table(&all)).expect("write metrics");
    }
    assert!(
        failures.is_empty(),
        "{} problem(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
