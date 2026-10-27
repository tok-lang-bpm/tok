//! Quality metrics of the automatic layout, shared by the regression test
//! `tests/layout_quality.rs` and the example `examples/layout_metrics.rs`.
//!
//! Every metric counts defects, so lower is better:
//!
//! | Metric | Counts |
//! |---|---|
//! | `cross` | crossings of two edges (pairs of segments that cross in their interiors) |
//! | `hits` | edges passing through a shape they do not connect (once per edge and shape) |
//! | `shapes` | pairs of overlapping shapes (nesting, boundary events on their host excepted) |
//! | `labels` | labels overlapping another label, a shape, or the border of a pool |
//! | `lbl_edge` | labels crossed by an edge |
//! | `bends` | bends of sequence and message flows |
//! | `msg` | message flows passing through a pool or lane holding neither of their ends |
//! | `flow_ovl` | pairs of edges running on top of each other (common source or target excepted) |
//! | `bnd` | flows of boundary events that do not leave downwards, cross their host or go back |
//! | `area` | area of the diagram (shapes and waypoints), in thousands of square units |
//!
//! Labels are read from the SVG drawing (texts of font size 9 and 10 outside activities);
//! everything else from the BPMNDI of the laid-out XML.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use tok_core::check::{BpmnIssue, BpmnRules};

const BPMN: &str = "http://www.omg.org/spec/BPMN/20100524/MODEL";
const BPMNDI: &str = "http://www.omg.org/spec/BPMN/20100524/DI";
const DC: &str = "http://www.omg.org/spec/DD/20100524/DC";
const DI: &str = "http://www.omg.org/spec/DD/20100524/DI";

pub const NAMES: [&str; 10] = [
    "cross", "hits", "shapes", "labels", "lbl_edge", "bends", "msg", "flow_ovl", "bnd", "area",
];
pub type Metrics = [i64; 10];

/// One model of the corpus: semantic BPMN XML (or a drawn file) to lay out.
pub struct Case {
    pub name: String,
    pub xml: String,
    /// Part of this repository (the course examples are optional).
    pub in_repo: bool,
}

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

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    v.sort();
    v
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn add_tok(out: &mut Vec<Case>, group: &str, dir: &Path, in_repo: bool) {
    for p in files(dir, "tok") {
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        // documents with checker errors (e.g. unknown resources) still export, so they are
        // part of the corpus; documents that do not parse are not
        if tok_core::parse(&text)
            .1
            .iter()
            .any(tok_core::Diag::is_error)
        {
            continue;
        }
        let (doc, _) = tok_core::load(&text, "full", Some(&Rules));
        out.push(Case {
            name: format!("{group}/{}", stem(&p)),
            xml: tok_core::export::export_xml(&doc, None),
            in_repo,
        });
    }
}

fn add_bpmn(out: &mut Vec<Case>, group: &str, dir: &Path) {
    for p in files(dir, "bpmn") {
        let Ok(bytes) = fs::read(&p) else {
            continue;
        };
        out.push(Case {
            name: format!("{group}/{}", stem(&p)),
            xml: String::from_utf8_lossy(&bytes).to_string(),
            in_repo: true,
        });
    }
}

/// The corpus: language examples, conformance cases that parse, the BPMN import corpus
/// (the course diagrams), the stress fixtures of this crate and, when the course repository
/// is checked out next to this one, the course examples.
pub fn corpus() -> Vec<Case> {
    let root = manifest().join("../..");
    let mut out = Vec::new();
    add_tok(&mut out, "examples", &root.join("examples"), true);
    add_tok(&mut out, "cases", &root.join("conformance/cases"), true);
    add_bpmn(&mut out, "import", &root.join("conformance/import"));
    add_bpmn(&mut out, "extra", &manifest().join("tests/fixtures/extra"));
    add_bpmn(
        &mut out,
        "camunda",
        &manifest().join("tests/fixtures/camunda"),
    );
    add_tok(
        &mut out,
        "course",
        &root.join("../tok-course/przyklady"),
        false,
    );
    out
}

// ------------------------------------------------------------------ geometry

type Pt = (f64, f64);
type Rect = [f64; 4];

fn shrink(r: Rect, d: f64) -> Rect {
    [r[0] + d, r[1] + d, r[2] - 2.0 * d, r[3] - 2.0 * d]
}

