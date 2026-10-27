//! Diagnostic codes, severities and messages (English and Polish). Mirrors `diagnostics.py`.

use crate::json::{Json, obj};

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
    fn polish(self) -> &'static str {
        match self {
            Severity::Error => "błąd",
            Severity::Warning => "ostrzeżenie",
            Severity::Note => "uwaga",
        }
    }
}

use Severity::{Error as E, Warning as W};

/// (code, severity, English template, Polish template)
pub const MESSAGES: &[(&str, Severity, &str, &str)] = &[
    (
        "T001",
        E,
        "invalid token: {detail}",
        "niepoprawny znak lub tekst: {detail}",
    ),
    (
        "T002",
        E,
        "tab character in indentation; use spaces",
        "tabulator we wcięciu; używaj spacji",
    ),
    (
        "T003",
        E,
        "indentation does not match any open block",
        "wcięcie nie pasuje do żadnego otwartego bloku",
    ),
    (
        "P001",
        E,
        "unknown keyword '{word}'",
        "nieznane słowo kluczowe „{word}”",
    ),
    (
        "P002",
        E,
        "'{keyword}' needs an identifier",
        "„{keyword}” wymaga identyfikatora",
    ),
    (
        "P003",
        E,
        "'{keyword}' cannot have an indented block",
        "„{keyword}” nie może mieć wciętego bloku",
    ),
    (
        "P004",
        E,
        "'{keyword}' is not allowed inside {context}",
        "„{keyword}” nie może wystąpić w bloku {context}",
    ),
    (
        "P005",
        E,
        "the document must start with the header 'tok 0.1'",
        "dokument musi zaczynać się nagłówkiem „tok 0.1”",
    ),
    (
        "P006",
        W,
        "unknown language version {version}; this tool reads version 0.1",
        "nieznana wersja języka {version}; to narzędzie czyta wersję 0.1",
    ),
    (
        "P007",
        E,
        "malformed attribute near '{near}'",
        "niepoprawny atrybut w pobliżu „{near}”",
    ),
    (
        "P008",
        E,
        "malformed {keyword} line: {detail}",
        "niepoprawny wiersz {keyword}: {detail}",
    ),
    (
        "P009",
        E,
        "malformed decision table row: {detail}",
        "niepoprawny wiersz tabeli decyzyjnej: {detail}",
    ),
    ("P010", E, "a second header", "drugi nagłówek dokumentu"),
    (
        "R001",
        E,
        "identifier '{id}' is already used on line {other}",
        "identyfikator „{id}” jest już użyty w wierszu {other}",
    ),
    (
        "R002",
        E,
        "unknown {what} '{id}'",
        "nieznany element ({what_pl}): „{id}”",
    ),
    (
        "R003",
        E,
        "'{id}' is a '{actual}' element; required here: {expected}",
        "„{id}” to {actual}, a tu wymagany jest element: {expected_pl}",
    ),
    (
        "R004",
        E,
        "identifier '{id}' must use only ASCII letters, digits, '_' and '-'",
        "identyfikator „{id}” może zawierać tylko litery ASCII, cyfry, „_” i „-”",
    ),
    (
        "R010",
        W,
        "unknown attribute '{key}' on {kind}",
        "nieznany atrybut „{key}” w elemencie {kind}",
    ),
    (
        "R011",
        E,
        "unknown flag '{flag}' on {kind}",
        "nieznana flaga „{flag}” w elemencie {kind}",
    ),
    (
        "R012",
        E,
        "an event can have only one trigger ({triggers})",
        "zdarzenie może mieć tylko jeden wyzwalacz ({triggers})",
    ),
    (
        "R013",
        E,
        "'{key}' requires {requirement}",
        "„{key}” wymaga: {requirement}",
    ),
    (
        "R020",
        E,
        "a flow line with a label or attributes must contain exactly one arrow",
        "wiersz flow z etykietą lub atrybutami musi mieć dokładnie jedną strzałkę",
    ),
    (
        "R021",
        E,
        "sequence flow {source} -> {target} crosses a process or sub-process boundary",
        "przepływ sekwencji {source} -> {target} przekracza granicę procesu lub podprocesu",
    ),
    (
        "R022",
        E,
        "message flow {source} -> {target} must connect two different pools",
        "przepływ komunikatów {source} -> {target} musi łączyć dwa różne baseny",
    ),
    (
        "R030",
        E,
        "'{id}' is in a pool with lanes but outside every lane",
        "„{id}” leży w basenie z liniami, ale poza liniami",
    ),
    (
        "R031",
        E,
        "'{id}' is declared at top level, but the document has pools; move it into a pool",
        "„{id}” zadeklarowano poza basenem, a dokument ma baseny; przenieś go do basenu",
    ),
    (
        "R032",
        E,
        "boundary event '{id}' needs on=ACTIVITY",
        "zdarzenie graniczne „{id}” wymaga on=CZYNNOŚĆ",
    ),
    (
        "R040",
        E,
        "'{keyword}' is not available at level {level}",
        "„{keyword}” nie jest dostępne na poziomie {level}",
    ),
    (
        "D001",
        E,
        "decision '{id}' has no header row",
        "decyzja „{id}” nie ma wiersza nagłówka",
    ),
    (
        "D002",
        E,
        "rule has {got} cells, the header has {expected}",
        "reguła ma {got} komórek, a nagłówek {expected}",
    ),
    (
        "D003",
        E,
        "unknown hit policy '{hit}'",
        "nieznana polityka trafień „{hit}”",
    ),
    (
        "D004",
        E,
        "header cell '{cell}' must be 'name: type' with type string, number, boolean or date",
        "komórka nagłówka „{cell}” musi mieć postać „nazwa: typ” (string, number, boolean, date)",
    ),
    (
        "D005",
        W,
        "decision '{id}' has no rules",
        "decyzja „{id}” nie ma reguł",
    ),
    (
        "D010",
        W,
        "rules {a} and {b} can both match {example} (hit policy unique)",
        "reguły {a} i {b} mogą pasować jednocześnie, np. {example} (polityka unique)",
    ),
    (
        "M001",
        E,
        "unknown field type '{type}'",
        "nieznany typ pola „{type}”",
    ),
    (
        "M002",
        E,
        "field '{id}' of type {type} needs options=\"value:Label, …\"",
        "pole „{id}” typu {type} wymaga options=\"wartość:Etykieta, …\"",
    ),
    (
        "F001",
        W,
        "FEEL expression outside the supported subset: {detail}",
        "wyrażenie FEEL spoza obsługiwanego podzbioru: {detail_pl}",
    ),
    (
        "F002",
        E,
        "FEEL syntax error: {detail}",
        "błąd składni FEEL: {detail_pl}",
    ),
    (
        "S001",
        E,
        "invalid distribution '{text}': {detail}",
        "niepoprawny rozkład „{text}”: {detail}",
    ),
    (
        "S002",
        W,
        "probabilities of the flows leaving '{id}' sum to {total}, not 1",
        "prawdopodobieństwa przepływów z „{id}” sumują się do {total}, a nie do 1",
    ),
    (
        "S003",
        W,
        "gateway '{id}' has neither conditions nor probabilities; the simulator takes the first flow",
        "bramka „{id}” nie ma ani warunków, ani prawdopodobieństw; symulator wybierze pierwszy przepływ",
    ),
];

