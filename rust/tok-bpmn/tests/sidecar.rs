//! Layout sidecar: the diagrams of the course files (`conformance/import`) and of the MIWG
//! reference models (`conformance/miwg`) survive `tok import` → `tok bpmn`. Every shape,
//! edge and label that the import keeps is drawn with the geometry of the original file
//! (in whole units, as the sidecar stores it).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn corpus(dir: &str) -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance")
        .join(dir);
    let mut files: Vec<PathBuf> = fs::read_dir(&root)
        .expect("corpus folder")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "bpmn"))
        .collect();
    files.sort();
    files
}

/// Geometry of a laid-out file: shapes, edges and labels by element, in whole units.
fn geometry(xml: &str) -> HashMap<String, Vec<i64>> {
    let doc = roxmltree::Document::parse(xml).expect("laid-out XML");
    let num = |n: roxmltree::Node, k: &str| -> i64 {
        n.attribute(k)
            .and_then(|v| v.parse::<f64>().ok())
            .map_or(0, |v| {
                #[allow(clippy::cast_possible_truncation)]
                let r = v.round() as i64;
                r
            })
    };
    let bounds = |n: roxmltree::Node| -> Option<Vec<i64>> {
        let b = n
            .children()
            .find(|c| c.has_tag_name("Bounds") || c.tag_name().name() == "Bounds")?;
        Some(vec![
            num(b, "x"),
            num(b, "y"),
            num(b, "width"),
            num(b, "height"),
        ])
    };
    let mut out = HashMap::new();
    for n in doc.descendants().filter(|n| n.is_element()) {
        let name = n.tag_name().name();
        if (name != "BPMNShape" && name != "BPMNEdge")
            || n.attribute("choreographyActivityShape").is_some()
        {
            continue;
        }
        let Some(el) = n.attribute("bpmnElement") else {
            continue;
        };
        let g = if name == "BPMNShape" {
            bounds(n).unwrap_or_default()
        } else {
            n.children()
                .filter(|c| c.tag_name().name() == "waypoint")
                .flat_map(|w| [num(w, "x"), num(w, "y")])
                .collect()
        };
        out.entry(el.to_string()).or_insert(g);
        if let Some(l) = n.children().find(|c| c.tag_name().name() == "BPMNLabel")
            && let Some(lb) = bounds(l)
        {
            out.entry(format!("{el}/label")).or_insert(lb);
        }
    }
    out
}

fn round_trip(path: &PathBuf) -> Result<usize, String> {
    let original = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (doc, _, ids) =
        tok_core::import::import_bpmn_mapped(&original, true).map_err(|e| e.to_string())?;
    let map: HashMap<String, String> = ids.into_iter().collect();
    let exported = tok_core::export::export_xml(&doc, None);
    let side = tok_bpmn::extract_sidecar(&original, &exported, &map).map_err(|e| e.to_string())?;
    // the sidecar is plain text: write it and read it back
    let side = tok_bpmn::Sidecar::parse(&side.to_text()).map_err(|e| e.to_string())?;
    let laid = tok_bpmn::layout_with(&exported, &side).map_err(|e| e.to_string())?;
    let got = geometry(&laid);
    let mut problems = Vec::new();
    for (id, nums) in &side.entries {
        match got.get(id) {
            Some(g) if g == nums => {}
            Some(g) => problems.push(format!("{id}: {nums:?} drawn as {g:?}")),
            None => problems.push(format!("{id}: not drawn")),
        }
    }
    for (id, l) in &side.labels {
        let key = format!("{id}/label");
        if got.get(&key).is_some_and(|g| g[..] != l[..]) {
            problems.push(format!("{key}: {l:?} drawn as {:?}", got[&key]));
        }
    }
    // every element of the original diagram that the document keeps is in the sidecar, with
    // the geometry of the file: that of the first diagram as it is, that of every further
    // diagram moved by one whole offset (tok draws one diagram)
    let entries: HashMap<&str, &Vec<i64>> =
        side.entries.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let orig_geometry = geometry(&original);
    let orig = roxmltree::Document::parse(&original).map_err(|e| e.to_string())?;
    let mut offsets: Vec<(String, (i64, i64))> = Vec::new();
    let mut first_plane: Option<String> = None;
    for n in orig.descendants().filter(|n| {
        matches!(n.tag_name().name(), "BPMNShape" | "BPMNEdge")
            && n.attribute("choreographyActivityShape").is_none()
    }) {
        let Some(el) = n.attribute("bpmnElement") else {
            continue;
        };
        let id = map.get(el).map_or(el, String::as_str);
        let plane = n
            .ancestors()
            .find(|a| a.tag_name().name() == "BPMNPlane")
            .and_then(|a| a.attribute("id"))
            .unwrap_or("")
            .to_string();
        let first = first_plane.get_or_insert_with(|| plane.clone()).clone();
        let Some(e) = entries.get(id) else {
            if got.contains_key(id) {
                problems.push(format!("{el}: drawn, but not in the sidecar"));
            }
            continue;
        };
        let Some(o) = orig_geometry.get(el).filter(|o| o.len() == e.len()) else {
            continue;
        };
        let d = (e[0] - o[0], e[1] - o[1]);
        let consistent = e.iter().zip(o.iter()).enumerate().all(|(k, (a, b))| {
            let shift = if n.tag_name().name() == "BPMNShape" && k >= 2 {
                0
            } else if k % 2 == 0 {
                d.0
            } else {
                d.1
            };
            a - b == shift
        });
        let expected = if plane == first {
            Some((0, 0))
        } else {
            offsets.iter().find(|(p, _)| *p == plane).map(|(_, v)| *v)
        };
        match expected {
            Some(x) if x != d || !consistent => {
                problems.push(format!("{el}: {o:?} in the file, {e:?} in the sidecar"));
            }
            None => offsets.push((plane, d)),
            _ => {}
        }
    }
    if problems.is_empty() {
        Ok(side.entries.len())
    } else {
        Err(problems.join("\n"))
    }
}

