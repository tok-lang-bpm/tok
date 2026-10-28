//! Language 0.7: evaluation of decision requirements graphs and boxed expressions, the
//! DMN 1.5 export and import round trip, and the FEEL of process expressions.

use std::fs;
use std::path::PathBuf;

use tok_core::drg::{dmnimport, eval, export, values};
use tok_core::feel3::Map;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn case(name: &str) -> tok_core::Document {
    let text = fs::read_to_string(corpus().join("cases").join(name)).expect("case");
    let (doc, diags) = tok_core::load(&text, "full", None);
    assert!(
        !diags
            .iter()
            .any(|d| d.is_error() && !d.code.starts_with('C')),
        "{name}: {diags:?}"
    );
    doc
}

/// The JSON text of the value of a decision for `name=value` inputs.
fn run(doc: &tok_core::Document, decision: &str, inputs: &[(&str, &str)]) -> String {
    let mut m = Map::new();
    for (k, v) in inputs {
        m.insert((*k).to_string(), values::cli_value(v));
    }
    let (v, problems) = eval::evaluate(doc, decision, &m);
    assert!(problems.is_empty(), "{decision}: {problems:?}");
    values::to_json(&v).compact()
}

#[test]
fn decision_requirements_graph() {
    let d = case("53-drd.tok");
    assert_eq!(
        run(&d, "limit", &[("wiek", "20"), ("konto", "student")]),
        "15"
    );
    assert_eq!(
        run(&d, "limit", &[("wiek", "16"), ("konto", "student")]),
        "5"
    );
    assert_eq!(
        run(
            &d,
            "blokada",
            &[
                ("wiek", "20"),
                ("konto", "student"),
                ("Zaległe opłaty", "60")
            ]
        ),
        "true"
    );
    assert_eq!(run(&d, "naleznosc", &[("dni-zwloki", "10")]), "5");
    assert_eq!(
        run(
            &d,
            "opis",
            &[
                ("wiek", "16"),
                ("konto", "gość"),
                ("Zaległe opłaty", "0"),
                ("dni-zwloki", "3")
            ]
        ),
        "\"limit 5, należność 1.5 zł\""
    );
}

#[test]
fn boxed_expressions() {
    let d = case("54-boxed.tok");
    assert_eq!(
        run(&d, "oplata", &[("rodzaj", "czasopismo"), ("dni", "7")]),
        "1.4"
    );
    assert_eq!(
        run(&d, "oplata", &[("rodzaj", "książka"), ("dni", "1000")]),
        "100"
    );
    assert_eq!(
        run(&d, "kategorie", &[]),
        "[\"student\", \"doktorant\", \"pracownik\"]"
    );
    let egz = r#"[{"sygnatura": "A1", "rok": 1930, "stan zachowania": "zły"}, {"sygnatura": "B2", "rok": 1999, "stan zachowania": "dobry"}]"#;
    assert_eq!(
        run(&d, "podsumowanie", &[("egzemplarze", egz)]),
        "{\"liczba\": 2, \"najstarszy\": 1930, \"do naprawy\": [\"A1\"], \"przeglad\": true}"
    );
    // a value that does not conform to the type of the input data is null
    let bad = r#"[{"sygnatura": "A1", "rok": 1930, "stan zachowania": "nowy"}]"#;
    assert_eq!(
        run(&d, "podsumowanie", &[("egzemplarze", bad)]),
        "{\"liczba\": null, \"najstarszy\": null, \"do naprawy\": null, \"przeglad\": false}"
    );
}

