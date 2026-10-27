//! Import BPMN 2.0 XML into a tok [`Document`] (the inverse of the exporter).
//! Mirrors `bpmn_import.py`, including its warning texts.

use std::collections::HashSet;

use indexmap::IndexMap;
use roxmltree::Node as X;

use crate::model::*;
use crate::parser::is_id;
use crate::pyutil::{ascii_fold, split_ws, strip};
use crate::xml::{BPMN_NS, ZEEBE_NS};

/// An error that stops the import (not well-formed XML or not a BPMN document).
#[derive(Debug, Clone)]
pub struct ImportError(pub String);

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ImportError {}

fn kind_of_tag(t: &str) -> Option<&'static str> {
    Some(match t {
        "startEvent" => "start",
        "endEvent" => "end",
        "intermediateCatchEvent" => "catch",
        "intermediateThrowEvent" => "throw",
        "boundaryEvent" => "boundary",
        "task" => "task",
        "userTask" => "user",
        "serviceTask" => "service",
        "manualTask" => "manual",
        "scriptTask" => "script",
        "businessRuleTask" => "rule",
        "sendTask" => "send",
        "receiveTask" => "receive",
        "callActivity" => "call",
        "subProcess" => "sub",
        "exclusiveGateway" => "xor",
        "parallelGateway" => "and",
        "inclusiveGateway" => "or",
        "eventBasedGateway" => "eventgw",
        "adHocSubProcess" => "adhoc",
        "transaction" => "sub",
        _ => return None,
    })
}

fn definition_key(t: &str) -> Option<&'static str> {
    Some(match t {
        "messageEventDefinition" => "message",
        "timerEventDefinition" => "timer",
        "signalEventDefinition" => "signal",
        "errorEventDefinition" => "error",
        "escalationEventDefinition" => "escalation",
        "conditionalEventDefinition" => "condition",
        "linkEventDefinition" => "link",
        "terminateEventDefinition" => "terminate",
        "compensateEventDefinition" => "compensate",
        _ => return None,
    })
}

fn local<'a>(e: X<'a, '_>) -> &'a str {
    e.tag_name().name()
}

fn clark(e: X) -> String {
    match e.tag_name().namespace() {
        Some(ns) => format!("{{{ns}}}{}", e.tag_name().name()),
        None => e.tag_name().name().to_string(),
    }
}

fn is_bpmn(e: X) -> bool {
    e.tag_name().namespace() == Some(BPMN_NS)
}

fn is_b(e: X, name: &str) -> bool {
    e.is_element() && is_bpmn(e) && local(e) == name
}

fn children<'a, 'i>(e: X<'a, 'i>) -> impl Iterator<Item = X<'a, 'i>> {
    e.children().filter(|c| c.is_element())
}

fn find<'a, 'i>(e: X<'a, 'i>, name: &str) -> Option<X<'a, 'i>> {
    children(e).find(|c| is_b(*c, name))
}

fn findall<'a, 'i>(e: X<'a, 'i>, name: &'static str) -> impl Iterator<Item = X<'a, 'i>> {
    children(e).filter(move |c| is_b(*c, name))
}

/// First Camunda (`zeebe:`) child element with the local name `name`.
fn zfind<'a, 'i>(e: X<'a, 'i>, name: &str) -> Option<X<'a, 'i>> {
    children(e).find(|c| c.tag_name().namespace() == Some(ZEEBE_NS) && local(*c) == name)
}

/// `e.find(B + "extensionElements/" + Z + name)`
fn ext_find<'a, 'i>(e: X<'a, 'i>, name: &str) -> Option<X<'a, 'i>> {
    findall(e, "extensionElements").find_map(|x| zfind(x, name))
}

const REST_CONNECTOR_TYPE: &str = "io.camunda:http-json:1";

fn binding_name(v: &str) -> String {
    match v {
        "versionTag" => "version-tag".into(),
        other => other.to_string(),
    }
}

fn connector_input(target: &str) -> Option<&'static str> {
    Some(match target {
        "method" => "method",
        "url" => "url",
        "headers" => "headers-in",
        "queryParameters" => "query",
        "body" => "body",
        _ => return None,
    })
}

/// Text of an element as ElementTree reports it (text before the first child element).
fn text<'a>(e: X<'a, '_>) -> Option<&'a str> {
    e.text()
}

fn slug(text: &str) -> String {
    let t = text.replace('ł', "l").replace('Ł', "L");
    let t = ascii_fold(&t);
    let mut out = String::new();
    let mut in_run = false;
    for c in t.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('_');
            in_run = true;
        }
    }
    let t = out.trim_matches('_').to_string();
    let t = if t.is_empty() { "id".to_string() } else { t };
    match t.chars().next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => t,
        _ => format!("_{t}"),
    }
}

fn clean_label(text: Option<&str>) -> Option<String> {
    let t = split_ws(text?).join(" ");
    if t.is_empty() { None } else { Some(t) }
}

fn strip_feel(text: Option<&str>) -> String {
    let t = strip(text.unwrap_or(""));
    match t.strip_prefix('=') {
        Some(r) => strip(r).to_string(),
        None => t.to_string(),
    }
}

struct Importer<'a, 'i> {
    root: X<'a, 'i>,
    keep_ids: bool,
    doc: Document,
    warnings: Vec<String>,
    idmap: IndexMap<String, String>,
    used: HashSet<String>,
    by_id: IndexMap<String, X<'a, 'i>>,
    warned: HashSet<String>,
    /// compensation boundary event id -> (activity, handler, association id)
    compensation: IndexMap<String, (String, String, Option<String>)>,
    /// element ids skipped by `container` (generated compensation boundaries and associations)
    skip_ids: HashSet<String>,
    /// an association without id was skipped (Python: `None in skip_ids`)
    skip_none: bool,
}

/// Read BPMN 2.0 XML text. Returns the document and warning strings `"id: explanation"`.
pub fn import_bpmn(xml: &str, keep_ids: bool) -> Result<(Document, Vec<String>), ImportError> {
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    let opts = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let tree = roxmltree::Document::parse_with_options(xml, opts)
        .map_err(|e| ImportError(e.to_string()))?;
    let root = tree.root_element();
    let mut by_id = IndexMap::new();
    for e in root.descendants().filter(|n| n.is_element()) {
        if let Some(id) = e.attribute("id").filter(|v| !v.is_empty()) {
            by_id.insert(id.to_string(), e);
        }
    }
    let imp = Importer {
        root,
        keep_ids,
        doc: Document::default(),
        warnings: Vec::new(),
        idmap: IndexMap::new(),
        used: HashSet::new(),
        by_id,
        warned: HashSet::new(),
        compensation: IndexMap::new(),
        skip_ids: HashSet::new(),
        skip_none: false,
    };
    imp.run()
}