/// English text for the BPMN rules (the validator's own messages are Polish).
pub const BPMN_EN: &[(&str, &str)] = &[
    ("E001", "the file is not well-formed XML"),
    ("E002", "the root element is not bpmn:definitions"),
    ("E003", "an id is used more than once"),
    ("E004", "reference to a non-existent id"),
    (
        "E005",
        "sequence flow without both ends or connecting data, a pool or an annotation",
    ),
    ("E006", "sequence flow between pools or into a sub-process"),
    ("E007", "message flow from an element outside the pools"),
    (
        "E008",
        "message flow inside one pool (use a sequence flow between lanes)",
    ),
    (
        "E009",
        "message flow to a gateway, data or a non-message event",
    ),
    ("E010", "trigger not allowed for this event position"),
    (
        "E011",
        "incoming flow to a start event or outgoing flow from an end event",
    ),
    (
        "E012",
        "invalid boundary event (on a gateway, with incoming flow, non-interrupting error, …)",
    ),
    (
        "E013",
        "event sub-process with sequence flows or without a triggered start event",
    ),
    ("E014", "compensation activity with sequence flows"),
    (
        "E015",
        "default flow that does not leave the element or has a condition",
    ),
    (
        "E016",
        "event-based gateway followed by something other than catch events or receive tasks",
    ),
    ("E017", "condition or default flow on a parallel gateway"),
    ("E018", "reference to a declaration of the wrong type"),
    ("E030", "element in two lanes"),
    ("W010", "timer event without a time"),
    (
        "W016",
        "event after an event-based gateway has other incoming flows",
    ),
    (
        "W020",
        "gateway with one incoming and one outgoing flow is unnecessary",
    ),
    (
        "W021",
        "branch of an XOR/OR split without label, condition or default status",
    ),
    ("W030", "lane refers to an element inside a sub-process"),
    ("W031", "element outside every lane"),
    ("W040", "process without a start or end event"),
    ("W041", "element not reachable from a start event"),
    (
        "W042",
        "path without an end event (element without outgoing flow)",
    ),
    ("W050", "OR split without a default flow"),
    (
        "W051",
        "join of a different type than the split (e.g. AND split joined by XOR)",
    ),
    (
        "W052",
        "boundary escalation/error on a sub-process that nothing inside throws",
    ),
    ("W053", "escalation thrown but never caught"),
    (
        "W054",
        "message task or event without a message flow in a model with several pools",
    ),
    ("L010", "message event without messageRef"),
    ("L020", "name of a splitting gateway is not a question"),
    ("L021", "gateway both joins and splits"),
    ("L030", "task without a name"),
];

