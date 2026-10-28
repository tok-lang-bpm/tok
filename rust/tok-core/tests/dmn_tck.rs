//! Conformance gate of decision evaluation and FEEL: the selected test cases of the DMN
//! Technology Compatibility Kit in `conformance/dmn-tck` (compliance levels 2 and 3,
//! design 0.7 §9). Every model is imported (`tok import`), printed as tok, parsed again
//! and evaluated for every test case; each result is compared with the expected value.
//!
//! The test fails when
//! - a model does not import or its tok text does not parse or is not in canonical form;
//! - a result differs from the expected value and `tests/fixtures/dmn-tck/baseline.tsv`
//!   does not list it (every listed case has a reason);
//! - a listed case passes (the baseline must be kept exact).
//!
//! After an improvement, store the new baseline and update `docs/dmn-tck.md`:
//!
//! ```text
//! TOK_BLESS=1 cargo test -p tok-core --test dmn_tck
//! ```
//!
//! `TOK_TCK_DIR` points the test to another copy of the TCK `TestCases` folder.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use roxmltree::Node;
use tok_core::drg::eval::Model;
use tok_core::feel3::eval::temporal_literal;
use tok_core::feel3::{Map, Num, Value, equal};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tck_dir() -> PathBuf {
    match std::env::var("TOK_TCK_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => root().join("conformance/dmn-tck"),
    }
}

fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dmn-tck/baseline.tsv")
}

fn local<'a>(n: Node<'a, '_>) -> &'a str {
    n.tag_name().name()
}

fn kids<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(|c| c.is_element())
}

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// A value of a test file: `<value>`, `<component>`s or a `<list>`.
fn value(n: Node) -> Value {
    if n.attribute((XSI, "nil")) == Some("true") {
        return Value::Null;
    }
    let comps: Vec<Node> = kids(n).filter(|c| local(*c) == "component").collect();
    if !comps.is_empty() {
        let mut m = Map::new();
        for c in comps {
            let name = c.attribute("name").unwrap_or("").to_string();
            m.insert(name, holder(c));
        }
        return Value::context(m);
    }
    if let Some(l) = kids(n).find(|c| local(*c) == "list") {
        return list(l);
    }
    let text: String = n.text().unwrap_or("").to_string();
    match n
        .attribute((XSI, "type"))
        .map(|t| t.rsplit(':').next().unwrap_or(t))
    {
        Some("decimal" | "double" | "integer" | "int" | "long" | "float") => match text.trim() {
            "NaN" | "INF" | "-INF" => Value::Null,
            t => Value::num(Num::parse(t)),
        },
        Some("boolean") => Value::Bool(text.trim() == "true"),
        Some("date" | "dateTime" | "time" | "duration") => temporal_literal(text.trim()),
        _ => Value::Str(text),
    }
}

fn list(l: Node) -> Value {
    Value::list(
        kids(l)
            .filter(|c| local(*c) == "item")
            .map(holder)
            .collect(),
    )
}

/// An element holding a value: `<value>`, `<list>` or components.
fn holder(n: Node) -> Value {
    if n.attribute((XSI, "nil")) == Some("true") {
        return Value::Null;
    }
    if let Some(v) = kids(n).find(|c| local(*c) == "value") {
        return value(v);
    }
    if let Some(l) = kids(n).find(|c| local(*c) == "list") {
        return list(l);
    }
    if kids(n).any(|c| local(c) == "component") {
        return value(n);
    }
    Value::Null
}

fn scale(n: &Num) -> i64 {
    let t = n.to_text();
    t.split_once('.').map_or(0, |(_, f)| f.len() as i64)
}

