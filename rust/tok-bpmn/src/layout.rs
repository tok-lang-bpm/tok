//! Automatic layout (`layout()` of `bpmn.py`): computes positions of all shapes
//! and waypoints of all edges, and writes them as BPMNDI into the document.

use std::collections::{HashMap, HashSet};

use crate::model::{Model, is_act, is_data, is_flownode, is_gw, is_subp, m};
use crate::pyfmt::round_str;
use crate::svg::wrap;
use crate::xml::{BPMNDI_NS, DC_NS, DI_NS, Id, qn};

const COL_GAP: i64 = 50;
const ROW_GAP: i64 = 50;
const PAD_L: i64 = 30;
const PAD_R: i64 = 30;
const PAD_T: i64 = 30;
const PAD_B: i64 = 30;
const SUB_PAD_T: i64 = 40;
const MIN_LANE_H: i64 = 130;
const POOL_GAP: i64 = 40;
const BB_H: i64 = 70;

/// Python `None` for a missing id or reference, as a dictionary key.
const NONE: &str = "\u{0}";

/// Key of an optional id: the id, or the `None` sentinel.
pub(crate) fn key(v: Option<&str>) -> String {
    v.unwrap_or(NONE).to_string()
}

type Rect = [i64; 4];
pub(crate) type Pt = (f64, f64);

/// Insertion-ordered map (Python `dict` semantics for the parts that need it).
#[derive(Debug, Clone, Default)]
pub(crate) struct OMap<V> {
    pub keys: Vec<String>,
    pub map: HashMap<String, V>,
}

