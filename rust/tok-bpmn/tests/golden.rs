//! Golden files of the automatic layout and of the drawing.
//!
//! For a representative set of fixtures (all `semantic` and `extra` models, the Camunda model
//! and four course diagrams, which are laid out again from scratch) the laid-out XML and the
//! SVG must equal the files in `tests/golden/`. The layout must also be deterministic: two
//! runs give the same bytes.
//!
//! Regenerate after a deliberate change of the layout or the drawing:
//!
//! ```text
//! TOK_BLESS=1 cargo test -p tok-bpmn --test golden
//! ```

use std::fs;
use std::path::{Path, PathBuf};

const COURSE: &[&str] = &[
    "04-wypozyczenie-wspolpraca",
    "05-oczekiwanie-na-odpowiedz",
    "06-odbior-ksiazki-wyjatki",
    "08-udostepnienie-archiwaliow",
];

fn models() -> Vec<(String, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out = Vec::new();
    for group in ["semantic", "extra", "camunda"] {
        let mut files: Vec<PathBuf> = fs::read_dir(root.join("fixtures").join(group))
            .expect("fixtures")
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "bpmn"))
            .collect();
        files.sort();
        for f in files {
            out.push((group.to_string(), f));
        }
    }
    for stem in COURSE {
        out.push((
            "import".to_string(),
            root.join("fixtures")
                .join("import")
                .join(format!("{stem}.bpmn")),
        ));
    }
    out
}

fn first_diff(a: &str, b: &str) -> String {
    for (n, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  now:    {x}\n  golden: {y}", n + 1);
        }
    }
    format!(
        "lengths differ: now {} lines, golden {} lines",
        a.lines().count(),
        b.lines().count()
    )
}

#[test]
fn layout_and_drawing_match_golden_files() {
    let bless = std::env::var_os("TOK_BLESS").is_some();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden");
    let mut failures = Vec::new();
    let mut checked = 0;
    for (group, path) in models() {
        let xml = String::from_utf8_lossy(&fs::read(&path).expect("read fixture")).to_string();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let laid = tok_bpmn::layout(&xml).expect("layout");
        assert_eq!(
            laid,
            tok_bpmn::layout(&xml).expect("layout"),
            "{group}/{stem}: layout is not deterministic"
        );
        // drawn from the new layout (a fixture with its own diagram would be drawn as it is)
        let svg = match tok_bpmn::render_svg(&laid, Some("model.bpmn")) {
            Ok(s) => s,
            Err(e) => format!("error: {e}\n"),
        };
        let out_dir = dir.join(&group);
        let outputs = [(format!("{stem}.bpmn"), laid), (format!("{stem}.svg"), svg)];
        for (name, text) in outputs {
            let file = out_dir.join(&name);
            if bless {
                fs::create_dir_all(&out_dir).expect("golden directory");
                fs::write(&file, &text).expect("write golden file");
                continue;
            }
            match fs::read_to_string(&file) {
                Ok(want) if want.replace("\r\n", "\n") == text => {}
                Ok(want) => failures.push(format!(
                    "{group}/{name}: {}",
                    first_diff(&text, &want.replace("\r\n", "\n"))
                )),
                Err(_) => failures.push(format!("{group}/{name}: golden file missing")),
            }
        }
        checked += 1;
    }
    assert!(checked >= 20, "only {checked} models");
    assert!(
        failures.is_empty(),
        "{} difference(s) from the golden files (after a deliberate change: TOK_BLESS=1 cargo \
         test -p tok-bpmn --test golden):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
