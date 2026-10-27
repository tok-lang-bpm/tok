//! Semantic content of a BPMN 2.0 XML document as a set of facts, and the MIWG round trip
//! that compares the facts of a reference model with those of its export after
//! `tok import` → tok text → `tok bpmn` (`tests/miwg.rs`, `examples/miwg.rs`).
//!
//! A fact is one statement about the model in a canonical text form, for example
//! `Task_1 name="Check invoice"` or `Flow_2 flow Gateway_1 -> Task_3`. Identifiers of flow
//! elements, artifacts, lanes, pools, processes and declarations that tok keeps are part of
//! the facts; the identifiers tok generates (data objects behind their references, event
//! definitions, input and output specifications and sets, data associations, associations,
//! categories, messages, signals, errors and escalations, which tok names) are not, and
//! those elements are compared by their content. Diagram interchange is not compared.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use roxmltree::Node;

pub const BPMN_NS: &str = "http://www.omg.org/spec/BPMN/20100524/MODEL";
pub const ZEEBE_NS: &str = "http://camunda.org/schema/zeebe/1.0";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
const XPATH: &str = "http://www.w3.org/1999/XPath";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

/// Fact categories in report order, with a short description.
pub const CATEGORIES: &[(&str, &str)] = &[
    (
        "structure",
        "definitions, collaborations, participants, processes, lanes and lane membership",
    ),
    (
        "nodes",
        "flow nodes by identifier and kind, names, containers",
    ),
    (
        "events",
        "event definitions with their references, timers and conditions; interrupting, parallel multiple, attachment",
    ),
    (
        "activities",
        "loop and multi-instance characteristics, compensation, called elements, scripts, messages, operations, performers, implementation and quantities",
    ),
    (
        "gateways",
        "gateway direction (stated or implied by the flows), event-based and complex gateway attributes",
    ),
    (
        "flows",
        "sequence flows (source, target, name, condition, default), message flows",
    ),
    (
        "data",
        "data objects, data stores, references, states, inputs, outputs, properties, data associations",
    ),
    (
        "declarations",
        "item definitions, messages, signals, errors, escalations, resources, interfaces, global tasks, correlation, imports",
    ),
    ("documentation", "documentation text of any element"),
    (
        "artifacts",
        "text annotations, associations, groups and categories",
    ),
    ("camunda", "Camunda 8 extension elements (zeebe:)"),
    (
        "vendor",
        "extension elements and attributes of other tools (not kept by design)",
    ),
];

/// Categories that tok does not keep by design: their facts are listed, never counted as
/// a loss of the round trip.
pub const BY_DESIGN: &[&str] = &["vendor"];

/// Facts of the export that tok adds by design: `zeebe:userTask` marks every user task of
/// an executable process as a Camunda user task.
pub fn added_by_design(f: &Fact) -> bool {
    f.cat == "camunda" && f.text.ends_with(" zeebe:userTask ")
}

/// A document without a name is titled after its process, and the title is the name of
/// the exported definitions.
fn titled_by_design(f: &Fact, before: &Facts) -> bool {
    f.text.starts_with("definitions name=")
        && !before
            .keys()
            .any(|b| b.text.starts_with("definitions name="))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fact {
    pub cat: &'static str,
    pub text: String,
}

/// Facts of one document, each with the XML identifier of the element it belongs to.
pub type Facts = BTreeMap<Fact, String>;

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text block: line ends normalized, trailing spaces of lines and surrounding blank lines removed.
fn block(s: &str) -> String {
    let t = s.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = t.lines().map(str::trim_end).collect();
    lines.join("\n").trim().to_string()
}

/// An expression: surrounding space and a leading `=` (Camunda's FEEL marker) removed.
fn expr(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix('=').unwrap_or(t);
    block(t)
}

fn q(s: &str) -> String {
    format!("{s:?}")
}

fn local<'a>(e: Node<'a, '_>) -> &'a str {
    e.tag_name().name()
}

fn is_bpmn(e: Node) -> bool {
    e.is_element() && e.tag_name().namespace() == Some(BPMN_NS)
}

fn is_b(e: Node, name: &str) -> bool {
    is_bpmn(e) && local(e) == name
}

fn kids<'a, 'i>(e: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    e.children().filter(|c| c.is_element())
}

fn find<'a, 'i>(e: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    kids(e).find(|c| is_b(*c, name))
}

fn findall<'a, 'i>(e: Node<'a, 'i>, name: &'static str) -> Vec<Node<'a, 'i>> {
    kids(e).filter(|c| is_b(*c, name)).collect()
}

fn text<'a>(e: Node<'a, '_>) -> &'a str {
    e.text().unwrap_or("")
}

fn clark(e: Node) -> String {
    match e.tag_name().namespace() {
        Some(ns) => format!("{{{ns}}}{}", local(e)),
        None => local(e).to_string(),
    }
}

fn attr_nonempty<'a>(e: Node<'a, '_>, name: &str) -> Option<&'a str> {
    e.attribute(name).filter(|v| !v.is_empty())
}

const EVENTS: &[&str] = &[
    "startEvent",
    "endEvent",
    "intermediateCatchEvent",
    "intermediateThrowEvent",
    "boundaryEvent",
    "implicitThrowEvent",
];
const ACTIVITIES: &[&str] = &[
    "task",
    "userTask",
    "serviceTask",
    "manualTask",
    "scriptTask",
    "businessRuleTask",
    "sendTask",
    "receiveTask",
    "callActivity",
    "subProcess",
    "adHocSubProcess",
    "transaction",
];
const GATEWAYS: &[&str] = &[
    "exclusiveGateway",
    "parallelGateway",
    "inclusiveGateway",
    "eventBasedGateway",
    "complexGateway",
];
const DEFINITIONS: &[&str] = &[
    "messageEventDefinition",
    "timerEventDefinition",
    "signalEventDefinition",
    "errorEventDefinition",
    "escalationEventDefinition",
    "conditionalEventDefinition",
    "linkEventDefinition",
    "terminateEventDefinition",
    "compensateEventDefinition",
    "cancelEventDefinition",
];
const GLOBAL_TASKS: &[&str] = &[
    "globalTask",
    "globalUserTask",
    "globalManualTask",
    "globalScriptTask",
    "globalBusinessRuleTask",
];

/// Default of `implementation` by element (BPMN 2.0.2 schema).
fn implementation_default(tag: &str) -> Option<&'static str> {
    match tag {
        "serviceTask" | "sendTask" | "receiveTask" => Some("##WebService"),
        "userTask" | "businessRuleTask" | "globalUserTask" | "globalBusinessRuleTask" => {
            Some("##unspecified")
        }
        _ => None,
    }
}

struct Ex<'a, 'i, 'm> {
    by_id: HashMap<&'a str, Node<'a, 'i>>,
    ids: &'m HashMap<String, String>,
    original: bool,
    out: Facts,
    /// sequence flows by source and by target (XML identifiers)
    outgoing: HashMap<&'a str, usize>,
    incoming: HashMap<&'a str, usize>,
    /// data inputs and outputs of activities and events that carry nothing but their one
    /// data association (a type, a collection flag, documentation or another name than the
    /// data element's would be something): they are part of the association
    plain: BTreeSet<&'a str>,
}

