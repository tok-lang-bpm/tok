//! Where the identifiers of a document stand: the token that declares each one and every
//! token that refers to it, found from the tokens of the lexer and the identifiers the
//! parser registered (spec §2: identifiers are unique in a document).
//!
//! An identifier is referred to by the node names of `flow` and `message` lines, by the
//! activity and data of `reads` and `writes`, and by the values of the attributes that
//! name elements (`on=`, `form=`, `requires="a b"`, …). In a document of language 0.7 the
//! FEEL expressions of decisions and business knowledge models (`expr=`, the right-hand
//! sides of `with=`, `default=` and the cells of their tables) also refer to the elements
//! of the decision requirements graph by their FEEL name, which is the identifier unless
//! `name=` gives another one; those names are found outside FEEL string literals, and only
//! the elements a decision or business knowledge model requires are visible in it (spec
//! §9.1), so a parameter with the name of an element is not a reference. FEEL resolves the
//! longest name of the document, so the first word of a longer name (`kara` in the entry
//! `kara za tydzień`) is not a reference either, nor is the key of a context (`{kara: …}`).
//! The types of the columns of a table name DMN types (`| konto: konto-t || … |`).

use tok_core::Document;
use tok_core::feel3::Names;
use tok_core::lexer::{Kind, Line};

use crate::text::Text;

/// One token that declares (`def`) or refers to identifier `id`: columns `start..end` of
/// line `line` (0-based), in characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub id: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub def: bool,
}

/// Attribute keys whose values name elements of the document; a value may name several,
/// separated by spaces (`pools=`, `requires=`) or as `inner:outer` pairs (`map=`).
pub const REF_KEYS: &[&str] = &[
    "on",
    "process",
    "task",
    "form",
    "decision",
    "by",
    "operation",
    "type",
    "compensate",
    "activity",
    "from",
    "to",
    "choreography",
    "collaboration",
    "input",
    "output",
    "members",
    "pools",
    "messages",
    "requires",
    "invoke",
    "map",
];

/// Statements of language 0.7 whose expressions are FEEL over the decision requirements
/// graph.
const DRG_STATEMENTS: &[&str] = &["decision", "bkm", "entry", "result", "element"];

/// Attributes of those statements that hold FEEL.
const FEEL_KEYS: &[&str] = &["expr", "with", "default"];

fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn id_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// A character of a word of a FEEL name with spaces (`kara za tydzień`).
fn name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '\''
}

struct Finder<'a> {
    doc: &'a Document,
    text: &'a Text,
    out: Vec<Occurrence>,
    /// The elements the current decision or business knowledge model requires.
    visible: Vec<String>,
    /// The FEEL names of the document (language 0.7 and 1.0), for names of several words.
    feel_names: Option<Names>,
}

