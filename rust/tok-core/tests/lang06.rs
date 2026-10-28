//! Language 0.6: BPMN conformance classes (`tok check --class`, R075) and the round trip
//! of conversations, choreographies and participant multiplicity through BPMN import.

use std::fs;
use std::path::PathBuf;

use tok_core::classes::{bpmn_class, check_class};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn doc(text: &str) -> tok_core::Document {
    let (d, diags) = tok_core::parse(text);
    assert!(
        !diags.iter().any(tok_core::Diag::is_error),
        "{text}: {diags:?}"
    );
    d
}

#[test]
fn smallest_class_of_a_document() {
    let descriptive = "tok 0.6 \"Opis\"\nstart s\nuser u \"Zadanie\"\nxor g \"Czy?\"\nend e\nend e2\n\
                       flow s -> u -> g\nflow g -> e\nflow g -> e2\n";
    assert_eq!(bpmn_class(&doc(descriptive)), "descriptive");
    let analytic = "tok 0.6 \"Analiza\"\nstart s\nsend w \"Wysłanie\" message=\"M\"\nor g \"Które?\"\n\
                    end e\nend e2\nflow s -> w -> g\nflow g -> e \"a\" when=\"x\"\nflow g -> e2 default\n";
    assert_eq!(bpmn_class(&doc(analytic)), "analytic");
    let executable = "tok 0.6 \"Wykonanie\"\nitem t structure=\"xsd:string\"\ninput i \"Wejście\" type=t\n\
                      start s\nservice z \"Usługa\"\nend e\nflow s -> z -> e\n";
    assert_eq!(bpmn_class(&doc(executable)), "common-executable");
    let full =
        "tok 0.6 \"Pełna\"\nstart s\nscript z \"Skrypt\" expr=\"1\"\nend e\nflow s -> z -> e\n";
    assert_eq!(bpmn_class(&doc(full)), "full");
}

#[test]
fn elements_outside_a_class_are_r075() {
    let text = fs::read_to_string(corpus().join("cases/48-choreography.tok")).expect("case");
    let d = doc(&text);
    let diags = check_class(&d, "analytic");
    assert!(!diags.is_empty());
    assert!(diags.iter().all(|x| x.code == "R075"));
    let msg = diags[0].message("en");
    assert!(
        msg.contains("outside the BPMN conformance class analytic"),
        "{msg}"
    );
    assert!(check_class(&d, "full").is_empty());
    // pools and lanes are not in the Common Executable class, but in the Descriptive one
    let pools = doc("tok 0.6\npool p \"P\"\n  start s\n  end e\n  flow s -> e\n");
    let r = check_class(&pools, "common-executable");
    assert_eq!(r.len(), 1);
    assert!(
        r[0].message("en")
            .ends_with("belongs to the class descriptive")
    );
    assert!(
        r[0].message("pl")
            .contains("wykracza poza klasę zgodności BPMN common-executable")
    );
}

/// Export, import with identifiers and export again: the same semantic BPMN and the same
/// canonical text.
#[test]
fn conversations_and_choreographies_survive_import() {
    for case in [
        "47-conversation",
        "48-choreography",
        "49-choreography-events",
        "50-multiplicity",
        "51-executable-06",
    ] {
        let text = fs::read_to_string(corpus().join(format!("cases/{case}.tok"))).expect("case");
        let d = doc(&text);
        let xml = tok_core::export::export_xml(&d, None);
        let (back, warnings) = tok_core::import::import_bpmn(&xml, true).expect("import");
        assert!(warnings.is_empty(), "{case}: {warnings:?}");
        assert_eq!(back.version.as_deref(), Some("0.6"), "{case}");
        let printed = tok_core::fmt(&back);
        let again = doc(&printed);
        assert_eq!(
            tok_core::export::semantic_xml(&again, None),
            tok_core::export::semantic_xml(&d, None),
            "{case}:\n{printed}"
        );
    }
}

#[test]
fn referenced_message_flows_keep_their_identifiers() {
    let text = fs::read_to_string(corpus().join("cases/47-conversation.tok")).expect("case");
    let d = doc(&text);
    let xml = tok_core::export::export_xml(&d, None);
    let (back, _) = tok_core::import::import_bpmn(&xml, false).expect("import");
    let printed = tok_core::fmt(&back);
    assert!(printed.contains("id=m-zamowienie"), "{printed}");
    assert!(
        printed.contains("messages=\"m-zamowienie m-odbior\""),
        "{printed}"
    );
}

#[test]
fn documents_of_older_languages_ignore_multiplicity() {
    // `min=`/`max=` are unknown attributes of a pool before language 0.6 (R010)
    let text = "tok 0.5\npool p \"P\" max=3\npool q \"Q\"\n";
    let (mut d, _) = tok_core::parse(text);
    let diags = tok_core::check(&mut d, "full", None);
    assert!(diags.iter().any(|x| x.code == "R010"));
    assert!(!tok_core::export::semantic_xml(&d, None).contains("participantMultiplicity"));
}

#[test]
fn groups_keep_conversations_and_process_data_in_their_elements() {
    // the category of a group is a root element before the collaboration; it once moved
    // the conversations into itself and the data of the process into the participant
    let text = "tok 0.6 \"Grupy\"\npool a \"A\"\n  start s\n  user u \"Zadanie\" compensate=h\n  \
                service h \"Cofnięcie\" compensation\n  end e\n  property p \"Licznik\"\n\
                pool b \"B\"\nflow s -> u -> e\nmessage u -> b \"M\" id=m\n\
                conversation k \"Rozmowa\" pools=\"a b\" messages=\"m\"\n\
                group g \"Grupa\" members=\"s u\"\n";
    let xml = tok_core::export::semantic_xml(&doc(text), None);
    let x = roxmltree::Document::parse(&xml).expect("well formed");
    let parent = |name: &str| {
        x.descendants()
            .find(|n| n.tag_name().name() == name)
            .and_then(|n| n.parent_element())
            .map(|p| p.tag_name().name().to_string())
    };
    assert_eq!(parent("conversation").as_deref(), Some("collaboration"));
    assert_eq!(parent("conversationLink").as_deref(), Some("collaboration"));
    assert_eq!(parent("property").as_deref(), Some("process"));
    assert_eq!(parent("categoryValue").as_deref(), Some("category"));
    assert_eq!(parent("category").as_deref(), Some("definitions"));
}