impl<'a, 'i> Ex<'a, 'i, '_> {
    /// An identifier as tok writes it (identity for the exported document).
    fn m(&self, id: &str) -> String {
        let id = id.trim();
        self.ids.get(id).cloned().unwrap_or_else(|| id.to_string())
    }

    fn add(&mut self, cat: &'static str, subject: &str, text: String) {
        self.out
            .entry(Fact { cat, text })
            .or_insert_with(|| subject.to_string());
    }

    /// Name of a referenced declaration (message, signal, …): its name, else its identifier.
    fn decl_name(&self, r: &str) -> String {
        let r = r.trim();
        match self.by_id.get(r) {
            Some(e) => match attr_nonempty(*e, "name")
                .map(norm)
                .filter(|n| !n.is_empty())
            {
                Some(n) => n,
                None => r.to_string(),
            },
            None => r.to_string(),
        }
    }

    fn id_of<'x>(e: Node<'x, '_>) -> &'x str {
        e.attribute("id").unwrap_or("")
    }

    /// Identifier of the element itself or of its nearest ancestor that has one.
    fn subject<'x>(e: Node<'x, '_>) -> &'x str {
        let mut x = Some(e);
        while let Some(n) = x {
            if let Some(i) = n.attribute("id").filter(|i| !i.is_empty()) {
                return i;
            }
            x = n.parent_element();
        }
        "definitions"
    }

    fn qname(e: Node, v: &str) -> String {
        match v.split_once(':') {
            Some((p, l)) => match e.lookup_namespace_uri(Some(p)) {
                Some(ns) => format!("{{{ns}}}{l}"),
                None => v.to_string(),
            },
            None => v.to_string(),
        }
    }

    /// Facts every element may carry: documentation, extension elements, foreign attributes.
    fn common(&mut self, e: Node<'a, 'i>) {
        let subj = Self::subject(e);
        let s = self.m(subj);
        for d in findall(e, "documentation") {
            let t = block(text(d));
            if !t.is_empty() {
                self.add(
                    "documentation",
                    subj,
                    format!("{s} documentation {}", q(&t)),
                );
            }
            if let Some(f) = d.attribute("textFormat").filter(|f| *f != "text/plain") {
                self.add(
                    "documentation",
                    subj,
                    format!("{s} documentation format={f}"),
                );
            }
        }
        // declarations are compared by name (tok names messages)
        let s = if matches!(local(e), "message" | "signal" | "error" | "escalation") {
            format!("{} {}", local(e), q(&self.decl_name(subj)))
        } else {
            s
        };
        for x in findall(e, "extensionElements") {
            for z in kids(x) {
                if z.tag_name().namespace() == Some(ZEEBE_NS) {
                    self.zeebe(z, &s, subj);
                } else if self.original {
                    self.add("vendor", subj, format!("{s} extension {}", clark(z)));
                }
            }
        }
        if self.original {
            for a in e.attributes() {
                match a.namespace() {
                    None => {}
                    Some(XSI_NS) if a.name() == "type" => {}
                    Some(ns) if ns == BPMN_NS => {}
                    Some(ns) => {
                        self.add(
                            "vendor",
                            subj,
                            format!("{s} attribute {{{ns}}}{}", a.name()),
                        );
                    }
                }
            }
        }
    }

    fn zeebe(&mut self, z: Node<'a, 'i>, s: &str, subj: &str) {
        let mut attrs: Vec<String> = z
            .attributes()
            // values Camunda assumes when the attribute is absent
            .filter(|a| {
                !matches!(
                    (a.name(), a.value()),
                    ("propagateAllChildVariables", "true")
                        | ("propagateAllParentVariables", "true")
                )
            })
            .map(|a| {
                let v = a.value();
                let v = if v.starts_with('=') {
                    expr(v)
                } else {
                    v.to_string()
                };
                format!("{}={}", a.name(), q(&v))
            })
            .collect();
        attrs.sort();
        self.add(
            "camunda",
            subj,
            format!("{s} zeebe:{} {}", local(z), attrs.join(" ")),
        );
        for c in kids(z) {
            self.zeebe(c, s, subj);
        }
    }

    fn run(&mut self, root: Node<'a, 'i>) {
        let rid = Self::id_of(root);
        let r = self.m(if rid.is_empty() { "definitions" } else { rid });
        self.add("structure", rid, format!("definitions id={r}"));
        if let Some(n) = attr_nonempty(root, "name") {
            self.add(
                "structure",
                rid,
                format!("definitions name={}", q(&norm(n))),
            );
        }
        if let Some(n) = attr_nonempty(root, "targetNamespace") {
            self.add("structure", rid, format!("definitions targetNamespace={n}"));
        }
        if let Some(l) = attr_nonempty(root, "expressionLanguage").filter(|l| *l != XPATH) {
            self.add(
                "structure",
                rid,
                format!("definitions expressionLanguage={l}"),
            );
        }
        if let Some(l) = attr_nonempty(root, "typeLanguage").filter(|l| *l != XSD_NS) {
            self.add("structure", rid, format!("definitions typeLanguage={l}"));
        }
        self.common(root);
        for e in kids(root).filter(|e| is_bpmn(*e)) {
            self.root_element(e);
        }
    }

    fn root_element(&mut self, e: Node<'a, 'i>) {
        let t = local(e);
        let subj = Self::subject(e);
        let id = self.m(Self::id_of(e));
        match t {
            "import" => {
                let ns = e.attribute("namespace").unwrap_or("");
                let loc = e.attribute("location").unwrap_or("");
                let ty = e.attribute("importType").unwrap_or("");
                self.add(
                    "declarations",
                    subj,
                    format!("import namespace={ns} location={} type={ty}", q(loc)),
                );
            }
            "itemDefinition" => {
                self.add("declarations", subj, format!("item {id}"));
                if let Some(s) = attr_nonempty(e, "structureRef") {
                    let v = Self::qname(e, s);
                    self.add("declarations", subj, format!("item {id} structure={v}"));
                }
                if e.attribute("itemKind") == Some("Physical") {
                    self.add("declarations", subj, format!("item {id} kind=Physical"));
                }
                if e.attribute("isCollection") == Some("true") {
                    self.add("declarations", subj, format!("item {id} collection"));
                }
                if let Some(i) = attr_nonempty(e, "import") {
                    self.add("declarations", subj, format!("item {id} import={i}"));
                }
            }
            "message" | "signal" | "error" | "escalation" => {
                let n = q(&self.decl_name(Self::id_of(e)));
                self.add("declarations", subj, format!("{t} {n}"));
                for (a, key) in [
                    ("itemRef", "item"),
                    ("structureRef", "structure"),
                    ("errorCode", "code"),
                    ("escalationCode", "code"),
                ] {
                    if let Some(v) = attr_nonempty(e, a) {
                        let v = if a == "itemRef" || a == "structureRef" {
                            self.m(v)
                        } else {
                            v.to_string()
                        };
                        self.add("declarations", subj, format!("{t} {n} {key}={v}"));
                    }
                }
            }
            "dataStore" => {
                let n = q(&self.decl_name(Self::id_of(e)));
                self.add("data", subj, format!("dataStore {n}"));
                if let Some(c) = attr_nonempty(e, "capacity") {
                    self.add("data", subj, format!("dataStore {n} capacity={c}"));
                }
                if e.attribute("isUnlimited") == Some("false") {
                    self.add("data", subj, format!("dataStore {n} limited"));
                }
                if let Some(v) = attr_nonempty(e, "itemSubjectRef") {
                    let v = self.m(v);
                    self.add("data", subj, format!("dataStore {n} type={v}"));
                }
            }
            "resource" => {
                self.add("declarations", subj, format!("resource {id}"));
                if let Some(n) = attr_nonempty(e, "name") {
                    self.add(
                        "declarations",
                        subj,
                        format!("resource {id} name={}", q(&norm(n))),
                    );
                }
                for p in findall(e, "resourceParameter") {
                    let n = p.attribute("name").unwrap_or("");
                    self.add(
                        "declarations",
                        subj,
                        format!("resource {id} parameter {}", q(n)),
                    );
                }
            }
            "interface" => {
                self.add("declarations", subj, format!("interface {id}"));
                if let Some(n) = attr_nonempty(e, "name") {
                    self.add(
                        "declarations",
                        subj,
                        format!("interface {id} name={}", q(&norm(n))),
                    );
                }
                if let Some(v) = attr_nonempty(e, "implementationRef") {
                    self.add(
                        "declarations",
                        subj,
                        format!("interface {id} implementation={v}"),
                    );
                }
                for op in findall(e, "operation") {
                    let osubj = Self::subject(op);
                    let oid = self.m(Self::id_of(op));
                    self.add("declarations", osubj, format!("operation {oid} of {id}"));
                    if let Some(n) = attr_nonempty(op, "name") {
                        self.add(
                            "declarations",
                            osubj,
                            format!("operation {oid} name={}", q(&norm(n))),
                        );
                    }
                    if let Some(v) = attr_nonempty(op, "implementationRef") {
                        self.add(
                            "declarations",
                            osubj,
                            format!("operation {oid} implementation={v}"),
                        );
                    }
                    for (child, key) in [
                        ("inMessageRef", "in"),
                        ("outMessageRef", "out"),
                        ("errorRef", "error"),
                    ] {
                        for c in kids(op).filter(|c| is_b(*c, child)) {
                            let n = q(&self.decl_name(text(c)));
                            self.add("declarations", osubj, format!("operation {oid} {key}={n}"));
                        }
                    }
                    self.common(op);
                }
            }
            "correlationProperty" => {
                let n = q(&self.decl_name(Self::id_of(e)));
                self.add("declarations", subj, format!("correlationProperty {n}"));
                if let Some(v) = attr_nonempty(e, "type") {
                    let v = self.m(v);
                    self.add(
                        "declarations",
                        subj,
                        format!("correlationProperty {n} type={v}"),
                    );
                }
                for x in findall(e, "correlationPropertyRetrievalExpression") {
                    let msg = q(&self.decl_name(x.attribute("messageRef").unwrap_or("")));
                    let path = find(x, "messagePath").map(text).unwrap_or("");
                    self.add(
                        "declarations",
                        subj,
                        format!(
                            "correlationProperty {n} message={msg} path={}",
                            q(&expr(path))
                        ),
                    );
                }
            }
            "category" => {
                if let Some(n) = attr_nonempty(e, "name") {
                    self.add("artifacts", subj, format!("category {}", q(&norm(n))));
                }
                for v in findall(e, "categoryValue") {
                    let val = v.attribute("value").unwrap_or("");
                    self.add(
                        "artifacts",
                        Self::subject(v),
                        format!("categoryValue {}", q(&norm(val))),
                    );
                    self.common(v);
                }
            }
            "collaboration" => self.collaboration(e),
            "process" => self.process(e),
            _ if GLOBAL_TASKS.contains(&t) => {
                self.add("declarations", subj, format!("{t} {id}"));
                if let Some(n) = attr_nonempty(e, "name") {
                    self.add(
                        "declarations",
                        subj,
                        format!("{t} {id} name={}", q(&norm(n))),
                    );
                }
                if let Some(v) = attr_nonempty(e, "implementation")
                    .filter(|v| Some(*v) != implementation_default(t))
                {
                    self.add("declarations", subj, format!("{t} {id} implementation={v}"));
                }
                self.performers(e, &id, subj, "declarations");
            }
            _ => {
                self.add("declarations", subj, format!("{t} {id}"));
            }
        }
        if !matches!(t, "collaboration" | "process" | "interface" | "category") {
            self.common(e);
        }
    }

    fn collaboration(&mut self, c: Node<'a, 'i>) {
        let subj = Self::subject(c);
        let cid = self.m(Self::id_of(c));
        self.add("structure", subj, format!("collaboration {cid}"));
        if let Some(n) = attr_nonempty(c, "name") {
            self.add(
                "structure",
                subj,
                format!("collaboration {cid} name={}", q(&norm(n))),
            );
        }
        self.common(c);
        for e in kids(c).filter(|e| is_bpmn(*e)) {
            let t = local(e);
            let subj = Self::subject(e);
            let id = self.m(Self::id_of(e));
            match t {
                "participant" => {
                    self.add("structure", subj, format!("participant {id}"));
                    if let Some(n) = attr_nonempty(e, "name") {
                        self.add(
                            "structure",
                            subj,
                            format!("participant {id} name={}", q(&norm(n))),
                        );
                    }
                    if let Some(p) = attr_nonempty(e, "processRef") {
                        let p = self.m(p);
                        self.add("structure", subj, format!("participant {id} process={p}"));
                    }
                    if let Some(pm) = find(e, "participantMultiplicity") {
                        let min = pm.attribute("minimum").unwrap_or("0");
                        let max = pm.attribute("maximum").unwrap_or("1");
                        self.add(
                            "structure",
                            subj,
                            format!("participant {id} multiplicity {min}..{max}"),
                        );
                    }
                    for r in findall(e, "interfaceRef") {
                        let v = self.m(text(r));
                        self.add("structure", subj, format!("participant {id} interface={v}"));
                    }
                    self.common(e);
                }
                "messageFlow" => {
                    let src = self.m(e.attribute("sourceRef").unwrap_or(""));
                    let tgt = self.m(e.attribute("targetRef").unwrap_or(""));
                    self.add("flows", subj, format!("{id} messageFlow {src} -> {tgt}"));
                    if let Some(n) = attr_nonempty(e, "name") {
                        self.add("flows", subj, format!("{id} name={}", q(&norm(n))));
                    }
                    if let Some(r) = attr_nonempty(e, "messageRef") {
                        let n = q(&self.decl_name(r));
                        self.add("flows", subj, format!("{id} message={n}"));
                    }
                    self.common(e);
                }
                "correlationKey" => {
                    let name = attr_nonempty(e, "name").map(norm).unwrap_or_default();
                    let mut props: Vec<String> = findall(e, "correlationPropertyRef")
                        .into_iter()
                        .map(|r| self.decl_name(text(r)))
                        .collect();
                    props.sort();
                    self.add(
                        "declarations",
                        subj,
                        format!(
                            "correlationKey {id} name={} properties={}",
                            q(&name),
                            q(&props.join(", "))
                        ),
                    );
                    self.common(e);
                }
                "textAnnotation" | "association" | "group" => self.artifact(e),
                "documentation" | "extensionElements" => {}
                _ => {
                    self.add("structure", subj, format!("{t} {id}"));
                    self.common(e);
                }
            }
        }
    }

    fn process(&mut self, p: Node<'a, 'i>) {
        let subj = Self::subject(p);
        let pid = self.m(Self::id_of(p));
        self.add("structure", subj, format!("process {pid}"));
        if let Some(n) = attr_nonempty(p, "name") {
            self.add(
                "structure",
                subj,
                format!("process {pid} name={}", q(&norm(n))),
            );
        }
        if p.attribute("isExecutable") == Some("true") {
            self.add("structure", subj, format!("process {pid} executable"));
        }
        if let Some(v) = attr_nonempty(p, "processType").filter(|v| *v != "None") {
            self.add("structure", subj, format!("process {pid} processType={v}"));
        }
        if p.attribute("isClosed") == Some("true") {
            self.add("structure", subj, format!("process {pid} closed"));
        }
        for r in findall(p, "supportedInterfaceRef") {
            let v = self.m(text(r));
            self.add("structure", subj, format!("process {pid} interface={v}"));
        }
        self.common(p);
        self.performers(p, &pid, subj, "structure");
        for ls in findall(p, "laneSet") {
            for l in findall(ls, "lane") {
                self.lane(l, None, &pid);
            }
        }
        self.container(p, &pid);
    }

    fn lane(&mut self, l: Node<'a, 'i>, parent: Option<&str>, pid: &str) {
        let subj = Self::subject(l);
        let id = self.m(Self::id_of(l));
        self.add("structure", subj, format!("lane {id} in={pid}"));
        if let Some(n) = attr_nonempty(l, "name") {
            self.add("structure", subj, format!("lane {id} name={}", q(&norm(n))));
        }
        if let Some(p) = parent {
            self.add("structure", subj, format!("lane {id} parent={p}"));
        }
        for r in findall(l, "flowNodeRef") {
            let v = self.m(text(r));
            self.add("structure", subj, format!("lane {id} member {v}"));
        }
        self.common(l);
        for cls in findall(l, "childLaneSet") {
            for c in findall(cls, "lane") {
                self.lane(c, Some(&id), pid);
            }
        }
    }

    /// Content of a process or sub-process.
    fn container(&mut self, c: Node<'a, 'i>, cid: &str) {
        let owner_kind = if is_b(c, "process") {
            "process"
        } else {
            "activity"
        };
        for e in kids(c).filter(|e| is_bpmn(*e)) {
            let t = local(e);
            if EVENTS.contains(&t) || ACTIVITIES.contains(&t) || GATEWAYS.contains(&t) {
                self.flow_node(e, cid);
            } else {
                match t {
                    "sequenceFlow" => self.sequence_flow(e),
                    "dataObject" => self.data_object(e, cid),
                    "dataObjectReference" => self.data_object_reference(e, cid),
                    "dataStoreReference" => self.data_store_reference(e, cid),
                    "ioSpecification" if owner_kind == "process" => self.io(e, cid, true),
                    "property" if owner_kind == "process" => self.property(e, cid, true),
                    "textAnnotation" | "association" | "group" => self.artifact(e),
                    "laneSet" if owner_kind == "activity" => {
                        for l in findall(e, "lane") {
                            self.lane(l, None, cid);
                        }
                    }
                    "documentation"
                    | "extensionElements"
                    | "laneSet"
                    | "supportedInterfaceRef"
                    | "performer"
                    | "humanPerformer"
                    | "potentialOwner"
                    | "incoming"
                    | "outgoing"
                    | "dataInputAssociation"
                    | "dataOutputAssociation"
                    | "standardLoopCharacteristics"
                    | "multiInstanceLoopCharacteristics"
                    | "completionCondition"
                    | "ioSpecification"
                    | "property" => {}
                    _ => {
                        let subj = Self::subject(e);
                        let id = self.m(Self::id_of(e));
                        self.add("nodes", subj, format!("{id} kind={t} in={cid}"));
                        self.common(e);
                    }
                }
            }
        }
    }

    fn flow_node(&mut self, e: Node<'a, 'i>, cid: &str) {
        let t = local(e);
        let subj = Self::subject(e);
        let id = self.m(Self::id_of(e));
        self.add("nodes", subj, format!("{id} kind={t}"));
        self.add("nodes", subj, format!("{id} in={cid}"));
        if let Some(n) = attr_nonempty(e, "name").map(norm).filter(|n| !n.is_empty()) {
            self.add("nodes", subj, format!("{id} name={}", q(&n)));
        }
        if let Some(d) = attr_nonempty(e, "default") {
            let d = self.m(d);
            self.add("flows", subj, format!("{id} default={d}"));
        }
        self.common(e);
        if EVENTS.contains(&t) {
            self.event(e, &id, subj);
        } else if ACTIVITIES.contains(&t) {
            self.activity(e, &id, subj);
        } else {
            self.gateway(e, &id, subj);
        }
        // data of activities and events
        let sub = matches!(t, "subProcess" | "transaction" | "adHocSubProcess");
        if let Some(io) = find(e, "ioSpecification") {
            self.io(io, &id, sub);
        }
        for x in findall(e, "dataInput") {
            self.io_item(x, "input", &id, false);
        }
        for x in findall(e, "dataOutput") {
            self.io_item(x, "output", &id, false);
        }
        for p in findall(e, "property") {
            self.property(p, &id, sub);
        }
        for a in kids(e)
            .filter(|a| is_b(*a, "dataInputAssociation") || is_b(*a, "dataOutputAssociation"))
        {
            self.data_association(a, &id);
        }
        for set in kids(e).filter(|s| is_b(*s, "inputSet") || is_b(*s, "outputSet")) {
            self.io_set(set, &id);
        }
        if matches!(t, "subProcess" | "transaction" | "adHocSubProcess") {
            self.container(e, &id);
        }
    }

    fn event(&mut self, e: Node<'a, 'i>, id: &str, subj: &str) {
        let defs: Vec<Node> = kids(e)
            .filter(|c| is_bpmn(*c) && DEFINITIONS.contains(&local(*c)))
            .collect();
        let mut kinds: Vec<&str> = defs
            .iter()
            .map(|d| local(*d).trim_end_matches("EventDefinition"))
            .collect();
        kinds.sort();
        if !kinds.is_empty() {
            self.add(
                "events",
                subj,
                format!("{id} definitions={}", kinds.join(",")),
            );
        }
        for d in defs {
            let k = local(d).trim_end_matches("EventDefinition");
            match k {
                "message" | "signal" | "error" | "escalation" => {
                    if let Some(r) = attr_nonempty(d, &format!("{k}Ref")) {
                        let n = q(&self.decl_name(r));
                        self.add("events", subj, format!("{id} {k}={n}"));
                    }
                    if k == "message"
                        && let Some(o) = attr_nonempty(d, "operationRef")
                    {
                        let o = self.m(o);
                        self.add("events", subj, format!("{id} message operation={o}"));
                    }
                }
                "timer" => {
                    for part in ["timeDate", "timeDuration", "timeCycle"] {
                        // an empty time says nothing
                        if let Some(x) = find(d, part).filter(|x| !text(*x).trim().is_empty()) {
                            self.add(
                                "events",
                                subj,
                                format!("{id} timer {part}={}", q(&expr(text(x)))),
                            );
                        }
                    }
                }
                "conditional" => {
                    if let Some(x) = find(d, "condition") {
                        self.add(
                            "events",
                            subj,
                            format!("{id} condition={}", q(&expr(text(x)))),
                        );
                    }
                }
                "link" => {
                    if let Some(n) = attr_nonempty(d, "name") {
                        self.add("events", subj, format!("{id} link={}", q(n)));
                    }
                    for (child, key) in [("source", "source"), ("target", "target")] {
                        for s in kids(d).filter(|c| is_b(*c, child)) {
                            let v = self.m(text(s));
                            self.add("events", subj, format!("{id} link {key}={v}"));
                        }
                    }
                }
                "compensate" => {
                    if let Some(a) = attr_nonempty(d, "activityRef") {
                        let a = self.m(a);
                        self.add("events", subj, format!("{id} compensate activity={a}"));
                    }
                    if d.attribute("waitForCompletion") == Some("false") {
                        self.add("events", subj, format!("{id} compensate no-wait"));
                    }
                }
                _ => {}
            }
            self.common(d);
        }
        for r in findall(e, "eventDefinitionRef") {
            let v = self.m(text(r));
            self.add("events", subj, format!("{id} eventDefinitionRef={v}"));
        }
        if e.attribute("parallelMultiple") == Some("true") {
            self.add("events", subj, format!("{id} parallelMultiple"));
        }
        if e.attribute("isInterrupting") == Some("false") {
            self.add("events", subj, format!("{id} non-interrupting"));
        }
        if e.attribute("cancelActivity") == Some("false") {
            self.add("events", subj, format!("{id} non-interrupting"));
        }
        if let Some(a) = attr_nonempty(e, "attachedToRef") {
            let a = self.m(a);
            self.add("events", subj, format!("{id} attachedTo={a}"));
        }
    }

    fn activity(&mut self, e: Node<'a, 'i>, id: &str, subj: &str) {
        let t = local(e);
        let cat = "activities";
        if e.attribute("isForCompensation") == Some("true") {
            self.add(cat, subj, format!("{id} compensation-handler"));
        }
        for a in ["startQuantity", "completionQuantity"] {
            if let Some(v) = attr_nonempty(e, a).filter(|v| *v != "1") {
                self.add(cat, subj, format!("{id} {a}={v}"));
            }
        }
        if let Some(v) =
            attr_nonempty(e, "implementation").filter(|v| Some(*v) != implementation_default(t))
        {
            self.add(cat, subj, format!("{id} implementation={v}"));
        }
        if t == "subProcess" && e.attribute("triggeredByEvent") == Some("true") {
            self.add(cat, subj, format!("{id} triggeredByEvent"));
        }
        if t == "transaction"
            && let Some(m) = attr_nonempty(e, "method").filter(|m| *m != "##Compensate")
        {
            self.add(cat, subj, format!("{id} method={m}"));
        }
        if t == "adHocSubProcess" {
            if let Some(o) = attr_nonempty(e, "ordering").filter(|o| *o != "Parallel") {
                self.add(cat, subj, format!("{id} ordering={o}"));
            }
            if e.attribute("cancelRemainingInstances") == Some("false") {
                self.add(cat, subj, format!("{id} keep-remaining-instances"));
            }
            if let Some(c) = find(e, "completionCondition") {
                self.add(
                    cat,
                    subj,
                    format!("{id} completionCondition={}", q(&expr(text(c)))),
                );
            }
        }
        if t == "callActivity"
            && let Some(c) = attr_nonempty(e, "calledElement")
        {
            let c = self.m(c);
            self.add(cat, subj, format!("{id} calledElement={c}"));
        }
        if t == "scriptTask" {
            if let Some(f) = attr_nonempty(e, "scriptFormat") {
                self.add(cat, subj, format!("{id} scriptFormat={f}"));
            }
            if let Some(s) = find(e, "script") {
                self.add(cat, subj, format!("{id} script={}", q(&block(text(s)))));
            }
        }
        if matches!(t, "sendTask" | "receiveTask") {
            if let Some(r) = attr_nonempty(e, "messageRef") {
                let n = q(&self.decl_name(r));
                self.add(cat, subj, format!("{id} message={n}"));
            }
            if e.attribute("instantiate") == Some("true") {
                self.add(cat, subj, format!("{id} instantiate"));
            }
        }
        if matches!(t, "serviceTask" | "sendTask" | "receiveTask")
            && let Some(o) = attr_nonempty(e, "operationRef")
        {
            let o = self.m(o);
            self.add(cat, subj, format!("{id} operation={o}"));
        }
        if let Some(lc) = find(e, "standardLoopCharacteristics") {
            self.add(cat, subj, format!("{id} loop=standard"));
            if lc.attribute("testBefore") == Some("true") {
                self.add(cat, subj, format!("{id} loop testBefore"));
            }
            if let Some(m) = attr_nonempty(lc, "loopMaximum") {
                self.add(cat, subj, format!("{id} loop maximum={m}"));
            }
            if let Some(c) = find(lc, "loopCondition") {
                self.add(
                    cat,
                    subj,
                    format!("{id} loop condition={}", q(&expr(text(c)))),
                );
            }
            self.common(lc);
        }
        if let Some(mi) = find(e, "multiInstanceLoopCharacteristics") {
            let seq = mi.attribute("isSequential") == Some("true");
            self.add(
                cat,
                subj,
                format!(
                    "{id} loop=multi-instance {}",
                    if seq { "sequential" } else { "parallel" }
                ),
            );
            for (child, key) in [
                ("loopCardinality", "cardinality"),
                ("completionCondition", "completionCondition"),
            ] {
                if let Some(c) = find(mi, child) {
                    self.add(
                        cat,
                        subj,
                        format!("{id} multi-instance {key}={}", q(&expr(text(c)))),
                    );
                }
            }
            for (child, key) in [
                ("loopDataInputRef", "input"),
                ("loopDataOutputRef", "output"),
            ] {
                if let Some(c) = find(mi, child) {
                    let v = self.m(text(c));
                    self.add(cat, subj, format!("{id} multi-instance {key}={v}"));
                }
            }
            for (child, key) in [
                ("inputDataItem", "inputItem"),
                ("outputDataItem", "outputItem"),
            ] {
                if let Some(c) = find(mi, child) {
                    let n = c
                        .attribute("name")
                        .map(norm)
                        .unwrap_or_else(|| self.m(Self::id_of(c)));
                    self.add(cat, subj, format!("{id} multi-instance {key}={}", q(&n)));
                }
            }
            if let Some(b) = attr_nonempty(mi, "behavior").filter(|b| *b != "All") {
                self.add(cat, subj, format!("{id} multi-instance behavior={b}"));
            }
            if find(mi, "complexBehaviorDefinition").is_some() {
                self.add(cat, subj, format!("{id} multi-instance complexBehavior"));
            }
            self.common(mi);
        }
        self.performers(e, id, subj, cat);
    }

    fn performers(&mut self, e: Node<'a, 'i>, id: &str, subj: &str, cat: &'static str) {
        for p in kids(e).filter(|p| {
            is_bpmn(*p)
                && matches!(
                    local(*p),
                    "performer" | "humanPerformer" | "potentialOwner" | "resourceRole"
                )
        }) {
            let t = local(p);
            if let Some(r) = find(p, "resourceRef") {
                let v = self.m(text(r));
                self.add(cat, subj, format!("{id} {t} resource={v}"));
            } else {
                self.add(cat, subj, format!("{id} {t}"));
            }
            if let Some(x) = find(p, "resourceAssignmentExpression") {
                let body = find(x, "expression")
                    .or_else(|| find(x, "formalExpression"))
                    .map(text)
                    .unwrap_or("");
                self.add(cat, subj, format!("{id} {t} assignment={}", q(&expr(body))));
            }
            for b in findall(p, "resourceParameterBinding") {
                let r = self.m(b.attribute("parameterRef").unwrap_or(""));
                self.add(cat, subj, format!("{id} {t} parameter={r}"));
            }
            self.common(p);
        }
    }

    fn gateway(&mut self, e: Node<'a, 'i>, id: &str, subj: &str) {
        let t = local(e);
        let raw = Self::id_of(e);
        let stated = attr_nonempty(e, "gatewayDirection").filter(|d| *d != "Unspecified");
        let n_in = self.incoming.get(raw).copied().unwrap_or(0);
        let n_out = self.outgoing.get(raw).copied().unwrap_or(0);
        let implied = match (n_in > 1, n_out > 1) {
            (true, true) => "Mixed",
            (true, false) => "Converging",
            (false, true) => "Diverging",
            (false, false) => "Unspecified",
        };
        let dir = stated.unwrap_or(implied);
        if dir != "Unspecified" {
            self.add("gateways", subj, format!("{id} direction={dir}"));
        }
        if t == "eventBasedGateway" {
            if e.attribute("instantiate") == Some("true") {
                self.add("gateways", subj, format!("{id} instantiate"));
            }
            if let Some(g) = attr_nonempty(e, "eventGatewayType").filter(|g| *g != "Exclusive") {
                self.add("gateways", subj, format!("{id} eventGatewayType={g}"));
            }
        }
        if t == "complexGateway"
            && let Some(c) = find(e, "activationCondition")
        {
            self.add(
                "gateways",
                subj,
                format!("{id} activation={}", q(&expr(text(c)))),
            );
        }
    }

    fn sequence_flow(&mut self, e: Node<'a, 'i>) {
        let subj = Self::subject(e);
        let id = self.m(Self::id_of(e));
        let src = self.m(e.attribute("sourceRef").unwrap_or(""));
        let tgt = self.m(e.attribute("targetRef").unwrap_or(""));
        self.add("flows", subj, format!("{id} flow {src} -> {tgt}"));
        if let Some(n) = attr_nonempty(e, "name").map(norm).filter(|n| !n.is_empty()) {
            self.add("flows", subj, format!("{id} name={}", q(&n)));
        }
        if let Some(c) = find(e, "conditionExpression") {
            self.add(
                "flows",
                subj,
                format!("{id} condition={}", q(&expr(text(c)))),
            );
            if let Some(l) = attr_nonempty(c, "language") {
                self.add("flows", subj, format!("{id} condition language={l}"));
            }
        }
        if let Some(v) = attr_nonempty(e, "isImmediate") {
            self.add("flows", subj, format!("{id} isImmediate={v}"));
        }
        self.common(e);
    }

    fn data_object(&mut self, e: Node<'a, 'i>, cid: &str) {
        let subj = Self::subject(e);
        let name = attr_nonempty(e, "name")
            .map(norm)
            .unwrap_or_else(|| self.m(Self::id_of(e)));
        let n = q(&name);
        self.add("data", subj, format!("dataObject {n} in={cid}"));
        if e.attribute("isCollection") == Some("true") {
            self.add("data", subj, format!("dataObject {n} collection"));
        }
        if let Some(v) = attr_nonempty(e, "itemSubjectRef") {
            let v = self.m(v);
            self.add("data", subj, format!("dataObject {n} type={v}"));
        }
        if let Some(st) = find(e, "dataState").and_then(|s| attr_nonempty(s, "name")) {
            self.add("data", subj, format!("dataObject {n} state={}", q(st)));
        }
        self.common(e);
    }

    fn data_object_reference(&mut self, e: Node<'a, 'i>, cid: &str) {
        let subj = Self::subject(e);
        let id = self.m(Self::id_of(e));
        self.add("data", subj, format!("{id} dataObjectReference in={cid}"));
        let obj = attr_nonempty(e, "dataObjectRef").and_then(|r| self.by_id.get(r).copied());
        let oname = obj.and_then(|o| attr_nonempty(o, "name")).map(norm);
        let name = attr_nonempty(e, "name").map(norm).or(oname.clone());
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            self.add("data", subj, format!("{id} name={}", q(&n)));
        }
        match obj {
            Some(o) => {
                let on = oname.unwrap_or_else(|| self.m(Self::id_of(o)));
                self.add("data", subj, format!("{id} object={}", q(&on)));
            }
            None => self.add("data", subj, format!("{id} object=none")),
        }
        if let Some(st) = find(e, "dataState").and_then(|s| attr_nonempty(s, "name")) {
            self.add("data", subj, format!("{id} state={}", q(st)));
        }
        if let Some(v) = attr_nonempty(e, "itemSubjectRef") {
            let v = self.m(v);
            self.add("data", subj, format!("{id} type={v}"));
        }
        self.common(e);
    }

    fn data_store_reference(&mut self, e: Node<'a, 'i>, cid: &str) {
        let subj = Self::subject(e);
        let id = self.m(Self::id_of(e));
        self.add("data", subj, format!("{id} dataStoreReference in={cid}"));
        let store = attr_nonempty(e, "dataStoreRef").and_then(|r| self.by_id.get(r).copied());
        let sname = store.and_then(|o| attr_nonempty(o, "name")).map(norm);
        if let Some(n) = attr_nonempty(e, "name")
            .map(norm)
            .or(sname.clone())
            .filter(|n| !n.is_empty())
        {
            self.add("data", subj, format!("{id} name={}", q(&n)));
        }
        if let Some(s) = store {
            let n = sname.unwrap_or_else(|| self.m(Self::id_of(s)));
            self.add("data", subj, format!("{id} store={}", q(&n)));
        }
        if let Some(v) = attr_nonempty(e, "itemSubjectRef") {
            let v = self.m(v);
            self.add("data", subj, format!("{id} type={v}"));
        }
        if let Some(st) = find(e, "dataState").and_then(|s| attr_nonempty(s, "name")) {
            self.add("data", subj, format!("{id} state={}", q(st)));
        }
        self.common(e);
    }

    /// `ioSpecification` of a process, a sub-process or an activity. Inputs and outputs of
    /// processes and sub-processes are compared by identifier, those of activities by name
    /// (tok names the ones its data associations use after the data element).
    fn io(&mut self, io: Node<'a, 'i>, owner: &str, by_id: bool) {
        for x in kids(io) {
            match local(x) {
                "dataInput" if is_bpmn(x) => self.io_item(x, "input", owner, by_id),
                "dataOutput" if is_bpmn(x) => self.io_item(x, "output", owner, by_id),
                _ => {}
            }
        }
        for set in kids(io).filter(|s| is_b(*s, "inputSet") || is_b(*s, "outputSet")) {
            self.io_set(set, owner);
        }
        self.common(io);
    }

    fn io_label(&self, x: Node) -> String {
        attr_nonempty(x, "name")
            .map(norm)
            .unwrap_or_else(|| self.m(Self::id_of(x)))
    }

    fn io_item(&mut self, x: Node<'a, 'i>, kind: &str, owner: &str, by_id: bool) {
        if !by_id && self.plain.contains(Self::id_of(x)) {
            return;
        }
        // inputs and outputs of activities and events belong to them
        let subj = if by_id {
            Self::subject(x)
        } else {
            x.ancestors()
                .skip(1)
                .find(|a| !is_b(*a, "ioSpecification"))
                .map(Self::subject)
                .unwrap_or_else(|| Self::subject(x))
        };
        let key = if by_id {
            self.m(Self::id_of(x))
        } else {
            format!("{owner} {kind} {}", q(&self.io_label(x)))
        };
        if by_id {
            self.add("data", subj, format!("{key} {kind} of {owner}"));
            if let Some(n) = attr_nonempty(x, "name") {
                self.add("data", subj, format!("{key} name={}", q(&norm(n))));
            }
        } else {
            self.add("data", subj, key.clone());
        }
        if let Some(v) = attr_nonempty(x, "itemSubjectRef") {
            let v = self.m(v);
            self.add("data", subj, format!("{key} type={v}"));
        }
        if x.attribute("isCollection") == Some("true") {
            self.add("data", subj, format!("{key} collection"));
        }
        self.common(x);
    }

    /// Input and output sets are facts only when they say more than "one set of all".
    fn io_set(&mut self, set: Node<'a, 'i>, owner: &str) {
        let input = is_b(set, "inputSet");
        let parent = set.parent_element().unwrap_or(set);
        let (item, sets) = if input {
            ("dataInput", "inputSet")
        } else {
            ("dataOutput", "outputSet")
        };
        let n_sets = kids(parent).filter(|s| is_b(*s, sets)).count();
        let all: BTreeSet<&str> = kids(parent)
            .filter(|x| is_b(*x, item))
            .map(Self::id_of)
            .collect();
        let refs_name = if input {
            "dataInputRefs"
        } else {
            "dataOutputRefs"
        };
        let members: BTreeSet<&str> = findall(set, refs_name).into_iter().map(text).collect();
        let special: Vec<Node> = kids(set)
            .filter(|c| {
                is_bpmn(*c)
                    && matches!(
                        local(*c),
                        "optionalInputRefs"
                            | "whileExecutingInputRefs"
                            | "optionalOutputRefs"
                            | "whileExecutingOutputRefs"
                            | "outputSetRefs"
                            | "inputSetRefs"
                    )
            })
            .collect();
        if n_sets <= 1 && special.is_empty() && (members == all || all.is_empty()) {
            return;
        }
        // the import reports sets by the element that owns them
        let subj = parent
            .parent_element()
            .and_then(|o| o.attribute("id"))
            .unwrap_or_else(|| Self::subject(set));
        let mut names: Vec<String> = members
            .iter()
            .map(|r| {
                self.by_id
                    .get(r.trim())
                    .map(|x| self.io_label(*x))
                    .unwrap_or_else(|| r.to_string())
            })
            .collect();
        names.sort();
        let mut extra: Vec<String> = special
            .iter()
            .map(|c| format!("{}={}", local(*c), self.m(text(*c))))
            .collect();
        extra.sort();
        self.add(
            "data",
            subj,
            format!("{owner} {sets} [{}] {}", names.join(", "), extra.join(" "))
                .trim_end()
                .to_string(),
        );
    }

    fn property(&mut self, p: Node<'a, 'i>, owner: &str, by_id: bool) {
        if p.attribute("name") == Some("__targetRef_placeholder") {
            return;
        }
        let subj = Self::subject(p);
        let key = if by_id {
            self.m(Self::id_of(p))
        } else {
            format!("{owner} property {}", q(&self.io_label(p)))
        };
        if by_id {
            self.add("data", subj, format!("{key} property of {owner}"));
            if let Some(n) = attr_nonempty(p, "name") {
                self.add("data", subj, format!("{key} name={}", q(&norm(n))));
            }
        } else {
            self.add("data", subj, key.clone());
        }
        if let Some(v) = attr_nonempty(p, "itemSubjectRef") {
            let v = self.m(v);
            self.add("data", subj, format!("{key} type={v}"));
        }
        self.common(p);
    }

    /// Name of the end of a data association inside the activity (its data input or
    /// output); empty for a property placeholder.
    fn inner_end(&self, r: &str) -> String {
        match self.by_id.get(r.trim()) {
            Some(x) if x.attribute("name") == Some("__targetRef_placeholder") => String::new(),
            Some(_) if self.plain.contains(r.trim()) => String::new(),
            Some(x) => self.io_label(*x),
            None => r.trim().to_string(),
        }
    }

    fn data_association(&mut self, a: Node<'a, 'i>, owner: &str) {
        let subj = Self::subject(a);
        let input = is_b(a, "dataInputAssociation");
        let sources: Vec<&str> = findall(a, "sourceRef").into_iter().map(text).collect();
        let target = find(a, "targetRef").map(text).unwrap_or("");
        if input {
            let inner = self.inner_end(target);
            for s in &sources {
                let s = self.m(s);
                self.add("data", subj, format!("{owner} reads {s}"));
                if !inner.is_empty() {
                    self.add(
                        "data",
                        subj,
                        format!("{owner} reads {s} into {}", q(&inner)),
                    );
                }
            }
            if sources.is_empty() {
                self.add(
                    "data",
                    subj,
                    format!("{owner} input association into {}", q(&inner)),
                );
            }
        } else {
            let t = self.m(target);
            self.add("data", subj, format!("{owner} writes {t}"));
            for s in &sources {
                let inner = self.inner_end(s);
                if !inner.is_empty() {
                    self.add(
                        "data",
                        subj,
                        format!("{owner} writes {t} from {}", q(&inner)),
                    );
                }
            }
        }
        if let Some(tr) = find(a, "transformation") {
            self.add(
                "data",
                subj,
                format!("{owner} association transformation={}", q(&expr(text(tr)))),
            );
        }
        for asg in findall(a, "assignment") {
            let from = find(asg, "from").map(text).unwrap_or("");
            let to = find(asg, "to").map(text).unwrap_or("");
            self.add(
                "data",
                subj,
                format!(
                    "{owner} association assignment {} -> {}",
                    q(&expr(from)),
                    q(&expr(to))
                ),
            );
        }
        self.common(a);
    }

    fn artifact(&mut self, e: Node<'a, 'i>) {
        let subj = Self::subject(e);
        let id = self.m(Self::id_of(e));
        match local(e) {
            "textAnnotation" => {
                let t = find(e, "text").map(text).unwrap_or("");
                self.add(
                    "artifacts",
                    subj,
                    format!("{id} annotation text={}", q(&norm(t))),
                );
                if let Some(f) = attr_nonempty(e, "textFormat").filter(|f| *f != "text/plain") {
                    self.add("artifacts", subj, format!("{id} annotation format={f}"));
                }
            }
            "association" => {
                let src = self.m(e.attribute("sourceRef").unwrap_or(""));
                let tgt = self.m(e.attribute("targetRef").unwrap_or(""));
                match attr_nonempty(e, "associationDirection").filter(|d| *d != "None") {
                    Some(d) => self.add(
                        "artifacts",
                        subj,
                        format!("association {src} -> {tgt} direction={d}"),
                    ),
                    // without a direction the ends are unordered
                    None => {
                        let (a, b) = if src <= tgt { (src, tgt) } else { (tgt, src) };
                        self.add("artifacts", subj, format!("association {a} -- {b}"));
                    }
                }
            }
            _ => {
                let cv = attr_nonempty(e, "categoryValueRef")
                    .and_then(|r| self.by_id.get(r))
                    .and_then(|c| c.attribute("value"))
                    .map(norm)
                    .unwrap_or_default();
                self.add("artifacts", subj, format!("{id} group category={}", q(&cv)));
            }
        }
        self.common(e);
    }
}

