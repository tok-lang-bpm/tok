//! Semantic checks (`tok check`). Mirrors `check.py`: references and structure (R),
//! executable profile (C), decisions and forms (D, M), FEEL (F), simulation (S), levels,
//! and finally the BPMN 2.0 rules, which run only on documents without errors.

use crate::diagnostics::{Diag, LEVELS, Severity};
use crate::dmn;
use crate::feel;
use crate::model::*;
use crate::simcheck;

/// One finding of the BPMN rule validator (`bpmnlib.validate`).
#[derive(Debug, Clone)]
pub struct BpmnIssue {
    /// `błąd`, `ostrzeżenie` or `uwaga`
    pub level: String,
    pub code: String,
    pub id: Option<String>,
    pub message: String,
}

/// The BPMN 2.0 rule validator, provided by the `tok-bpmn` crate.
pub trait BpmnRules {
    /// Validate BPMN XML text as produced by [`crate::export::export_xml`].
    fn validate(&self, xml: &str) -> Vec<BpmnIssue>;
}

const BASIC_KINDS: &[&str] = &[
    "start", "end", "task", "user", "service", "manual", "xor", "and",
];
const ANALYTIC_EXCLUDED_KEYS: &[&str] =
    &["form", "decision", "expr", "result", "type", "correlation"];

fn kind_name(what: &str) -> (&'static str, &'static str) {
    match what {
        "node" => ("flow node", "element przepływu"),
        "activity" => ("activity", "czynność"),
        "data" => ("data object or store", "obiekt lub magazyn danych"),
        "form" => ("form", "formularz"),
        "decision" => ("decision", "decyzja"),
        "resource" => ("resource", "zasób"),
        _ => ("pool", "basen"),
    }
}

struct Checker<'a> {
    doc: &'a mut Document,
    level: &'static str,
    diags: Vec<Diag>,
}

/// Run all checks. Parser diagnostics are not included.
pub fn check(doc: &mut Document, level: &str, bpmn: Option<&dyn BpmnRules>) -> Vec<Diag> {
    let level = LEVELS
        .iter()
        .find(|l| **l == level)
        .copied()
        .unwrap_or("full");
    let mut c = Checker {
        doc,
        level,
        diags: Vec::new(),
    };
    c.attributes();
    c.structure();
    c.references();
    c.flows();
    if c.doc.executable_any() {
        c.executable();
    }
    c.decisions_and_forms();
    c.feel();
    c.simulation();
    c.levels();
    if let Some(rules) = bpmn
        && !c.diags.iter().any(Diag::is_error)
    {
        c.bpmn_rules(rules);
    }
    c.diags
}

fn s(v: &str) -> String {
    v.to_string()
}

