//! Writes a generated model to standard output.
//!
//! ```text
//! cargo run --release -p tok-bench --example generate -- process 1000 > big.tok
//! cargo run --release -p tok-bench --example generate -- collaboration 10 5 20
//! cargo run --release -p tok-bench --example generate -- nested 30
//! cargo run --release -p tok-bench --example generate -- table 1000
//! cargo run --release -p tok-bench --example generate -- graph 200
//! cargo run --release -p tok-bench --example generate -- feel 1000
//! ```

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n = |k: usize, default: usize| -> usize {
        args.get(k).and_then(|a| a.parse().ok()).unwrap_or(default)
    };
    let text = match args.first().map(String::as_str) {
        Some("process") => tok_bench::process(n(1, 1000)),
        Some("collaboration") => tok_bench::collaboration(n(1, 10), n(2, 5), n(3, 20)),
        Some("nested") => tok_bench::nested(n(1, 30)),
        Some("table") => tok_bench::decision_table(n(1, 1000)),
        Some("graph") => tok_bench::graph(n(1, 100)),
        Some("feel") => tok_bench::feel_document(n(1, 1000)),
        _ => {
            eprintln!(
                "usage: generate process N | collaboration POOLS LANES TASKS | nested DEPTH | table ROWS | graph N | feel TERMS"
            );
            std::process::exit(2);
        }
    };
    let _ = std::io::stdout().write_all(text.as_bytes());
}
