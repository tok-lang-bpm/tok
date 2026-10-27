//! The XES event log of a simulation run (`SimResult::to_xes`).
//!
//! Every document is parsed back and checked against the rules of the XES standard
//! (IEEE 1849-2016): the element order of its XML schema, typed attribute values, declared
//! extension prefixes, global attributes present on every trace and event, classifiers
//! over global event attributes, unique keys, and the transitions of the standard
//! lifecycle model. The document of `fixtures/xes/repair-desk.tok` is also compared with
//! `fixtures/xes/repair-desk.xes`; set `TOK_BLESS=1` to rewrite that file after a
//! deliberate change of the format. The model uses only uniform and triangular draws, so
//! the file is the same on every platform.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tok_sim::xes::EXTENSIONS;
use tok_sim::{Param, SimResult, simulate};

fn fixture(parts: &[&str]) -> PathBuf {
    let mut p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    for part in parts {
        p = p.join(part);
    }
    p
}

fn repo_file(parts: &[&str]) -> PathBuf {
    let mut p = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    for part in parts {
        p = p.join(part);
    }
    p
}

fn run(path: &Path, cases: Option<i64>, seed: Option<i64>) -> Option<SimResult> {
    let text = std::fs::read_to_string(path).ok()?.replace("\r\n", "\n");
    let (doc, diags) = tok_core::load(&text, "full", None);
    if diags.iter().any(|d| d.is_error()) {
        return None;
    }
    simulate(
        &doc,
        cases.map(|c| Param::Int(c.to_string())),
        seed.map(|s| Param::Int(s.to_string())),
    )
    .ok()
}

fn attr<'a>(n: &roxmltree::Node<'a, 'a>, name: &str) -> &'a str {
    n.attribute(name)
        .unwrap_or_else(|| panic!("<{}> without {name}", n.tag_name().name()))
}

/// `YYYY-MM-DDTHH:MM:SS.mmm` with an optional `+HH:MM`/`-HH:MM` offset.
fn is_xes_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| r.into_iter().all(|i| b[i].is_ascii_digit());
    let base = b.len() >= 23
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19)
        && b[19] == b'.'
        && digits(20..23);
    base && (b.len() == 23
        || (b.len() == 29
            && (b[23] == b'+' || b[23] == b'-')
            && digits(24..26)
            && b[26] == b':'
            && digits(27..29)))
}

/// The attributes (key → (type, value)) of an element, checking types, values and keys.
fn attributes(el: &roxmltree::Node, prefixes: &HashSet<&str>) -> HashMap<String, (String, String)> {
    let mut out = HashMap::new();
    for a in el.children().filter(roxmltree::Node::is_element) {
        let tag = a.tag_name().name();
        if !matches!(tag, "string" | "date" | "int" | "float" | "boolean") {
            continue;
        }
        let key = attr(&a, "key");
        let value = attr(&a, "value");
        if let Some((prefix, _)) = key.split_once(':') {
            assert!(prefixes.contains(prefix), "undeclared prefix in {key}");
        }
        assert!(
            key.starts_with(|c: char| c.is_alphabetic() || c == '_')
                && key
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')),
            "key {key:?} is not an XML name"
        );
        match tag {
            "date" => assert!(is_xes_timestamp(value), "date {value:?}"),
            "int" => assert!(value.parse::<i64>().is_ok(), "int {value:?}"),
            "float" => assert!(
                value.parse::<f64>().is_ok() || matches!(value, "NaN" | "INF" | "-INF"),
                "float {value:?}"
            ),
            "boolean" => assert!(value == "true" || value == "false", "boolean {value:?}"),
            _ => {}
        }
        assert!(
            out.insert(key.to_string(), (tag.to_string(), value.to_string()))
                .is_none(),
            "key {key} twice in one element"
        );
    }
    out
}

