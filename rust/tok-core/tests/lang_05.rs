//! Language 0.5 next to 0.1 and 0.3: the header versions, the meaning kept by documents of
//! the older versions, and the diagnostic catalogue of the new codes.

use tok_core::diagnostics::MESSAGES;

fn codes(text: &str) -> Vec<String> {
    let (_, diags) = tok_core::load(text, "full", None);
    diags.into_iter().map(|d| d.code).collect()
}

fn xml(text: &str) -> String {
    let (doc, _) = tok_core::parse(text);
    tok_core::export::semantic_xml(&doc, None)
}

#[test]
fn every_version_is_accepted() {
    for v in ["0.1", "0.3", "0.5"] {
        let text = format!("tok {v} \"x\"\nstart s\nend e\nflow s -> e\n");
        assert!(!codes(&text).contains(&"P006".to_string()), "tok {v}");
    }
    assert!(codes("tok 0.7\nstart s\nend e\nflow s -> e\n").contains(&"P006".to_string()));
}

#[test]
fn several_triggers_need_language_05() {
    let body = "start s message=\"A\" signal=\"B\"\nend e\nflow s -> e\n";
    let old = format!("tok 0.3\n{body}");
    let new = format!("tok 0.5\n{body}");
    assert!(codes(&old).contains(&"R012".to_string()));
    assert!(xml(&old).contains("s-def") && !xml(&old).contains("s-def-2"));
    assert!(!codes(&new).contains(&"R012".to_string()));
    assert!(xml(&new).contains("s-def-2"));
    // a repeated key keeps its last value in a document of 0.3, as before
    let repeated = "tok 0.3\nstart s message=\"A\" message=\"B\"\nend e\nflow s -> e\n";
    let x = xml(repeated);
    assert!(x.contains("Message_B") && !x.contains("Message_A"));
}

#[test]
fn resources_are_bpmn_only_in_language_05() {
    let body = "start s\nuser u \"U\" by=r\nend e\nflow s -> u -> e\nresource r \"R\" count=1\n";
    assert!(!xml(&format!("tok 0.3\n{body}")).contains("humanPerformer"));
    let x = xml(&format!("tok 0.5\n{body}"));
    assert!(x.contains("<bpmn:humanPerformer id=\"u-performer\">"));
    assert!(x.contains("<bpmn:resource id=\"r\" name=\"R\" />"));
}

#[test]
fn new_codes_have_both_messages() {
    let new = [
        "R060", "R061", "R062", "R063", "R064", "R065", "R066", "C100", "C101", "C102", "C103",
        "C104", "C105", "C106", "C107", "C108", "C109", "C110",
    ];
    for code in new {
        let entry = MESSAGES.iter().find(|e| e.0 == code);
        let Some((_, _, en, pl)) = entry else {
            panic!("{code} is not in the catalogue");
        };
        assert!(!en.is_empty() && !pl.is_empty(), "{code}");
    }
}

#[test]
fn polish_messages_of_the_checker() {
    let text = "tok 0.5\nstart s\nuser u \"U\" while=\"x < 3\"\nend e\nflow s -> u -> e\n";
    let (_, diags) = tok_core::load(text, "full", None);
    let d = diags.iter().find(|d| d.code == "R061").expect("R061");
    assert_eq!(d.message("en"), "while on 'u' requires the flag loop");
    assert_eq!(d.message("pl"), "while w „u” wymaga flagi loop");
}