impl<'a, 'i> Importer<'a, 'i> {
    fn warn(&mut self, id: Option<&str>, text: &str, once: Option<String>) {
        if let Some(o) = once {
            if self.warned.contains(&o) {
                return;
            }
            self.warned.insert(o);
        }
        let id = id.filter(|i| !i.is_empty()).unwrap_or("-");
        self.warnings.push(format!("{id}: {text}"));
    }

    fn tid(&mut self, xml_id: Option<&str>) -> Option<String> {
        let x = xml_id?;
        if let Some(v) = self.idmap.get(x) {
            return Some(v.clone());
        }
        let new0 = if is_id(x) { x.to_string() } else { slug(x) };
        let mut new = new0.clone();
        let mut n = 2;
        while self.used.contains(&new) {
            new = format!("{new0}-{n}");
            n += 1;
        }
        if new != x {
            let msg = format!("identifier changed to '{new}'");
            self.warn(Some(x), &msg, None);
        }
        self.idmap.insert(x.to_string(), new.clone());
        self.used.insert(new.clone());
        Some(new)
    }

    fn tid_s(&mut self, xml_id: Option<&str>) -> String {
        self.tid(xml_id).unwrap_or_default()
    }

    fn reserve_ids(&mut self) {
        let keys: Vec<String> = self.by_id.keys().cloned().collect();
        for x in keys {
            if is_id(&x) {
                self.used.insert(x.clone());
                self.idmap.insert(x.clone(), x);
            }
        }
    }

    fn named(&mut self, r: &str, kind: &str) -> String {
        match self.by_id.get(r).copied() {
            None => {
                self.warn(Some(r), &format!("unknown {kind} reference"), None);
                r.to_string()
            }
            Some(e) => clean_label(e.attribute("name"))
                .unwrap_or_else(|| e.attribute("id").unwrap_or("").to_string()),
        }
    }

    fn uses_03(&mut self) {
        self.doc.version = Some("0.3".into());
    }

    /// Compensation boundary events with an association to a handler: tok writes them as
    /// `compensate=` on the activity.
    fn compensation_boundaries(&mut self) {
        let assocs: Vec<X> = self
            .root
            .descendants()
            .filter(|e| is_b(*e, "association"))
            .collect();
        for a in assocs {
            let src = a
                .attribute("sourceRef")
                .and_then(|r| self.by_id.get(r))
                .copied();
            let tgt = a
                .attribute("targetRef")
                .and_then(|r| self.by_id.get(r))
                .copied();
            let (Some(src), Some(tgt)) = (src, tgt) else {
                continue;
            };
            let attached = src.attribute("attachedToRef").filter(|v| !v.is_empty());
            if local(src) == "boundaryEvent"
                && find(src, "compensateEventDefinition").is_some()
                && tgt.attribute("isForCompensation") == Some("true")
                && let Some(act) = attached
            {
                self.compensation.insert(
                    src.attribute("id").unwrap_or("").to_string(),
                    (
                        act.to_string(),
                        tgt.attribute("id").unwrap_or("").to_string(),
                        a.attribute("id").map(str::to_string),
                    ),
                );
            }
        }
        for (bid, (_, _, assoc)) in &self.compensation {
            self.skip_ids.insert(bid.clone());
            match assoc {
                Some(i) => {
                    self.skip_ids.insert(i.clone());
                }
                None => self.skip_none = true,
            }
        }
    }

    fn read_binding(&mut self, z: X, a: &mut Attrs) {
        if let Some(b) = z.attribute("bindingType").filter(|v| !v.is_empty()) {
            a.insert("binding".into(), binding_name(b));
            self.uses_03();
        }
        if let Some(v) = z.attribute("versionTag").filter(|v| !v.is_empty()) {
            a.insert("version".into(), v.to_string());
            self.uses_03();
        }
    }