/// Data inputs and outputs used by one data association of their activity or event, with
/// one data element at its other end, named after it (or unnamed) and without a type, a
/// collection flag or documentation.
fn plain_io<'a, 'i>(
    root: Node<'a, 'i>,
    by_id: &HashMap<&'a str, Node<'a, 'i>>,
) -> BTreeSet<&'a str> {
    let mut out = BTreeSet::new();
    let label = |d: &str| -> String {
        match by_id.get(d) {
            Some(e) => attr_nonempty(*e, "name")
                .or_else(|| {
                    attr_nonempty(*e, "dataObjectRef")
                        .and_then(|r| by_id.get(r))
                        .and_then(|o| attr_nonempty(*o, "name"))
                })
                .map(norm)
                .unwrap_or_else(|| d.to_string()),
            None => d.to_string(),
        }
    };
    for e in root.descendants().filter(|e| is_bpmn(*e)) {
        let assocs: Vec<Node> = kids(e)
            .filter(|a| is_b(*a, "dataInputAssociation") || is_b(*a, "dataOutputAssociation"))
            .collect();
        let mut ends: Vec<(&str, Node)> = Vec::new();
        for a in &assocs {
            let inner = if is_b(*a, "dataInputAssociation") {
                "targetRef"
            } else {
                "sourceRef"
            };
            for r in findall(*a, inner) {
                ends.push((text(r).trim(), *a));
            }
        }
        for (id, a) in &ends {
            let Some(x) = by_id.get(id).copied() else {
                continue;
            };
            if !(is_b(x, "dataInput") || is_b(x, "dataOutput")) {
                continue;
            }
            let outer_name = if is_b(*a, "dataInputAssociation") {
                "sourceRef"
            } else {
                "targetRef"
            };
            let outer: Vec<&str> = findall(*a, outer_name)
                .into_iter()
                .map(|r| text(r).trim())
                .collect();
            let once = ends.iter().filter(|(i, _)| i == id).count() == 1;
            let bare = attr_nonempty(x, "itemSubjectRef").is_none()
                && x.attribute("isCollection") != Some("true")
                && find(x, "documentation").is_none();
            let named = match outer.as_slice() {
                [d] => attr_nonempty(x, "name").is_none_or(|n| norm(n) == label(d)),
                _ => false,
            };
            if once && bare && named {
                out.insert(*id);
            }
        }
    }
    out
}