/// Check one XES document against the standard's rules and against the result it shows.
fn check_xes(xml: &str, res: &SimResult, label: &str) {
    let doc = roxmltree::Document::parse(xml).unwrap_or_else(|e| panic!("{label}: {e}"));
    let log = doc.root_element();
    assert_eq!(log.tag_name().name(), "log", "{label}");
    assert_eq!(log.tag_name().namespace(), None, "{label}");
    let version = attr(&log, "xes.version");
    assert!(
        version.parse::<f64>().is_ok(),
        "{label}: xes.version {version}"
    );

    // element order of LogType: extension*, global*, classifier*, attributes, trace*
    let order = |tag: &str| match tag {
        "extension" => 0,
        "global" => 1,
        "classifier" => 2,
        "string" | "date" | "int" | "float" | "boolean" | "id" | "list" => 3,
        "trace" => 4,
        "event" => 5,
        other => panic!("{label}: unexpected <{other}>"),
    };
    let ranks: Vec<i32> = log
        .children()
        .filter(roxmltree::Node::is_element)
        .map(|n| order(n.tag_name().name()))
        .collect();
    assert!(
        ranks.windows(2).all(|w| w[0] <= w[1]),
        "{label}: order {ranks:?}"
    );

    let extensions: Vec<(&str, &str, &str)> = log
        .children()
        .filter(|n| n.has_tag_name("extension"))
        .map(|n| (attr(&n, "name"), attr(&n, "prefix"), attr(&n, "uri")))
        .collect();
    assert_eq!(extensions, EXTENSIONS.to_vec(), "{label}");
    let prefixes: HashSet<&str> = extensions.iter().map(|e| e.1).collect();

    let mut globals: HashMap<&str, HashMap<String, (String, String)>> = HashMap::new();
    for g in log.children().filter(|n| n.has_tag_name("global")) {
        globals.insert(attr(&g, "scope"), attributes(&g, &prefixes));
    }
    let trace_globals = &globals["trace"];
    let event_globals = &globals["event"];
    assert!(trace_globals.contains_key("concept:name"));
    for k in [
        "concept:name",
        "concept:instance",
        "lifecycle:transition",
        "time:timestamp",
        "activity",
    ] {
        assert!(event_globals.contains_key(k), "{label}: global {k}");
    }
    let classifiers: Vec<&str> = log
        .children()
        .filter(|n| n.has_tag_name("classifier"))
        .map(|n| {
            let name = attr(&n, "name");
            assert!(!name.contains(' '), "{label}: classifier name {name:?}");
            for key in attr(&n, "keys").split_whitespace() {
                assert!(
                    event_globals.contains_key(key),
                    "{label}: classifier key {key}"
                );
            }
            name
        })
        .collect();
    assert!(classifiers.contains(&"Activity"), "{label}");
    let log_attrs = attributes(&log, &prefixes);
    assert_eq!(
        log_attrs["lifecycle:model"],
        ("string".into(), "standard".into())
    );

    let o = &res.outcome;
    let traces: Vec<_> = log.children().filter(|n| n.has_tag_name("trace")).collect();
    assert_eq!(traces.len(), o.cases.len(), "{label}: traces");
    let mut events_total = 0;
    for (t, case) in traces.iter().zip(&o.cases) {
        let ta = attributes(t, &prefixes);
        for (k, (ty, _)) in trace_globals {
            assert_eq!(&ta[k].0, ty, "{label}: trace global {k}");
        }
        assert_eq!(ta["concept:name"].1, case.id.to_string());
        assert_eq!(ta.contains_key("end"), case.end.is_some());
        let rows: Vec<_> = o.rows.iter().filter(|r| r.case == case.id).collect();
        let expected = rows
            .iter()
            .map(|r| 1 + usize::from(r.start.is_some()) + usize::from(r.end.is_some()))
            .sum::<usize>();
        let events: Vec<_> = t.children().filter(|n| n.has_tag_name("event")).collect();
        assert_eq!(
            events.len(),
            expected,
            "{label}: events of case {}",
            case.id
        );
        events_total += events.len();
        let mut last = String::new();
        let mut lifecycles: HashMap<String, Vec<String>> = HashMap::new();
        for e in &events {
            let ea = attributes(e, &prefixes);
            for (k, (ty, _)) in event_globals {
                assert_eq!(&ea[k].0, ty, "{label}: event global {k}");
            }
            let ts = ea["time:timestamp"].1.clone();
            assert!(ts >= last, "{label}: case {} goes back in time", case.id);
            last = ts;
            let row: usize = ea["concept:instance"].1.parse().expect("instance number");
            let r = &o.rows[row - 1];
            assert_eq!(r.case, case.id);
            assert_eq!(ea["activity"].1, r.activity);
            let name = if r.label.is_empty() {
                &r.activity
            } else {
                &r.label
            };
            assert_eq!(&ea["concept:name"].1, name);
            match ea.get("org:resource") {
                Some((_, v)) => assert_eq!(v, &r.resource),
                None => assert!(r.resource.is_empty()),
            }
            lifecycles
                .entry(ea["concept:instance"].1.clone())
                .or_default()
                .push(ea["lifecycle:transition"].1.clone());
        }
        for (instance, seq) in lifecycles {
            let seq: Vec<&str> = seq.iter().map(String::as_str).collect();
            assert!(
                matches!(
                    seq.as_slice(),
                    ["schedule", "start", "complete"]
                        | ["schedule", "start", "ati_abort"]
                        | ["schedule", "withdraw"]
                ),
                "{label}: instance {instance}: {seq:?}"
            );
        }
    }
    assert!(events_total >= o.rows.len() * 2, "{label}");
}

