//! Language 1.0: a document with the header `tok 1.0` means what it means with the header
//! `tok 0.7`. Every conformance case of 0.5, 0.6 and 0.7, with its header changed to 1.0,
//! gives the same canonical text (but the header), the same diagnostics at every level,
//! the same BPMN and the same DMN files (but `exporterVersion`); the deprecation `R090` is
//! reported only under the header 1.0.

use std::fs;
use std::path::PathBuf;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases")
}

/// The text with the version of its header (`tok VERSION`) replaced by `to`.
fn with_version(text: &str, from: &str, to: &str) -> Option<String> {
    let head = format!("tok {from}");
    let pos = text.find(&head)?;
    if !text[..pos]
        .lines()
        .all(|l| l.trim().is_empty() || l.trim_start().starts_with('#'))
    {
        return None;
    }
    Some(format!(
        "{}tok {to}{}",
        &text[..pos],
        &text[pos + head.len()..]
    ))
}

fn codes(text: &str, level: &str) -> Vec<String> {
    let (_, diags) = tok_core::load(text, level, None);
    diags.into_iter().map(|d| d.code).collect()
}

fn outputs(text: &str, version: &str) -> (String, String, Vec<(String, String)>) {
    let (doc, diags) = tok_core::parse(text);
    let fmt = tok_core::fmt(&doc).replacen(&format!("tok {version}"), "tok X", 1);
    let errors = diags.iter().any(tok_core::Diag::is_error);
    let mark = format!("exporterVersion=\"{version}\"");
    let bpmn = if errors {
        String::new()
    } else {
        tok_core::export::semantic_xml(&doc, None).replace(&mark, "exporterVersion=\"X\"")
    };
    let dmn = if errors || !doc.lang_07() {
        Vec::new()
    } else {
        tok_core::drg::export::export_dmn15(&doc)
            .into_iter()
            .map(|(stem, xml)| (stem, xml.replace(&mark, "exporterVersion=\"X\"")))
            .collect()
    };
    (fmt, bpmn, dmn)
}

#[test]
fn documents_of_05_06_and_07_mean_the_same_under_10() {
    let mut names: Vec<_> = fs::read_dir(corpus())
        .expect("cases")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tok"))
        .collect();
    names.sort();
    let mut compared = 0;
    for path in names {
        let text = fs::read_to_string(&path).expect("case");
        let Some((version, ten)) = ["0.5", "0.6", "0.7"]
            .iter()
            .find_map(|v| with_version(&text, v, "1.0").map(|t| (*v, t)))
        else {
            continue;
        };
        let name = path.display();
        // documents of 0.5 and 0.6 do not have the statements of 0.7, which keep their
        // meaning only in a 0.7 document; compare them with their text as 0.7
        let seven = with_version(&text, version, "0.7").expect("header");
        if version != "0.7" {
            let (_, older) = tok_core::parse(&text);
            let (_, as07) = tok_core::parse(&seven);
            let older: Vec<&str> = older.iter().map(|d| d.code.as_str()).collect();
            let as07: Vec<&str> = as07.iter().map(|d| d.code.as_str()).collect();
            if older != as07 {
                continue;
            }
        }
        for level in ["basic", "analytic", "full"] {
            assert_eq!(
                codes(&seven, level),
                codes(&ten, level),
                "{name}: diagnostics at the level {level}"
            );
        }
        assert_eq!(
            outputs(&seven, "0.7"),
            outputs(&ten, "1.0"),
            "{name}: outputs"
        );
        compared += 1;
    }
    assert!(compared >= 25, "only {compared} cases compared");
}

#[test]
fn exporter_version_is_the_language_version() {
    let text = "tok 1.0 \"T\"\nstart s\nend e\nflow s -> e\ninputdata a \"A\" type=number\n\
                decision d \"D\" requires=\"a\" expr=\"a + 1\"\n";
    let (doc, diags) = tok_core::parse(text);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(tok_core::export::semantic_xml(&doc, None).contains("exporterVersion=\"1.0\""));
    let dmn = tok_core::drg::export::export_dmn15(&doc);
    assert_eq!(dmn.len(), 1);
    assert!(dmn[0].1.contains("exporterVersion=\"1.0\""));
    assert!(
        dmn[0]
            .1
            .contains("xmlns=\"https://www.omg.org/spec/DMN/20230324/MODEL/\"")
    );
}

#[test]
fn repeated_keys_and_flags_are_deprecated_only_in_10() {
    let body = " \"T\" id=a id=b\nstart s \"S\" message=\"M\" message=\"N\"\n\
                user u \"U\" doc=\"x\" doc=\"y\" loop loop\nend e\nflow s -> u -> e\n";
    let r090 = |version: &str| {
        let (_, diags) = tok_core::load(&format!("tok {version}{body}"), "full", None);
        diags
            .iter()
            .filter(|d| d.code == "R090")
            .map(|d| (d.line, d.message("en")))
            .collect::<Vec<_>>()
    };
    let ten = r090("1.0");
    let lines: Vec<usize> = ten.iter().map(|(l, _)| *l).collect();
    // the header's `id`, then `doc` and `loop` of the user task; the triggers of the start
    // event may repeat
    assert_eq!(lines, vec![1, 3, 3]);
    assert_eq!(
        ten[0].1,
        "'id' is given more than once on this line; only the last value counts (deprecated in language 1.0)"
    );
    for older in ["0.1", "0.3", "0.5", "0.6", "0.7", "1.1"] {
        assert!(r090(older).is_empty(), "{older}");
    }
    // the canonical form keeps the last value of a key and one copy of a flag
    let (doc, _) = tok_core::parse(&format!("tok 1.0{body}"));
    let text = tok_core::fmt(&doc);
    assert!(text.starts_with("tok 1.0 \"T\" id=b\n"), "{text}");
    assert!(text.contains("user u \"U\" loop doc=\"y\"\n"), "{text}");
    let (_, again) = tok_core::load(&text, "full", None);
    assert!(again.iter().all(|d| d.code != "R090"));
}

#[test]
fn unknown_and_malformed_versions() {
    let codes = |header: &str| {
        let (doc, diags) = tok_core::parse(&format!("{header}\nstart s\nend e\nflow s -> e\n"));
        (
            doc.version.clone(),
            diags.into_iter().map(|d| d.code).collect::<Vec<_>>(),
        )
    };
    assert_eq!(codes("tok 1.0 \"T\""), (Some("1.0".into()), vec![]));
    assert_eq!(codes("tok 1 \"T\"").1, vec!["P006"]);
    assert_eq!(codes("tok 1.1 \"T\"").1, vec!["P006"]);
    assert_eq!(codes("tok 1.00 \"T\"").1, vec!["P006"]);
    assert_eq!(codes("tok v1.0 \"T\""), (None, vec!["P005".to_string()]));
    assert_eq!(tok_core::LANGUAGE_VERSION, "1.0");
}