impl Finder<'_> {
    fn known(&self, id: &str) -> bool {
        self.doc.ids.contains_key(id)
    }

    /// An element of the decision requirements graph whose FEEL name is its identifier.
    fn feel_named(&self, id: &str) -> bool {
        if !self.visible.iter().any(|v| v == id) {
            return false;
        }
        let Some(entry) = self.doc.ids.get(id) else {
            return false;
        };
        let base = match entry.kind.as_str() {
            "decision" => self.doc.decisions.get(id).map(|d| &d.base),
            "bkm" => self.doc.bkms.get(id).map(|d| &d.base),
            "inputdata" => self.doc.inputdata.get(id),
            _ => None,
        };
        base.is_some_and(|b| b.attr("name").is_none())
    }

    /// The end of the longest FEEL name of two or more words that starts at column `j`
    /// (`kara za tydzień`, an entry or a parameter): FEEL resolves the longest name of the
    /// document, so the words inside it do not refer to elements.
    fn long_name(&self, chars: &[char], j: usize, end: usize) -> Option<usize> {
        let names = self.feel_names.as_ref()?;
        let mut ends = Vec::new();
        let mut p = j;
        while ends.len() < 8 {
            let mut q = p;
            while q < end && name_char(chars[q]) {
                q += 1;
            }
            if q == p {
                break;
            }
            ends.push(q);
            if q + 1 < end && chars[q] == ' ' && name_char(chars[q + 1]) {
                p = q + 1;
            } else {
                break;
            }
        }
        ends.iter().skip(1).rev().copied().find(|&e| {
            let name: String = chars[j..e].iter().collect();
            names.contains(&name)
        })
    }

    fn push(&mut self, id: &str, line: usize, start: usize, end: usize, def: bool) {
        self.out.push(Occurrence {
            id: id.to_string(),
            line,
            start,
            end,
            def,
        });
    }

    /// The names of elements in the source columns `start..end` of a line: runs of
    /// identifier characters that are identifiers of the document (`only` restricts the
    /// kinds of element).
    fn names(&mut self, line: usize, start: usize, end: usize, only: Option<&str>) {
        let chars = self.text.chars(line);
        let end = end.min(chars.len());
        let mut j = start;
        while j < end {
            if id_char(chars[j]) && (j == start || !id_char(chars[j - 1])) {
                let mut k = j;
                while k < end && id_char(chars[k]) {
                    k += 1;
                }
                let word: String = chars[j..k].iter().collect();
                let kind_ok = |f: &Finder, w: &str| match only {
                    Some(kind) => f.doc.ids.get(w).is_some_and(|e| e.kind == kind),
                    None => f.known(w),
                };
                if kind_ok(self, &word) {
                    self.push(&word, line, j, k, false);
                }
                j = k;
            } else {
                j += 1;
            }
        }
    }

    /// The FEEL names of graph elements in columns `start..end` of a line. `escaped`: the
    /// text is the inside of a tok string, where FEEL string literals are written `\"…\"`;
    /// otherwise (a table row) they are written `"…"`. `bindings`: the text is a list of
    /// `name = FEEL` pairs separated by `;`, and only the right-hand sides are FEEL.
    fn feel(&mut self, line: usize, start: usize, end: usize, escaped: bool, bindings: bool) {
        let chars = self.text.chars(line);
        let end = end.min(chars.len());
        let mut in_string = false;
        let mut lhs = bindings;
        let mut j = start;
        while j < end {
            let c = chars[j];
            if escaped && c == '\\' {
                if chars.get(j + 1) == Some(&'"') {
                    in_string = !in_string;
                }
                j += 2;
                continue;
            }
            if !escaped && c == '"' {
                in_string = !in_string;
                j += 1;
                continue;
            }
            if !escaped && in_string && c == '\\' {
                j += 2;
                continue;
            }
            if in_string {
                j += 1;
                continue;
            }
            if bindings && c == ';' {
                lhs = true;
            } else if bindings && c == '=' {
                lhs = false;
            }
            let path = chars[start..j]
                .iter()
                .rev()
                .find(|c| !c.is_whitespace())
                .is_some_and(|&c| c == '.');
            if !lhs
                && !path
                && name_char(c)
                && (j == start || !name_char(chars[j - 1]))
                && let Some(k) = self.long_name(chars, j, end)
            {
                j = k;
                continue;
            }
            if !lhs && id_start(c) && (j == start || !id_char(chars[j - 1])) && !path {
                let mut k = j;
                while k < end && id_char(chars[k]) {
                    k += 1;
                }
                let word: String = chars[j..k].iter().collect();
                // a key of a context (`{limit: …}`) or a parameter (`function(x: number)`)
                let key = chars[k..end]
                    .iter()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|&c| c == ':');
                if key {
                    j = k;
                    continue;
                }
                if self.feel_named(&word) {
                    self.push(&word, line, j, k, false);
                } else if word.contains('-') {
                    // `a-b` is FEEL subtraction unless `a-b` itself is a name
                    let mut p = j;
                    for part in word.split('-') {
                        let n = part.chars().count();
                        if self.feel_named(part) {
                            self.push(part, line, p, p + n, false);
                        }
                        p += n + 1;
                    }
                }
                j = k;
                continue;
            }
            j += 1;
        }
    }

    /// A row of a decision table or a relation: the input expressions of the header
    /// (before `:` in each cell, before the first `||`), and the cells of a rule up to its
    /// annotation. The column names of a relation are not references.
    fn row(&mut self, line: usize, start: usize, end: usize, header: bool) {
        let chars = self.text.chars(line).to_vec();
        let end = end.min(chars.len());
        // split at `|` and `||` outside strings
        let mut cells: Vec<(usize, usize)> = Vec::new();
        let mut bars: Vec<usize> = Vec::new();
        let mut in_string = false;
        let mut cell_start = start;
        let mut j = start;
        while j < end {
            let c = chars[j];
            if in_string && c == '\\' {
                j += 2;
                continue;
            }
            if c == '"' {
                in_string = !in_string;
            } else if c == '|' && !in_string {
                let double = chars.get(j + 1) == Some(&'|');
                if j > cell_start {
                    cells.push((cell_start, j));
                }
                if double {
                    bars.push(cells.len());
                    j += 1;
                }
                cell_start = j + 1;
            }
            j += 1;
        }
        let first_double = bars.first().copied();
        let second_double = bars.get(1).copied();
        for (n, &(a, b)) in cells.iter().enumerate() {
            if header {
                if second_double.is_some_and(|s| n >= s) {
                    break;
                }
                let colon = (a..b).find(|&k| chars[k] == ':').unwrap_or(b);
                if first_double.is_some_and(|outputs| n < outputs) {
                    self.feel(line, a, colon, false, false);
                }
                // the type of a column may be a DMN type of the document (`konto: konto-t`),
                // before the values of the column (`= "a", "b"`)
                if colon < b {
                    let eq = (colon..b).find(|&k| chars[k] == '=').unwrap_or(b);
                    self.names(line, colon + 1, eq, Some("item"));
                }
            } else {
                if second_double.is_some_and(|s| n >= s) {
                    break;
                }
                self.feel(line, a, b, false, false);
            }
        }
    }

    fn statement(&mut self, line: &Line, header_row: bool, in_drg: bool) {
        let n = line.number.saturating_sub(1);
        let toks = &line.tokens;
        let spans = &line.spans;
        if toks.len() != spans.len() || toks.is_empty() {
            return;
        }
        if toks[0].kind == Kind::Row {
            if in_drg {
                self.row(n, spans[0].0, spans[0].1, header_row);
            }
            return;
        }
        if toks[0].kind != Kind::Word {
            return;
        }
        let kw = toks[0].value.as_str();
        if let Some(t) = toks.get(1)
            && t.kind == Kind::Word
            && self
                .doc
                .ids
                .get(&t.value)
                .is_some_and(|e| e.line == line.number)
            && !matches!(kw, "flow" | "message" | "reads" | "writes")
        {
            self.push(&t.value, n, spans[1].0, spans[1].1, true);
        }
        match kw {
            "flow" | "message" => {
                let mut i = 1;
                while i < toks.len() && toks[i].kind == Kind::Word {
                    if i + 1 < toks.len() && toks[i + 1].kind == Kind::Equals {
                        break;
                    }
                    if self.known(&toks[i].value) {
                        self.push(&toks[i].value, n, spans[i].0, spans[i].1, false);
                    }
                    if i + 1 < toks.len() && toks[i + 1].kind == Kind::Arrow {
                        i += 2;
                    } else {
                        break;
                    }
                }
            }
            "reads" | "writes" => {
                for i in 1..toks.len().min(3) {
                    if toks[i].kind == Kind::Word && self.known(&toks[i].value) {
                        self.push(&toks[i].value, n, spans[i].0, spans[i].1, false);
                    }
                }
            }
            _ => {}
        }
        let drg = in_drg && DRG_STATEMENTS.contains(&kw);
        let mut i = 1;
        while i + 1 < toks.len() {
            let is_attr = toks[i].kind == Kind::Word
                && toks[i + 1].kind == Kind::Equals
                && i + 2 < toks.len()
                && matches!(toks[i + 2].kind, Kind::Word | Kind::Str | Kind::Number);
            if !is_attr {
                i += 1;
                continue;
            }
            let key = toks[i].value.as_str();
            let value = &toks[i + 2];
            let (a, b) = spans[i + 2];
            let (a, b) = if value.kind == Kind::Str {
                (a + 1, b.saturating_sub(1))
            } else {
                (a, b)
            };
            if key == "id" {
                if value.kind == Kind::Word
                    && self
                        .doc
                        .ids
                        .get(&value.value)
                        .is_some_and(|e| e.line == line.number)
                {
                    self.push(&value.value, n, a, b, true);
                }
            } else if REF_KEYS.contains(&key) && value.kind != Kind::Number {
                // `type=` names a DMN type or an item definition, or else a FEEL type
                let only = if key == "type" { Some("item") } else { None };
                self.names(n, a, b, only);
            } else if drg && FEEL_KEYS.contains(&key) && value.kind == Kind::Str {
                self.feel(n, a, b, true, key == "with");
            }
            i += 3;
        }
    }
}