#[test]
fn hit_policies_of_language_07() {
    let d = case("55-tables-07.tok");
    let late = [("termin", "2027-01-10"), ("zwrot", "2027-02-20")];
    let with = |extra: &[(&'static str, &'static str)]| {
        let mut v: Vec<(&str, &str)> = late.to_vec();
        v.extend_from_slice(extra);
        v
    };
    assert_eq!(run(&d, "ocena", &with(&[("wartosc", "100")])), "\"pilne\"");
    assert_eq!(run(&d, "punkty", &with(&[("wartosc", "600")])), "6");
    assert_eq!(
        run(
            &d,
            "punkty",
            &[
                ("termin", "2027-01-10"),
                ("zwrot", "2027-01-05"),
                ("wartosc", "1")
            ]
        ),
        "0"
    );
    assert_eq!(
        run(
            &d,
            "powiadomienia",
            &[
                ("konto", "student"),
                ("termin", "2027-01-10"),
                ("zwrot", "2027-02-20")
            ]
        ),
        "[{\"kanal\": \"e-mail\", \"tresc\": \"Minął termin zwrotu.\"}, {\"kanal\": \"SMS\", \"tresc\": \"Prosimy o zwrot.\"}]"
    );
    assert_eq!(
        run(&d, "kolejnosc", &[("konto", "student")]),
        "[{\"kanal\": \"SMS\", \"waga\": 2}, {\"kanal\": \"e-mail\", \"waga\": 1}]"
    );
}

#[test]
fn violated_hit_policy_is_reported() {
    let text = "tok 0.7 \"x\"\ninputdata a \"A\" type=number\ndecision d \"D\" requires=\"a\"\n  | a: number || w: number |\n  | > 1       || 1         |\n  | > 2       || 2         |\n";
    let (doc, _) = tok_core::parse(text);
    let mut m = Map::new();
    m.insert("a".into(), values::cli_value("5"));
    let (v, problems) = eval::evaluate(&doc, "d", &m);
    assert!(v.is_null());
    assert_eq!(
        problems,
        vec!["decision 'd': rules 1, 2 all match (hit policy unique)".to_string()]
    );
}

#[test]
fn dmn_round_trip_keeps_the_graph() {
    for name in ["53-drd.tok", "54-boxed.tok", "55-tables-07.tok"] {
        let d = case(name);
        let original = tok_core::fmt(&d);
        for (_, xml) in export::export_dmn15(&d) {
            let (back, warnings) = dmnimport::import_dmn(&xml).expect("import");
            assert!(
                warnings.iter().all(|w| w.contains("DMNDI")),
                "{name}: {warnings:?}"
            );
            for (id, dec) in &back.decisions {
                let orig = d.decisions.get(id).expect("decision kept");
                assert_eq!(
                    tok_core::drg::requires(&dec.base).len(),
                    tok_core::drg::requires(&orig.base).len(),
                    "{name}: {id}"
                );
                for k in [
                    "expr",
                    "invoke",
                    "with",
                    "hit",
                    "aggregation",
                    "default",
                    "name",
                ] {
                    assert_eq!(dec.base.attr(k), orig.base.attr(k), "{name}: {id} {k}");
                }
                assert_eq!(dec.rules.len(), orig.rules.len(), "{name}: {id}");
                assert_eq!(dec.lines.len(), orig.lines.len(), "{name}: {id}");
            }
            // the imported document is valid and prints canonically
            let text = tok_core::fmt(&back);
            let (again, diags) = tok_core::load(&text, "full", None);
            assert!(
                !diags.iter().any(tok_core::Diag::is_error),
                "{name}: {diags:?}"
            );
            assert_eq!(tok_core::fmt(&again), text);
        }
        assert!(!original.is_empty());
    }
}

#[test]
fn older_documents_keep_their_decision_semantics() {
    // a 0.6 table keeps the object of outputs and the 28 digits of tok 0.1–0.6
    let text = "tok 0.6 \"x\"\ndecision d \"D\"\n  | a: number || w: number |\n  | -         || a / 3     |\n";
    let (doc, _) = tok_core::parse(text);
    let mut ctx = tok_core::feel_eval::Context::new();
    ctx.insert(
        "a".into(),
        tok_core::feel_eval::Value::Num(tok_core::pydec::Decimal::from_i128(1)),
    );
    let r = values::evaluate_values(&doc, "d", &ctx).expect("eval");
    let json = r.to_json().expect("json").compact();
    assert_eq!(json, "{\"w\": 0.3333333333333333}");
    // the same table in 0.7: the value of its only output, with 34 digits
    let text07 = text.replace("tok 0.6", "tok 0.7").replace(
        "decision d \"D\"",
        "inputdata a \"A\" type=number\ndecision d \"D\" requires=\"a\"",
    );
    let (doc07, _) = tok_core::parse(&text07);
    assert_eq!(
        run(&doc07, "d", &[("a", "1")]),
        "0.3333333333333333333333333333333333"
    );
}

