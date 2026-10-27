//! Semantic BPMN 2.0 export (no diagram interchange). Mirrors `bpmn_export.py`.

use indexmap::IndexMap;

use crate::model::*;
use crate::pyutil::{ascii_fold, strip};
use crate::xml::{BPMN_NS, MODELER_NS, XElem, XSI_NS, ZEEBE_NS};

fn tag(kind: &str) -> &'static str {
    match kind {
        "start" => "startEvent",
        "end" => "endEvent",
        "catch" => "intermediateCatchEvent",
        "throw" => "intermediateThrowEvent",
        "boundary" => "boundaryEvent",
        "task" => "task",
        "user" => "userTask",
        "service" => "serviceTask",
        "manual" => "manualTask",
        "script" => "scriptTask",
        "rule" => "businessRuleTask",
        "send" => "sendTask",
        "receive" => "receiveTask",
        "call" => "callActivity",
        "sub" | "eventsub" => "subProcess",
        "adhoc" => "adHocSubProcess",
        "xor" => "exclusiveGateway",
        "and" => "parallelGateway",
        "or" => "inclusiveGateway",
        _ => "eventBasedGateway",
    }
}

fn definition(key: &str) -> &'static str {
    match key {
        "message" => "messageEventDefinition",
        "timer" => "timerEventDefinition",
        "signal" => "signalEventDefinition",
        "error" => "errorEventDefinition",
        "escalation" => "escalationEventDefinition",
        "condition" => "conditionalEventDefinition",
        "link" => "linkEventDefinition",
        "terminate" => "terminateEventDefinition",
        _ => "compensateEventDefinition",
    }
}

/// `'Zamrożenie budżetu' -> 'Zamrozenie_budzetu'` (ASCII identifier fragment of a label).
pub fn slug(text: &str) -> String {
    let t = text.replace('ł', "l").replace('Ł', "L");
    let t = ascii_fold(&t);
    let mut out = String::new();
    let mut in_run = false;
    for c in t.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    let t = out.trim_matches('_').to_string();
    match t.chars().next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => t,
        _ => format!("_{t}"),
    }
}

fn timer_kind(v: &str) -> &'static str {
    let v = strip(v);
    if v.starts_with('=') {
        "timeDuration"
    } else if v.starts_with('R') {
        "timeCycle"
    } else if v.starts_with('P') {
        "timeDuration"
    } else {
        "timeDate"
    }
}

const REST_CONNECTOR_TYPE: &str = "io.camunda:http-json:1";

fn binding_type(v: &str) -> &str {
    match v {
        "latest" => "latest",
        "deployment" => "deployment",
        "version-tag" => "versionTag",
        other => other,
    }
}

/// `'a = x; b = y'` (sep `=`) or `'k: v; …'` (sep `:`) into stripped pairs; items without
/// the separator are skipped (Python `split_pairs` of `bpmn_export.py`).
pub fn split_pairs(text: &str, sep: char) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for item in text.split(';') {
        let item = strip(item);
        if item.is_empty() {
            continue;
        }
        if let Some((l, r)) = item.split_once(sep) {
            out.push((strip(l).to_string(), strip(r).to_string()));
        }
    }
    out
}

/// One listener entry `event: type` or `event: type (retries)` (Python regex
/// `^([a-z]+)\s*:\s*(.+?)(?:\s*\((\d+)\))?$` on the stripped item).
pub fn parse_listener(item: &str) -> Option<(String, String, Option<String>)> {
    let item = strip(item);
    let ev_len = item.bytes().take_while(u8::is_ascii_lowercase).count();
    if ev_len == 0 {
        return None;
    }
    let event = &item[..ev_len];
    let rest = item[ev_len..].trim_start_matches(crate::pyutil::is_space);
    let rest = rest.strip_prefix(':')?;
    let rest = rest.trim_start_matches(crate::pyutil::is_space);
    if rest.is_empty() || rest.contains('\n') {
        return None;
    }
    // optional "(digits)" suffix, preceded by optional whitespace; the type keeps at least one char
    if let Some(body) = rest.strip_suffix(')')
        && let Some(open) = body.rfind('(')
    {
        let digits = &body[open + 1..];
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            let before = body[..open].trim_end_matches(crate::pyutil::is_space);
            if !before.is_empty() {
                return Some((
                    event.to_string(),
                    strip(before).to_string(),
                    Some(digits.to_string()),
                ));
            }
        }
    }
    Some((event.to_string(), strip(rest).to_string(), None))
}

#[derive(Debug, Clone)]
struct Decl {
    id: String,
    code: Option<String>,
    correlation: Option<String>,
}

fn nonempty(s: Option<&str>) -> Option<&str> {
    s.filter(|x| !x.is_empty())
}

/// Path of a container (process or sub-process) inside the root element.
type Path = Vec<usize>;

struct Exporter<'a> {
    doc: &'a Document,
    camunda: bool,
    decls: [IndexMap<String, Decl>; 4],
    stores: IndexMap<String, String>,
    containers: IndexMap<String, Path>,
    root: XElem,
    collab: Option<Path>,
}

const DECL_KINDS: [&str; 4] = ["message", "signal", "error", "escalation"];

fn bsub(parent: &mut XElem, name: &str, attrs: &[(&str, Option<&str>)]) -> usize {
    let mut e = XElem::new(BPMN_NS, name);
    for (k, v) in attrs {
        if let Some(v) = v {
            e.set(k, *v);
        }
    }
    parent.push(e)
}