#[test]
fn golden_file() {
    let res = run(&fixture(&["xes", "repair-desk.tok"]), None, None).expect("simulation");
    let xml = res.to_xes().expect("XES");
    check_xes(&xml, &res, "repair-desk");
    let golden = fixture(&["xes", "repair-desk.xes"]);
    if std::env::var_os("TOK_BLESS").is_some() {
        std::fs::write(&golden, &xml).expect("write the golden file");
    }
    let want = std::fs::read_to_string(&golden)
        .expect("golden file")
        .replace("\r\n", "\n");
    assert_eq!(
        xml,
        want,
        "the XES document differs from {}",
        golden.display()
    );
    // interrupted instances: aborted while running and withdrawn while waiting
    assert!(xml.contains("\"ati_abort\""));
    assert!(xml.contains("\"withdraw\""));
    // typed case variables: text, float, boolean, int (FEEL) and date (FEEL)
    assert!(xml.contains("<string key=\"kind\" value=\"phone\"/>"));
    assert!(xml.contains("<float key=\"items\" value=\"3.0\"/>"));
    assert!(xml.contains("<boolean key=\"urgent\" value=\"false\"/>"));
    assert!(xml.contains("<int key=\"parts\" value=\"6\"/>"));
    assert!(xml.contains("<date key=\"received\" value=\"2027-03-01T00:00:00.000\"/>"));
    assert!(xml.contains("value=\"Triage &amp; &lt;sort&gt;\""));
    assert!(xml.contains("value=\"Return &quot;device&quot;\""));
    assert!(xml.contains("<date key=\"time:timestamp\" value=\"2027-03-01T09:00:00.000+01:00\"/>"));
}

#[test]
fn writes_the_file() {
    let res = run(&fixture(&["xes", "repair-desk.tok"]), Some(3), Some(1)).expect("simulation");
    let dir = std::env::temp_dir().join(format!("tok-xes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("folder");
    let path = dir.join("log.xes");
    res.write_xes(&path.to_string_lossy()).expect("written");
    let text = std::fs::read_to_string(&path).expect("read back");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(text, res.to_xes().expect("XES"));
    let missing = dir.join("no-such-folder").join("log.xes");
    let err = res
        .write_xes(&missing.to_string_lossy())
        .expect_err("no folder");
    assert_eq!(err.kind, "FileNotFoundError");
}

#[test]
fn examples_and_synthetic_models_give_valid_logs() {
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in [repo_file(&["examples"]), fixture(&["sim"])] {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("folder")
            .map(|e| e.expect("entry").path())
            .filter(|p| p.extension().is_some_and(|e| e == "tok"))
            .collect();
        found.sort();
        paths.extend(found);
    }
    let mut checked = 0;
    for p in &paths {
        for (cases, seed) in [(Some(4), Some(1)), (Some(2), Some(42))] {
            let Some(res) = run(p, cases, seed) else {
                continue;
            };
            let Ok(xml) = res.to_xes() else {
                continue;
            };
            check_xes(&xml, &res, &p.display().to_string());
            assert_eq!(xml, res.to_xes().expect("XES"), "deterministic");
            checked += 1;
        }
    }
    assert!(checked > 40, "only {checked} runs checked");
}

#[test]
fn times_without_an_offset_stay_local() {
    let res = run(
        &repo_file(&["examples", "kwerenda-symulacja.tok"]),
        Some(4),
        None,
    )
    .expect("simulation");
    let xml = res.to_xes().expect("XES");
    check_xes(&xml, &res, "kwerenda");
    assert!(xml.contains("<date key=\"start\" value=\"2027-01-04T08:00:00.000\"/>"));
}

#[test]
fn a_run_without_cases_has_no_traces() {
    let text = "tok 0.3 \"Empty\"\nstart s \"S\"\nend e \"E\"\nflow s -> e\n";
    let (doc, _) = tok_core::load(text, "full", None);
    let res = simulate(&doc, Some(Param::Int("0".into())), None).expect("simulation");
    let xml = res.to_xes().expect("XES");
    check_xes(&xml, &res, "empty");
    assert!(!xml.contains("<trace>"));
}
