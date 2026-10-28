//! Timing of the engine on generated models (`tok_bench`): parsing and checking,
//! formatting, BPMN export with layout, SVG, import, decision evaluation and simulation.
//!
//! ```text
//! cargo bench -p tok-bench                 every benchmark
//! cargo bench -p tok-bench -- layout       the benchmarks whose name contains "layout"
//! cargo bench -p tok-bench -- --quick      one run of each (a smoke test of the harness)
//! ```
//!
//! Each benchmark runs until it has taken about a second (at least three times) and prints
//! the median time of one run; `docs/performance.md` keeps the results.

use std::hint::black_box;
use std::time::{Duration, Instant};

use tok_cli::api;
use tok_core::drg::eval::evaluate;
use tok_core::feel3::Map;
use tok_core::feel3::Value;
use tok_core::feel3::eval::{Env, Eval, Types};
use tok_core::feel3::parse::{Names, parse_expression};
use tok_core::{Diag, export, import, printer};

struct Bench {
    filter: Vec<String>,
    quick: bool,
}

impl Bench {
    /// Whether the benchmark `full` (`group/operation/size`) is selected: its name contains
    /// a word of the filter, and a word ending with a size names that size exactly.
    fn selected(&self, full: &str) -> bool {
        self.filter.is_empty()
            || self.filter.iter().any(|w| {
                let sized = w.ends_with(|c: char| c.is_ascii_digit()) && w.contains('/');
                if sized {
                    full.ends_with(w.as_str())
                } else {
                    full.contains(w.as_str())
                }
            })
    }

    /// Whether any of the operations `ops` of `group` at `size` is selected; the models
    /// of the others are not even generated.
    fn wants(&self, group: &str, ops: &[&str], size: usize) -> bool {
        ops.iter()
            .any(|op| self.selected(&format!("{group}/{op}/{size}")))
    }

    fn run<T>(&self, name: &str, size: usize, mut f: impl FnMut() -> T) {
        let full = format!("{name}/{size}");
        if !self.selected(&full) {
            return;
        }
        let mut times = Vec::new();
        let budget = Duration::from_secs(1);
        let start = Instant::now();
        loop {
            let t = Instant::now();
            black_box(f());
            times.push(t.elapsed());
            if self.quick || (times.len() >= 3 && start.elapsed() > budget) || times.len() >= 100 {
                break;
            }
        }
        times.sort();
        let median = times[times.len() / 2];
        println!(
            "{full:<32} {:>12.3} ms   ({} runs)",
            median.as_secs_f64() * 1e3,
            times.len()
        );
    }
}

fn valid(text: &str) -> tok_core::Document {
    let (doc, diags) = tok_core::load(text, "full", None);
    let errors: Vec<&Diag> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
        "a generated model has errors: {errors:?}"
    );
    doc
}

/// Parsing, checking (with the BPMN rules), formatting, export, layout, SVG and import of
/// one model.
const MODEL_OPS: &[&str] = &[
    "parse",
    "check",
    "fmt",
    "export",
    "layout",
    "svg",
    "import",
    "import-sidecar",
];

fn model(b: &Bench, name: &str, size: usize, text: impl FnOnce() -> String) {
    if !b.wants(name, MODEL_OPS, size) {
        return;
    }
    let text = &text();
    let doc = valid(text);
    b.run(&format!("{name}/parse"), size, || tok_core::parse(text));
    b.run(&format!("{name}/check"), size, || api::check(text, "{}"));
    b.run(&format!("{name}/fmt"), size, || printer::fmt(&doc));
    b.run(&format!("{name}/export"), size, || {
        export::export_xml(&doc, None)
    });
    let xml = export::export_xml(&doc, None);
    b.run(&format!("{name}/layout"), size, || tok_bpmn::layout(&xml));
    let laid = tok_bpmn::layout(&xml).unwrap_or_default();
    b.run(&format!("{name}/svg"), size, || {
        tok_bpmn::render_svg(&laid, None)
    });
    b.run(&format!("{name}/import"), size, || {
        import::import_bpmn(&laid, true)
    });
    b.run(&format!("{name}/import-sidecar"), size, || {
        api::import_file(&laid, "{}")
    });
}

fn main() {
    let mut filter = Vec::new();
    let mut quick = false;
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "--quick" => quick = true,
            "--bench" => {}
            s if s.starts_with("--") => {}
            s => filter.push(s.to_string()),
        }
    }
    let b = Bench { filter, quick };

    for n in [100, 1_000, 10_000] {
        model(&b, "process", n, || tok_bench::process(n));
    }
    for (pools, lanes, per_lane) in [(5, 4, 5), (10, 5, 20)] {
        let size = pools * lanes * per_lane;
        model(&b, "collaboration", size, || {
            tok_bench::collaboration(pools, lanes, per_lane)
        });
    }
    for depth in [10, 60] {
        model(&b, "nested", depth, || tok_bench::nested(depth));
    }

    for n in [100, 1_000] {
        if !b.wants("sim", &["process"], n) {
            continue;
        }
        let text = tok_bench::process(n);
        b.run("sim/process", n, || {
            api::sim(&text, r#"{"cases": 100, "seed": 1}"#)
        });
    }

    for rows in [100, 1_000, 10_000] {
        if !b.wants("table", &["check", "eval", "dmn"], rows) {
            continue;
        }
        let text = tok_bench::decision_table(rows);
        let doc = valid(&text);
        b.run("table/check", rows, || tok_core::load(&text, "full", None));
        let mut inputs = Map::new();
        inputs.insert(
            "amount".into(),
            Value::int(i64::try_from(rows * 10 - 5).unwrap_or(0)),
        );
        inputs.insert("kind".into(), Value::str(format!("k{}", (rows - 1) % 7)));
        b.run("table/eval", rows, || evaluate(&doc, "rate", &inputs));
        b.run("table/dmn", rows, || {
            tok_core::drg::export::decision_files(&doc)
        });
    }
    for n in [10, 100, 500] {
        if !b.wants("graph", &["check", "eval", "dmn"], n) {
            continue;
        }
        let text = tok_bench::graph(n);
        let doc = valid(&text);
        let mut inputs = Map::new();
        inputs.insert("x".into(), Value::int(1));
        let last = format!("d{}", n - 1);
        b.run("graph/check", n, || tok_core::load(&text, "full", None));
        b.run("graph/eval", n, || evaluate(&doc, &last, &inputs));
        b.run("graph/dmn", n, || {
            tok_core::drg::export::decision_files(&doc)
        });
    }
    for terms in [100, 1_000, 10_000] {
        if !b.wants("feel", &["parse", "eval", "check"], terms) {
            continue;
        }
        let expr = tok_bench::feel(terms);
        let mut names = Names::builtins();
        names.add("x");
        b.run("feel/parse", terms, || {
            parse_expression(&expr, &names).is_ok()
        });
        let Ok(e) = parse_expression(&expr, &names) else {
            panic!("the generated expression does not parse");
        };
        let types = Types::default();
        let ev = Eval::new(&types);
        let mut vars = Map::new();
        vars.insert(
            "x".into(),
            Value::int(i64::try_from(terms / 2).unwrap_or(0)),
        );
        let env = Env::root(vars);
        b.run("feel/eval", terms, || ev.eval(&e, &env));
        let text = tok_bench::feel_document(terms);
        b.run("feel/check", terms, || tok_core::load(&text, "full", None));
    }
}