fn ext_index(e: &mut XElem) -> usize {
    if let Some(k) = e
        .children
        .iter()
        .position(|c| c.ns == BPMN_NS && c.name == "extensionElements")
    {
        return k;
    }
    e.insert(0, XElem::new(BPMN_NS, "extensionElements"));
    0
}

fn zsub(parent: &mut XElem, name: &str, attrs: &[(&str, &str)]) {
    let mut e = XElem::new(ZEEBE_NS, name);
    for (k, v) in attrs {
        e.set(k, *v);
    }
    parent.push(e);
}

impl<'a> Exporter<'a> {
    fn new(doc: &'a Document, target: Option<&str>) -> Self {
        let t = target
            .map(str::to_string)
            .or_else(|| doc.attrs.get("target").cloned())
            .unwrap_or_else(|| "camunda".into());
        Exporter {
            doc,
            camunda: doc.executable_any() && t == "camunda",
            decls: Default::default(),
            stores: IndexMap::new(),
            containers: IndexMap::new(),
            root: XElem::new(BPMN_NS, "definitions"),
            collab: None,
        }
    }

    fn feel(&self, expr: &str) -> String {
        if self.camunda {
            format!("={expr}")
        } else {
            expr.to_string()
        }
    }

    fn declare(&mut self, kind: &str, name: &str, code: Option<&str>) -> String {
        let k = DECL_KINDS.iter().position(|x| *x == kind).unwrap_or(0);
        let reg = &mut self.decls[k];
        if let Some(d) = reg.get_mut(name) {
            if let Some(c) = nonempty(code)
                && d.code.as_deref().map(|x| x.is_empty()).unwrap_or(true)
            {
                d.code = Some(c.to_string());
            }
            return d.id.clone();
        }
        let prefix = ["Message", "Signal", "Error", "Escalation"][k];
        let id = format!("{prefix}_{}", slug(name));
        reg.insert(
            name.to_string(),
            Decl {
                id: id.clone(),
                code: code.map(str::to_string),
                correlation: None,
            },
        );
        id
    }

    fn at(&mut self, path: &[usize]) -> &mut XElem {
        let mut e = &mut self.root;
        for &k in path {
            e = &mut e.children[k];
        }
        e
    }