/// The comparison of the TCK runners: numbers equal, or equal at the precision of the
/// expected value when it has eight or more decimals (the results of `exp`, `log`,
/// `sqrt` and of floating-point computations are given rounded).
fn same(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Num(a), Value::Num(e)) => {
            if a.eq(e) {
                return true;
            }
            let s = scale(e);
            if s >= 8 {
                // results of floating-point computations: equal within 1e-10 relative
                let tol = e
                    .abs()
                    .mul(&Num::parse("1e-10").unwrap_or_else(Num::zero))
                    .unwrap_or_else(Num::zero);
                if a.sub(e)
                    .is_some_and(|d| d.abs().cmp(&tol) != std::cmp::Ordering::Greater)
                {
                    return true;
                }
                return a
                    .round(s, tok_core::pydec::Rounding::HalfEven)
                    .is_some_and(|r| r.eq(e))
                    || a.sub(e).is_some_and(|d| {
                        d.abs()
                            .cmp(&Num::parse(&format!("1e-{s}")).unwrap_or_else(Num::zero))
                            != std::cmp::Ordering::Greater
                    });
            }
            false
        }
        (Value::List(a), Value::List(e)) => {
            a.len() == e.len() && a.iter().zip(e.iter()).all(|(x, y)| same(x, y))
        }
        (Value::Context(a), Value::Context(e)) => {
            e.len() == a.len() && e.iter().all(|(k, v)| a.get(k).is_some_and(|x| same(x, v)))
        }
        _ => equal(actual, expected) == Some(true),
    }
}

/// One evaluated result: (level, test, case, result node) → failure text or `None`.
type Results = BTreeMap<(String, String, String, String), Option<String>>;

fn run_test(level: &str, dir: &Path, results: &mut Results, errors: &mut Vec<String>) {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut models = Vec::new();
    let mut tests = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let f = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if f.ends_with(".dmn") {
            models.push(p);
        } else if f.ends_with(".xml") && f.contains("-test-") {
            tests.push(p);
        }
    }
    if models.len() != 1 {
        return;
    }
    let xml = fs::read_to_string(&models[0]).unwrap_or_default();
    let (doc, _warnings) = match tok_core::drg::dmnimport::import_dmn(&xml) {
        Ok(r) => r,
        Err(e) => {
            errors.push(format!("{level}/{name}: import failed: {e}"));
            return;
        }
    };
    let text = tok_core::fmt(&doc);
    let (doc, diags) = tok_core::parse(&text);
    if let Some(d) = diags.iter().find(|d| d.is_error()) {
        errors.push(format!(
            "{level}/{name}: the imported text does not parse: {} line {}",
            d.code, d.line
        ));
        return;
    }
    if tok_core::fmt(&doc) != text {
        errors.push(format!(
            "{level}/{name}: the imported text is not in canonical form"
        ));
    }
    let model = Model::new(&doc);
    for t in tests {
        let txt = fs::read_to_string(&t).unwrap_or_default();
        let Ok(tree) = roxmltree::Document::parse(&txt) else {
            errors.push(format!("{level}/{name}: test file does not parse"));
            continue;
        };
        for tc in kids(tree.root_element()).filter(|c| local(*c) == "testCase") {
            if tc.attribute("type").is_some_and(|t| t != "decision") {
                continue;
            }
            let case = tc.attribute("id").unwrap_or("?").to_string();
            let mut inputs = Map::new();
            for i in kids(tc).filter(|c| local(*c) == "inputNode") {
                inputs.insert(i.attribute("name").unwrap_or("").to_string(), holder(i));
            }
            for r in kids(tc).filter(|c| local(*c) == "resultNode") {
                let rname = r.attribute("name").unwrap_or("").to_string();
                let expected = kids(r)
                    .find(|c| local(*c) == "expected")
                    .map(holder)
                    .unwrap_or(Value::Null);
                let error_result = r.attribute("errorResult") == Some("true");
                let id = doc
                    .decisions
                    .values()
                    .find(|d| tok_core::drg::var_name(&d.base) == rname)
                    .map(|d| d.base.id().to_string());
                let outcome = match id {
                    None => Some(format!("no decision named '{rname}'")),
                    Some(id) => {
                        let (v, _) = model.decision(&id, &inputs);
                        let ok = if error_result {
                            v.is_null()
                        } else {
                            same(&v, &expected)
                        };
                        (!ok)
                            .then(|| format!("got {} expected {}", v.literal(), expected.literal()))
                    }
                };
                results.insert(
                    (level.to_string(), name.clone(), case.clone(), rname),
                    outcome,
                );
            }
        }
    }
}