#[test]
fn course_diagrams_keep_their_geometry() {
    let mut failures = Vec::new();
    let mut kept = 0;
    for f in corpus("import") {
        match round_trip(&f) {
            Ok(n) => kept += n,
            Err(e) => failures.push(format!("{}:\n{e}", f.display())),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    assert!(kept > 300, "only {kept} shapes and edges kept");
}

#[test]
fn miwg_diagrams_keep_their_geometry() {
    let mut failures = Vec::new();
    let mut kept = 0;
    for f in corpus("miwg") {
        match round_trip(&f) {
            Ok(n) => kept += n,
            Err(e) => failures.push(format!("{}:\n{e}", f.display())),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    assert!(kept > 1000, "only {kept} shapes and edges kept");
}

#[test]
fn sidecar_text_round_trip() {
    let text = "tok-layout 1\na 10 20 100 80\nf 110 60 200 60\nf/label 140 40 30 14\n";
    let s = tok_bpmn::Sidecar::parse(text).expect("parse");
    assert_eq!(s.entries.len(), 2);
    assert_eq!(s.to_text(), text);
    assert!(tok_bpmn::Sidecar::parse("a 1 2 3 4\n").is_err());
    assert!(tok_bpmn::Sidecar::parse("tok-layout 1\na 1 2 3\n").is_err());
}

/// A task added to an imported model is placed next to its predecessor; the shapes of
/// the file keep their place.
#[test]
fn new_elements_are_placed_around_kept_ones() {
    let original = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/import/01-wypozyczenie-pierwszy-model.bpmn"),
    )
    .expect("course file");
    let (doc, _, ids) = tok_core::import::import_bpmn_mapped(&original, true).expect("import");
    let map: HashMap<String, String> = ids.into_iter().collect();
    let side =
        tok_bpmn::extract_sidecar(&original, &tok_core::export::export_xml(&doc, None), &map)
            .expect("sidecar");
    let text = tok_core::fmt(&doc);
    let (first, _) = tok_core::parse(&text);
    let end = first
        .nodes
        .values()
        .find(|n| n.base.kind == "end")
        .map(|n| n.base.id().to_string())
        .expect("an end event");
    let last = first
        .flows
        .iter()
        .find(|f| f.target == end)
        .map(|f| f.source.clone())
        .expect("a flow to the end");
    // insert `task nowe` between the last activity and the end event
    let mut edited = String::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with(&format!("end {end}")) {
            let pad = &line[..line.len() - t.len()];
            edited.push_str(&format!("{pad}task nowe \"Nowe zadanie\"\n"));
        }
        if t.starts_with("flow ") && t.contains(&format!("{last} -> {end}")) {
            edited.push_str(&format!("flow {last} -> nowe\nflow nowe -> {end}\n"));
            continue;
        }
        edited.push_str(line);
        edited.push('\n');
    }
    let (doc2, diags) = tok_core::parse(&edited);
    assert!(!diags.iter().any(tok_core::Diag::is_error), "{edited}");
    let laid =
        tok_bpmn::layout_with(&tok_core::export::export_xml(&doc2, None), &side).expect("layout");
    let got = geometry(&laid);
    for (id, nums) in &side.entries {
        if nums.len() == 4 && doc2.nodes.contains_key(id) {
            assert_eq!(got.get(id), Some(nums), "{id} moved");
        }
    }
    let new = &got["nowe"];
    let prev = &got[&last];
    assert!(
        new[0] >= prev[0] + prev[2],
        "new task {new:?} not right of {prev:?}"
    );
    for (id, g) in &got {
        if g.len() == 4 && doc2.nodes.contains_key(id) && id != "nowe" && !id.ends_with("/label") {
            let overlap = new[0] < g[0] + g[2]
                && g[0] < new[0] + new[2]
                && new[1] < g[1] + g[3]
                && g[1] < new[1] + new[3];
            let container = doc2
                .nodes
                .get(id)
                .is_some_and(|n| tok_core::model::is_sub(&n.base.kind));
            assert!(
                !overlap || container,
                "new task {new:?} overlaps {id} {g:?}"
            );
        }
    }
}