    fn run(mut self) -> XElem {
        let d = self.doc;
        self.root.set(
            "id",
            d.attrs
                .get("id")
                .cloned()
                .unwrap_or_else(|| "definitions".into()),
        );
        if let Some(t) = nonempty(d.title.as_deref()) {
            self.root.set("name", t);
        }
        self.root.set(
            "targetNamespace",
            d.attrs
                .get("namespace")
                .cloned()
                .unwrap_or_else(|| "http://bpmn.io/schema/bpmn".into()),
        );
        self.root.set("exporter", "tok");
        self.root.set(
            "exporterVersion",
            d.version.clone().unwrap_or_else(|| "0.1".into()),
        );
        if self.camunda {
            self.root
                .set_ns(MODELER_NS, "executionPlatform", "Camunda Cloud");
            self.root
                .set_ns(MODELER_NS, "executionPlatformVersion", "8.10.0");
        }
        let decl_slot = self.root.children.len();

        let mut processes: Vec<(String, Option<String>, bool, Option<usize>)> = Vec::new();
        if !d.pools.is_empty() {
            let collab_id = d
                .attrs
                .get("collaboration")
                .cloned()
                .unwrap_or_else(|| "collaboration".into());
            let ci = bsub(&mut self.root, "collaboration", &[("id", Some(&collab_id))]);
            let inner = d.pools.iter().filter(|p| !p.blackbox).count();
            for (pi, p) in d.pools.iter().enumerate() {
                let part = {
                    let collab = &mut self.root.children[ci];
                    bsub(
                        collab,
                        "participant",
                        &[
                            ("id", p.base.id.as_deref()),
                            ("name", p.base.label.as_deref()),
                        ],
                    )
                };
                if !p.blackbox {
                    let pid = p.process_id();
                    self.root.children[ci].children[part].set("processRef", pid.clone());
                    let pname = if inner == 1 && nonempty(d.title.as_deref()).is_some() {
                        d.title.clone()
                    } else {
                        p.base.label.clone()
                    };
                    processes.push((
                        pid,
                        pname,
                        p.base.has("executable") || d.executable(),
                        Some(pi),
                    ));
                }
            }
            for m in &d.messages {
                let mf = {
                    let collab = &mut self.root.children[ci];
                    bsub(
                        collab,
                        "messageFlow",
                        &[
                            ("id", m.base.id.as_deref()),
                            ("name", m.base.label.as_deref()),
                            ("sourceRef", Some(&m.source)),
                            ("targetRef", Some(&m.target)),
                        ],
                    )
                };
                let name = nonempty(m.base.attr("message"))
                    .or(m.base.label.as_deref())
                    .map(str::to_string);
                if let Some(name) = name.filter(|n| !n.is_empty()) {
                    let id = self.declare("message", &name, None);
                    self.root.children[ci].children[mf].set("messageRef", id);
                }
            }
            self.collab = Some(vec![ci]);
        } else {
            processes.push((
                d.default_process_id(),
                d.title.clone(),
                d.executable(),
                None,
            ));
        }

        let n_processes = processes.len();
        for (pid, name, executable, pool) in processes {
            let pi = bsub(
                &mut self.root,
                "process",
                &[
                    ("id", Some(&pid)),
                    ("name", name.as_deref()),
                    (
                        "isExecutable",
                        Some(if executable { "true" } else { "false" }),
                    ),
                ],
            );
            self.containers.insert(pid.clone(), vec![pi]);
            let tag = pool
                .and_then(|px| d.pools[px].base.attrs.get("version-tag").cloned())
                .filter(|t| !t.is_empty())
                .or_else(|| {
                    if pool.is_none() || n_processes == 1 {
                        d.attrs.get("version-tag").cloned()
                    } else {
                        None
                    }
                })
                .filter(|t| !t.is_empty());
            if self.camunda
                && executable
                && let Some(tag) = tag
            {
                let x = ext_index(&mut self.root.children[pi]);
                zsub(
                    &mut self.root.children[pi].children[x],
                    "versionTag",
                    &[("value", &tag)],
                );
            }
            match pool {
                Some(px) => {
                    let pool = &d.pools[px];
                    let lanes: Vec<&Lane> = pool
                        .children
                        .iter()
                        .filter_map(|c| d.lanes.get(c))
                        .collect();
                    if !lanes.is_empty() {
                        let mut ls = XElem::new(BPMN_NS, "laneSet");
                        ls.set("id", format!("{}-lanes", pool.base.id()));
                        for lane in lanes {
                            self.lane(&mut ls, lane);
                        }
                        self.root.children[pi].push(ls);
                    }
                    let ids = self.ordered_nodes(&pool.children);
                    self.block(vec![pi], &ids);
                }
                None => {
                    let ids = d.top.clone();
                    self.block(vec![pi], &ids);
                }
            }
        }

        self.flows();
        self.data();
        self.artifacts();
        self.compensation_links();
        self.order_adhoc();

        let mut decl = Vec::new();
        for (k, kind) in DECL_KINDS.iter().enumerate() {
            for (name, info) in &self.decls[k] {
                let mut e = XElem::new(BPMN_NS, kind);
                e.set("id", info.id.clone());
                e.set("name", name.clone());
                if let Some(code) = nonempty(info.code.as_deref()) {
                    if *kind == "error" {
                        e.set("errorCode", code);
                    }
                    if *kind == "escalation" {
                        e.set("escalationCode", code);
                    }
                }
                if *kind == "message"
                    && self.camunda
                    && let Some(corr) = nonempty(info.correlation.as_deref())
                {
                    let mut x = XElem::new(BPMN_NS, "extensionElements");
                    zsub(
                        &mut x,
                        "subscription",
                        &[("correlationKey", &self.feel(corr))],
                    );
                    e.push(x);
                }
                decl.push(e);
            }
        }
        for (name, sid) in &self.stores {
            let mut e = XElem::new(BPMN_NS, "dataStore");
            e.set("id", sid.clone());
            e.set("name", name.clone());
            decl.push(e);
        }
        for (k, e) in decl.into_iter().enumerate() {
            self.root.insert(decl_slot + k, e);
        }
        self.root
    }

    fn ordered_nodes(&self, children: &[String]) -> Vec<String> {
        fn walk(d: &Document, ids: &[String], out: &mut Vec<String>) {
            for i in ids {
                if let Some(l) = d.lanes.get(i) {
                    walk(d, &l.children, out);
                } else if d.nodes.contains_key(i) {
                    out.push(i.clone());
                }
            }
        }
        let mut out = Vec::new();
        walk(self.doc, children, &mut out);
        out
    }

    fn lane(&self, parent: &mut XElem, lane: &Lane) {
        let d = self.doc;
        let mut e = XElem::new(BPMN_NS, "lane");
        if let Some(id) = &lane.base.id {
            e.set("id", id.clone());
        }
        if let Some(l) = &lane.base.label {
            e.set("name", l.clone());
        }
        for c in &lane.children {
            if let Some(node) = d.nodes.get(c) {
                let mut r = XElem::new(BPMN_NS, "flowNodeRef");
                r.text = Some(c.clone());
                e.push(r);
                if node.base.attrs.contains_key("compensate") {
                    let mut r = XElem::new(BPMN_NS, "flowNodeRef");
                    r.text = Some(compensation_id(c));
                    e.push(r);
                }
            }
        }
        let sublanes: Vec<&Lane> = lane
            .children
            .iter()
            .filter_map(|c| d.lanes.get(c))
            .collect();
        if !sublanes.is_empty() {
            let mut cls = XElem::new(BPMN_NS, "childLaneSet");
            cls.set("id", format!("{}-lanes", lane.base.id()));
            for s in sublanes {
                self.lane(&mut cls, s);
            }
            e.push(cls);
        }
        parent.push(e);
    }

    fn block(&mut self, container: Path, ids: &[String]) {
        for i in ids {
            if let Some(n) = self.doc.nodes.get(i) {
                self.node(container.clone(), n);
            }
        }
    }