fn read_baseline() -> BTreeMap<(String, String, String, String), String> {
    let mut out = BTreeMap::new();
    let text = fs::read_to_string(baseline_path()).unwrap_or_default();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() >= 5 {
            out.insert(
                (p[0].into(), p[1].into(), p[2].into(), p[3].into()),
                p[4].to_string(),
            );
        }
    }
    out
}

/// The reason of a new baseline entry from `reasons.tsv` (`test/case`, `test/prefix*`
/// or `test`, a tab, the reason).
fn reason_for(test: &str, case: &str, detail: &str) -> String {
    let reasons =
        fs::read_to_string(baseline_path().with_file_name("reasons.tsv")).unwrap_or_default();
    let full = format!("{test}/{case}");
    let mut fallback = None;
    for line in reasons.lines() {
        if let Some((k, v)) = line.split_once('\t') {
            if k == full || (k.ends_with('*') && full.starts_with(k.trim_end_matches('*'))) {
                return v.to_string();
            }
            if k == test {
                fallback = Some(v.to_string());
            }
        }
    }
    fallback.unwrap_or_else(|| format!("unexplained ({detail})"))
}

#[test]
fn dmn_tck() {
    let dir = tck_dir();
    let mut results = Results::new();
    let mut errors = Vec::new();
    for level in ["compliance-level-2", "compliance-level-3"] {
        let Ok(rd) = fs::read_dir(dir.join(level)) else {
            continue;
        };
        let mut tests: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        tests.sort();
        for t in tests {
            run_test(level, &t, &mut results, &mut errors);
        }
    }
    assert!(
        !results.is_empty(),
        "no DMN TCK test cases in {}",
        dir.display()
    );
    let baseline = read_baseline();
    let bless = std::env::var("TOK_BLESS").is_ok();
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut new_failures = Vec::new();
    let mut fixed = Vec::new();
    let mut lines = vec![
        "# level\ttest\tcase\tresult\treason (cases of the DMN TCK expected to fail)".to_string(),
    ];
    for (key, outcome) in &results {
        let c = counts.entry(key.0.clone()).or_default();
        c.0 += 1;
        match outcome {
            None => {
                c.1 += 1;
                if baseline.contains_key(key) {
                    fixed.push(format!("{}/{} {} {}", key.0, key.1, key.2, key.3));
                }
            }
            Some(detail) => {
                let reason = baseline
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| reason_for(&key.1, &key.2, detail));
                if !baseline.contains_key(key) {
                    new_failures.push(format!("{}/{} {} {}: {detail}", key.0, key.1, key.2, key.3));
                }
                lines.push(format!(
                    "{}\t{}\t{}\t{}\t{reason}",
                    key.0, key.1, key.2, key.3
                ));
            }
        }
    }
    for (level, (total, passed)) in &counts {
        println!("{level}: {passed} of {total} results pass");
    }
    if std::env::var("TOK_TCK_REPORT").is_ok() {
        let mut per_test: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
        for (key, outcome) in &results {
            let c = per_test.entry((key.0.clone(), key.1.clone())).or_default();
            c.0 += 1;
            if outcome.is_none() {
                c.1 += 1;
            }
        }
        for ((l, t), (total, passed)) in per_test {
            println!("| {l} | {t} | {passed} | {total} |");
        }
        for f in &new_failures {
            println!("FAIL {f}");
        }
    }
    if bless {
        if let Some(parent) = baseline_path().parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(baseline_path(), lines.join("\n") + "\n");
        return;
    }
    assert!(errors.is_empty(), "DMN TCK models:\n{}", errors.join("\n"));
    assert!(
        new_failures.is_empty(),
        "{} DMN TCK results fail and are not in the baseline:\n{}",
        new_failures.len(),
        new_failures
            .iter()
            .take(200)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        fixed.is_empty(),
        "{} DMN TCK results of the baseline pass now; store the baseline with TOK_BLESS=1:\n{}",
        fixed.len(),
        fixed.join("\n")
    );
}
