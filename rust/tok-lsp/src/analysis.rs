//! One analysis of a document: the parsed and checked model, its diagnostics, the tokens
//! with their columns, the identifiers and the outline.

use serde_json::{Value, json};
use tok_core::check::{BpmnIssue, BpmnRules};
use tok_core::diagnostics::{Diag, Severity};
use tok_core::lexer::{self, Kind, Line};
use tok_core::{Document, classes, diagnostics};

use crate::index::{self, Occurrence};
use crate::keywords;
use crate::text::Text;

/// Settings of the checker: the language of the messages, the teaching level and the
/// BPMN conformance class (spec §13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub lang: String,
    pub level: String,
    pub class: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: "en".into(),
            level: "full".into(),
            class: None,
        }
    }
}

/// BPMN rules from the `tok-bpmn` crate, as the command line checks them.
struct Rules;

impl BpmnRules for Rules {
    fn validate(&self, xml: &str) -> Vec<BpmnIssue> {
        tok_bpmn::validate(xml)
            .unwrap_or_default()
            .into_iter()
            .map(|i| BpmnIssue {
                level: match i.level {
                    tok_bpmn::Level::Error => "błąd",
                    tok_bpmn::Level::Warning => "ostrzeżenie",
                    tok_bpmn::Level::Note => "uwaga",
                }
                .to_string(),
                code: i.code,
                id: i.id,
                message: i.message,
            })
            .collect()
    }
}

/// Universal newlines, as the command line reads files.
pub fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// A statement of the outline, with the statements of its block.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub detail: String,
    pub kind: u32,
    /// First and last line of the statement with its block (0-based).
    pub first: usize,
    pub last: usize,
    /// Line and columns of the name.
    pub name_span: (usize, usize),
    pub children: Vec<Symbol>,
}

pub struct Analysis {
    pub text: Text,
    pub doc: Document,
    pub diags: Vec<Diag>,
    pub lines: Vec<Line>,
    pub occurrences: Vec<Occurrence>,
}

impl Analysis {
    pub fn new(source: &str, settings: &Settings) -> Self {
        let normalized = normalize(source);
        let rules = Rules;
        let (doc, mut diags) = tok_core::load(&normalized, &settings.level, Some(&rules));
        if let Some(c) = &settings.class
            && !diags.iter().any(Diag::is_error)
        {
            diags.extend(classes::check_class(&doc, c));
            diagnostics::sort(&mut diags);
        }
        let (lines, _) = lexer::tokenize(&normalized);
        let text = Text::new(source);
        let occurrences = index::occurrences(&doc, &lines, &text);
        Analysis {
            text,
            doc,
            diags,
            lines,
            occurrences,
        }
    }

    /// The lexer line of document line `line` (0-based), when it holds a statement.
    pub fn line(&self, line: usize) -> Option<&Line> {
        self.lines
            .iter()
            .find(|l| l.number == line + 1 && !l.standalone)
    }

    /// The range a diagnostic points at: the token of its element on its line, else the
    /// statement of the line.
    pub fn diag_range(&self, d: &Diag) -> Value {
        let line = d
            .line
            .saturating_sub(1)
            .min(self.text.len().saturating_sub(1));
        if let (Some(id), Some(l)) = (d.id.as_deref(), self.line(line))
            && let Some(i) = l
                .tokens
                .iter()
                .position(|t| t.kind == Kind::Word && t.value == id)
            && let Some(&(a, b)) = l.spans.get(i)
        {
            return self.text.range(line, a, b);
        }
        self.text.line_range(line)
    }

    /// The diagnostics in the form of `textDocument/publishDiagnostics`.
    pub fn lsp_diagnostics(&self, lang: &str) -> Vec<Value> {
        self.diags
            .iter()
            .map(|d| {
                let severity = match d.severity() {
                    Severity::Error => 1,
                    Severity::Warning => 2,
                    Severity::Note => 3,
                };
                json!({
                    "range": self.diag_range(d),
                    "severity": severity,
                    "code": d.code,
                    "source": "tok",
                    "message": d.message(lang),
                })
            })
            .collect()
    }

    /// The outline: statements with an identifier or a name, nested as their blocks.
    pub fn symbols(&self) -> Vec<Symbol> {
        // (indentation, symbol) of the open blocks
        let mut stack: Vec<(usize, Symbol)> = Vec::new();
        let mut roots: Vec<Symbol> = Vec::new();
        fn close(stack: &mut Vec<(usize, Symbol)>, roots: &mut Vec<Symbol>, indent: usize) {
            while stack.last().is_some_and(|(i, _)| *i >= indent) {
                if let Some((_, sym)) = stack.pop() {
                    match stack.last_mut() {
                        Some((_, parent)) => parent.children.push(sym),
                        None => roots.push(sym),
                    }
                }
            }
        }
        for l in self.lines.iter().filter(|l| !l.standalone) {
            let n = l.number - 1;
            close(&mut stack, &mut roots, l.indent);
            for (_, open) in stack.iter_mut() {
                open.last = n;
            }
            let Some(first) = l.tokens.first() else {
                continue;
            };
            if first.kind != Kind::Word {
                continue;
            }
            let Some(kind) = keywords::symbol_kind(&first.value) else {
                continue;
            };
            let Some((name, span)) = l
                .tokens
                .get(1)
                .filter(|t| matches!(t.kind, Kind::Word | Kind::Str))
                .zip(l.spans.get(1))
                .map(|(t, s)| (t.value.clone(), *s))
                .or_else(|| (first.value == "result").then(|| ("result".to_string(), l.spans[0])))
            else {
                continue;
            };
            let label = l
                .tokens
                .get(2)
                .filter(|t| t.kind == Kind::Str)
                .map(|t| format!(" \"{}\"", t.value))
                .unwrap_or_default();
            let sym = Symbol {
                name,
                detail: format!("{}{label}", first.value),
                kind,
                first: n,
                last: n,
                name_span: span,
                children: Vec::new(),
            };
            stack.push((l.indent, sym));
        }
        close(&mut stack, &mut roots, 0);
        roots
    }

    /// A symbol as an LSP `DocumentSymbol`.
    pub fn symbol_json(&self, s: &Symbol) -> Value {
        json!({
            "name": s.name,
            "detail": s.detail,
            "kind": s.kind,
            "range": {
                "start": {"line": s.first, "character": 0},
                "end": {"line": s.last, "character": self.text.end(s.last)},
            },
            "selectionRange": self.text.range(s.first, s.name_span.0, s.name_span.1),
            "children": s.children.iter().map(|c| self.symbol_json(c)).collect::<Vec<_>>(),
        })
    }

    /// The range of an occurrence.
    pub fn occ_range(&self, o: &Occurrence) -> Value {
        self.text.range(o.line, o.start, o.end)
    }

    /// The occurrences of one identifier.
    pub fn of(&self, id: &str) -> impl Iterator<Item = &Occurrence> {
        self.occurrences.iter().filter(move |o| o.id == id)
    }
}