    fn run(mut self) -> Result<(Document, Vec<String>), ImportError> {
        let root = self.root;
        self.compensation_boundaries();
        if !is_b(root, "definitions") {
            return Err(ImportError(format!(
                "not a BPMN 2.0 document: root element is {}",
                clark(root)
            )));
        }
        self.reserve_ids();
        self.doc.version = Some("0.1".into());
        self.doc.title = clean_label(root.attribute("name"));
        if let Some(id) = root
            .attribute("id")
            .filter(|i| !i.is_empty() && *i != "definitions")
        {
            let t = self.tid_s(Some(id));
            self.doc.attrs.insert("id".into(), t);
        }
        if let Some(ns) = root
            .attribute("targetNamespace")
            .filter(|n| !n.is_empty() && *n != "http://bpmn.io/schema/bpmn")
        {
            self.doc.attrs.insert("namespace".into(), ns.to_string());
        }
        let processes: Vec<X> = findall(root, "process").collect();
        let collabs: Vec<X> = findall(root, "collaboration").collect();
        for e in children(root) {
            let t = local(e);
            if [
                "choreography",
                "globalTask",
                "globalUserTask",
                "globalManualTask",
                "globalScriptTask",
                "globalBusinessRuleTask",
            ]
            .contains(&t)
            {
                self.warn(
                    e.attribute("id"),
                    &format!("{t} is not supported and was skipped"),
                    None,
                );
            }
        }
        if collabs.len() > 1 {
            self.warn(
                collabs[1].attribute("id"),
                "only the first collaboration is imported",
                None,
            );
        }
        let collab = collabs.first().copied();
        if let Some(c) = collab
            && c.attribute("id") != Some("collaboration")
        {
            let t = self.tid(c.attribute("id")).unwrap_or_else(|| "None".into());
            self.doc.attrs.insert("collaboration".into(), t);
        }
        let n_exec = processes
            .iter()
            .filter(|p| p.attribute("isExecutable") == Some("true"))
            .count();
        let all_exec = !processes.is_empty() && n_exec == processes.len();
        if all_exec {
            self.doc.flags.push("executable".into());
        }
        let mut proc_by_id: IndexMap<Option<&str>, X> = IndexMap::new();
        for p in &processes {
            proc_by_id.insert(p.attribute("id"), *p);
        }
        if collab.is_none() && processes.len() == 1 {
            let p = processes[0];
            if p.attribute("id") != Some("process") {
                let t = self.tid(p.attribute("id")).unwrap_or_else(|| "None".into());
                self.doc.attrs.insert("process".into(), t);
            }
            if self.doc.title.is_none() {
                self.doc.title = clean_label(p.attribute("name"));
            }
            self.process(p, None);
        } else {
            let mut referenced: HashSet<&str> = HashSet::new();
            let participants: Vec<X> = collab
                .map(|c| findall(c, "participant").collect())
                .unwrap_or_default();
            for part in participants {
                let pid = self.tid_s(part.attribute("id"));
                let base = Base::new(
                    "pool",
                    Some(pid.clone()),
                    clean_label(part.attribute("name")),
                    Attrs::new(),
                    Vec::new(),
                    0,
                );
                self.doc.pools.push(Pool {
                    base,
                    blackbox: true,
                    children: Vec::new(),
                });
                self.doc.ids.insert(
                    pid.clone(),
                    IdEntry {
                        kind: "pool".into(),
                        line: 0,
                    },
                );
                let r = part.attribute("processRef");
                if let Some(r) = r.filter(|r| !r.is_empty()) {
                    if let Some(p) = proc_by_id.get(&Some(r)).copied() {
                        referenced.insert(r);
                        let idx = self.doc.pools.len() - 1;
                        self.attach_process(idx, p, all_exec);
                    } else {
                        let msg = format!("process '{r}' not found; imported as a black-box pool");
                        self.warn(part.attribute("id"), &msg, None);
                    }
                }
            }
            for p in &processes {
                if p.attribute("id")
                    .map(|i| referenced.contains(i))
                    .unwrap_or(false)
                {
                    continue;
                }
                let pid = self.tid_s(p.attribute("id"));
                let mut pool_id = format!("{pid}-pool");
                while self.used.contains(&pool_id) {
                    pool_id.push('_');
                }
                self.used.insert(pool_id.clone());
                let label = clean_label(p.attribute("name")).or_else(|| Some(pid.clone()));
                let base = Base::new(
                    "pool",
                    Some(pool_id.clone()),
                    label,
                    Attrs::new(),
                    Vec::new(),
                    0,
                );
                self.doc.pools.push(Pool {
                    base,
                    blackbox: true,
                    children: Vec::new(),
                });
                self.doc.ids.insert(
                    pool_id.clone(),
                    IdEntry {
                        kind: "pool".into(),
                        line: 0,
                    },
                );
                let idx = self.doc.pools.len() - 1;
                self.attach_process(idx, *p, all_exec);
                let msg = format!("process without a participant was placed in pool '{pool_id}'");
                self.warn(p.attribute("id"), &msg, None);
            }
        }
        let inner: Vec<usize> = self
            .doc
            .pools
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.blackbox)
            .map(|(i, _)| i)
            .collect();
        if self.doc.title.is_none() && inner.len() == 1 {
            let target = self.doc.pools[inner[0]].process_id();
            let mut proc = proc_by_id.get(&Some(target.as_str())).copied();
            if proc.is_none() {
                for p in &processes {
                    if self.tid(p.attribute("id")).as_deref() == Some(target.as_str()) {
                        proc = Some(*p);
                        break;
                    }
                }
            }
            if let Some(p) = proc
                && let Some(l) = clean_label(p.attribute("name"))
            {
                self.doc.title = Some(l);
            }
        }
        if let Some(c) = collab {
            let mfs: Vec<X> = findall(c, "messageFlow").collect();
            for mf in mfs {
                self.message_flow(mf);
            }
            for e in children(c) {
                let t = local(e);
                if [
                    "conversation",
                    "subConversation",
                    "callConversation",
                    "conversationLink",
                    "participantAssociation",
                    "correlationKey",
                ]
                .contains(&t)
                {
                    self.warn(
                        e.attribute("id"),
                        &format!("{t} is not supported and was skipped"),
                        None,
                    );
                }
            }
        }
        self.flows_and_artifacts();
        self.derive_flow_ids();
        Ok((self.doc, self.warnings))
    }

    fn attach_process(&mut self, pool_idx: usize, p: X<'a, 'i>, all_exec: bool) {
        self.doc.pools[pool_idx].blackbox = false;
        let pool_id = self.doc.pools[pool_idx].base.id().to_string();
        if p.attribute("id") != Some(format!("{pool_id}-process").as_str()) {
            let t = self.tid(p.attribute("id")).unwrap_or_else(|| "None".into());
            self.doc.pools[pool_idx]
                .base
                .attrs
                .insert("process".into(), t);
        }
        if p.attribute("isExecutable") == Some("true") && !all_exec {
            self.doc.pools[pool_idx]
                .base
                .flags
                .push("executable".into());
        }
        self.process(p, Some(pool_idx));
    }

    fn process(&mut self, p: X<'a, 'i>, pool: Option<usize>) {
        if let Some(vt) = ext_find(p, "versionTag")
            && let Some(v) = vt.attribute("value").filter(|v| !v.is_empty())
        {
            match pool {
                Some(i) => {
                    self.doc.pools[i]
                        .base
                        .attrs
                        .insert("version-tag".into(), v.to_string());
                }
                None => {
                    self.doc.attrs.insert("version-tag".into(), v.to_string());
                }
            }
            self.uses_03();
        }
        let scope = match pool {
            Some(i) => self.doc.pools[i].process_id(),
            None => self.doc.default_process_id(),
        };
        let pool_id = pool.map(|i| self.doc.pools[i].base.id().to_string());
        let mut lane_of: IndexMap<String, String> = IndexMap::new();
        let mut lanes = Vec::new();
        let sets: Vec<X> = findall(p, "laneSet").collect();
        for ls in sets {
            let ls_lanes: Vec<X> = findall(ls, "lane").collect();
            for lane in ls_lanes {
                lanes.push(self.lane(lane, None, pool_id.as_deref(), &mut lane_of));
            }
        }
        let order = self.container(p, &scope, pool_id.as_deref(), None);
        let Some(pi) = pool else {
            self.doc.top.extend(order.children);
            return;
        };
        if lanes.is_empty() {
            let pool = &mut self.doc.pools[pi];
            pool.children.extend(order.children);
            pool.children.extend(order.data);
            return;
        }
        let pos: IndexMap<String, usize> = order
            .nodes
            .iter()
            .enumerate()
            .map(|(k, n)| (n.clone(), k))
            .collect();
        let mut outside = Vec::new();
        for nid in &order.nodes {
            match lane_of.get(nid).cloned() {
                None => outside.push(nid.clone()),
                Some(lid) => {
                    if let Some(n) = self.doc.nodes.get_mut(nid) {
                        n.lane = Some(lid.clone());
                    }
                    if let Some(l) = self.doc.lanes.get_mut(&lid) {
                        l.children.push(nid.clone());
                    }
                }
            }
        }
        self.sort_lanes(&lanes, &pos);
        let merged = self.merge(&lanes, &[], &pos);
        self.doc.pools[pi].children.extend(merged);
        for nid in outside {
            self.doc.pools[pi].children.push(nid.clone());
            self.warn(
                Some(&nid),
                "node is in a pool with lanes but in no lane",
                None,
            );
        }
        self.doc.pools[pi].children.extend(order.data);
    }

    fn lane(
        &mut self,
        e: X<'a, 'i>,
        parent: Option<&str>,
        pool: Option<&str>,
        lane_of: &mut IndexMap<String, String>,
    ) -> String {
        let lid = self.tid_s(e.attribute("id"));
        let base = Base::new(
            "lane",
            Some(lid.clone()),
            clean_label(e.attribute("name")),
            Attrs::new(),
            Vec::new(),
            0,
        );
        self.doc.lanes.insert(
            lid.clone(),
            Lane {
                base,
                pool: pool.map(str::to_string),
                parent: parent.map(str::to_string),
                children: Vec::new(),
                sub: Vec::new(),
            },
        );
        self.doc.ids.insert(
            lid.clone(),
            IdEntry {
                kind: "lane".into(),
                line: 0,
            },
        );
        let refs: Vec<X> = findall(e, "flowNodeRef").collect();
        for r in refs {
            let xid = strip(text(r).unwrap_or("")).to_string();
            if !xid.is_empty() {
                let t = self.tid_s(Some(&xid));
                lane_of.insert(t, lid.clone());
            }
        }
        let sets: Vec<X> = findall(e, "childLaneSet").collect();
        for cls in sets {
            let subs: Vec<X> = findall(cls, "lane").collect();
            for c in subs {
                let sid = self.lane(c, Some(&lid), pool, lane_of);
                if let Some(l) = self.doc.lanes.get_mut(&lid) {
                    l.sub.push(sid);
                }
            }
        }
        lid
    }

    fn sort_lanes(&mut self, lanes: &[String], pos: &IndexMap<String, usize>) {
        for lid in lanes {
            let sub = self
                .doc
                .lanes
                .get(lid)
                .map(|l| l.sub.clone())
                .unwrap_or_default();
            self.sort_lanes(&sub, pos);
            let nodes: Vec<String> = self
                .doc
                .lanes
                .get(lid)
                .map(|l| {
                    l.children
                        .iter()
                        .filter(|c| self.doc.nodes.contains_key(*c))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            let merged = self.merge(&sub, &nodes, pos);
            if let Some(l) = self.doc.lanes.get_mut(lid) {
                l.children = merged;
            }
        }
    }

    fn first_pos(&self, lid: &str, pos: &IndexMap<String, usize>) -> Option<usize> {
        let mut best: Option<usize> = None;
        let lane = self.doc.lanes.get(lid)?;
        for c in &lane.children {
            let k = if self.doc.nodes.contains_key(c) {
                pos.get(c).copied()
            } else {
                self.first_pos(c, pos)
            };
            if let Some(k) = k
                && best.map(|b| k < b).unwrap_or(true)
            {
                best = Some(k);
            }
        }
        best
    }

    fn merge(
        &self,
        lanes: &[String],
        nodes: &[String],
        pos: &IndexMap<String, usize>,
    ) -> Vec<String> {
        let mut items: Vec<(f64, u8, usize, String)> = Vec::new();
        let mut last = -1.0f64;
        for (k, lid) in lanes.iter().enumerate() {
            let p = self.first_pos(lid, pos);
            let key = match p {
                Some(p) => p as f64,
                None => last + 0.001,
            };
            last = key;
            items.push((key, 1, k, lid.clone()));
        }
        for nid in nodes {
            items.push((
                pos.get(nid).map(|p| *p as f64).unwrap_or(1e9),
                0,
                0,
                nid.clone(),
            ));
        }
        items.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
                .then(a.2.cmp(&b.2))
                .then(a.3.cmp(&b.3))
        });
        items.into_iter().map(|x| x.3).collect()
    }

    fn container(
        &mut self,
        e: X<'a, 'i>,
        scope: &str,
        pool: Option<&str>,
        sub: Option<&str>,
    ) -> Order {
        let mut order = Order::default();
        let kids: Vec<X> = children(e).collect();
        for c in kids {
            if !is_bpmn(c) {
                continue;
            }
            let skip = match c.attribute("id") {
                Some(i) => self.skip_ids.contains(i),
                None => self.skip_none,
            };
            if skip {
                continue;
            }
            let t = local(c);
            if kind_of_tag(t).is_some() {
                let nid = self.node(c, t, scope, pool);
                order.children.push(nid.clone());
                order.nodes.push(nid.clone());
                let is_sub_kind = self
                    .doc
                    .nodes
                    .get(&nid)
                    .map(|n| is_sub(&n.base.kind))
                    .unwrap_or(false);
                if is_sub_kind {
                    let inner = self.container(c, &nid, pool, Some(&nid));
                    if let Some(n) = self.doc.nodes.get_mut(&nid) {
                        n.children = inner.children.into_iter().chain(inner.data).collect();
                    }
                }
            } else if [
                "complexGateway",
                "choreographyTask",
                "subChoreography",
                "callChoreography",
            ]
            .contains(&t)
            {
                self.warn(
                    c.attribute("id"),
                    &format!("{t} is not supported in tok and was skipped"),
                    None,
                );
            } else if t == "dataObjectReference" || t == "dataStoreReference" {
                let block = sub.or(pool).map(str::to_string);
                let did = self.data(c, t, scope, block);
                order.data.push(did);
            } else if t == "laneSet" && sub.is_some() {
                self.warn(
                    c.attribute("id"),
                    "lanes inside a sub-process are not supported and were ignored",
                    None,
                );
            } else if t == "ioSpecification" {
                self.warn(
                    e.attribute("id"),
                    "process input/output specification was skipped",
                    Some("ioSpec".into()),
                );
            }
        }
        if sub.is_none() && pool.is_none() {
            let data = order.data.clone();
            order.children.extend(data);
        }
        order
    }

    fn node(&mut self, e: X<'a, 'i>, t: &str, scope: &str, pool: Option<&str>) -> String {
        let mut kind = kind_of_tag(t).unwrap_or("task");
        if t == "transaction" {
            self.warn(
                e.attribute("id"),
                &format!("{t} imported as an embedded sub-process"),
                None,
            );
        }
        if t == "subProcess" && e.attribute("triggeredByEvent") == Some("true") {
            kind = "eventsub";
        }
        let nid = self.tid_s(e.attribute("id"));
        let mut n = Node {
            base: Base::new(
                kind,
                Some(nid.clone()),
                clean_label(e.attribute("name")),
                Attrs::new(),
                Vec::new(),
                0,
            ),
            scope: Some(scope.to_string()),
            lane: None,
            pool: pool.map(str::to_string),
            children: Vec::new(),
        };
        if kind == "boundary" {
            let on = self
                .tid(e.attribute("attachedToRef"))
                .unwrap_or_else(|| "None".into());
            n.base.attrs.insert("on".into(), on);
            if e.attribute("cancelActivity") == Some("false") {
                n.base.flags.push("noninterrupting".into());
            }
        }
        if kind == "start" && e.attribute("isInterrupting") == Some("false") {
            n.base.flags.push("noninterrupting".into());
        }
        if kind == "call"
            && let Some(ce) = e.attribute("calledElement").filter(|c| !c.is_empty())
        {
            n.base.attrs.insert("process".into(), ce.to_string());
        }
        if kind == "adhoc" {
            self.uses_03();
            if e.attribute("ordering") == Some("Sequential") {
                n.base.flags.push("sequential".into());
            }
            if e.attribute("cancelRemainingInstances") == Some("false") {
                n.base.flags.push("wait".into());
            }
            if let Some(cc) = find(e, "completionCondition")
                && !strip(text(cc).unwrap_or("")).is_empty()
            {
                n.base.attrs.insert("until".into(), strip_feel(text(cc)));
            }
        }
        let own_id = e.attribute("id").unwrap_or("");
        let handlers: Vec<String> = self
            .compensation
            .values()
            .filter(|(act, _, _)| act == own_id)
            .map(|(_, h, _)| h.clone())
            .collect();
        for h in handlers {
            let t = self.tid_s(Some(&h));
            n.base.attrs.insert("compensate".into(), t);
            self.uses_03();
        }
        if e.attribute("isForCompensation") == Some("true") {
            n.base.flags.push("compensation".into());
        }
        if (kind == "send" || kind == "receive")
            && let Some(r) = e.attribute("messageRef").filter(|r| !r.is_empty())
        {
            let name = self.named(r, "message");
            n.base.attrs.insert("message".into(), name);
            if kind == "receive" && self.correlation(r).is_some() {
                self.warn(
                    Some(&nid),
                    "correlation key of a receive task cannot be expressed in tok; use a message catch event",
                    None,
                );
            }
        }
        if kind == "script"
            && let Some(s) = find(e, "script")
            && !strip(text(s).unwrap_or("")).is_empty()
        {
            n.base.attrs.insert("expr".into(), strip_feel(text(s)));
        }
        if is_event(kind) {
            self.event_definitions(e, &mut n);
        }
        if find(e, "standardLoopCharacteristics").is_some() {
            n.base.flags.push("loop".into());
        }
        if let Some(mi) = find(e, "multiInstanceLoopCharacteristics") {
            n.base.flags.push(
                if mi.attribute("isSequential") == Some("true") {
                    "sequential"
                } else {
                    "parallel"
                }
                .into(),
            );
            if let Some(lc) = ext_find(mi, "loopCharacteristics") {
                self.uses_03();
                let a = &mut n.base.attrs;
                let get = |name: &str| lc.attribute(name).filter(|v| !v.is_empty());
                if let Some(v) = get("inputCollection") {
                    a.insert("collection".into(), strip_feel(Some(v)));
                }
                if let Some(v) = get("inputElement") {
                    a.insert("item".into(), v.to_string());
                }
                if let Some(v) = get("outputCollection") {
                    a.insert("output".into(), v.to_string());
                }
                if let Some(v) = get("outputElement") {
                    a.insert("output-item".into(), strip_feel(Some(v)));
                }
            }
            if let Some(cc) = find(mi, "completionCondition")
                && !strip(text(cc).unwrap_or("")).is_empty()
            {
                n.base.attrs.insert("until".into(), strip_feel(text(cc)));
                self.uses_03();
            }
        }
        self.extensions(e, &mut n);
        if find(e, "ioSpecification").is_some() && kind != "sub" {
            self.warn(
                Some(&nid),
                "input/output specification was skipped",
                Some(format!("io-{nid}")),
            );
        }
        self.doc.ids.insert(
            nid.clone(),
            IdEntry {
                kind: kind.to_string(),
                line: 0,
            },
        );
        self.doc.nodes.insert(nid.clone(), n);
        nid
    }

    fn correlation(&self, msg_ref: &str) -> Option<String> {
        let msg = self.by_id.get(msg_ref)?;
        let ext = find(*msg, "extensionElements")?;
        let sub = children(ext)
            .find(|c| c.tag_name().namespace() == Some(ZEEBE_NS) && local(*c) == "subscription")?;
        let key = sub.attribute("correlationKey").filter(|k| !k.is_empty())?;
        Some(strip_feel(Some(key)))
    }

    fn event_definitions(&mut self, e: X<'a, 'i>, n: &mut Node) {
        let nid = n.base.id().to_string();
        let defs: Vec<X> = children(e)
            .filter(|c| is_bpmn(*c) && definition_key(local(*c)).is_some())
            .collect();
        for c in children(e) {
            if is_bpmn(c) && local(c) == "eventDefinitionRef" {
                self.warn(
                    Some(&nid),
                    "reusable event definitions (eventDefinitionRef) are not supported",
                    None,
                );
            }
        }
        let Some(c) = defs.first().copied() else {
            return;
        };
        if defs.len() > 1 {
            self.warn(
                Some(&nid),
                "multiple event definitions: only the first one was imported",
                None,
            );
        }
        let key = definition_key(local(c)).unwrap_or("message");
        let label_or_id = n
            .base
            .label
            .clone()
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| nid.clone());
        match key {
            "terminate" | "compensate" => {
                n.base.flags.push(key.into());
                if key == "compensate"
                    && let Some(r) = c.attribute("activityRef").filter(|a| !a.is_empty())
                {
                    let t = self.tid_s(Some(r));
                    n.base.attrs.insert("activity".into(), t);
                    self.uses_03();
                }
            }
            "message" => {
                let r = c.attribute("messageRef").filter(|r| !r.is_empty());
                let name = match r {
                    Some(r) => self.named(r, "message"),
                    None => label_or_id.clone(),
                };
                n.base.attrs.insert("message".into(), name);
                if r.is_none() {
                    self.warn(Some(&nid), "message event without messageRef; the event label was used as message name", None);
                }
                if let Some(corr) = r
                    .and_then(|r| self.correlation(r))
                    .filter(|c| !c.is_empty())
                {
                    n.base.attrs.insert("correlation".into(), corr);
                }
            }
            "signal" | "error" | "escalation" => {
                let attr = format!("{key}Ref");
                match c.attribute(attr.as_str()).filter(|r| !r.is_empty()) {
                    Some(r) => {
                        let name = self.named(r, key);
                        n.base.attrs.insert(key.into(), name);
                        let code_attr = format!("{key}Code");
                        if let Some(code) = self
                            .by_id
                            .get(r)
                            .and_then(|d| d.attribute(code_attr.as_str()))
                            .filter(|c| !c.is_empty())
                        {
                            n.base.attrs.insert("code".into(), code.to_string());
                        }
                    }
                    None if CATCH_ALL_FLAGS.contains(&key)
                        && (n.base.kind == "boundary" || n.base.kind == "start") =>
                    {
                        n.base.flags.push(key.into());
                        self.uses_03();
                    }
                    None => {
                        n.base.attrs.insert(key.into(), label_or_id.clone());
                        let msg = format!(
                            "{key} event without a reference; the event label was used as name"
                        );
                        self.warn(Some(&nid), &msg, None);
                    }
                }
            }
            "timer" => {
                let mut value = None;
                for part in ["timeDuration", "timeCycle", "timeDate"] {
                    if let Some(x) = find(c, part)
                        && !strip(text(x).unwrap_or("")).is_empty()
                    {
                        value = Some(strip(text(x).unwrap_or("")).to_string());
                        break;
                    }
                }
                let value = match value {
                    Some(v) => v,
                    None => {
                        self.warn(Some(&nid), "timer without a value; P1D was used", None);
                        "P1D".into()
                    }
                };
                n.base.attrs.insert("timer".into(), value);
            }
            "condition" => {
                let x = find(c, "condition");
                let v = strip_feel(x.and_then(text));
                n.base.attrs.insert(
                    "condition".into(),
                    if v.is_empty() { "true".into() } else { v },
                );
            }
            _ => {
                let v = c
                    .attribute("name")
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
                    .unwrap_or(label_or_id);
                n.base.attrs.insert("link".into(), v);
            }
        }
    }

    fn extensions(&mut self, e: X<'a, 'i>, n: &mut Node) {
        let Some(x) = find(e, "extensionElements") else {
            return;
        };
        let nid = n.base.id().to_string();
        let kind = n.base.kind.clone();
        for z in children(x) {
            let t = local(z);
            if z.tag_name().namespace() != Some(ZEEBE_NS) {
                let tag = clark(z);
                self.warn(
                    Some(&nid),
                    &format!("extension element {tag} was skipped"),
                    Some(format!("ext-{tag}")),
                );
                continue;
            }
            let attr = |name: &str| z.attribute(name).filter(|v| !v.is_empty());
            if t == "userTask" && kind == "user" {
                continue;
            }
            if t == "formDefinition" && (kind == "user" || kind == "start") {
                match attr("formId") {
                    Some(f) => {
                        n.base.attrs.insert("form".into(), f.to_string());
                        let mut a = std::mem::take(&mut n.base.attrs);
                        self.read_binding(z, &mut a);
                        n.base.attrs = a;
                    }
                    None => self.warn(
                        Some(&nid),
                        "form reference without formId (formKey or external form) was skipped",
                        None,
                    ),
                }
            } else if t == "calledDecision" && kind == "rule" {
                let mut a = std::mem::take(&mut n.base.attrs);
                if let Some(d) = attr("decisionId") {
                    a.insert("decision".into(), d.to_string());
                }
                if let Some(r) = attr("resultVariable") {
                    a.insert("result".into(), r.to_string());
                }
                self.read_binding(z, &mut a);
                n.base.attrs = a;
            } else if t == "calledElement" && kind == "call" {
                let mut a = std::mem::take(&mut n.base.attrs);
                if let Some(p) = attr("processId") {
                    a.insert("process".into(), p.to_string());
                }
                self.read_binding(z, &mut a);
                if z.attribute("propagateAllChildVariables") == Some("false") {
                    a.insert("child-vars".into(), "none".into());
                }
                if z.attribute("propagateAllParentVariables") == Some("false") {
                    a.insert("parent-vars".into(), "none".into());
                }
                if let Some(b) = attr("businessId") {
                    a.insert("business-id".into(), b.to_string());
                }
                n.base.attrs = a;
                if z.attributes().count() > 1 {
                    self.uses_03();
                }
            } else if t == "adHoc" && kind == "adhoc" {
                if let Some(v) = attr("activeElementsCollection") {
                    n.base.attrs.insert("active".into(), strip_feel(Some(v)));
                }
            } else if t == "assignmentDefinition" && kind == "user" {
                for (at, key) in [
                    ("assignee", "assignee"),
                    ("candidateUsers", "candidate-users"),
                    ("candidateGroups", "candidate-groups"),
                ] {
                    if let Some(v) = attr(at) {
                        n.base.attrs.insert(key.into(), v.to_string());
                    }
                }
                self.uses_03();
            } else if t == "taskSchedule" && kind == "user" {
                for (at, key) in [("dueDate", "due"), ("followUpDate", "follow-up")] {
                    if let Some(v) = attr(at) {
                        n.base.attrs.insert(key.into(), v.to_string());
                    }
                }
                self.uses_03();
            } else if t == "priorityDefinition" && kind == "user" {
                if let Some(v) = attr("priority") {
                    n.base.attrs.insert("priority".into(), v.to_string());
                }
                self.uses_03();
            } else if t == "ioMapping" {
                let pair = |c: X, name: &str| -> Option<(String, String)> {
                    if c.tag_name().namespace() == Some(ZEEBE_NS) && local(c) == name {
                        Some((
                            c.attribute("target").unwrap_or("None").to_string(),
                            c.attribute("source").unwrap_or("").to_string(),
                        ))
                    } else {
                        None
                    }
                };
                let mut ins: Vec<(String, String)> =
                    children(z).filter_map(|c| pair(c, "input")).collect();
                let outs: Vec<(String, String)> =
                    children(z).filter_map(|c| pair(c, "output")).collect();
                let a = &mut n.base.attrs;
                if a.get("connector").map(String::as_str) == Some("rest")
                    || a.get("type").map(String::as_str) == Some(REST_CONNECTOR_TYPE)
                {
                    let mut rest = Vec::new();
                    for (target, source) in ins {
                        if let Some(key) = connector_input(&target) {
                            a.insert(key.into(), source);
                        } else if target == "authentication.type" && source == "noAuth" {
                            continue;
                        } else {
                            rest.push((target, source));
                        }
                    }
                    ins = rest;
                    a.insert("connector".into(), "rest".into());
                    a.shift_remove("type");
                }
                if !ins.is_empty() {
                    let v: Vec<String> = ins
                        .iter()
                        .map(|(tg, src)| format!("{tg} = {}", strip_feel(Some(src))))
                        .collect();
                    a.insert("inputs".into(), v.join("; "));
                }
                if !outs.is_empty() {
                    let v: Vec<String> = outs
                        .iter()
                        .map(|(tg, src)| format!("{tg} = {}", strip_feel(Some(src))))
                        .collect();
                    a.insert("outputs".into(), v.join("; "));
                }
                self.uses_03();
            } else if t == "taskHeaders" {
                let mut hs: Vec<(String, String)> = children(z)
                    .filter(|c| c.tag_name().namespace() == Some(ZEEBE_NS) && local(*c) == "header")
                    .map(|h| {
                        (
                            h.attribute("key").unwrap_or("None").to_string(),
                            h.attribute("value").unwrap_or("").to_string(),
                        )
                    })
                    .collect();
                let is_rest = zfind(x, "taskDefinition")
                    .is_some_and(|td| td.attribute("type") == Some(REST_CONNECTOR_TYPE));
                let a = &mut n.base.attrs;
                if a.get("connector").map(String::as_str) == Some("rest") || is_rest {
                    let mut keep = Vec::new();
                    for (k, v) in hs {
                        if k == "resultVariable" {
                            a.insert("result".into(), v);
                        } else {
                            keep.push((k, v));
                        }
                    }
                    hs = keep;
                }
                if !hs.is_empty() {
                    let v: Vec<String> = hs.iter().map(|(k, v)| format!("{k}: {v}")).collect();
                    a.insert("headers".into(), v.join("; "));
                }
                self.uses_03();
            } else if t == "conditionalFilter" {
                if let Some(v) = attr("variableEvents") {
                    n.base.attrs.insert("events".into(), v.to_string());
                    self.uses_03();
                }
            } else if t == "executionListeners" || t == "taskListeners" {
                let mut items = Vec::new();
                for li in children(z) {
                    let ev = li.attribute("eventType").filter(|v| !v.is_empty());
                    let typ = li.attribute("type").filter(|v| !v.is_empty());
                    if let (Some(ev), Some(typ)) = (ev, typ) {
                        let retries = li
                            .attribute("retries")
                            .filter(|v| !v.is_empty())
                            .map(|r| format!(" ({r})"))
                            .unwrap_or_default();
                        items.push(format!("{ev}: {typ}{retries}"));
                    }
                }
                if !items.is_empty() {
                    let a = &mut n.base.attrs;
                    let mut all: Vec<String> = a
                        .get("listeners")
                        .filter(|p| !p.is_empty())
                        .cloned()
                        .into_iter()
                        .collect();
                    all.extend(items);
                    a.insert("listeners".into(), all.join("; "));
                    self.uses_03();
                }
            } else if t == "script" && kind == "script" {
                let a = &mut n.base.attrs;
                if let Some(x) = attr("expression") {
                    a.insert("expr".into(), strip_feel(Some(x)));
                }
                if let Some(r) = attr("resultVariable") {
                    a.insert("result".into(), r.to_string());
                }
            } else if t == "taskDefinition"
                && (kind == "service"
                    || kind == "send"
                    || ((kind == "throw" || kind == "end") && n.base.attrs.contains_key("message")))
            {
                if let Some(ty) = attr("type") {
                    n.base.attrs.insert("type".into(), ty.to_string());
                    if ty == REST_CONNECTOR_TYPE && kind == "service" {
                        n.base.attrs.insert("connector".into(), "rest".into());
                        n.base.attrs.shift_remove("type");
                        self.uses_03();
                    }
                }
                if let Some(r) = attr("retries") {
                    n.base.attrs.insert("retries".into(), r.to_string());
                    self.uses_03();
                }
                if kind == "throw" || kind == "end" {
                    self.uses_03();
                }
            } else {
                self.warn(
                    Some(&nid),
                    &format!("Camunda extension zeebe:{t} was skipped"),
                    Some(format!("zeebe-{t}-{kind}")),
                );
            }
        }
    }

    fn data(&mut self, e: X<'a, 'i>, t: &str, scope: &str, block: Option<String>) -> String {
        let did = self.tid_s(e.attribute("id"));
        let mut x = if t == "dataObjectReference" {
            let obj = e
                .attribute("dataObjectRef")
                .and_then(|r| self.by_id.get(r).copied());
            let label = clean_label(e.attribute("name"))
                .or_else(|| obj.and_then(|o| clean_label(o.attribute("name"))));
            let mut x = Data {
                base: Base::new(
                    "data",
                    Some(did.clone()),
                    label,
                    Attrs::new(),
                    Vec::new(),
                    0,
                ),
                scope: None,
                block: None,
            };
            if let Some(st) = find(e, "dataState")
                && let Some(name) = st.attribute("name").filter(|v| !v.is_empty())
            {
                x.base.attrs.insert("state".into(), name.to_string());
            }
            if obj.and_then(|o| o.attribute("isCollection")) == Some("true") {
                x.base.flags.push("collection".into());
            }
            x
        } else {
            let store = e
                .attribute("dataStoreRef")
                .and_then(|r| self.by_id.get(r).copied());
            let label = clean_label(e.attribute("name"))
                .or_else(|| store.and_then(|o| clean_label(o.attribute("name"))));
            Data {
                base: Base::new(
                    "store",
                    Some(did.clone()),
                    label,
                    Attrs::new(),
                    Vec::new(),
                    0,
                ),
                scope: None,
                block: None,
            }
        };
        if let Some(b) = block {
            x.scope = Some(scope.to_string());
            x.block = Some(b);
        }
        self.doc.ids.insert(
            did.clone(),
            IdEntry {
                kind: x.base.kind.clone(),
                line: 0,
            },
        );
        self.doc.data.insert(did.clone(), x);
        did
    }

    fn flows_and_artifacts(&mut self) {
        let mut flows = Vec::new();
        let mut assocs = Vec::new();
        let mut notes = Vec::new();
        let mut groups = Vec::new();
        for e in self
            .root
            .descendants()
            .filter(|n| n.is_element() && is_bpmn(*n))
        {
            match local(e) {
                "sequenceFlow" => flows.push(e),
                "dataInputAssociation" | "dataOutputAssociation" => assocs.push(e),
                "textAnnotation" => notes.push(e),
                "group" => groups.push(e),
                _ => {}
            }
        }
        for e in flows {
            self.sequence_flow(e);
        }
        self.associations(&assocs);
        let mut annotated: IndexMap<String, Option<String>> = IndexMap::new();
        for a in self.root.descendants().filter(|n| is_b(*n, "association")) {
            let (src, tgt) = (a.attribute("sourceRef"), a.attribute("targetRef"));
            let is_note = |r: Option<&str>| {
                r.and_then(|r| self.by_id.get(r))
                    .map(|e| local(*e) == "textAnnotation")
                    .unwrap_or(false)
            };
            if is_note(src) {
                annotated
                    .entry(src.unwrap_or("").to_string())
                    .or_insert(tgt.map(str::to_string));
            } else if is_note(tgt) {
                annotated
                    .entry(tgt.unwrap_or("").to_string())
                    .or_insert(src.map(str::to_string));
            }
        }
        for e in notes {
            let t = find(e, "text").and_then(text);
            let id = self.tid_s(e.attribute("id"));
            let label = clean_label(Some(t.unwrap_or(""))).unwrap_or_default();
            let mut note = Base::new(
                "note",
                Some(id.clone()),
                Some(label),
                Attrs::new(),
                Vec::new(),
                0,
            );
            let target = e
                .attribute("id")
                .and_then(|i| annotated.get(i))
                .cloned()
                .flatten();
            if let Some(tg) = target.filter(|t| !t.is_empty()) {
                let t = self.tid_s(Some(&tg));
                note.attrs.insert("on".into(), t);
            }
            self.doc.ids.insert(
                id,
                IdEntry {
                    kind: "note".into(),
                    line: 0,
                },
            );
            self.doc.notes.push(note);
        }
        for e in groups {
            let cv = e
                .attribute("categoryValueRef")
                .and_then(|r| self.by_id.get(r).copied());
            let label = cv.and_then(|c| clean_label(c.attribute("value")));
            let id = self.tid_s(e.attribute("id"));
            self.doc.groups.push(Base::new(
                "group",
                Some(id.clone()),
                label,
                Attrs::new(),
                Vec::new(),
                0,
            ));
            self.doc.ids.insert(
                id.clone(),
                IdEntry {
                    kind: "group".into(),
                    line: 0,
                },
            );
            self.warn(
                Some(&id),
                "group membership is not stored in BPMN; add members=\"…\" by hand",
                Some("groups".into()),
            );
        }
    }

    fn sequence_flow(&mut self, e: X<'a, 'i>) {
        let src = self.tid_s(e.attribute("sourceRef"));
        let tgt = self.tid_s(e.attribute("targetRef"));
        if !self.doc.nodes.contains_key(&src) || !self.doc.nodes.contains_key(&tgt) {
            self.warn(
                e.attribute("id"),
                "sequence flow to a skipped element was dropped",
                None,
            );
            return;
        }
        let fid = self.tid_s(e.attribute("id"));
        let mut base = Base::new(
            "flow",
            Some(fid),
            clean_label(e.attribute("name")),
            Attrs::new(),
            Vec::new(),
            0,
        );
        if let Some(c) = find(e, "conditionExpression")
            && !strip(text(c).unwrap_or("")).is_empty()
        {
            base.attrs.insert("when".into(), strip_feel(text(c)));
        }
        if let Some(srcel) = e.attribute("sourceRef").and_then(|r| self.by_id.get(r))
            && srcel.attribute("default").is_some()
            && srcel.attribute("default") == e.attribute("id")
        {
            base.flags.push("default".into());
        }
        self.doc.flows.push(Flow {
            base,
            source: src,
            target: tgt,
            explicit_id: false,
        });
    }

    fn associations(&mut self, assocs: &[X<'a, 'i>]) {
        let mut items: Vec<((u8, u64), usize, Association)> = Vec::new();
        for (k, e) in assocs.iter().enumerate() {
            let Some(owner) = e.parent_element() else {
                continue;
            };
            let Some(oid) = owner.attribute("id").filter(|i| !i.is_empty()) else {
                continue;
            };
            let act = self.tid_s(Some(oid));
            let (refname, kind) = if local(*e) == "dataInputAssociation" {
                ("sourceRef", "reads")
            } else {
                ("targetRef", "writes")
            };
            let refs: Vec<String> = children(*e)
                .filter(|r| is_b(*r, refname))
                .filter_map(|r| {
                    text(r)
                        .filter(|t| !t.is_empty())
                        .map(|t| strip(t).to_string())
                })
                .collect();
            let mut data_ids = Vec::new();
            for r in &refs {
                let t = self.tid_s(Some(r));
                if self.doc.data.contains_key(&t) {
                    data_ids.push(t);
                }
            }
            if data_ids.is_empty() {
                self.warn(
                    e.attribute("id"),
                    "data association without a data object or store was skipped",
                    None,
                );
                continue;
            }
            let id = e.attribute("id").unwrap_or("");
            let key = match tail_number(id) {
                Some(n) => (0u8, n),
                None => (1u8, k as u64),
            };
            for x in data_ids {
                let base = Base::new(
                    kind,
                    Some(format!("{kind}-{act}-{x}")),
                    None,
                    Attrs::new(),
                    Vec::new(),
                    0,
                );
                items.push((
                    key,
                    k,
                    Association {
                        base,
                        activity: act.clone(),
                        data: x,
                    },
                ));
            }
        }
        items.sort_by_key(|a| (a.0, a.1));
        self.doc.assocs.extend(items.into_iter().map(|t| t.2));
    }

    fn message_flow(&mut self, e: X<'a, 'i>) {
        let src = self.tid_s(e.attribute("sourceRef"));
        let tgt = self.tid_s(e.attribute("targetRef"));
        let mid = self.tid_s(e.attribute("id"));
        let label = clean_label(e.attribute("name"));
        let mut base = Base::new(
            "message",
            Some(mid),
            label.clone(),
            Attrs::new(),
            Vec::new(),
            0,
        );
        if let Some(r) = e.attribute("messageRef").filter(|r| !r.is_empty()) {
            let name = self.named(r, "message");
            if Some(&name) != label.as_ref() {
                base.attrs.insert("message".into(), name);
            }
        }
        self.doc.messages.push(MessageFlow {
            base,
            source: src,
            target: tgt,
            explicit_id: false,
        });
    }

    fn derive_flow_ids(&mut self) {
        let d = &mut self.doc;
        let mut known: HashSet<String> = d.nodes.keys().cloned().collect();
        known.extend(d.lanes.keys().cloned());
        known.extend(d.data.keys().cloned());
        known.extend(d.pools.iter().map(|p| p.base.id().to_string()));
        let mut counter: IndexMap<String, usize> = IndexMap::new();
        let peek =
            |counter: &IndexMap<String, usize>, known: &HashSet<String>, src: &str, tgt: &str| {
                let base = format!("f-{src}-{tgt}");
                let mut n = counter.get(&base).copied().unwrap_or(0) + 1;
                let mut fid = if n == 1 {
                    base.clone()
                } else {
                    format!("{base}-{n}")
                };
                while known.contains(&fid) {
                    n += 1;
                    fid = format!("{base}-{n}");
                }
                (base, fid, n)
            };
        for f in &mut d.flows {
            let (base, expected, n) = peek(&counter, &known, &f.source, &f.target);
            if self.keep_ids && f.base.id() != expected {
                f.explicit_id = true;
            } else {
                f.base.id = Some(expected);
                counter.insert(base, n);
            }
            known.insert(f.base.id().to_string());
            d.ids.insert(
                f.base.id().to_string(),
                IdEntry {
                    kind: "flow".into(),
                    line: 0,
                },
            );
        }
        for m in &mut d.messages {
            let (base, expected, n) = peek(&counter, &known, &format!("m-{}", m.source), &m.target);
            let expected = expected[2..].to_string();
            if self.keep_ids && m.base.id() != expected {
                m.explicit_id = true;
            } else {
                m.base.id = Some(expected);
                counter.insert(base, n);
            }
            known.insert(m.base.id().to_string());
            d.ids.insert(
                m.base.id().to_string(),
                IdEntry {
                    kind: "message".into(),
                    line: 0,
                },
            );
        }
        let mut renamed = Vec::new();
        for x in d.notes.iter_mut().chain(d.groups.iter_mut()) {
            if known.contains(x.id()) {
                let old = x.id().to_string();
                let new = if x.kind == "note" {
                    format!("{old}-note")
                } else {
                    format!("{old}-group")
                };
                x.id = Some(new.clone());
                renamed.push((old, new));
            }
        }
        for (old, new) in renamed {
            self.warn(
                Some(&old),
                &format!("renamed to '{new}' to avoid a clash with a derived flow identifier"),
                None,
            );
        }
    }
}

/// `-(?:dia|doa)-(\d+)$`
fn tail_number(id: &str) -> Option<u64> {
    let digits_start = id
        .rfind(|c: char| !c.is_ascii_digit())
        .map(|i| i + 1)
        .unwrap_or(0);
    let digits = &id[digits_start..];
    if digits.is_empty() {
        return None;
    }
    let head = &id[..digits_start];
    if head.ends_with("-dia-") || head.ends_with("-doa-") {
        digits.parse().ok()
    } else {
        None
    }
}

#[derive(Default)]
struct Order {
    children: Vec<String>,
    nodes: Vec<String>,
    data: Vec<String>,
}
