//! Round trip of the BPMN MIWG reference models (`conformance/miwg`) through tok.
//!
//! ```text
//! cargo run -p tok-core --example miwg            # summary and every lost or added fact
//! cargo run -p tok-core --example miwg -- C.3.0   # one model, with its warnings and tok text
//! cargo run -p tok-core --example miwg -- --table # coverage table in Markdown
//! ```
//!
//! See `tests/semantic/mod.rs` for the facts that are compared and `docs/miwg.md` for the
//! published results.

#[path = "../tests/semantic/mod.rs"]
mod semantic;

use semantic::{BY_DESIGN, CATEGORIES, RoundTrip};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let table = args.iter().any(|a| a == "--table");
    let only: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let results: Vec<RoundTrip> = semantic::models()
        .into_iter()
        .filter(|(name, _)| only.is_empty() || only.contains(&name))
        .map(|(name, xml)| semantic::round_trip(&name, &xml))
        .collect();
    if table {
        print_table(&results);
        return;
    }
    for r in &results {
        let (total, kept) = totals(r);
        println!(
            "== {}: {kept}/{total} facts kept, {} lost, {} added, {} by design, {} silent",
            r.name,
            r.losses().count(),
            r.added.len(),
            r.lost.len() - r.losses().count(),
            r.silent.len()
        );
        for e in &r.errors {
            println!("  error: {e}");
        }
        for (f, s) in r.losses() {
            println!("  lost   {:<13} {}   [{s}]", f.cat, f.text);
        }
        for (f, s) in &r.added {
            println!("  added  {:<13} {}   [{s}]", f.cat, f.text);
        }
        for f in &r.silent {
            println!("  silent {:<13} {}", f.cat, f.text);
        }
        if !only.is_empty() {
            for (f, s) in r.lost.iter().filter(|(f, _)| BY_DESIGN.contains(&f.cat)) {
                println!("  design {:<13} {}   [{s}]", f.cat, f.text);
            }
            for w in &r.warnings {
                println!("  warning {w}");
            }
            println!("{}", r.tok);
        }
    }
}

/// Facts of the reference model outside the categories not kept by design: (total, kept).
fn totals(r: &RoundTrip) -> (usize, usize) {
    r.counts
        .iter()
        .filter(|(c, _)| !BY_DESIGN.contains(c))
        .fold((0, 0), |a, (_, (t, k))| (a.0 + t, a.1 + k))
}

fn print_table(results: &[RoundTrip]) {
    let cats: Vec<&str> = CATEGORIES
        .iter()
        .map(|(c, _)| *c)
        .filter(|c| results.iter().any(|r| r.counts.contains_key(c)))
        .collect();
    println!("| Model | {} | Kept |", cats.join(" | "));
    println!("|---|{}---|", "---|".repeat(cats.len()));
    let mut sum = (0, 0);
    for r in results {
        let cells: Vec<String> = cats
            .iter()
            .map(|c| match r.counts.get(c) {
                None => "–".to_string(),
                Some((t, k)) if BY_DESIGN.contains(c) => format!("{t} not kept"),
                Some((t, k)) if t == k => format!("{t}"),
                Some((t, k)) => format!("{k}/{t}"),
            })
            .collect();
        let (t, k) = totals(r);
        sum.0 += t;
        sum.1 += k;
        println!("| {} | {} | {k}/{t} |", r.name, cells.join(" | "));
    }
    println!();
    println!(
        "{} models, {}/{} facts kept ({} lost, {} added)",
        results.len(),
        sum.1,
        sum.0,
        results.iter().map(|r| r.losses().count()).sum::<usize>(),
        results.iter().map(|r| r.added.len()).sum::<usize>()
    );
}