#[test]
fn process_expressions_use_feel_of_level_3() {
    let d = case("56-feel3.tok");
    assert_eq!(
        run(
            &d,
            "dni-zwloki",
            &[("termin", "2027-01-10"), ("zwrot", "2027-02-20")]
        ),
        "41"
    );
    // full FEEL in a condition is no F001 in a 0.7 document, but is in a 0.6 one
    let cond = "start s\nxor g \"Czy?\"\nend a\nend b\nflow s -> g\nflow g -> a \"tak\" when=\"some x in [1, 2] satisfies x > 1\"\nflow g -> b \"nie\" default\n";
    let codes = |v: &str| {
        let (_, diags) = tok_core::load(&format!("tok {v} \"x\"\n{cond}"), "full", None);
        diags.into_iter().map(|d| d.code).collect::<Vec<_>>()
    };
    assert!(!codes("0.7").contains(&"F001".to_string()));
    assert!(codes("0.6").contains(&"F001".to_string()));
}

#[test]
fn drd_picture_and_diagram() {
    let d = case("53-drd.tok");
    let files = export::decision_files(&d);
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["limit.dmn", "limit.drd.svg"]);
    let xml = &files[0].1;
    assert!(xml.contains("xmlns=\"https://www.omg.org/spec/DMN/20230324/MODEL/\""));
    assert!(xml.contains("<dmndi:DMNShape id=\"DMNShape_opis\" dmnElementRef=\"opis\">"));
    assert!(xml.contains("<authorityRequirement id=\"dyrektor-requires-zaleglosci\">"));
    let (shapes, edges) = export::layout(&d, &tok_core::drg::graphs(&d)[0]);
    // every requirement is drawn from below its requirer, without overlapping shapes
    for e in &edges {
        assert!(e.points[0].1 >= e.points[1].1, "{}", e.id);
    }
    for (i, a) in shapes.iter().enumerate() {
        for b in &shapes[i + 1..] {
            let apart =
                a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
            assert!(apart, "{} and {} overlap", a.id, b.id);
        }
    }
}

#[test]
fn names_for_camunda_are_quoted() {
    use tok_core::feel3::Names;
    use tok_core::feel3::parse::camunda_text;
    let mut names = Names::builtins();
    for n in [
        "Zaległe opłaty",
        "dni-zwloki",
        "stan zachowania",
        "opłata",
        "do zapłaty",
    ] {
        names.add(n);
    }
    assert_eq!(
        camunda_text("Zaległe opłaty > 50", &names, false),
        "`Zaległe opłaty` > 50"
    );
    assert_eq!(
        camunda_text("dni-zwloki * 2", &names, false),
        "`dni-zwloki` * 2"
    );
    assert_eq!(camunda_text("opłata + 1", &names, false), "opłata + 1");
    assert_eq!(
        camunda_text("l[stan zachowania = \"stan zachowania\"]", &names, false),
        "l[`stan zachowania` = \"stan zachowania\"]"
    );
    assert_eq!(
        camunda_text("{do zapłaty: 1}", &names, false),
        "{\"do zapłaty\": 1}"
    );
    assert_eq!(
        camunda_text("string length(\"a\")", &names, false),
        "string length(\"a\")"
    );
    assert_eq!(camunda_text("> dni-zwloki", &names, true), "> `dni-zwloki`");
    // a document for Camunda is exported with the quoted names, another one as written
    let d = case("61-executable-decisions.tok");
    let xml: String = export::export_dmn15(&d)
        .into_iter()
        .map(|(_, x)| x)
        .collect();
    assert!(xml.contains("<text>`Zaległe opłaty` &gt; 50 or limit &lt; 1</text>"));
    let d53 = case("53-drd.tok");
    let xml53: String = export::export_dmn15(&d53)
        .into_iter()
        .map(|(_, x)| x)
        .collect();
    assert!(xml53.contains("<text>Zaległe opłaty &gt; 50 or limit &lt; 1</text>"));
}