/// The facts of BPMN XML text; `ids` maps XML identifiers to the identifiers tok uses.
pub fn facts(xml: &str, ids: &HashMap<String, String>, original: bool) -> Result<Facts, String> {
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    let opts = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let doc = roxmltree::Document::parse_with_options(xml, opts).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    let mut by_id = HashMap::new();
    let mut outgoing = HashMap::new();
    let mut incoming = HashMap::new();
    for e in root.descendants().filter(|n| n.is_element()) {
        if let Some(id) = e.attribute("id").filter(|v| !v.is_empty()) {
            by_id.entry(id).or_insert(e);
        }
        if is_b(e, "sequenceFlow") {
            if let Some(s) = e.attribute("sourceRef") {
                *outgoing.entry(s).or_insert(0) += 1;
            }
            if let Some(t) = e.attribute("targetRef") {
                *incoming.entry(t).or_insert(0) += 1;
            }
        }
    }
    let plain = plain_io(root, &by_id);
    let mut ex = Ex {
        by_id,
        ids,
        original,
        out: Facts::new(),
        outgoing,
        incoming,
        plain,
    };
    ex.run(root);
    Ok(ex.out)
}

/// Identifier changes the importer reported (`X: identifier changed to 'Y'`,
/// `X: renamed to 'Y' …`).
pub fn id_map(warnings: &[String]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for w in warnings {
        let Some((id, rest)) = w.split_once(": ") else {
            continue;
        };
        for head in ["identifier changed to '", "renamed to '"] {
            if let Some(r) = rest.strip_prefix(head)
                && let Some((new, _)) = r.split_once('\'')
            {
                out.insert(id.to_string(), new.to_string());
            }
        }
    }
    out
}