    fn node(&mut self, container: Path, n: &Node) {
        let a = &n.base.attrs;
        let mut e = XElem::new(BPMN_NS, tag(&n.base.kind));
        if let Some(id) = &n.base.id {
            e.set("id", id.clone());
        }
        if let Some(l) = &n.base.label {
            e.set("name", l.clone());
        }
        let kind = n.base.kind.as_str();
        if kind == "boundary" {
            e.set("attachedToRef", a.get("on").cloned().unwrap_or_default());
            if n.base.has("noninterrupting") {
                e.set("cancelActivity", "false");
            }
        }
        if kind == "start" && n.base.has("noninterrupting") {
            e.set("isInterrupting", "false");
        }
        if kind == "eventsub" {
            e.set("triggeredByEvent", "true");
        }
        if kind == "call"
            && let Some(p) = a.get("process")
            && !p.starts_with('=')
        {
            e.set("calledElement", p.clone());
        }
        if kind == "adhoc" {
            if n.base.has("sequential") {
                e.set("ordering", "Sequential");
            }
            if self.camunda && n.base.has("wait") {
                e.set("cancelRemainingInstances", "false");
            }
        }
        if n.base.has("compensation") {
            e.set("isForCompensation", "true");
        }
        if (kind == "send" || kind == "receive")
            && let Some(m) = a.get("message")
        {
            let id = self.declare("message", m, None);
            e.set("messageRef", id);
        }
        if kind == "script"
            && !self.camunda
            && let Some(x) = a.get("expr")
        {
            e.set("scriptFormat", "feel");
            let mut s = XElem::new(BPMN_NS, "script");
            s.text = Some(x.clone());
            e.push(s);
        }
        if self.camunda {
            self.zeebe(&mut e, n);
        }
        if is_event(kind) {
            self.event_definition(&mut e, n);
        }
        if n.base.has("loop") {
            e.push(XElem::new(BPMN_NS, "standardLoopCharacteristics"));
        } else if kind != "adhoc" && (n.base.has("parallel") || n.base.has("sequential")) {
            let mut mi = XElem::new(BPMN_NS, "multiInstanceLoopCharacteristics");
            mi.set(
                "isSequential",
                if n.base.has("sequential") {
                    "true"
                } else {
                    "false"
                },
            );
            if self.camunda
                && let Some(coll) = a.get("collection")
            {
                let mut lc: Vec<(&str, String)> = vec![("inputCollection", self.feel(coll))];
                if let Some(v) = a.get("item") {
                    lc.push(("inputElement", v.clone()));
                }
                if let Some(v) = a.get("output") {
                    lc.push(("outputCollection", v.clone()));
                }
                if let Some(v) = a.get("output-item") {
                    lc.push(("outputElement", self.feel(v)));
                }
                let x = ext_index(&mut mi);
                let pairs: Vec<(&str, &str)> = lc.iter().map(|(k, v)| (*k, v.as_str())).collect();
                zsub(&mut mi.children[x], "loopCharacteristics", &pairs);
            }
            if self.camunda
                && let Some(u) = a.get("until")
            {
                mi.push(self.formal("completionCondition", u));
            }
            e.push(mi);
        }
        let idx = self.at(&container).push(e);
        if is_sub(kind) {
            let mut path = container.clone();
            path.push(idx);
            self.containers
                .insert(n.base.id().to_string(), path.clone());
            let children = n.children.clone();
            self.block(path.clone(), &children);
            if kind == "adhoc"
                && self.camunda
                && let Some(u) = a.get("until")
            {
                let cc = self.formal("completionCondition", u);
                self.at(&path).push(cc);
            }
        }
        if a.contains_key("compensate") {
            let bid = compensation_id(n.base.id());
            let mut b = XElem::new(BPMN_NS, "boundaryEvent");
            b.set("id", bid.clone());
            b.set("attachedToRef", n.base.id().to_string());
            let mut df = XElem::new(BPMN_NS, "compensateEventDefinition");
            df.set("id", format!("{bid}-def"));
            b.push(df);
            self.at(&container).push(b);
        }
    }

    /// `<bpmn:NAME xsi:type="bpmn:tFormalExpression">=expr</bpmn:NAME>`
    fn formal(&self, name: &str, expr: &str) -> XElem {
        let mut c = XElem::new(BPMN_NS, name);
        c.set_ns(XSI_NS, "type", "bpmn:tFormalExpression");
        c.text = Some(self.feel(expr));
        c
    }

    fn node_executable(&self, n: &Node) -> bool {
        let d = self.doc;
        if d.executable() {
            return true;
        }
        d.pools
            .iter()
            .any(|p| p.base.id.as_deref() == n.pool.as_deref() && p.base.has("executable"))
    }

    /// Associations from generated compensation boundary events to their handlers.
    fn compensation_links(&mut self) {
        let d = self.doc;
        for n in d.nodes.values() {
            let Some(h) = n.base.attrs.get("compensate") else {
                continue;
            };
            if !d.nodes.contains_key(h) {
                continue;
            }
            let Some(path) = n
                .scope
                .as_ref()
                .and_then(|s| self.containers.get(s))
                .cloned()
            else {
                continue;
            };
            let bid = compensation_id(n.base.id());
            let mut a = XElem::new(BPMN_NS, "association");
            a.set("id", format!("{bid}-assoc"));
            a.set("associationDirection", "One");
            a.set("sourceRef", bid);
            a.set("targetRef", h.clone());
            self.at(&path).push(a);
        }
    }