impl Checker<'_> {
    fn add(&mut self, code: &str, line: usize, id: Option<&str>, args: &[(&str, String)]) {
        self.diags.push(Diag::new(code, line, id, args));
    }

    /// Kind of the element named `id`, or a diagnostic (R002, R003) when it is missing or wrong.
    fn reference(
        &mut self,
        id: &str,
        line: usize,
        what: &str,
        ok: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        let (en, pl) = kind_name(what);
        let Some(entry) = self.doc.ids.get(id) else {
            self.add(
                "R002",
                line,
                Some(id),
                &[("what", s(en)), ("what_pl", s(pl)), ("id", s(id))],
            );
            return None;
        };
        let kind = entry.kind.clone();
        if !ok(&kind) {
            self.add(
                "R003",
                line,
                Some(id),
                &[
                    ("id", s(id)),
                    ("actual", kind),
                    ("expected", s(en)),
                    ("expected_pl", s(pl)),
                ],
            );
            return None;
        }
        Some(kind)
    }

    fn attributes(&mut self) {
        let mut found = Vec::new();
        let d = &*self.doc;
        for n in d.nodes.values() {
            let b = &n.base;
            let keys = node_keys(&b.kind);
            let flags = node_flags(&b.kind);
            for k in b.attrs.keys() {
                if !keys.contains(&k.as_str()) {
                    found.push(Diag::new(
                        "R010",
                        b.line,
                        b.id.as_deref(),
                        &[("key", k.clone()), ("kind", b.kind.clone())],
                    ));
                }
            }
            for f in &b.flags {
                if !flags.contains(&f.as_str()) {
                    found.push(Diag::new(
                        "R011",
                        b.line,
                        b.id.as_deref(),
                        &[("flag", f.clone()), ("kind", b.kind.clone())],
                    ));
                }
            }
            if is_event(&b.kind) {
                let mut trig: Vec<&str> = TRIGGERS
                    .iter()
                    .copied()
                    .filter(|k| b.attrs.contains_key(*k))
                    .collect();
                trig.extend(TRIGGER_FLAGS.iter().copied().filter(|f| b.has(f)));
                if trig.len() > 1 {
                    found.push(Diag::new(
                        "R012",
                        b.line,
                        b.id.as_deref(),
                        &[("triggers", trig.join(", "))],
                    ));
                }
            }
        }
        let mut groups: Vec<(Vec<&Base>, &[&str], &[&str])> = vec![
            (
                d.flows.iter().map(|f| &f.base).collect(),
                FLOW_KEYS,
                FLOW_FLAGS,
            ),
            (
                d.messages.iter().map(|m| &m.base).collect(),
                MESSAGE_KEYS,
                &[],
            ),
            (
                d.data.values().map(|x| &x.base).collect(),
                DATA_KEYS,
                DATA_FLAGS,
            ),
            (d.notes.iter().collect(), NOTE_KEYS, &[]),
            (d.groups.iter().collect(), GROUP_KEYS, &[]),
            (
                d.pools.iter().map(|p| &p.base).collect(),
                POOL_KEYS,
                POOL_FLAGS,
            ),
            (d.lanes.values().map(|l| &l.base).collect(), &[], &[]),
            (
                d.decisions.values().map(|x| &x.base).collect(),
                DECISION_KEYS,
                &[],
            ),
            (d.resources.values().collect(), RESOURCE_KEYS, &[]),
        ];
        if let Some(sim) = &d.simulate {
            groups.push((vec![sim], SIMULATE_KEYS, &[]));
        }
        for f in d.forms.values() {
            groups.push((
                f.items
                    .iter()
                    .map(|i| &i.base)
                    .filter(|b| b.kind == "field")
                    .collect(),
                FIELD_KEYS,
                FIELD_FLAGS,
            ));
        }
        for (items, keys, flags) in groups {
            for b in items {
                for k in b.attrs.keys() {
                    if !keys.contains(&k.as_str()) && k != "id" {
                        found.push(Diag::new(
                            "R010",
                            b.line,
                            b.id.as_deref(),
                            &[("key", k.clone()), ("kind", b.kind.clone())],
                        ));
                    }
                }
                for f in &b.flags {
                    if !flags.contains(&f.as_str()) {
                        found.push(Diag::new(
                            "R011",
                            b.line,
                            b.id.as_deref(),
                            &[("flag", f.clone()), ("kind", b.kind.clone())],
                        ));
                    }
                }
            }
        }
        for k in d.attrs.keys() {
            if !HEADER_KEYS.contains(&k.as_str()) {
                found.push(Diag::new(
                    "R010",
                    d.header_line,
                    None,
                    &[("key", k.clone()), ("kind", s("tok"))],
                ));
            }
        }
        for f in &d.flags {
            if !HEADER_FLAGS.contains(&f.as_str()) {
                found.push(Diag::new(
                    "R011",
                    d.header_line,
                    None,
                    &[("flag", f.clone()), ("kind", s("tok"))],
                ));
            }
        }
        self.diags.extend(found);
    }

    fn structure(&mut self) {
        let mut found = Vec::new();
        let d = &*self.doc;
        if !d.pools.is_empty() {
            for i in &d.top {
                if let Some(n) = d.nodes.get(i) {
                    found.push(Diag::new(
                        "R031",
                        n.base.line,
                        Some(i),
                        &[("id", i.clone())],
                    ));
                }
            }
        }
        for p in &d.pools {
            if p.children.iter().any(|c| d.lanes.contains_key(c)) {
                for c in &p.children {
                    if let Some(n) = d.nodes.get(c) {
                        found.push(Diag::new(
                            "R030",
                            n.base.line,
                            Some(c),
                            &[("id", c.clone())],
                        ));
                    }
                }
            }
        }
        self.diags.extend(found);
    }

    fn references(&mut self) {
        let is_act = |k: &str| is_activity(k);
        let node_ids: Vec<String> = self.doc.nodes.keys().cloned().collect();
        for nid in node_ids {
            let n = self.doc.nodes[&nid].clone();
            let a = &n.base.attrs;
            let line = n.base.line;
            if n.base.kind == "boundary" {
                match a.get("on") {
                    None => self.add("R032", line, Some(&nid), &[("id", nid.clone())]),
                    Some(on) => {
                        if self.reference(on, line, "activity", &is_act).is_some() {
                            let scope = self.doc.nodes.get(on).and_then(|x| x.scope.clone());
                            if let Some(node) = self.doc.nodes.get_mut(&nid) {
                                node.scope = scope;
                            }
                        }
                    }
                }
            } else if a.contains_key("on") {
                self.add(
                    "R013",
                    line,
                    Some(&nid),
                    &[("key", s("on")), ("requirement", s("a boundary event"))],
                );
            }
            if let Some(f) = a.get("form") {
                self.reference(f, line, "form", &|k| k == "form");
                if n.base.kind != "user" && n.base.kind != "start" {
                    self.add(
                        "R013",
                        line,
                        Some(&nid),
                        &[
                            ("key", s("form")),
                            ("requirement", s("a user task or a start event")),
                        ],
                    );
                }
            }
            if let Some(dec) = a.get("decision") {
                self.reference(dec, line, "decision", &|k| k == "decision");
                if n.base.kind != "rule" {
                    self.add(
                        "R013",
                        line,
                        Some(&nid),
                        &[("key", s("decision")), ("requirement", s("a rule task"))],
                    );
                }
            }
            if let Some(by) = a.get("by") {
                self.reference(by, line, "resource", &|k| k == "resource");
            }
            if a.contains_key("expr") && n.base.kind != "script" {
                self.add(
                    "R013",
                    line,
                    Some(&nid),
                    &[("key", s("expr")), ("requirement", s("a script task"))],
                );
            }
            if a.contains_key("process") && n.base.kind != "call" {
                self.add(
                    "R013",
                    line,
                    Some(&nid),
                    &[("key", s("process")), ("requirement", s("a call activity"))],
                );
            }
        }
        let assocs: Vec<(String, String, usize)> = self
            .doc
            .assocs
            .iter()
            .map(|x| (x.activity.clone(), x.data.clone(), x.base.line))
            .collect();
        for (act, data, line) in assocs {
            self.reference(&act, line, "activity", &is_act);
            self.reference(&data, line, "data", &|k| k == "data" || k == "store");
        }
        let notes: Vec<(Option<String>, usize)> = self
            .doc
            .notes
            .iter()
            .map(|n| (n.attrs.get("on").cloned(), n.line))
            .collect();
        for (on, line) in notes {
            if let Some(on) = on {
                self.reference(&on, line, "node", &|_| true);
            }
        }
        let groups: Vec<(String, usize)> = self
            .doc
            .groups
            .iter()
            .map(|g| (g.attrs.get("members").cloned().unwrap_or_default(), g.line))
            .collect();
        for (members, line) in groups {
            for m in crate::pyutil::split_ws(&members) {
                self.reference(m, line, "node", &|k| is_node(k));
            }
        }
    }

    fn flows(&mut self) {
        let flows: Vec<(String, String, usize, String)> = self
            .doc
            .flows
            .iter()
            .map(|f| {
                (
                    f.source.clone(),
                    f.target.clone(),
                    f.base.line,
                    f.base.id().to_string(),
                )
            })
            .collect();
        for (src, tgt, line, id) in flows {
            let a = self.reference(&src, line, "node", &|k| is_node(k));
            let b = self.reference(&tgt, line, "node", &|k| is_node(k));
            if a.is_some() && b.is_some() {
                let ss = self.doc.nodes.get(&src).and_then(|n| n.scope.clone());
                let ts = self.doc.nodes.get(&tgt).and_then(|n| n.scope.clone());
                if ss != ts {
                    self.add(
                        "R021",
                        line,
                        Some(&id),
                        &[("source", src.clone()), ("target", tgt.clone())],
                    );
                }
            }
        }
        let msgs: Vec<(String, String, usize, String)> = self
            .doc
            .messages
            .iter()
            .map(|m| {
                (
                    m.source.clone(),
                    m.target.clone(),
                    m.base.line,
                    m.base.id().to_string(),
                )
            })
            .collect();
        let ok = |k: &str| is_node(k) || k == "pool";
        for (src, tgt, line, id) in msgs {
            let a = self.reference(&src, line, "node", &ok);
            let b = self.reference(&tgt, line, "node", &ok);
            if let (Some(ka), Some(kb)) = (a, b) {
                let pool_of = |id: &str, kind: &str| -> Option<String> {
                    if kind == "pool" {
                        Some(id.to_string())
                    } else {
                        self.doc.nodes.get(id).and_then(|n| n.pool.clone())
                    }
                };
                let ps = pool_of(&src, &ka);
                let pt = pool_of(&tgt, &kb);
                if ps.is_none() || pt.is_none() || ps == pt {
                    self.add(
                        "R022",
                        line,
                        Some(&id),
                        &[("source", src.clone()), ("target", tgt.clone())],
                    );
                }
            }
        }
    }

    fn executable(&mut self) {
        let mut found = Vec::new();
        let d = &*self.doc;
        let mut outs: indexmap::IndexMap<&str, Vec<&Flow>> = Default::default();
        for f in &d.flows {
            outs.entry(f.source.as_str()).or_default().push(f);
        }
        for n in d.nodes.values() {
            let a = &n.base.attrs;
            let id = n.base.id();
            let kind = n.base.kind.as_str();
            let line = n.base.line;
            if kind == "user" && !a.contains_key("form") {
                found.push(Diag::text(
                    "C011",
                    line,
                    Some(id),
                    Severity::Note,
                    format!("Zadanie użytkownika „{id}” nie ma formularza (form=)."),
                    Some(format!("user task '{id}' has no form (form=)")),
                ));
            } else if (kind == "service" || kind == "send") && !a.contains_key("type") {
                let pl_kind = if kind == "service" {
                    "Zadanie usługowe"
                } else {
                    "Zadanie wysyłania"
                };
                found.push(Diag::text(
                    "C020",
                    line,
                    Some(id),
                    Severity::Error,
                    format!("{pl_kind} „{id}” wymaga type=\"…\" (typ zadania dla workera)."),
                    Some(format!(
                        "{kind} task '{id}' needs type=\"…\" (job type for a worker)"
                    )),
                ));
            } else if kind == "rule" && !a.contains_key("decision") {
                found.push(Diag::text(
                    "C030",
                    line,
                    Some(id),
                    Severity::Error,
                    format!("Zadanie reguły biznesowej „{id}” wymaga decision=ID."),
                    Some(format!("rule task '{id}' needs decision=ID")),
                ));
            } else if kind == "script" && !a.contains_key("expr") {
                found.push(Diag::text(
                    "C040",
                    line,
                    Some(id),
                    Severity::Error,
                    format!("Zadanie skryptowe „{id}” wymaga expr=\"…\"."),
                    Some(format!("script task '{id}' needs expr=\"…\"")),
                ));
            }
            if (((kind == "catch" || kind == "boundary") && a.contains_key("message"))
                || kind == "receive")
                && !a.contains_key("correlation")
            {
                found.push(Diag::text(
                    "C052",
                    line,
                    Some(id),
                    Severity::Error,
                    format!("„{id}” czeka na wiadomość i wymaga correlation=\"…\"."),
                    Some(format!(
                        "'{id}' waits for a message and needs correlation=\"…\""
                    )),
                ));
            }
            if kind == "xor" || kind == "or" {
                let fl = outs.get(id).cloned().unwrap_or_default();
                if fl.len() > 1 {
                    for f in fl {
                        if !f.base.attrs.contains_key("when") && !f.base.has("default") {
                            found.push(Diag::text(
                                "C070",
                                f.base.line,
                                f.base.id.as_deref(),
                                Severity::Error,
                                format!(
                                    "Przepływ {} -> {} z bramki „{id}” wymaga when=\"…\" albo flagi default.",
                                    f.source, f.target
                                ),
                                Some(format!(
                                    "flow {} -> {} leaving gateway '{id}' needs when=\"…\" or the default flag",
                                    f.source, f.target
                                )),
                            ));
                        }
                    }
                }
            }
        }
        self.diags.extend(found);
    }

    fn decisions_and_forms(&mut self) {
        let mut found = Vec::new();
        let d = &*self.doc;
        for dec in d.decisions.values() {
            let id = dec.base.id();
            let line = dec.base.line;
            if !HIT_POLICIES.contains(&dec.hit()) {
                found.push(Diag::new(
                    "D003",
                    line,
                    Some(id),
                    &[("hit", dec.hit().to_string())],
                ));
            }
            if dec.inputs.is_empty() && dec.outputs.is_empty() {
                found.push(Diag::new("D001", line, Some(id), &[("id", s(id))]));
            } else if dec.rules.is_empty() {
                found.push(Diag::new("D005", line, Some(id), &[("id", s(id))]));
            }
            for (code, l, args) in dmn::check_decision(dec) {
                if !["D001", "D003", "D005"].contains(&code) {
                    found.push(Diag::new(code, l, Some(id), &args));
                }
            }
        }
        for f in d.forms.values() {
            for it in &f.items {
                if it.base.kind != "field" {
                    continue;
                }
                let t = it
                    .base
                    .attrs
                    .get("type")
                    .cloned()
                    .unwrap_or_else(|| "textfield".into());
                if !FIELD_TYPES.contains(&t.as_str()) {
                    found.push(Diag::new(
                        "M001",
                        it.base.line,
                        it.base.id.as_deref(),
                        &[("type", t)],
                    ));
                } else if (t == "select" || t == "radio") && !it.base.attrs.contains_key("options")
                {
                    found.push(Diag::new(
                        "M002",
                        it.base.line,
                        it.base.id.as_deref(),
                        &[("id", it.base.id().to_string()), ("type", t)],
                    ));
                }
            }
        }
        self.diags.extend(found);
    }

    fn feel(&mut self) {
        let mut exprs: Vec<(String, usize, Option<String>)> = Vec::new();
        for n in self.doc.nodes.values() {
            for k in ["expr", "condition", "correlation"] {
                if let Some(t) = n.base.attrs.get(k) {
                    exprs.push((t.clone(), n.base.line, n.base.id.clone()));
                }
            }
        }
        for f in &self.doc.flows {
            if let Some(t) = f.base.attrs.get("when") {
                exprs.push((t.clone(), f.base.line, f.base.id.clone()));
            }
        }
        for (text, line, id) in exprs {
            if let Some((code, detail)) = feel::check_expr(&text) {
                let pl = feel::detail_pl(&detail);
                self.add(
                    code,
                    line,
                    id.as_deref(),
                    &[("detail", detail), ("detail_pl", pl)],
                );
            }
        }
    }

    fn simulation(&mut self) {
        for (code, line, id, mut args) in simcheck::check_sim(self.doc) {
            if code == "R002" {
                if self
                    .diags
                    .iter()
                    .any(|x| x.code == "R002" && x.line == line)
                {
                    continue;
                }
                let target = args
                    .iter()
                    .find(|a| a.0 == "id")
                    .map(|a| a.1.clone())
                    .unwrap_or_default();
                args = vec![
                    ("what", s("resource")),
                    ("id", target),
                    ("what_pl", s("zasób")),
                ];
            }
            self.add(code, line, id.as_deref(), &args);
        }
    }

    fn levels(&mut self) {
        if self.level == "full" {
            return;
        }
        let level = self.level;
        let mut found = Vec::new();
        let d = &*self.doc;
        for n in d.nodes.values() {
            let b = &n.base;
            let bad = if level == "basic" {
                !BASIC_KINDS.contains(&b.kind.as_str())
                    || TRIGGERS.iter().any(|k| b.attrs.contains_key(*k))
                    || TRIGGER_FLAGS
                        .iter()
                        .chain(TASK_FLAGS.iter())
                        .any(|f| b.has(f))
            } else {
                ANALYTIC_EXCLUDED_KEYS
                    .iter()
                    .any(|k| b.attrs.contains_key(*k))
            };
            if bad {
                found.push(Diag::new(
                    "R040",
                    b.line,
                    b.id.as_deref(),
                    &[("keyword", b.kind.clone()), ("level", s(level))],
                ));
            }
        }
        if level == "basic"
            && (!d.decisions.is_empty() || !d.forms.is_empty() || d.simulate.is_some())
        {
            let el = d
                .decisions
                .values()
                .map(|x| &x.base)
                .chain(d.forms.values().map(|f| &f.base))
                .chain(d.simulate.iter())
                .next();
            let (line, kw) = el.map(|e| (e.line, e.kind.clone())).unwrap_or((0, s("?")));
            found.push(Diag::new(
                "R040",
                line,
                None,
                &[("keyword", kw), ("level", s(level))],
            ));
        }
        if level == "analytic"
            && (!d.decisions.is_empty() || !d.forms.is_empty() || d.executable_any())
        {
            let el = d
                .decisions
                .values()
                .map(|x| &x.base)
                .chain(d.forms.values().map(|f| &f.base))
                .next();
            let (line, kw) = el
                .map(|e| (e.line, e.kind.clone()))
                .unwrap_or((d.header_line, s("executable")));
            found.push(Diag::new(
                "R040",
                line,
                None,
                &[("keyword", kw), ("level", s(level))],
            ));
        }
        self.diags.extend(found);
    }

    fn bpmn_rules(&mut self, rules: &dyn BpmnRules) {
        let xml = crate::export::export_xml(self.doc, None);
        for issue in rules.validate(&xml) {
            if issue.code.starts_with("D0") || issue.code.starts_with('K') {
                continue;
            }
            let sev = match issue.level.as_str() {
                "ostrzeżenie" => Severity::Warning,
                "uwaga" => Severity::Note,
                _ => Severity::Error,
            };
            let line = issue
                .id
                .as_ref()
                .and_then(|i| self.doc.ids.get(i))
                .map(|e| e.line)
                .unwrap_or(0);
            self.diags.push(Diag::text(
                &issue.code,
                line,
                issue.id.as_deref(),
                sev,
                issue.message.clone(),
                None,
            ));
        }
    }
}