/// Result of the round trip of one model.
#[derive(Debug, Default, Clone)]
pub struct RoundTrip {
    pub name: String,
    pub warnings: Vec<String>,
    /// facts of the reference model, by category: (total, kept)
    pub counts: BTreeMap<&'static str, (usize, usize)>,
    /// facts of the reference model missing from the export
    pub lost: Vec<(Fact, String)>,
    /// facts of the export missing from the reference model
    pub added: Vec<(Fact, String)>,
    /// lost or added facts no import warning names (by element identifier or vendor tag)
    pub silent: Vec<Fact>,
    pub tok: String,
    pub errors: Vec<String>,
}

impl RoundTrip {
    /// Lost facts that are not by design.
    pub fn losses(&self) -> impl Iterator<Item = &(Fact, String)> {
        self.lost
            .iter()
            .filter(|(f, _)| !BY_DESIGN.contains(&f.cat))
    }
}

/// A warning of the import accounts for the fact: it names the element the fact belongs
/// to, or (for what the import reports once per document) the extension element or
/// attribute, `isImmediate`, the expression language or the import.
fn explained(f: &Fact, subject: &str, warnings: &[String], ids: &HashMap<String, String>) -> bool {
    let mapped = ids.get(subject).map(String::as_str).unwrap_or(subject);
    let named = warnings.iter().any(|w| {
        w.split_once(": ").is_some_and(|(id, text)| {
            id == subject
                || id == mapped
                || text.contains(&format!("'{subject}'"))
                || text.contains(&format!("'{mapped}'"))
        })
    });
    if named {
        return true;
    }
    let mut tags: Vec<String> = Vec::new();
    // a declaration by name: `message "Name"`
    if let Some(rest) = ["message \"", "signal \"", "error \"", "escalation \""]
        .iter()
        .find_map(|p| f.text.strip_prefix(p))
        && let Some((name, _)) = rest.split_once('"')
    {
        tags.push(format!("'{name}'"));
        tags.push(format!("{name}: "));
    }
    if let Some((_, t)) = f
        .text
        .split_once(" extension ")
        .or_else(|| f.text.split_once(" attribute "))
    {
        tags.push(t.to_string());
    }
    if f.cat == "camunda"
        && let Some(z) = f.text.split_whitespace().find(|w| w.starts_with("zeebe:"))
    {
        tags.push(format!("{{{ZEEBE_NS}}}{}", &z[6..]));
        tags.push(z.to_string());
        tags.push("not executable".into());
    }
    if f.text.contains(" isImmediate=") {
        tags.push("isImmediate".into());
    }
    if let Some((_, l)) = f.text.split_once(" language=") {
        tags.push(format!("expression language {l}"));
    }
    if f.text.starts_with("import ") {
        tags.push("import of".into());
    }
    if f.text.starts_with("definitions expressionLanguage=")
        || f.text.starts_with("definitions typeLanguage=")
    {
        tags.push(f.text["definitions ".len()..].replace('=', " "));
    }
    tags.iter()
        .any(|t| warnings.iter().any(|w| w.contains(t.as_str())))
}