impl<V: Clone> OMap<V> {
    pub fn insert(&mut self, k: &str, v: V) {
        if !self.map.contains_key(k) {
            self.keys.push(k.to_string());
        }
        self.map.insert(k.to_string(), v);
    }
    pub fn get(&self, k: &str) -> Option<&V> {
        self.map.get(k)
    }
    pub fn contains(&self, k: &str) -> bool {
        self.map.contains_key(k)
    }
    pub fn items(&self) -> Vec<(String, V)> {
        self.keys
            .iter()
            .map(|k| (k.clone(), self.map[k].clone()))
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
struct LBox {
    pos: OMap<Rect>,
    children: HashMap<String, LBox>,
    w: i64,
    h: i64,
    lanes: Vec<(Option<String>, i64, i64)>,
    back: HashSet<String>,
}

fn even10(v: i64) -> i64 {
    (v + 9).div_euclid(10) * 10
}

fn node_size(md: &Model, e: Id) -> (i64, i64) {
    let t = md.tag(e);
    if is_act(&t) {
        return (100, 80);
    }
    if crate::model::is_evt(&t) {
        return (36, 36);
    }
    if is_gw(&t) {
        return (50, 50);
    }
    if t == "dataObjectReference" {
        return (36, 50);
    }
    if t == "dataStoreReference" {
        return (50, 50);
    }
    if t == "textAnnotation" {
        let txt = md.doc.findtext(e, &m("text")).unwrap_or_default();
        let lines = wrap(txt.trim(), 120.0, 11.0).len().max(1);
        let lines = i64::try_from(lines).unwrap_or(i64::MAX / 32);
        return (130, (even10(lines * 14 + 10)).max(30));
    }
    (100, 80)
}

fn strip_text(md: &Model, e: Id) -> String {
    md.doc.text(e).unwrap_or("").trim().to_string()
}

#[allow(clippy::too_many_lines)]
fn layout_container(md: &Model, cont: Id, lane_list: Option<&[Id]>, top_pad: i64) -> LBox {
    let mut bx = LBox::default();
    let d = &md.doc;
    let kids: Vec<Id> = d.children(cont).to_vec();
    let nodes: Vec<Id> = kids
        .iter()
        .copied()
        .filter(|c| {
            let t = md.tag(*c);
            is_flownode(&t) && t != "boundaryEvent" && !md.is_event_sub(*c)
        })
        .collect();
    let evsubs: Vec<Id> = kids
        .iter()
        .copied()
        .filter(|c| md.is_event_sub(*c))
        .collect();
    let extras: Vec<Id> = kids
        .iter()
        .copied()
        .filter(|c| {
            let t = md.tag(*c);
            is_data(&t) || t == "textAnnotation"
        })
        .collect();
    let kid = |e: Id| key(md.id_of(e));
    let mut sizes: HashMap<String, (i64, i64)> = HashMap::new();
    for &n in nodes.iter().chain(evsubs.iter()) {
        let nid = kid(n);
        if is_subp(&md.tag(n)) && md.has_flow_children(n) {
            let cb = layout_container(md, n, None, SUB_PAD_T);
            bx.back.extend(cb.back.iter().cloned());
            sizes.insert(nid.clone(), (cb.w, cb.h));
            bx.children.insert(nid, cb);
        } else {
            sizes.insert(nid, node_size(md, n));
        }
    }
    for &x in &extras {
        sizes.insert(kid(x), node_size(md, x));
    }

    let ids: Vec<String> = nodes.iter().map(|n| kid(*n)).collect();
    let idset: HashSet<String> = ids.iter().cloned().collect();
    let mut host_of: HashMap<String, String> = HashMap::new();
    for &n in &nodes {
        for &b in md.boundaries_of(md.id_of(n)) {
            host_of.insert(kid(b), kid(n));
        }
    }
    let mut succ: HashMap<String, Vec<(String, bool)>> =
        ids.iter().map(|i| (i.clone(), Vec::new())).collect();
    let mut pred: HashMap<String, Vec<(String, bool)>> =
        ids.iter().map(|i| (i.clone(), Vec::new())).collect();
    let mut flows_of: HashMap<(String, String), Vec<String>> = HashMap::new();
    for &f in &md.seq {
        if md.parent.get(&f).copied() != Some(cont) {
            continue;
        }
        let s = key(md.attr(f, "sourceRef"));
        let t = key(md.attr(f, "targetRef"));
        let via_b = host_of.contains_key(&s);
        let s2 = host_of.get(&s).cloned().unwrap_or(s);
        if idset.contains(&s2) && idset.contains(&t) {
            if let Some(v) = succ.get_mut(&s2) {
                v.push((t.clone(), via_b));
            }
            if let Some(v) = pred.get_mut(&t) {
                v.push((s2.clone(), via_b));
            }
            flows_of.entry((s2, t)).or_default().push(kid(f));
        }
    }

    // reversed edges (loops): DFS
    let starts: Vec<String> = ids
        .iter()
        .filter(|i| {
            md.get(Some(i.as_str()))
                .is_some_and(|e| md.tag(e) == "startEvent")
        })
        .cloned()
        .collect();
    let mut roots: Vec<String> = starts.clone();
    roots.extend(
        ids.iter()
            .filter(|i| pred[*i].is_empty() && !starts.contains(i))
            .cloned(),
    );
    roots.extend(ids.iter().cloned());
    let mut color: HashMap<String, u8> = ids.iter().map(|i| (i.clone(), 0)).collect();
    let mut disc: HashMap<String, usize> = HashMap::new();
    let mut back: HashSet<(String, String)> = HashSet::new();
    let mut cnt = 0usize;
    for rt in &roots {
        if color[rt] != 0 {
            continue;
        }
        color.insert(rt.clone(), 1);
        disc.insert(rt.clone(), cnt);
        cnt += 1;
        let mut stack: Vec<(String, usize)> = vec![(rt.clone(), 0)];
        while let Some((u, pos)) = stack.last().cloned() {
            let nxt = succ[&u].get(pos).cloned();
            let Some((v, _)) = nxt else {
                color.insert(u, 2);
                stack.pop();
                continue;
            };
            if let Some(top) = stack.last_mut() {
                top.1 += 1;
            }
            match color[&v] {
                1 => {
                    back.insert((u.clone(), v.clone()));
                }
                0 => {
                    color.insert(v.clone(), 1);
                    disc.insert(v.clone(), cnt);
                    cnt += 1;
                    stack.push((v, 0));
                }
                _ => {}
            }
        }
    }
    for (u, v) in &back {
        if let Some(fl) = flows_of.get(&(u.clone(), v.clone())) {
            bx.back.extend(fl.iter().cloned());
        }
    }

    // layers
    let mut layer: HashMap<String, i64> = ids.iter().map(|i| (i.clone(), 0)).collect();
    let mut indeg: HashMap<String, i64> = ids.iter().map(|i| (i.clone(), 0)).collect();
    let mut fwd: HashMap<String, Vec<String>> =
        ids.iter().map(|i| (i.clone(), Vec::new())).collect();
    for u in &ids {
        for (v, _) in succ[u].clone() {
            if back.contains(&(u.clone(), v.clone())) {
                continue;
            }
            if let Some(x) = fwd.get_mut(u) {
                x.push(v.clone());
            }
            if let Some(x) = indeg.get_mut(&v) {
                *x += 1;
            }
        }
    }
    let mut queue: Vec<String> = ids.iter().filter(|i| indeg[*i] == 0).cloned().collect();
    queue.sort_by_key(|i| disc.get(i).copied().unwrap_or(0));
    let mut topo: Vec<String> = Vec::new();
    while !queue.is_empty() {
        let u = queue.remove(0);
        topo.push(u.clone());
        for v in fwd[&u].clone() {
            let lu = layer[&u];
            if let Some(x) = layer.get_mut(&v) {
                *x = (*x).max(lu + 1);
            }
            let mut dv = 1;
            if let Some(x) = indeg.get_mut(&v) {
                *x -= 1;
                dv = *x;
            }
            if dv == 0 {
                queue.push(v);
            }
        }
    }
    for i in &ids {
        if !topo.contains(i) {
            topo.push(i.clone());
        }
    }

    // lanes
    let nl = lane_list.map_or(1, <[Id]>::len);
    let mut lane_idx: HashMap<String, usize> = HashMap::new();
    if let Some(ll) = lane_list {
        for (k, ln) in ll.iter().enumerate() {
            for rf in d.findall(*ln, &m("flowNodeRef")) {
                lane_idx.entry(strip_text(md, rf)).or_insert(k);
            }
        }
    }
    for i in &topo {
        if !lane_idx.contains_key(i) {
            let ps: Vec<&String> = pred[i]
                .iter()
                .map(|(p, _)| p)
                .filter(|p| lane_idx.contains_key(*p))
                .collect();
            let v = ps.first().map_or(0, |p| lane_idx[*p]);
            lane_idx.insert(i.clone(), v);
        }
    }

    // rows
    let mut occ: HashSet<(usize, i64, i64)> = HashSet::new();
    let mut row: HashMap<String, i64> = HashMap::new();
    let place = |i: &str,
                 lay: i64,
                 pref: i64,
                 occ: &mut HashSet<(usize, i64, i64)>,
                 row: &mut HashMap<String, i64>,
                 lane_idx: &HashMap<String, usize>| {
        let li = lane_idx[i];
        let mut rr = pref.max(0);
        while occ.contains(&(li, lay, rr)) {
            rr += 1;
        }
        occ.insert((li, lay, rr));
        row.insert(i.to_string(), rr);
    };
    let mut order_ids = ids.clone();
    order_ids.sort_by_key(|x| (layer[x], disc.get(x).copied().unwrap_or(1_000_000)));
    for i in &order_ids {
        let mut pref = 0;
        for (p, vb) in &pred[i] {
            if back.contains(&(p.clone(), i.clone())) {
                continue;
            }
            if lane_idx.get(p) == lane_idx.get(i) && row.contains_key(p) {
                pref = row[p] + i64::from(*vb);
                break;
            }
        }
        place(i, layer[i], pref, &mut occ, &mut row, &lane_idx);
    }

    // data objects and annotations
    let mut assoc_target: OMap<Vec<String>> = OMap::default();
    let link = |xid: &str, nid: &str, at: &mut OMap<Vec<String>>| {
        let mut lst = at.get(xid).cloned().unwrap_or_default();
        if !lst.iter().any(|x| x == nid) {
            lst.push(nid.to_string());
        }
        at.insert(xid, lst);
    };
    let evsub_ids: HashSet<String> = evsubs.iter().map(|e| kid(*e)).collect();
    let lift = |e0: Id| -> String {
        let mut e = e0;
        loop {
            let eid = kid(e);
            if idset.contains(&eid) || evsub_ids.contains(&eid) {
                return eid;
            }
            if let Some(h) = host_of.get(&eid) {
                return h.clone();
            }
            match md.parent.get(&e).copied() {
                None => return eid,
                Some(p) if p == cont => return eid,
                Some(p) => e = p,
            }
        }
    };
    for n in d.iter(cont) {
        if n == cont || !is_flownode(&md.tag(n)) {
            continue;
        }
        let nid = lift(n);
        for da in d.findall(n, &m("dataInputAssociation")) {
            for s in d.findall(da, &m("sourceRef")) {
                link(&strip_text(md, s), &nid, &mut assoc_target);
            }
        }
        for da in d.findall(n, &m("dataOutputAssociation")) {
            if let Some(t) = d.find(da, &m("targetRef")) {
                link(&strip_text(md, t), &nid, &mut assoc_target);
            }
        }
    }
    for &a in &md.assoc {
        let s = key(md.attr(a, "sourceRef"));
        let t = key(md.attr(a, "targetRef"));
        if idset.contains(&t) {
            link(&s, &t, &mut assoc_target);
        }
        if idset.contains(&s) {
            link(&t, &s, &mut assoc_target);
        }
    }
    for &x in &extras {
        let xid = kid(x);
        let targets = assoc_target.get(&xid).cloned().unwrap_or_default();
        let hosts: Vec<String> = targets
            .iter()
            .filter(|h| idset.contains(*h))
            .cloned()
            .collect();
        if hosts.is_empty() {
            let evh: Vec<String> = targets
                .iter()
                .filter(|h| evsub_ids.contains(*h))
                .cloned()
                .collect();
            if let (Some(first), Some(ll)) = (evh.first(), lane_list) {
                for (kk, ln) in ll.iter().enumerate() {
                    if d.findall(*ln, &m("flowNodeRef"))
                        .iter()
                        .any(|r_| strip_text(md, *r_) == *first)
                    {
                        lane_idx.insert(xid.clone(), kk);
                    }
                }
                layer.insert(xid.clone(), 0);
                lane_idx.entry(xid.clone()).or_insert(nl - 1);
                place(&xid, 0, 0, &mut occ, &mut row, &lane_idx);
                continue;
            }
        }
        let mut host: Option<String> = None;
        for h in &hosts {
            if !occ.contains(&(lane_idx[h], layer[h], row[h] + 1)) {
                host = Some(h.clone());
                break;
            }
        }
        if host.is_none() {
            host = hosts.first().cloned();
        }

        let pref = match host.filter(|h| idset.contains(h)) {
            Some(h) => {
                layer.insert(xid.clone(), layer[&h]);
                lane_idx.insert(xid.clone(), lane_idx[&h]);
                row[&h] + 1
            }
            None => {
                layer.insert(xid.clone(), 0);
                lane_idx.insert(xid.clone(), 0);
                0
            }
        };
        place(&xid, layer[&xid], pref, &mut occ, &mut row, &lane_idx);
    }

    // event sub-processes: own row at the bottom of their lane
    let mut ev_rows: OMap<i64> = OMap::default();
    for &ev in &evsubs {
        let eid = kid(ev);
        let mut k = nl - 1;
        if let Some(ll) = lane_list {
            for (kk, ln) in ll.iter().enumerate() {
                if d.findall(*ln, &m("flowNodeRef"))
                    .iter()
                    .any(|r_| strip_text(md, *r_) == eid)
                {
                    k = kk;
                }
            }
        }
        lane_idx.insert(eid.clone(), k);
        let mut used: Vec<i64> = occ
            .iter()
            .filter(|(ll, _, _)| *ll == k)
            .map(|(_, _, rr)| *rr)
            .collect();
        for (e2, rr) in ev_rows.items() {
            if lane_idx[&e2] == k {
                used.push(rr);
            }
        }
        let v = used.iter().max().map_or(0, |x| x + 1);
        ev_rows.insert(&eid, v);
    }

    // columns
    let mut all_ids: Vec<String> = ids.clone();
    all_ids.extend(extras.iter().map(|x| kid(*x)));
    let nlay = all_ids.iter().map(|i| layer[i]).max().map_or(1, |x| x + 1);
    let nlay_u = usize::try_from(nlay).unwrap_or(1);
    let mut colw: Vec<i64> = vec![36; nlay_u];
    for i in &all_ids {
        let l = usize::try_from(layer[i]).unwrap_or(0);
        colw[l] = colw[l].max(sizes[i].0);
    }
    let mut colx: Vec<i64> = Vec::new();
    let mut x = PAD_L;
    for w in &colw {
        colx.push(x);
        x += w + COL_GAP;
    }
    let mut content_w = x - COL_GAP + PAD_R;
    for &ev in &evsubs {
        content_w = content_w.max(PAD_L + sizes[&kid(ev)].0 + PAD_R);
    }
    content_w = even10(content_w);

    // row and lane heights
    let mut rowh: OMap<i64> = OMap::default();
    for i in &all_ids {
        let k = format!("{}:{}", lane_idx[i], row[i]);
        let v = rowh.get(&k).copied().unwrap_or(0).max(sizes[i].1);
        rowh.insert(&k, v);
    }
    for (eid, rr) in ev_rows.items() {
        let k = format!("{}:{}", lane_idx[&eid], rr);
        let v = rowh.get(&k).copied().unwrap_or(0).max(sizes[&eid].1);
        rowh.insert(&k, v);
    }
    let rowh_get = |k: usize, rr: i64| rowh.get(&format!("{k}:{rr}")).copied().unwrap_or(0);
    let mut lane_geo: Vec<(i64, i64, HashMap<i64, i64>)> = Vec::new();
    let mut y = 0;
    for k in 0..nl {
        let mut rows: Vec<i64> = rowh
            .keys
            .iter()
            .filter_map(|s| {
                let (a, b) = s.split_once(':')?;
                (a.parse::<usize>().ok()? == k)
                    .then(|| b.parse::<i64>().ok())
                    .flatten()
            })
            .collect();
        rows.sort_unstable();
        let top = y + top_pad;
        let mut rtop: HashMap<i64, i64> = HashMap::new();
        let mut yy = top;
        for rr in &rows {
            rtop.insert(*rr, yy);
            yy += rowh_get(k, *rr) + ROW_GAP;
        }
        let bottom = if rows.is_empty() { top } else { yy - ROW_GAP };
        let min_h = if lane_list.is_some() { MIN_LANE_H } else { 0 };
        let lane_h = even10((bottom - y + PAD_B).max(min_h));
        lane_geo.push((y, lane_h, rtop));
        y += lane_h;
    }
    bx.h = y;
    bx.w = content_w;
    for i in &all_ids {
        let k = lane_idx[i];
        let rr = row[i];
        let rtop = &lane_geo[k].2;
        let (w, h) = sizes[i];
        let l = usize::try_from(layer[i]).unwrap_or(0);
        let cx = colx[l] + colw[l].div_euclid(2);
        let cy = rtop[&rr] + rowh_get(k, rr).div_euclid(2);
        bx.pos
            .insert(i, [cx - w.div_euclid(2), cy - h.div_euclid(2), w, h]);
    }
    for (eid, rr) in ev_rows.items() {
        let k = lane_idx[&eid];
        let rtop = &lane_geo[k].2;
        let (w, h) = sizes[&eid];
        bx.pos.insert(&eid, [PAD_L, rtop[&rr], w, h]);
    }
    bx.lanes = match lane_list {
        Some(ll) => (0..nl)
            .map(|k| {
                (
                    md.id_of(ll[k]).map(str::to_string),
                    lane_geo[k].0,
                    lane_geo[k].1,
                )
            })
            .collect(),
        None => vec![(None, 0, bx.h)],
    };
    bx
}

fn place_box(bx: &LBox, x0: i64, y0: i64, out: &mut OMap<Rect>) {
    for (i, r) in bx.pos.items() {
        out.insert(&i, [x0 + r[0], y0 + r[1], r[2], r[3]]);
        if let Some(cb) = bx.children.get(&i) {
            place_box(cb, x0 + r[0], y0 + r[1], out);
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn rf(r: Rect) -> [f64; 4] {
    [r[0] as f64, r[1] as f64, r[2] as f64, r[3] as f64]
}

fn clip(b: Rect, px: f64, py: f64) -> Pt {
    let [x, y, w, h] = rf(b);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let (dx, dy) = (px - cx, py - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let sx = if dx == 0.0 { 1e9 } else { (w / 2.0) / dx.abs() };
    let sy = if dy == 0.0 { 1e9 } else { (h / 2.0) / dy.abs() };
    let s = sx.min(sy);
    (cx + dx * s, cy + dy * s)
}

fn seg_hits(p1: Pt, p2: Pt, b: [f64; 4]) -> bool {
    let [x, y, w, h] = b;
    let (x0, y0, x1, y1) = (x + 3.0, y + 3.0, x + w - 3.0, y + h - 3.0);
    let ((ax, ay), (bx, by)) = (p1, p2);
    if (ax - bx).abs() < 0.5 {
        return x0 < ax && ax < x1 && ay.min(by).max(y0) < ay.max(by).min(y1);
    }
    if (ay - by).abs() < 0.5 {
        return y0 < ay && ay < y1 && ax.min(bx).max(x0) < ax.max(bx).min(x1);
    }
    false
}

fn score(pts: &[Pt], obstacles: &[[f64; 4]]) -> usize {
    let mut hits = 0;
    for k in 0..pts.len().saturating_sub(1) {
        for ob in obstacles {
            if seg_hits(pts[k], pts[k + 1], *ob) {
                hits += 1;
            }
        }
    }
    hits * 100 + pts.len()
}

/// `min(cands, key=score)`: first candidate with the lowest score.
fn best(cands: Vec<Vec<Pt>>, obs: &[[f64; 4]]) -> Vec<Pt> {
    let mut best: Option<(usize, Vec<Pt>)> = None;
    for c in cands {
        let s = score(&c, obs);
        if best.as_ref().is_none_or(|(bs, _)| s < *bs) {
            best = Some((s, c));
        }
    }
    best.map(|(_, c)| c).unwrap_or_default()
}

fn obstacles_for(
    md: &Model,
    out: &OMap<Rect>,
    skip: &HashSet<String>,
    container: Option<Id>,
) -> Vec<[f64; 4]> {
    let mut anc: HashSet<String> = HashSet::new();
    let mut c = container;
    while let Some(x) = c {
        anc.insert(key(md.id_of(x)));
        c = md.container(x);
    }
    let mut obs = Vec::new();
    for (i, bx) in out.items() {
        if skip.contains(&i) || anc.contains(&i) {
            continue;
        }
        let Some(e) = md.get(Some(&i)) else {
            continue;
        };
        let t = md.tag(e);
        if is_flownode(&t) || is_data(&t) || t == "textAnnotation" {
            if t == "boundaryEvent" && skip.contains(&key(md.attr(e, "attachedToRef"))) {
                continue;
            }
            obs.push(rf(bx));
        }
    }
    obs
}

#[allow(clippy::too_many_lines)]
fn route_seq(
    md: &Model,
    f: Id,
    out: &OMap<Rect>,
    back_ids: &HashSet<String>,
    counter: &mut i64,
) -> Vec<Pt> {
    let s_id = key(md.attr(f, "sourceRef"));
    let t_id = key(md.attr(f, "targetRef"));
    let [sx, sy, sw, sh] = rf(out.map[&s_id]);
    let [tx, ty, tw, th] = rf(out.map[&t_id]);
    let se = md.get(Some(&s_id));
    let te = md.get(Some(&t_id));
    let stag = se.map(|e| md.tag(e)).unwrap_or_default();
    let ttag = te.map(|e| md.tag(e)).unwrap_or_default();
    let (scx, scy) = (sx + sw / 2.0, sy + sh / 2.0);
    let (tcx, tcy) = (tx + tw / 2.0, ty + th / 2.0);
    let (sb, tb) = (sy + sh, ty + th);
    let s_split = is_gw(&stag) && md.out_of(Some(&s_id)).len() > 1;
    let t_join = is_gw(&ttag) && md.inc_of(Some(&t_id)).len() > 1;
    let mut skip: HashSet<String> = [s_id.clone(), t_id.clone()].into_iter().collect();
    if stag == "boundaryEvent" {
        skip.insert(key(se.and_then(|e| md.attr(e, "attachedToRef"))));
    }
    let obs = obstacles_for(md, out, &skip, md.parent.get(&f).copied());
    let mut cands: Vec<Vec<Pt>> = Vec::new();
    #[allow(clippy::cast_precision_loss)]
    let cnt = |c: i64| c as f64;

    if stag == "boundaryEvent" {
        if tx >= sx + sw {
            if tcy >= sb + 5.0 {
                cands.push(vec![(scx, sb), (scx, tcy), (tx, tcy)]);
            }
            let yb = sb + 20.0;
            cands.push(vec![
                (scx, sb),
                (scx, yb),
                (tx - 20.0, yb),
                (tx - 20.0, tcy),
                (tx, tcy),
            ]);
        }
        if ty > sb + 30.0 {
            cands.push(vec![
                (scx, sb),
                (scx, ty - 20.0),
                (tcx, ty - 20.0),
                (tcx, ty),
            ]);
        }
        *counter += 1;
        let yb = sb.max(tb) + 25.0 + 8.0 * cnt(counter.rem_euclid(4));
        cands.push(vec![(scx, sb), (scx, yb), (tcx, yb), (tcx, tb)]);
        return best(cands, &obs);
    }

    let backward = back_ids.contains(&key(md.id_of(f))) || tx < sx + sw - 1.0;
    let half_gap = (COL_GAP / 2) as f64;
    if !backward {
        if (scy - tcy).abs() < 1.5 {
            cands.push(vec![(sx + sw, scy), (tx, tcy)]);
        }
        if s_split {
            let ey = if tcy > scy { sb } else { sy };
            cands.push(vec![(scx, ey), (scx, tcy), (tx, tcy)]);
        }
        if t_join {
            let ey = if scy < tcy { ty } else { tb };
            cands.push(vec![(sx + sw, scy), (tcx, scy), (tcx, ey)]);
        }
        let midx = tx - half_gap;
        cands.push(vec![(sx + sw, scy), (midx, scy), (midx, tcy), (tx, tcy)]);
        let midx2 = sx + sw + half_gap;
        if midx2 < tx - 5.0 {
            cands.push(vec![(sx + sw, scy), (midx2, scy), (midx2, tcy), (tx, tcy)]);
        }
        if s_split {
            let ey = if tcy > scy { sb } else { sy };
            let jy = ey + if tcy > scy { 15.0 } else { -15.0 };
            cands.push(vec![
                (scx, ey),
                (scx, jy),
                (midx, jy),
                (midx, tcy),
                (tx, tcy),
            ]);
        }
        return best(cands, &obs);
    }
    // backward flow (loop)
    if ty - 20.0 > sb + 5.0 {
        cands.push(vec![
            (scx, sb),
            (scx, ty - 20.0),
            (tcx, ty - 20.0),
            (tcx, ty),
        ]);
    }
    if tb + 20.0 < sy - 5.0 {
        cands.push(vec![
            (scx, sy),
            (scx, tb + 20.0),
            (tcx, tb + 20.0),
            (tcx, tb),
        ]);
    }
    *counter += 1;
    let k = 8.0 * cnt(counter.rem_euclid(4));
    let lo: Vec<&[f64; 4]> = obs
        .iter()
        .filter(|b| b[0] < scx.max(tcx) && b[0] + b[2] > scx.min(tcx))
        .collect();
    let yb = sb.max(tb) + 25.0 + k;
    cands.push(vec![(scx, sb), (scx, yb), (tcx, yb), (tcx, tb)]);
    let ya = sy.min(ty) - 25.0 - k;
    cands.push(vec![(scx, sy), (scx, ya), (tcx, ya), (tcx, ty)]);
    if !lo.is_empty() {
        let mut m_ = sb.max(tb);
        for b in &lo {
            if b[1] >= sy.min(ty) - 5.0 {
                m_ = m_.max(b[1] + b[3]);
            }
        }
        let yb2 = m_ + 25.0 + k;
        cands.push(vec![(scx, sb), (scx, yb2), (tcx, yb2), (tcx, tb)]);
    }
    best(cands, &obs)
}

fn event_side_point(b: Rect, right: bool) -> Pt {
    let [x, y, w, h] = rf(b);
    let dy = 8.0;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    if (w - h).abs() < f64::EPSILON && w <= 40.0 {
        let r = w / 2.0;
        let dx = (r * r - dy * dy).powf(0.5);
        return if right {
            (cx + dx, cy + dy)
        } else {
            (cx - dx, cy + dy)
        };
    }
    if right {
        (x + w, cy + dy)
    } else {
        (x, cy + dy)
    }
}

fn route_msg_node_pool(
    md: &Model,
    node_id: &str,
    pool: Rect,
    out: &OMap<Rect>,
    from_node: bool,
    k: i64,
) -> Vec<Pt> {
    let n = out.map[node_id];
    let [nx, ny, nw, nh] = rf(n);
    let ncx = nx + nw / 2.0;
    let [_px, py, _pw, ph] = rf(pool);
    let below = py >= ny + nh;
    let edge_y = if below { py } else { py + ph };
    let skip: HashSet<String> = std::iter::once(node_id.to_string()).collect();
    let obs = obstacles_for(md, out, &skip, None);
    let mut cands: Vec<Vec<Pt>> = Vec::new();
    if below {
        cands.push(vec![(ncx, ny + nh), (ncx, edge_y)]);
    } else {
        cands.push(vec![(ncx, ny), (ncx, edge_y)]);
    }
    #[allow(clippy::cast_precision_loss)]
    let kf = k as f64;
    for right in [true, false] {
        let p0 = event_side_point(n, right);
        let chx = if right {
            nx + nw + 14.0 + 6.0 * kf
        } else {
            nx - 14.0 - 6.0 * kf
        };
        cands.push(vec![p0, (chx, p0.1), (chx, edge_y)]);
    }
    let mut b = best(cands, &obs);
    if !from_node {
        b.reverse();
    }
    b
}

fn spread_ports(md: &Model, wp: &mut OMap<Vec<Pt>>, out: &OMap<Rect>) {
    let mut sides: OMap<Vec<(String, usize)>> = OMap::default();
    for (fid, pts) in wp.items() {
        let Some(e) = md.get(Some(&fid)) else {
            continue;
        };
        let t = md.tag(e);
        if (t != "sequenceFlow" && t != "messageFlow") || pts.len() < 2 {
            continue;
        }
        for (idx, nid) in [
            (0usize, key(md.attr(e, "sourceRef"))),
            (pts.len() - 1, key(md.attr(e, "targetRef"))),
        ] {
            let ne = md.get(Some(&nid));
            if !out.contains(&nid) || ne.is_none_or(|x| !is_act(&md.tag(x))) {
                continue;
            }
            let [x, y, w, h] = rf(out.map[&nid]);
            let (px, py) = pts[idx];
            let side = if (px - (x + w / 2.0)).abs() < 0.6 && (py - y).abs() < 0.6 {
                Some("top")
            } else if (px - (x + w / 2.0)).abs() < 0.6 && (py - (y + h)).abs() < 0.6 {
                Some("bottom")
            } else if (px - x).abs() < 0.6 && (py - (y + h / 2.0)).abs() < 0.6 {
                Some("left")
            } else if (px - (x + w)).abs() < 0.6 && (py - (y + h / 2.0)).abs() < 0.6 {
                Some("right")
            } else {
                None
            };
            if let Some(side) = side {
                let k = format!("{nid}\u{1}{side}");
                let mut lst = sides.get(&k).cloned().unwrap_or_default();
                lst.push((fid.clone(), idx));
                sides.insert(&k, lst);
            }
        }
    }
    for (k, mut lst) in sides.items() {
        if lst.len() < 2 {
            continue;
        }
        let (nid, side) = k.split_once('\u{1}').unwrap_or((&k, ""));
        let [_x, _y, w, h] = rf(out.map[nid]);
        let horiz = side == "top" || side == "bottom";
        let far = |item: &(String, usize)| -> f64 {
            let pts = &wp.map[&item.0];
            let q = if item.1 == 0 {
                pts[pts.len() - 1]
            } else {
                pts[0]
            };
            if horiz { q.0 } else { q.1 }
        };
        let mut keyed: Vec<(f64, (String, usize))> =
            lst.drain(..).map(|it| (far(&it), it)).collect();
        keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        #[allow(clippy::cast_precision_loss)]
        let n = keyed.len() as f64;
        let span = if horiz { w } else { h };
        let step = 16.0_f64.min((span - 20.0) / (n - 1.0).max(1.0));
        for (k, (_, (fid, idx))) in keyed.into_iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let d = (k as f64 - (n - 1.0) / 2.0) * step;
            let Some(pts) = wp.map.get_mut(&fid) else {
                continue;
            };
            let nb = if idx == 0 { 1 } else { pts.len() - 2 };
            if horiz {
                let old = pts[idx].0;
                pts[idx] = (old + d, pts[idx].1);
                if pts.len() == 2 || (pts[nb].0 - old).abs() < 0.6 {
                    pts[nb] = (pts[nb].0 + d, pts[nb].1);
                }
            } else {
                let old = pts[idx].1;
                pts[idx] = (pts[idx].0, old + d);
                if pts.len() == 2 || (pts[nb].1 - old).abs() < 0.6 {
                    pts[nb] = (pts[nb].0, pts[nb].1 + d);
                }
            }
        }
    }
}

/// Generate the complete BPMNDI of the model (replacing an existing diagram).
/// Returns the number of shapes, like `bpmn.py layout()`.
#[allow(clippy::too_many_lines)]
pub fn layout_model(md: &mut Model) -> usize {
    let mut out: OMap<Rect> = OMap::default();
    let mut back: HashSet<String> = HashSet::new();
    let mut lanes_out: OMap<Rect> = OMap::default();
    let mut pools: HashMap<String, Rect> = HashMap::new();
    let plane_ref: Option<String>;
    if let Some(&collab) = md.collabs.first() {
        plane_ref = md.id_of(collab).map(str::to_string);
        struct Entry {
            p: Id,
            lb: Option<LBox>,
            lanes_all: Vec<(Id, usize, Option<Id>)>,
            maxd: i64,
        }
        let mut entries: Vec<Entry> = Vec::new();
        for p in md.doc.findall(collab, &m("participant")) {
            let proc = md.get(md.attr(p, "processRef"));
            match proc {
                Some(pr) if md.tag(pr) == "process" && md.has_flow_children(pr) => {
                    let lanes_all = md.lanes_of(pr);
                    let leaves: Vec<Id> = lanes_all
                        .iter()
                        .filter(|(l, _, _)| md.is_leaf_lane(*l))
                        .map(|(l, _, _)| *l)
                        .collect();
                    let maxd = lanes_all
                        .iter()
                        .map(|(_, dd, _)| i64::try_from(*dd).unwrap_or(0))
                        .max()
                        .map_or(0, |x| x + 1);
                    let lb = layout_container(
                        md,
                        pr,
                        if leaves.is_empty() {
                            None
                        } else {
                            Some(&leaves)
                        },
                        PAD_T,
                    );
                    back.extend(lb.back.iter().cloned());
                    entries.push(Entry {
                        p,
                        lb: Some(lb),
                        lanes_all,
                        maxd,
                    });
                }
                _ => entries.push(Entry {
                    p,
                    lb: None,
                    lanes_all: Vec::new(),
                    maxd: 0,
                }),
            }
        }
        let mut w_all = 600;
        for en in &entries {
            if let Some(lb) = &en.lb {
                w_all = w_all.max(30 + en.maxd * 30 + lb.w);
            }
        }
        let mut y = 0;
        for en in &entries {
            let pid = key(md.id_of(en.p));
            let Some(lb) = &en.lb else {
                out.insert(&pid, [0, y, w_all, BB_H]);
                pools.insert(pid.clone(), [0, y, w_all, BB_H]);
                y += BB_H + POOL_GAP;
                continue;
            };
            out.insert(&pid, [0, y, w_all, lb.h]);
            pools.insert(pid.clone(), [0, y, w_all, lb.h]);
            let x0 = 30 + en.maxd * 30;
            place_box(lb, x0, y, &mut out);
            let band: HashMap<String, (i64, i64)> = lb
                .lanes
                .iter()
                .filter_map(|(lid, ly, lh)| {
                    lid.clone()
                        .filter(|l| !l.is_empty())
                        .map(|l| (l, (*ly, *lh)))
                })
                .collect();
            if !en.lanes_all.is_empty() {
                fn lane_range(
                    md: &Model,
                    lane: Id,
                    band: &HashMap<String, (i64, i64)>,
                    h: i64,
                ) -> (i64, i64) {
                    if md.is_leaf_lane(lane) {
                        return band.get(&key(md.id_of(lane))).copied().unwrap_or((0, h));
                    }
                    let cls = md.doc.find(lane, &m("childLaneSet"));
                    let kids = cls
                        .map(|c| md.doc.findall(c, &m("lane")))
                        .unwrap_or_default();
                    let rs: Vec<(i64, i64)> =
                        kids.iter().map(|c| lane_range(md, *c, band, h)).collect();
                    let y0 = rs.iter().map(|(a, _)| *a).min().unwrap_or(0);
                    let y1 = rs.iter().map(|(a, b)| a + b).max().unwrap_or(0);
                    (y0, y1 - y0)
                }
                for (lane, depth, _par) in &en.lanes_all {
                    let (ly, lh) = lane_range(md, *lane, &band, lb.h);
                    let dep = i64::try_from(*depth).unwrap_or(0);
                    lanes_out.insert(
                        &key(md.id_of(*lane)),
                        [30 + dep * 30, y + ly, w_all - 30 - dep * 30, lh],
                    );
                }
            }
            y += lb.h + POOL_GAP;
        }
    } else {
        plane_ref = md
            .processes
            .first()
            .and_then(|p| md.id_of(*p))
            .map(str::to_string);
        let mut y = 0;
        for &proc in &md.processes.clone() {
            let lb = layout_container(md, proc, None, PAD_T);
            back.extend(lb.back.iter().cloned());
            place_box(&lb, 0, y, &mut out);
            y += lb.h + POOL_GAP;
        }
    }

    // boundary events
    for (host, bl) in md.boundaries.clone() {
        let Some(&[hx, hy, hw, hh]) = out.get(&host) else {
            continue;
        };
        for (k, b) in bl.iter().enumerate() {
            let k = i64::try_from(k).unwrap_or(0);
            let cx = (hx + hw - 30 - 44 * k).max(hx + 18);
            out.insert(&key(md.id_of(*b)), [cx - 18, hy + hh - 18, 36, 36]);
        }
    }

    // edges
    let mut wp: OMap<Vec<Pt>> = OMap::default();
    let mut counter = 0i64;
    for &f in &md.seq {
        if out.contains(&key(md.attr(f, "sourceRef")))
            && out.contains(&key(md.attr(f, "targetRef")))
        {
            let pts = route_seq(md, f, &out, &back, &mut counter);
            wp.insert(&key(md.id_of(f)), pts);
        }
    }
    let mut allbox = out.clone();
    for (k, v) in lanes_out.items() {
        allbox.insert(&k, v);
    }
    let mut mcount: HashMap<(String, String), i64> = HashMap::new();
    for &f in &md.msg {
        let s_id = key(md.attr(f, "sourceRef"));
        let t_id = key(md.attr(f, "targetRef"));
        if !allbox.contains(&s_id) || !allbox.contains(&t_id) {
            continue;
        }
        let se = md.get(Some(&s_id));
        let te = md.get(Some(&t_id));
        let s = allbox.map[&s_id];
        let t = allbox.map[&t_id];
        let ps = md.participant_of(se);
        let pt = md.participant_of(te);
        let s_is_pool = se.is_some_and(|e| md.tag(e) == "participant");
        let t_is_pool = te.is_some_and(|e| md.tag(e) == "participant");
        let kk = (
            ps.map_or_else(|| s_id.clone(), |p| key(md.id_of(p))),
            pt.map_or_else(|| t_id.clone(), |p| key(md.id_of(p))),
        );
        let k = mcount.get(&kk).copied().unwrap_or(0);
        mcount.insert(kk, k + 1);
        let fid = key(md.id_of(f));
        if t_is_pool && !s_is_pool && out.contains(&s_id) {
            wp.insert(&fid, route_msg_node_pool(md, &s_id, t, &out, true, k));
            continue;
        }
        if s_is_pool && !t_is_pool && out.contains(&t_id) {
            wp.insert(&fid, route_msg_node_pool(md, &t_id, s, &out, false, k));
            continue;
        }
        let s_pool: Option<Rect> = match ps {
            Some(p) => pools.get(&key(md.id_of(p))).copied(),
            None => Some(s),
        };
        let [sx, sy, sw, sh] = rf(s);
        let [tx, ty, tw, th] = rf(t);
        let mut sxc = sx + sw / 2.0;
        let mut txc = tx + tw / 2.0;
        if s_is_pool {
            sxc = txc.max(sx + 40.0).min(sx + sw - 10.0);
        }
        if t_is_pool {
            txc = sxc.max(tx + 40.0).min(tx + tw - 10.0);
        }
        #[allow(clippy::cast_precision_loss)]
        let (kf, gap) = (k as f64, POOL_GAP as f64);
        let (y1, y2, gy);
        if sy + sh / 2.0 <= ty + th / 2.0 {
            y1 = sy + sh;
            y2 = ty;
            gy = match s_pool {
                Some(sp) => {
                    let sp = rf(sp);
                    sp[1] + sp[3] + gap / 2.0 - 6.0 + 6.0 * kf
                }
                None => (y1 + y2) / 2.0,
            };
        } else {
            y1 = sy;
            y2 = ty + th;
            gy = match s_pool {
                Some(sp) => rf(sp)[1] - gap / 2.0 + 6.0 - 6.0 * kf,
                None => (y1 + y2) / 2.0,
            };
        }
        if (sxc - txc).abs() < 1.0 {
            wp.insert(&fid, vec![(sxc, y1), (txc, y2)]);
        } else {
            wp.insert(&fid, vec![(sxc, y1), (sxc, gy), (txc, gy), (txc, y2)]);
        }
    }
    spread_ports(md, &mut wp, &out);
    for &a in &md.assoc {
        let s_id = key(md.attr(a, "sourceRef"));
        let t_id = key(md.attr(a, "targetRef"));
        if out.contains(&s_id) && out.contains(&t_id) {
            let s = out.map[&s_id];
            let t = out.map[&t_id];
            let [sx, sy, sw, sh] = rf(s);
            let [tx, ty, tw, th] = rf(t);
            let p1 = clip(s, tx + tw / 2.0, ty + th / 2.0);
            let p2 = clip(t, sx + sw / 2.0, sy + sh / 2.0);
            wp.insert(&key(md.id_of(a)), vec![p1, p2]);
        }
    }
    let mut n_da = 0;
    for e in md.doc.iter(md.doc.root) {
        let t = md.tag(e);
        if t != "dataInputAssociation" && t != "dataOutputAssociation" {
            continue;
        }
        let Some(owner) = md.parent.get(&e).copied() else {
            continue;
        };
        let oid = key(md.id_of(owner));
        if !out.contains(&oid) {
            continue;
        }
        let pair: Vec<(String, String)> = if t == "dataInputAssociation" {
            md.doc
                .findall(e, &m("sourceRef"))
                .iter()
                .map(|s| strip_text(md, *s))
                .filter(|sid| out.contains(sid))
                .map(|sid| (sid, oid.clone()))
                .collect()
        } else {
            let tid = md
                .doc
                .find(e, &m("targetRef"))
                .map(|tg| strip_text(md, tg))
                .unwrap_or_default();
            if out.contains(&tid) {
                vec![(oid.clone(), tid)]
            } else {
                Vec::new()
            }
        };
        if pair.is_empty() {
            continue;
        }
        if md.id_of(e).is_none_or(str::is_empty) {
            n_da += 1;
            let mut nid = format!("DataAssociation_{n_da}");
            while md.ids.contains_key(&nid) {
                n_da += 1;
                nid = format!("DataAssociation_{n_da}");
            }
            md.doc.set(e, "id", &nid);
            md.ids.insert(nid, e);
        }
        let (a, b) = pair[0].clone();
        let s = out.map[&a];
        let tb = out.map[&b];
        let [sx, sy, sw, sh] = rf(s);
        let [tx, ty, tw, th] = rf(tb);
        let p1 = clip(s, tx + tw / 2.0, ty + th / 2.0);
        let p2 = clip(tb, sx + sw / 2.0, sy + sh / 2.0);
        wp.insert(&key(md.id_of(e)), vec![p1, p2]);
    }

    // writing DI
    let root = md.doc.root;
    for dg in md.doc.findall(root, &qn(BPMNDI_NS, "BPMNDiagram")) {
        md.doc.remove_child(root, dg);
    }
    let diag = md.doc.sub_element(
        root,
        &qn(BPMNDI_NS, "BPMNDiagram"),
        &[("id", "BPMNDiagram_1")],
    );
    let plane_ref_s = plane_ref.unwrap_or_default();
    let plane = md.doc.sub_element(
        diag,
        &qn(BPMNDI_NS, "BPMNPlane"),
        &[("id", "BPMNPlane_1"), ("bpmnElement", &plane_ref_s)],
    );
    let mut shapes = out.clone();
    for (k, v) in lanes_out.items() {
        shapes.insert(&k, v);
    }
    let mut order: Vec<String> = Vec::new();
    for &p in &md.participants {
        let pid = key(md.id_of(p));
        if shapes.contains(&pid) {
            order.push(pid);
        }
    }
    for &proc in &md.processes {
        for (ln, _, _) in md.lanes_of(proc) {
            let lid = key(md.id_of(ln));
            if shapes.contains(&lid) {
                order.push(lid);
            }
        }
    }
    for e in md.doc.iter(root) {
        let i = key(md.id_of(e));
        if shapes.contains(&i) && !order.contains(&i) && md.tag(e) != "boundaryEvent" {
            order.push(i);
        }
    }
    for e in md.doc.iter_tag(root, &m("boundaryEvent")) {
        let i = key(md.id_of(e));
        if shapes.contains(&i) {
            order.push(i);
        }
    }
    for i in &order {
        let Some(e) = md.get(Some(i)) else {
            continue;
        };
        let t = md.tag(e);
        let di_id = format!("{i}_di");
        let mut attrs: Vec<(&str, &str)> = vec![("id", &di_id), ("bpmnElement", i)];
        if t == "participant" || t == "lane" {
            attrs.push(("isHorizontal", "true"));
        }
        if is_subp(&t) {
            attrs.push((
                "isExpanded",
                if md.has_flow_children(e) {
                    "true"
                } else {
                    "false"
                },
            ));
        }
        if t == "exclusiveGateway" {
            attrs.push(("isMarkerVisible", "true"));
        }
        let sh = md
            .doc
            .sub_element(plane, &qn(BPMNDI_NS, "BPMNShape"), &attrs);
        let [x, y, w, h] = rf(shapes.map[i]);
        let (xs, ys, ws, hs) = (round_str(x), round_str(y), round_str(w), round_str(h));
        md.doc.sub_element(
            sh,
            &qn(DC_NS, "Bounds"),
            &[("x", &xs), ("y", &ys), ("width", &ws), ("height", &hs)],
        );
    }
    for e in md.doc.iter(root) {
        if md.id_of(e).is_none() {
            continue;
        }
        let i = key(md.id_of(e));
        if let Some(pts) = wp.get(&i).cloned() {
            let di_id = format!("{i}_di");
            let ed = md.doc.sub_element(
                plane,
                &qn(BPMNDI_NS, "BPMNEdge"),
                &[("id", &di_id), ("bpmnElement", &i)],
            );
            for (px, py) in pts {
                let (xs, ys) = (round_str(px), round_str(py));
                md.doc
                    .sub_element(ed, &qn(DI_NS, "waypoint"), &[("x", &xs), ("y", &ys)]);
            }
        }
    }
    md.reindex();
    order.len()
}
