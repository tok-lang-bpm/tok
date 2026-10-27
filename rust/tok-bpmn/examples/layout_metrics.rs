//! Quality metrics and timing of the automatic layout over the corpus.
//!
//! ```text
//! cargo run --release -p tok-bpmn --example layout_metrics [BASELINE.tsv]
//! ```
//!
//! Prints one line of metrics per model (see `tests/quality/mod.rs` for their meaning), the
//! totals and the time spent in `layout` and `render_svg`. With a baseline table (for example
//! `tests/layout/baseline-0.3.tsv`), every cell is printed as `now (before)` where it differs,
//! with the totals of both. With `TOK_LAYOUT_DUMP=DIR` the laid-out XML and the SVG of every
//! model are written to `DIR`.

#[path = "../tests/quality/mod.rs"]
mod quality;

use std::time::Instant;

use quality::{Metrics, NAMES};

fn main() {
    let baseline = std::env::args()
        .nth(1)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| quality::read_table(&t));
    let corpus = quality::corpus();
    // timing: every model laid out and drawn several times
    let reps = 20;
    let t0 = Instant::now();
    for _ in 0..reps {
        for c in &corpus {
            let laid = tok_bpmn::layout(&c.xml).expect("layout");
            let _ = tok_bpmn::render_svg(&laid, None);
        }
    }
    let per_model = t0.elapsed().as_secs_f64() * 1e6 / f64::from(reps) / corpus.len() as f64;

    let dump = std::env::var_os("TOK_LAYOUT_DUMP").map(std::path::PathBuf::from);
    let mut rows: Vec<(String, Metrics)> = Vec::new();
    for c in &corpus {
        let (laid, svg) = quality::run(&c.xml);
        if let Some(dir) = &dump {
            let file = c.name.replace('/', "--");
            std::fs::create_dir_all(dir).expect("dump directory");
            std::fs::write(dir.join(format!("{file}.bpmn")), &laid).expect("write");
            std::fs::write(dir.join(format!("{file}.svg")), &svg).expect("write");
        }
        rows.push((c.name.clone(), quality::measure(&laid, &svg)));
    }
    let mut total: Metrics = [0; 10];
    let mut total_before: Metrics = [0; 10];
    println!("model\t{}", NAMES.join("\t"));
    for (name, m) in &rows {
        let before = baseline.as_ref().and_then(|b| b.get(name));
        let cells: Vec<String> = m
            .iter()
            .enumerate()
            .map(|(k, v)| match before {
                Some(b) if b[k] != *v => format!("{v} ({})", b[k]),
                _ => v.to_string(),
            })
            .collect();
        println!("{name}\t{}", cells.join("\t"));
        for k in 0..m.len() {
            total[k] += m[k];
            total_before[k] += before.map_or(m[k], |b| b[k]);
        }
    }
    let cells: Vec<String> = total
        .iter()
        .zip(total_before.iter())
        .map(|(v, b)| {
            if baseline.is_some() && v != b {
                format!("{v} ({b})")
            } else {
                v.to_string()
            }
        })
        .collect();
    println!("TOTAL ({} models)\t{}", rows.len(), cells.join("\t"));
    println!("layout + svg: {per_model:.0} µs per model on average");
}