/// `tok import` (keeping identifiers) → tok text → parse → semantic BPMN, compared with
/// the reference model.
pub fn round_trip(name: &str, xml: &str) -> RoundTrip {
    let mut r = RoundTrip {
        name: name.to_string(),
        ..Default::default()
    };
    let (doc, warnings) = match tok_core::import::import_bpmn(xml, true) {
        Ok(x) => x,
        Err(e) => {
            r.errors.push(format!("import failed: {e}"));
            return r;
        }
    };
    let text = tok_core::fmt(&doc);
    let (doc2, diags) = tok_core::parse(&text);
    for d in diags.iter().filter(|d| d.is_error()) {
        r.errors.push(format!(
            "parse: {} line {}: {}",
            d.code,
            d.line,
            d.message("en")
        ));
    }
    if tok_core::fmt(&doc2) != text {
        r.errors
            .push("the imported text is not in canonical form".into());
    }
    let out = tok_core::export::semantic_xml(&doc2, None);
    let ids = id_map(&warnings);
    let reverse: HashMap<String, String> =
        ids.iter().map(|(k, v)| (v.clone(), k.clone())).collect();
    let before = match facts(xml, &ids, true) {
        Ok(f) => f,
        Err(e) => {
            r.errors.push(format!("reference model: {e}"));
            return r;
        }
    };
    let after = match facts(&out, &HashMap::new(), false) {
        Ok(f) => f,
        Err(e) => {
            r.errors.push(format!("export: {e}"));
            return r;
        }
    };
    for (f, subj) in &before {
        let e = r.counts.entry(f.cat).or_insert((0, 0));
        e.0 += 1;
        if after.contains_key(f) {
            e.1 += 1;
        } else {
            r.lost.push((f.clone(), subj.clone()));
            if !explained(f, subj, &warnings, &ids) {
                r.silent.push(f.clone());
            }
        }
    }
    for (f, subj) in &after {
        if !before.contains_key(f) && !added_by_design(f) && !titled_by_design(f, &before) {
            r.added.push((f.clone(), subj.clone()));
            let orig = reverse.get(subj).cloned().unwrap_or_else(|| subj.clone());
            if !explained(f, &orig, &warnings, &ids) {
                r.silent.push(f.clone());
            }
        }
    }
    r.warnings = warnings;
    r.tok = text;
    r
}