/// The segment passes through the open interior of the rectangle (Liang-Barsky clipping).
fn seg_in_rect(a: Pt, b: Pt, r: Rect) -> bool {
    if r[2] <= 0.0 || r[3] <= 0.0 {
        return false;
    }
    let (x0, y0, x1, y1) = (r[0], r[1], r[0] + r[2], r[1] + r[3]);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut t0 = 0.0_f64;
    let mut t1 = 1.0_f64;
    for (p, q) in [
        (-dx, a.0 - x0),
        (dx, x1 - a.0),
        (-dy, a.1 - y0),
        (dy, y1 - a.1),
    ] {
        if p.abs() < 1e-12 {
            if q <= 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t1 - t0 > 1e-9
}

fn poly_in_rect(pts: &[Pt], r: Rect) -> bool {
    pts.windows(2).any(|w| seg_in_rect(w[0], w[1], r))
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}

fn contains(outer: Rect, inner: Rect) -> bool {
    inner[0] >= outer[0] - 0.5
        && inner[1] >= outer[1] - 0.5
        && inner[0] + inner[2] <= outer[0] + outer[2] + 0.5
        && inner[1] + inner[3] <= outer[1] + outer[3] + 0.5
}

fn orient(a: Pt, b: Pt, c: Pt) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Proper crossing: the segments cross at a point inside both of them.
fn seg_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let eps = 1e-6
        * (1.0 + (b.0 - a.0).abs() + (b.1 - a.1).abs())
        * (1.0 + (d.0 - c.0).abs() + (d.1 - c.1).abs());
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    ((o1 > eps && o2 < -eps) || (o1 < -eps && o2 > eps))
        && ((o3 > eps && o4 < -eps) || (o3 < -eps && o4 > eps))
}

/// Two axis-parallel segments on the same line sharing more than 4 units.
fn seg_overlap(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let horiz = |p: Pt, q: Pt| (p.1 - q.1).abs() < 0.5 && (p.0 - q.0).abs() > 0.5;
    let vert = |p: Pt, q: Pt| (p.0 - q.0).abs() < 0.5 && (p.1 - q.1).abs() > 0.5;
    if horiz(a, b) && horiz(c, d) && (a.1 - c.1).abs() < 1.0 {
        let lo = a.0.min(b.0).max(c.0.min(d.0));
        let hi = a.0.max(b.0).min(c.0.max(d.0));
        return hi - lo > 4.0;
    }
    if vert(a, b) && vert(c, d) && (a.0 - c.0).abs() < 1.0 {
        let lo = a.1.min(b.1).max(c.1.min(d.1));
        let hi = a.1.max(b.1).min(c.1.max(d.1));
        return hi - lo > 4.0;
    }
    false
}

fn bends(pts: &[Pt]) -> i64 {
    let mut dirs: Vec<Pt> = Vec::new();
    for w in pts.windows(2) {
        let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let l = (dx * dx + dy * dy).sqrt();
        if l < 0.5 {
            continue;
        }
        dirs.push((dx / l, dy / l));
    }
    let mut n = 0;
    for w in dirs.windows(2) {
        if (w[0].0 - w[1].0).abs() + (w[0].1 - w[1].1).abs() > 1e-3 {
            n += 1;
        }
    }
    n
}

// ------------------------------------------------------------------ model

#[derive(Default)]
struct El {
    tag: String,
    /// containers (sub-processes) from the innermost outwards
    anc: Vec<String>,
    process: Option<String>,
    source: Option<String>,
    target: Option<String>,
    attached: Option<String>,
}

const ACTS: &[&str] = &[
    "task",
    "userTask",
    "serviceTask",
    "sendTask",
    "receiveTask",
    "manualTask",
    "scriptTask",
    "businessRuleTask",
    "subProcess",
    "adHocSubProcess",
    "transaction",
    "callActivity",
];

fn is_node_tag(t: &str) -> bool {
    ACTS.contains(&t)
        || t.ends_with("Event")
        || t.ends_with("Gateway")
        || t == "dataObjectReference"
        || t == "dataStoreReference"
        || t == "textAnnotation"
}

fn is_sub(t: &str) -> bool {
    matches!(t, "subProcess" | "adHocSubProcess" | "transaction")
}

fn num(n: roxmltree::Node, k: &str) -> f64 {
    n.attribute(k)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

struct Label {
    rect: Rect,
}

fn svg_labels(svg: &str, acts: &[Rect]) -> Vec<Label> {
    let Ok(doc) = roxmltree::Document::parse(svg) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for t in doc.descendants().filter(|n| n.has_tag_name("text")) {
        let fs = num(t, "font-size");
        if !(fs == 9.0 || fs == 10.0) {
            continue;
        }
        let (x, y) = (num(t, "x"), num(t, "y"));
        if acts
            .iter()
            .any(|r| x > r[0] && x < r[0] + r[2] && y > r[1] && y < r[1] + r[3])
        {
            continue;
        }
        let lines: Vec<String> = t
            .children()
            .filter(|c| c.has_tag_name("tspan"))
            .map(|c| c.text().unwrap_or("").to_string())
            .collect();
        if lines.is_empty() {
            continue;
        }
        let chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        #[allow(clippy::cast_precision_loss)]
        let (w, n) = (chars as f64 * fs * 0.55, lines.len() as f64);
        let x0 = match t.attribute("text-anchor") {
            Some("middle") => x - w / 2.0,
            Some("end") => x - w,
            _ => x,
        };
        let top = y - 0.75 * fs;
        let h = (n - 1.0) * 1.2 * fs + fs;
        out.push(Label {
            rect: [x0, top, w, h],
        });
    }
    out
}

/// Metrics of one laid-out model (`laid`: BPMN XML with BPMNDI, `svg`: its drawing).
#[allow(clippy::too_many_lines)]
pub fn measure(laid: &str, svg: &str) -> Metrics {
    let doc = roxmltree::Document::parse(laid).expect("laid-out XML is well formed");
    let mut els: HashMap<String, El> = HashMap::new();
    let mut part_of_proc: HashMap<String, String> = HashMap::new();
    let mut lane_nodes: HashMap<String, HashSet<String>> = HashMap::new();
    let mut shapes: Vec<(String, Rect)> = Vec::new();
    let mut edges: Vec<(String, Vec<Pt>)> = Vec::new();
    for n in doc.descendants().filter(roxmltree::Node::is_element) {
        let ns = n.tag_name().namespace().unwrap_or("");
        let tag = n.tag_name().name();
        if ns == BPMNDI && tag == "BPMNShape" {
            if let Some(b) = n
                .children()
                .find(|c| c.tag_name().namespace() == Some(DC) && c.tag_name().name() == "Bounds")
            {
                shapes.push((
                    n.attribute("bpmnElement").unwrap_or("").to_string(),
                    [num(b, "x"), num(b, "y"), num(b, "width"), num(b, "height")],
                ));
            }
            continue;
        }
        if ns == BPMNDI && tag == "BPMNEdge" {
            let pts: Vec<Pt> = n
                .children()
                .filter(|c| {
                    c.tag_name().namespace() == Some(DI) && c.tag_name().name() == "waypoint"
                })
                .map(|c| (num(c, "x"), num(c, "y")))
                .collect();
            edges.push((n.attribute("bpmnElement").unwrap_or("").to_string(), pts));
            continue;
        }
        if ns != BPMN {
            continue;
        }
        if tag == "participant"
            && let (Some(id), Some(p)) = (n.attribute("id"), n.attribute("processRef"))
        {
            part_of_proc.insert(p.to_string(), id.to_string());
        }
        if tag == "lane" {
            let id = n.attribute("id").unwrap_or("").to_string();
            let mut set = HashSet::new();
            for d in n
                .descendants()
                .filter(|c| c.has_tag_name((BPMN, "flowNodeRef")))
            {
                set.insert(d.text().unwrap_or("").trim().to_string());
            }
            lane_nodes.insert(id, set);
        }
        let Some(id) = n.attribute("id") else {
            continue;
        };
        let mut el = El {
            tag: tag.to_string(),
            ..El::default()
        };
        for a in n.ancestors().skip(1) {
            if a.has_tag_name((BPMN, "process")) {
                el.process = a.attribute("id").map(str::to_string);
                break;
            }
            if is_sub(a.tag_name().name()) && a.tag_name().namespace() == Some(BPMN) {
                el.anc.push(a.attribute("id").unwrap_or("").to_string());
            }
        }
        el.source = n.attribute("sourceRef").map(str::to_string);
        el.target = n.attribute("targetRef").map(str::to_string);
        el.attached = n.attribute("attachedToRef").map(str::to_string);
        if tag == "dataInputAssociation" || tag == "dataOutputAssociation" {
            let owner = n
                .parent()
                .and_then(|p| p.attribute("id"))
                .map(str::to_string);
            let child = |k: &str| {
                n.children()
                    .find(|c| c.has_tag_name((BPMN, k)))
                    .and_then(|c| c.text())
                    .map(|s| s.trim().to_string())
            };
            if tag == "dataInputAssociation" {
                el.source = child("sourceRef");
                el.target = owner;
            } else {
                el.source = owner;
                el.target = child("targetRef");
            }
        }
        els.insert(id.to_string(), el);
    }
    let tag_of = |i: &str| els.get(i).map_or("", |e| e.tag.as_str());
    let anc_of = |i: &str| -> Vec<String> { els.get(i).map(|e| e.anc.clone()).unwrap_or_default() };
    let pool_of = |i: &str| -> Option<String> {
        if tag_of(i) == "participant" {
            return Some(i.to_string());
        }
        els.get(i)
            .and_then(|e| e.process.as_ref())
            .and_then(|p| part_of_proc.get(p).cloned())
    };
    let nodes: Vec<(String, Rect)> = shapes
        .iter()
        .filter(|(i, _)| is_node_tag(tag_of(i)))
        .cloned()
        .collect();
    let pools: Vec<(String, Rect)> = shapes
        .iter()
        .filter(|(i, _)| tag_of(i) == "participant")
        .cloned()
        .collect();
    let lanes: Vec<(String, Rect)> = shapes
        .iter()
        .filter(|(i, _)| tag_of(i) == "lane")
        .cloned()
        .collect();
    let edges: Vec<(String, Vec<Pt>)> = edges.into_iter().filter(|(_, p)| p.len() >= 2).collect();
    let ends = |i: &str| -> (String, String) {
        let e = els.get(i);
        (
            e.and_then(|e| e.source.clone()).unwrap_or_default(),
            e.and_then(|e| e.target.clone()).unwrap_or_default(),
        )
    };
    let mut m: Metrics = [0; 10];

    // cross, flow_ovl
    for a in 0..edges.len() {
        for b in a + 1..edges.len() {
            let (pa, pb) = (&edges[a].1, &edges[b].1);
            let mut ovl = false;
            for s in pa.windows(2) {
                for t in pb.windows(2) {
                    if seg_cross(s[0], s[1], t[0], t[1]) {
                        m[0] += 1;
                    }
                    if seg_overlap(s[0], s[1], t[0], t[1]) {
                        ovl = true;
                    }
                }
            }
            let (sa, ta) = ends(&edges[a].0);
            let (sb, tb) = ends(&edges[b].0);
            if ovl && sa != sb && ta != tb {
                m[7] += 1;
            }
        }
    }

    // hits, bends, bnd
    for (fid, pts) in &edges {
        let (s, t) = ends(fid);
        let mut exempt: HashSet<String> = [s.clone(), t.clone()].into_iter().collect();
        exempt.extend(anc_of(&s));
        exempt.extend(anc_of(&t));
        let host = els.get(&s).and_then(|e| e.attached.clone());
        if let Some(h) = &host {
            exempt.insert(h.clone());
        }
        for (nid, r) in &nodes {
            if !exempt.contains(nid) && poly_in_rect(pts, shrink(*r, 3.0)) {
                m[1] += 1;
            }
        }
        let tag = tag_of(fid);
        if tag == "sequenceFlow" || tag == "messageFlow" {
            m[5] += bends(pts);
        }
        if tag == "sequenceFlow" && tag_of(&s) == "boundaryEvent" {
            let (a, b) = (pts[0], pts[1]);
            if !(b.1 > a.1 + 0.5 && (b.0 - a.0).abs() < 0.5) {
                m[8] += 1;
            }
            if let Some(hr) = host
                .as_ref()
                .and_then(|h| shapes.iter().find(|(i, _)| i == h).map(|(_, r)| *r))
                && poly_in_rect(pts, shrink(hr, 3.0))
            {
                m[8] += 1;
            }
            let left: f64 = pts.windows(2).map(|w| (w[0].0 - w[1].0).max(0.0)).sum();
            let need = (pts[0].0 - pts[pts.len() - 1].0).max(0.0);
            if left > need + 1.0 {
                m[8] += 1;
            }
        }
        if tag == "messageFlow" {
            let mine: Vec<Option<String>> = vec![pool_of(&s), pool_of(&t)];
            for (pid, r) in &pools {
                if !mine.contains(&Some(pid.clone())) && poly_in_rect(pts, shrink(*r, 1.0)) {
                    m[6] += 1;
                }
            }
            for (lid, r) in &lanes {
                let set = lane_nodes.get(lid);
                let holds = set.is_some_and(|x| x.contains(&s) || x.contains(&t));
                let own_pool = pool_of(&s)
                    .is_some_and(|p| pools.iter().any(|(q, pr)| *q == p && contains(*pr, *r)))
                    || pool_of(&t)
                        .is_some_and(|p| pools.iter().any(|(q, pr)| *q == p && contains(*pr, *r)));
                // lanes of a foreign pool are already counted with their pool
                if !holds && own_pool && poly_in_rect(pts, shrink(*r, 1.0)) {
                    m[6] += 1;
                }
            }
        }
    }

    // shapes
    for a in 0..nodes.len() {
        for b in a + 1..nodes.len() {
            let (ia, ra) = (&nodes[a].0, nodes[a].1);
            let (ib, rb) = (&nodes[b].0, nodes[b].1);
            if anc_of(ia).contains(ib) || anc_of(ib).contains(ia) {
                continue;
            }
            let att = |x: &str| els.get(x).and_then(|e| e.attached.clone());
            if att(ia).as_deref() == Some(ib.as_str()) || att(ib).as_deref() == Some(ia.as_str()) {
                continue;
            }
            if rects_overlap(shrink(ra, 1.0), shrink(rb, 1.0)) {
                m[2] += 1;
            }
        }
    }
    for a in 0..pools.len() {
        for b in a + 1..pools.len() {
            if rects_overlap(shrink(pools[a].1, 1.0), shrink(pools[b].1, 1.0)) {
                m[2] += 1;
            }
        }
    }

    // labels
    let acts: Vec<Rect> = nodes
        .iter()
        .filter(|(i, _)| ACTS.contains(&tag_of(i)))
        .map(|(_, r)| *r)
        .collect();
    let labels = svg_labels(svg, &acts);
    for a in 0..labels.len() {
        let ra = shrink(labels[a].rect, 1.0);
        for lb in &labels[a + 1..] {
            if rects_overlap(ra, shrink(lb.rect, 1.0)) {
                m[3] += 1;
            }
        }
        for (nid, r) in &nodes {
            if is_sub(tag_of(nid)) && r[2] > 100.0 {
                continue;
            }
            if rects_overlap(ra, *r) {
                m[3] += 1;
            }
        }
        for (_, r) in &pools {
            if rects_overlap(ra, *r) && !contains(*r, labels[a].rect) {
                m[3] += 1;
            }
        }
        for (_, pts) in &edges {
            if poly_in_rect(pts, ra) {
                m[4] += 1;
            }
        }
    }

    // area
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for (_, r) in &shapes {
        xs.extend([r[0], r[0] + r[2]]);
        ys.extend([r[1], r[1] + r[3]]);
    }
    for (_, pts) in &edges {
        for p in pts {
            xs.push(p.0);
            ys.push(p.1);
        }
    }
    if !xs.is_empty() {
        let w = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - xs.iter().copied().fold(f64::INFINITY, f64::min);
        let h = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - ys.iter().copied().fold(f64::INFINITY, f64::min);
        #[allow(clippy::cast_possible_truncation)]
        {
            m[9] = (w * h / 1000.0).round() as i64;
        }
    }
    m
}

/// Lay out and draw one model; returns the laid-out XML and the SVG.
pub fn run(xml: &str) -> (String, String) {
    let laid = tok_bpmn::layout(xml).expect("layout");
    let svg = tok_bpmn::render_svg(&laid, Some("model.bpmn")).unwrap_or_default();
    (laid, svg)
}

/// Table of metrics: header line and one tab-separated line per model.
pub fn table(rows: &[(String, Metrics)]) -> String {
    let mut s = format!("model\t{}\n", NAMES.join("\t"));
    for (name, m) in rows {
        let cells: Vec<String> = m.iter().map(ToString::to_string).collect();
        s.push_str(&format!("{name}\t{}\n", cells.join("\t")));
    }
    s
}

/// Read a table written by [`table`].
pub fn read_table(text: &str) -> HashMap<String, Metrics> {
    let mut out = HashMap::new();
    for line in text.lines().skip(1) {
        let cells: Vec<&str> = line.split('\t').collect();
        if cells.len() != NAMES.len() + 1 {
            continue;
        }
        let mut m: Metrics = [0; 10];
        for (k, c) in cells[1..].iter().enumerate() {
            m[k] = c.trim().parse().unwrap_or(0);
        }
        out.insert(cells[0].to_string(), m);
    }
    out
}