pub const LEVELS: &[&str] = &["basic", "analytic", "full"];

fn entry(code: &str) -> Option<&'static (&'static str, Severity, &'static str, &'static str)> {
    MESSAGES.iter().find(|e| e.0 == code)
}

/// One diagnostic. Arguments are stored as text, as Python formats them.
#[derive(Debug, Clone)]
pub struct Diag {
    pub code: String,
    pub line: usize,
    pub id: Option<String>,
    pub args: Vec<(String, String)>,
    pub text_pl: Option<String>,
    pub text_en: Option<String>,
    pub severity_override: Option<Severity>,
}

impl Diag {
    pub fn new(code: &str, line: usize, id: Option<&str>, args: &[(&str, String)]) -> Self {
        Diag {
            code: code.to_string(),
            line,
            id: id.map(str::to_string),
            args: args
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            text_pl: None,
            text_en: None,
            severity_override: None,
        }
    }

    /// A diagnostic with ready texts (executable profile and BPMN rules).
    pub fn text(
        code: &str,
        line: usize,
        id: Option<&str>,
        sev: Severity,
        pl: String,
        en: Option<String>,
    ) -> Self {
        Diag {
            code: code.to_string(),
            line,
            id: id.map(str::to_string),
            args: Vec::new(),
            text_pl: Some(pl),
            text_en: en,
            severity_override: Some(sev),
        }
    }

    pub fn severity(&self) -> Severity {
        if let Some(s) = self.severity_override {
            return s;
        }
        entry(&self.code).map(|e| e.1).unwrap_or(Severity::Error)
    }

    pub fn is_error(&self) -> bool {
        self.severity() == Severity::Error
    }

    pub fn message(&self, lang: &str) -> String {
        if let Some(pl) = &self.text_pl {
            if lang == "en" {
                if let Some(en) = self.text_en.as_deref().filter(|t| !t.is_empty()) {
                    return en.to_string();
                }
                if let Some((_, t)) = BPMN_EN.iter().find(|e| e.0 == self.code) {
                    return match self.id.as_deref() {
                        Some(id) if !id.is_empty() => format!("{t} ({id})"),
                        _ => t.to_string(),
                    };
                }
            }
            return pl.clone();
        }
        let Some(e) = entry(&self.code) else {
            return self.code.clone();
        };
        let template = if lang == "en" { e.2 } else { e.3 };
        format_template(template, &self.args).unwrap_or_else(|| template.to_string())
    }

    pub fn to_json(&self, lang: &str) -> Json {
        obj(vec![
            ("code", Json::Str(self.code.clone())),
            ("severity", Json::Str(self.severity().as_str().into())),
            ("line", Json::Int(self.line as i64)),
            ("id", self.id.clone().map(Json::Str).unwrap_or(Json::Null)),
            ("message", Json::Str(self.message(lang))),
        ])
    }
}

/// Python `str.format(**args)` for `{name}` placeholders; None when a name is missing.
fn format_template(template: &str, args: &[(String, String)]) -> Option<String> {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('}')?;
        let name = &after[..end];
        let value = args.iter().find(|(k, _)| k == name)?;
        out.push_str(&value.1);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// One line of `tok check` output.
pub fn format_diag(d: &Diag, lang: &str, path: Option<&str>) -> String {
    let sev = if lang == "en" {
        d.severity().as_str()
    } else {
        d.severity().polish()
    };
    let place = match path {
        Some(p) => format!("{p}:{}", d.line),
        None if lang == "en" => format!("line {}", d.line),
        None => format!("wiersz {}", d.line),
    };
    format!("{place}: {sev} {}: {}", d.code, d.message(lang))
}

/// Stable sort by (line, code), as the Python CLI does.
pub fn sort(diags: &mut [Diag]) {
    diags.sort_by(|a, b| (a.line, &a.code).cmp(&(b.line, &b.code)));
}