/// The reference models of `conformance/miwg`, sorted by name.
pub fn models() -> Vec<(String, String)> {
    let dir = miwg_dir();
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .expect("conformance/miwg")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "bpmn"))
        .map(|p| {
            let name = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let bytes = fs::read(&p).expect("model");
            (name, String::from_utf8_lossy(&bytes).into_owned())
        })
        .collect();
    out.sort_by_key(|a| natural(&a.0));
    out
}

/// `C.10.0` after `C.9.2`.
fn natural(name: &str) -> Vec<(String, u64)> {
    name.split('.')
        .map(|p| match p.parse::<u64>() {
            Ok(n) => (String::new(), n),
            Err(_) => (p.to_string(), 0),
        })
        .collect()
}

pub fn miwg_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/miwg")
}

pub fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/miwg/baseline.tsv")
}

/// Baseline lines: `MODEL<TAB>lost|added<TAB>CATEGORY<TAB>FACT`.
pub fn baseline_lines(r: &RoundTrip) -> Vec<String> {
    let mut out = Vec::new();
    for (f, _) in r.losses() {
        out.push(format!("{}\tlost\t{}\t{}", r.name, f.cat, f.text));
    }
    for (f, _) in &r.added {
        out.push(format!("{}\tadded\t{}\t{}", r.name, f.cat, f.text));
    }
    out
}

pub fn read_baseline(path: &Path) -> BTreeSet<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(str::to_string)
        .collect()
}