/// Every declaration of and reference to an identifier, in document order.
pub fn occurrences(doc: &Document, lines: &[Line], text: &Text) -> Vec<Occurrence> {
    let mut f = Finder {
        doc,
        text,
        out: Vec::new(),
        visible: Vec::new(),
        feel_names: doc.lang_07().then(|| tok_core::drg::names(doc)),
    };
    let lang07 = doc.lang_07();
    let mut prev_row = false;
    // the indentation of the statement whose block holds decision logic, while inside it
    let mut drg_indent: Option<usize> = None;
    for line in lines.iter().filter(|l| !l.standalone) {
        let row = line.tokens.first().is_some_and(|t| t.kind == Kind::Row);
        if let Some(indent) = drg_indent
            && line.indent <= indent
            && !row
        {
            drg_indent = None;
        }
        if !row
            && let Some(t) = line.tokens.first()
            && (t.value == "decision" || t.value == "bkm")
        {
            drg_indent = Some(line.indent);
            let id = line.tokens.get(1).map(|t| t.value.as_str()).unwrap_or("");
            let base = if t.value == "bkm" {
                doc.bkms.get(id)
            } else {
                doc.decisions.get(id)
            };
            f.visible = base
                .and_then(|d| d.base.attr("requires"))
                .map(|r| r.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default();
        }
        let in_drg = lang07 && (drg_indent.is_some() || row);
        f.statement(line, row && !prev_row, in_drg);
        prev_row = row;
    }
    f.out
}

/// The occurrence at a position (column in characters); the end of a token counts, so
/// that the cursor just after an identifier finds it.
pub fn at(occs: &[Occurrence], line: usize, col: usize) -> Option<&Occurrence> {
    occs.iter()
        .find(|o| o.line == line && o.start <= col && col <= o.end)
}

/// The kind and label of an element, as the parser recorded them.
pub fn describe(doc: &Document, id: &str) -> Option<(String, Option<String>)> {
    let entry = doc.ids.get(id)?;
    let label = if let Some(n) = doc.nodes.get(id) {
        n.base.label.clone()
    } else if let Some(p) = doc.pools.iter().find(|p| p.base.id() == id) {
        p.base.label.clone()
    } else if let Some(l) = doc.lanes.get(id) {
        l.base.label.clone()
    } else if let Some(d) = doc.data.get(id) {
        d.base.label.clone()
    } else if let Some(d) = doc.decisions.get(id).or_else(|| doc.bkms.get(id)) {
        d.base.label.clone()
    } else if let Some(f) = doc.forms.get(id) {
        f.base.label.clone()
    } else if let Some(c) = doc.conversations.get(id) {
        c.base.label.clone()
    } else if let Some(c) = doc.choreographies.get(id) {
        c.base.label.clone()
    } else if let Some(i) = doc.interfaces.get(id) {
        i.base.label.clone()
    } else if let Some(k) = doc.keys.get(id) {
        k.base.label.clone()
    } else if let Some(f) = doc.flows.iter().find(|f| f.base.id() == id) {
        f.base
            .label
            .clone()
            .or_else(|| Some(format!("{} -> {}", f.source, f.target)))
    } else if let Some(m) = doc.messages.iter().find(|m| m.base.id() == id) {
        m.base
            .label
            .clone()
            .or_else(|| Some(format!("{} -> {}", m.source, m.target)))
    } else {
        let bases = [
            doc.resources.get(id),
            doc.items.get(id),
            doc.globals.get(id),
            doc.inputdata.get(id),
            doc.sources.get(id),
        ];
        bases
            .into_iter()
            .flatten()
            .next()
            .map(|b| b.label.clone())
            .or_else(|| {
                doc.interfaces
                    .values()
                    .flat_map(|i| i.operations.iter())
                    .find(|o| o.id() == id)
                    .map(|o| o.label.clone())
            })
            .flatten()
    };
    Some((entry.kind.clone(), label))
}