    /// `completionCondition` is the last child of an ad-hoc sub-process (BPMN schema order).
    fn order_adhoc(&mut self) {
        fn walk(e: &mut XElem) {
            if e.ns == BPMN_NS && e.name == "adHocSubProcess" {
                let mut moved = Vec::new();
                let mut k = 0;
                while k < e.children.len() {
                    if e.children[k].ns == BPMN_NS && e.children[k].name == "completionCondition" {
                        moved.push(e.children.remove(k));
                    } else {
                        k += 1;
                    }
                }
                e.children.extend(moved);
            }
            for c in &mut e.children {
                walk(c);
            }
        }
        walk(&mut self.root);
    }

    fn binding_attrs(attrs: &mut Vec<(&'static str, String)>, a: &Attrs) {
        if let Some(b) = a.get("binding") {
            attrs.push(("bindingType", binding_type(b).to_string()));
        }
        if let Some(v) = a.get("version") {
            attrs.push(("versionTag", v.clone()));
        }
    }

    fn zsub_owned(parent: &mut XElem, name: &str, attrs: &[(&'static str, String)]) {
        let pairs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        zsub(parent, name, &pairs);
    }

    fn zeebe(&mut self, e: &mut XElem, n: &Node) {
        if !self.node_executable(n) {
            // only the 0.1 executable profile applies to processes that are not executable
            self.zeebe_01(e, n);
            return;
        }
        let a = &n.base.attrs;
        let kind = n.base.kind.as_str();
        if kind == "user" {
            let x = ext_index(e);
            zsub(&mut e.children[x], "userTask", &[]);
            if let Some(f) = a.get("form") {
                let mut at = vec![("formId", f.clone())];
                Self::binding_attrs(&mut at, a);
                Self::zsub_owned(&mut e.children[x], "formDefinition", &at);
            }
            let mut assign = Vec::new();
            for (key, attr) in [
                ("assignee", "assignee"),
                ("candidate-users", "candidateUsers"),
                ("candidate-groups", "candidateGroups"),
            ] {
                if let Some(v) = a.get(key) {
                    assign.push((attr, v.clone()));
                }
            }
            if !assign.is_empty() {
                Self::zsub_owned(&mut e.children[x], "assignmentDefinition", &assign);
            }
            let mut sched = Vec::new();
            for (key, attr) in [("due", "dueDate"), ("follow-up", "followUpDate")] {
                if let Some(v) = a.get(key) {
                    sched.push((attr, v.clone()));
                }
            }
            if !sched.is_empty() {
                Self::zsub_owned(&mut e.children[x], "taskSchedule", &sched);
            }
            if let Some(p) = a.get("priority") {
                zsub(&mut e.children[x], "priorityDefinition", &[("priority", p)]);
            }
        } else if kind == "start" && a.contains_key("form") {
            let mut at = vec![("formId", a["form"].clone())];
            Self::binding_attrs(&mut at, a);
            let x = ext_index(e);
            Self::zsub_owned(&mut e.children[x], "formDefinition", &at);
        } else if kind == "rule" && a.contains_key("decision") {
            let res = match nonempty(a.get("result").map(String::as_str)) {
                Some(r) => r.to_string(),
                None => self.default_result(&a["decision"]),
            };
            let mut at = vec![
                ("decisionId", a["decision"].clone()),
                ("resultVariable", res),
            ];
            Self::binding_attrs(&mut at, a);
            let x = ext_index(e);
            Self::zsub_owned(&mut e.children[x], "calledDecision", &at);
        } else if kind == "script" && a.contains_key("expr") {
            let expr = self.feel(&a["expr"]);
            let res = a.get("result").cloned().unwrap_or_else(|| "result".into());
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "script",
                &[("expression", &expr), ("resultVariable", &res)],
            );
        } else if kind == "call" && a.contains_key("process") {
            let mut ce = vec![("processId", a["process"].clone())];
            Self::binding_attrs(&mut ce, a);
            if a.get("child-vars").map(String::as_str) == Some("none") {
                ce.push(("propagateAllChildVariables", "false".into()));
            }
            if a.get("parent-vars").map(String::as_str) == Some("none") {
                ce.push(("propagateAllParentVariables", "false".into()));
            }
            if let Some(b) = a.get("business-id") {
                ce.push(("businessId", b.clone()));
            }
            let x = ext_index(e);
            Self::zsub_owned(&mut e.children[x], "calledElement", &ce);
        } else if kind == "adhoc" && a.contains_key("active") {
            let v = self.feel(&a["active"]);
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "adHoc",
                &[("activeElementsCollection", &v)],
            );
        }
        let job_kind = kind == "service"
            || kind == "send"
            || ((kind == "throw" || kind == "end") && a.contains_key("message"));
        let inputs = split_pairs(a.get("inputs").map(String::as_str).unwrap_or(""), '=');
        let mut headers = split_pairs(a.get("headers").map(String::as_str).unwrap_or(""), ':');
        if kind == "service" && a.get("connector").map(String::as_str) == Some("rest") {
            let mut td = vec![("type", REST_CONNECTOR_TYPE.to_string())];
            if let Some(r) = a.get("retries") {
                td.push(("retries", r.clone()));
            }
            let x = ext_index(e);
            Self::zsub_owned(&mut e.children[x], "taskDefinition", &td);
            let mut conn: Vec<(String, String)> = vec![
                ("authentication.type".into(), "noAuth".into()),
                (
                    "method".into(),
                    a.get("method").cloned().unwrap_or_else(|| "GET".into()),
                ),
                ("url".into(), a.get("url").cloned().unwrap_or_default()),
            ];
            for (key, target) in [
                ("headers-in", "headers"),
                ("query", "queryParameters"),
                ("body", "body"),
            ] {
                if let Some(v) = a.get(key) {
                    conn.push((target.into(), v.clone()));
                }
            }
            Self::io_mapping(e, &conn, &[]);
            if let Some(r) = a.get("result") {
                headers.insert(0, ("resultVariable".into(), r.clone()));
            }
        } else if job_kind && a.contains_key("type") {
            let mut td = vec![("type", a["type"].clone())];
            if let Some(r) = a.get("retries") {
                td.push(("retries", r.clone()));
            }
            let x = ext_index(e);
            Self::zsub_owned(&mut e.children[x], "taskDefinition", &td);
        }
        if !inputs.is_empty() || a.contains_key("outputs") {
            let ins: Vec<(String, String)> = inputs
                .iter()
                .map(|(t, s)| (t.clone(), self.feel(s)))
                .collect();
            let outs: Vec<(String, String)> =
                split_pairs(a.get("outputs").map(String::as_str).unwrap_or(""), '=')
                    .iter()
                    .map(|(t, s)| (t.clone(), self.feel(s)))
                    .collect();
            Self::io_mapping(e, &ins, &outs);
        }
        if !headers.is_empty() {
            let x = ext_index(e);
            let mut th = XElem::new(ZEEBE_NS, "taskHeaders");
            for (k, v) in &headers {
                let mut h = XElem::new(ZEEBE_NS, "header");
                h.set("key", k.clone());
                h.set("value", v.clone());
                th.push(h);
            }
            e.children[x].push(th);
        }
        if let Some(ev) = a.get("events")
            && matches!(kind, "start" | "catch" | "boundary")
        {
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "conditionalFilter",
                &[("variableEvents", ev)],
            );
        }
        if let Some(text) = a.get("listeners") {
            let mut execl = Vec::new();
            let mut taskl = Vec::new();
            for item in text.split(';') {
                if let Some((ev, typ, retries)) = parse_listener(item) {
                    let mut li: Vec<(&'static str, String)> =
                        vec![("eventType", ev.clone()), ("type", typ)];
                    if let Some(r) = retries {
                        li.push(("retries", r));
                    }
                    if ev == "start" || ev == "end" {
                        execl.push(li);
                    } else {
                        taskl.push(li);
                    }
                }
            }
            for (box_name, item_name, items) in [
                ("executionListeners", "executionListener", execl),
                ("taskListeners", "taskListener", taskl),
            ] {
                if items.is_empty() {
                    continue;
                }
                let x = ext_index(e);
                let mut bx = XElem::new(ZEEBE_NS, box_name);
                for li in &items {
                    let mut it = XElem::new(ZEEBE_NS, item_name);
                    for (k, v) in li {
                        it.set(k, v.clone());
                    }
                    bx.push(it);
                }
                e.children[x].push(bx);
            }
        }
    }

    fn io_mapping(e: &mut XElem, inputs: &[(String, String)], outputs: &[(String, String)]) {
        let x = ext_index(e);
        let ext = &mut e.children[x];
        let k = match ext
            .children
            .iter()
            .position(|c| c.ns == ZEEBE_NS && c.name == "ioMapping")
        {
            Some(k) => k,
            None => ext.push(XElem::new(ZEEBE_NS, "ioMapping")),
        };
        let bx = &mut ext.children[k];
        for (target, source) in inputs {
            let mut i = XElem::new(ZEEBE_NS, "input");
            i.set("source", source.clone());
            i.set("target", target.clone());
            bx.push(i);
        }
        for (target, source) in outputs {
            let mut o = XElem::new(ZEEBE_NS, "output");
            o.set("source", source.clone());
            o.set("target", target.clone());
            bx.push(o);
        }
    }

    /// The executable profile of language 0.1 (used where the process itself is not executable).
    fn zeebe_01(&mut self, e: &mut XElem, n: &Node) {
        let a = &n.base.attrs;
        let kind = n.base.kind.as_str();
        if kind == "user" {
            let x = ext_index(e);
            zsub(&mut e.children[x], "userTask", &[]);
            if let Some(f) = a.get("form") {
                zsub(&mut e.children[x], "formDefinition", &[("formId", f)]);
            }
        } else if kind == "start" && a.contains_key("form") {
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "formDefinition",
                &[("formId", &a["form"])],
            );
        } else if kind == "rule" && a.contains_key("decision") {
            let res = match nonempty(a.get("result").map(String::as_str)) {
                Some(r) => r.to_string(),
                None => self.default_result(&a["decision"]),
            };
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "calledDecision",
                &[("decisionId", &a["decision"]), ("resultVariable", &res)],
            );
        } else if kind == "script" && a.contains_key("expr") {
            let expr = self.feel(&a["expr"]);
            let res = a.get("result").cloned().unwrap_or_else(|| "result".into());
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "script",
                &[("expression", &expr), ("resultVariable", &res)],
            );
        } else if (kind == "service" || kind == "send") && a.contains_key("type") {
            let x = ext_index(e);
            zsub(
                &mut e.children[x],
                "taskDefinition",
                &[("type", &a["type"])],
            );
        }
    }

    fn default_result(&self, decision: &str) -> String {
        match self.doc.decisions.get(decision) {
            Some(d) if d.outputs.len() == 1 => d.outputs[0].0.clone(),
            _ => "result".into(),
        }
    }

    fn event_definition(&mut self, e: &mut XElem, n: &Node) {
        let a = &n.base.attrs;
        let def_id = format!("{}-def", n.base.id());
        for key in TRIGGERS {
            let Some(v) = a.get(*key) else { continue };
            let mut df = XElem::new(BPMN_NS, definition(key));
            df.set("id", def_id.clone());
            match *key {
                "message" => {
                    let id = self.declare("message", v, None);
                    if let Some(c) = a.get("correlation") {
                        let k = 0;
                        if let Some(info) = self.decls[k].get_mut(v.as_str()) {
                            info.correlation = Some(c.clone());
                        }
                    }
                    df.set("messageRef", id);
                }
                "signal" => {
                    let id = self.declare("signal", v, None);
                    df.set("signalRef", id);
                }
                "error" => {
                    let id = self.declare("error", v, a.get("code").map(String::as_str));
                    df.set("errorRef", id);
                }
                "escalation" => {
                    let id = self.declare("escalation", v, a.get("code").map(String::as_str));
                    df.set("escalationRef", id);
                }
                "timer" => {
                    let mut t = XElem::new(BPMN_NS, timer_kind(v));
                    t.set_ns(XSI_NS, "type", "bpmn:tFormalExpression");
                    t.text = Some(v.clone());
                    df.push(t);
                }
                "condition" => {
                    let mut c = XElem::new(BPMN_NS, "condition");
                    c.set_ns(XSI_NS, "type", "bpmn:tFormalExpression");
                    c.text = Some(self.feel(v));
                    df.push(c);
                }
                _ => df.set("name", v.clone()),
            }
            e.push(df);
            return;
        }
        for flag in TRIGGER_FLAGS.iter().chain(CATCH_ALL_FLAGS) {
            if n.base.has(flag) {
                let mut df = XElem::new(BPMN_NS, definition(flag));
                df.set("id", def_id);
                if *flag == "compensate"
                    && let Some(act) = a.get("activity")
                {
                    df.set("activityRef", act.clone());
                }
                e.push(df);
                return;
            }
        }
    }

    fn flows(&mut self) {
        let d = self.doc;
        for f in &d.flows {
            let (Some(src), Some(tgt)) = (d.nodes.get(&f.source), d.nodes.get(&f.target)) else {
                continue;
            };
            if src.scope != tgt.scope {
                continue;
            }
            let Some(path) = src
                .scope
                .as_ref()
                .and_then(|s| self.containers.get(s))
                .cloned()
            else {
                continue;
            };
            let mut e = XElem::new(BPMN_NS, "sequenceFlow");
            if let Some(id) = &f.base.id {
                e.set("id", id.clone());
            }
            if let Some(l) = &f.base.label {
                e.set("name", l.clone());
            }
            e.set("sourceRef", f.source.clone());
            e.set("targetRef", f.target.clone());
            if let Some(w) = f.base.attrs.get("when") {
                let mut c = XElem::new(BPMN_NS, "conditionExpression");
                c.set_ns(XSI_NS, "type", "bpmn:tFormalExpression");
                c.text = Some(self.feel(w));
                e.push(c);
            }
            self.at(&path).push(e);
            if f.base.has("default")
                && let Some(p) = self.root.find_path(src.base.id())
                && let Some(el) = self.root.at_mut(&p)
            {
                el.set("default", f.base.id().to_string());
            }
        }
    }

    fn data_scope(&self, x: &Data) -> String {
        let d = self.doc;
        if let Some(s) = nonempty(x.scope.as_deref()) {
            return s.to_string();
        }
        for a in &d.assocs {
            if a.data == x.base.id()
                && let Some(n) = d.nodes.get(&a.activity)
            {
                return n.scope.clone().unwrap_or_default();
            }
        }
        for p in &d.pools {
            if !p.blackbox {
                return p.process_id();
            }
        }
        d.default_process_id()
    }

    fn data(&mut self) {
        let d = self.doc;
        for x in d.data.values() {
            let scope = self.data_scope(x);
            let Some(path) = self.containers.get(&scope).cloned() else {
                continue;
            };
            let id = x.base.id().to_string();
            if x.base.kind == "data" {
                let mut obj = XElem::new(BPMN_NS, "dataObject");
                obj.set("id", format!("{id}-object"));
                if let Some(l) = &x.base.label {
                    obj.set("name", l.clone());
                }
                if x.base.has("collection") {
                    obj.set("isCollection", "true");
                }
                let mut r = XElem::new(BPMN_NS, "dataObjectReference");
                r.set("id", id.clone());
                if let Some(l) = &x.base.label {
                    r.set("name", l.clone());
                }
                r.set("dataObjectRef", format!("{id}-object"));
                if let Some(st) = x.base.attrs.get("state") {
                    let mut s = XElem::new(BPMN_NS, "dataState");
                    s.set("id", format!("{id}-state"));
                    s.set("name", st.clone());
                    r.push(s);
                }
                let c = self.at(&path);
                c.push(obj);
                c.push(r);
            } else {
                let name = nonempty(x.base.label.as_deref()).unwrap_or(&id).to_string();
                let sid = self
                    .stores
                    .entry(name)
                    .or_insert_with(|| format!("{id}-store"))
                    .clone();
                let mut r = XElem::new(BPMN_NS, "dataStoreReference");
                r.set("id", id.clone());
                if let Some(l) = &x.base.label {
                    r.set("name", l.clone());
                }
                r.set("dataStoreRef", sid);
                self.at(&path).push(r);
            }
        }
        let mut n = 0;
        for a in &d.assocs {
            let Some(path) = self.root.find_path(&a.activity) else {
                continue;
            };
            if !d.data.contains_key(&a.data) {
                continue;
            }
            n += 1;
            let Some(act) = self.root.at_mut(&path) else {
                continue;
            };
            let pos = if act
                .children
                .iter()
                .any(|c| c.ns == BPMN_NS && c.name == "extensionElements")
            {
                1
            } else {
                0
            };
            if a.base.kind == "reads" {
                let prop_id = format!("{}-in-{n}", a.activity);
                let mut prop = XElem::new(BPMN_NS, "property");
                prop.set("id", prop_id.clone());
                prop.set("name", "__targetRef_placeholder");
                let mut assoc = XElem::new(BPMN_NS, "dataInputAssociation");
                assoc.set("id", format!("{}-dia-{n}", a.activity));
                let mut s = XElem::new(BPMN_NS, "sourceRef");
                s.text = Some(a.data.clone());
                assoc.push(s);
                let mut t = XElem::new(BPMN_NS, "targetRef");
                t.text = Some(prop_id);
                assoc.push(t);
                act.insert(pos, prop);
                let ap = after_props(act);
                act.insert(ap, assoc);
            } else {
                let mut assoc = XElem::new(BPMN_NS, "dataOutputAssociation");
                assoc.set("id", format!("{}-doa-{n}", a.activity));
                let mut t = XElem::new(BPMN_NS, "targetRef");
                t.text = Some(a.data.clone());
                assoc.push(t);
                let ap = after_props(act);
                act.insert(ap, assoc);
            }
        }
    }

    fn first_container(&self) -> Option<Path> {
        self.containers.values().next().cloned()
    }

    fn artifacts(&mut self) {
        let d = self.doc;
        for note in &d.notes {
            let target = note.attrs.get("on").cloned();
            let mut cont = None;
            if let Some(t) = &target
                && let Some(node) = d.nodes.get(t)
            {
                cont = node
                    .scope
                    .as_ref()
                    .and_then(|s| self.containers.get(s))
                    .cloned();
            }
            if cont.is_none() {
                cont = self.collab.clone().or_else(|| self.first_container());
            }
            let Some(path) = cont else { continue };
            let mut t = XElem::new(BPMN_NS, "textAnnotation");
            t.set("id", note.id().to_string());
            let mut tx = XElem::new(BPMN_NS, "text");
            tx.text = Some(note.label.clone().unwrap_or_default());
            t.push(tx);
            let c = self.at(&path);
            c.push(t);
            if let Some(tg) = target.filter(|t| !t.is_empty()) {
                let mut a = XElem::new(BPMN_NS, "association");
                a.set("id", format!("{}-assoc", note.id()));
                a.set("sourceRef", note.id().to_string());
                a.set("targetRef", tg);
                c.push(a);
            }
        }
        if !d.groups.is_empty() {
            let mut cat = XElem::new(BPMN_NS, "category");
            cat.set("id", "categories");
            for g in &d.groups {
                let mut cv = XElem::new(BPMN_NS, "categoryValue");
                let cvid = format!("{}-value", g.id());
                cv.set("id", cvid.clone());
                cv.set(
                    "value",
                    nonempty(g.label.as_deref()).unwrap_or(g.id()).to_string(),
                );
                cat.push(cv);
                let path = self
                    .collab
                    .clone()
                    .or_else(|| self.first_container())
                    .unwrap_or_default();
                let mut ge = XElem::new(BPMN_NS, "group");
                ge.set("id", g.id().to_string());
                ge.set("categoryValueRef", cvid);
                self.at(&path).push(ge);
            }
            self.root.insert(0, cat);
        }
    }
}

fn compensation_id(activity_id: &str) -> String {
    format!("{activity_id}-compensate")
}

fn after_props(act: &XElem) -> usize {
    let mut pos = 0;
    for (k, c) in act.children.iter().enumerate() {
        if c.ns == BPMN_NS
            && matches!(
                c.name.as_str(),
                "extensionElements" | "property" | "dataInputAssociation" | "dataOutputAssociation"
            )
        {
            pos = k + 1;
        }
    }
    pos
}

/// The semantic BPMN tree (attributes in insertion order, no layout).
pub fn export_tree(doc: &Document, target: Option<&str>) -> XElem {
    Exporter::new(doc, target).run()
}

/// The exported tree serialized without indentation, as `ET.tostring(root)` would print it.
pub fn export_xml(doc: &Document, target: Option<&str>) -> String {
    let root = export_tree(doc, target);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}",
        root.to_string_root()
    )
}

/// Canonical semantic XML (attributes sorted, indented), used by the conformance corpus.
pub fn semantic_xml(doc: &Document, target: Option<&str>) -> String {
    let mut root = export_tree(doc, target);
    root.sort_attrs();
    root.indent(0);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n",
        root.to_string_root()
    )
}
