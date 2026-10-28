//! Every input that a fuzz target once failed on, run through the property of that
//! target (`rust/fuzz/src/oracles.rs`, shared with the fuzz targets). The inputs are in
//! `rust/fuzz/regressions/TARGET/`; see `docs/fuzzing.md`.

#[allow(dead_code)]
#[path = "../../fuzz/src/oracles.rs"]
mod oracles;

use std::path::Path;

/// Stack of the thread that runs the inputs: the default main thread of Windows (1 MiB),
/// the smallest stack the engine is called on (Python, R and the C interface call it on
/// the threads of their callers). Unoptimized builds have frames about twice as large.
const STACK: usize = if cfg!(debug_assertions) { 2 } else { 1 } * 1024 * 1024;

fn run(target: &str, property: fn(&[u8])) -> usize {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fuzz/regressions")
        .join(target);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    let mut files: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    files.sort();
    for path in &files {
        let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = format!(
            "{target}/{}",
            path.file_name().unwrap_or_default().display()
        );
        // a stack overflow aborts the process; its message names the thread
        let outcome = std::thread::Builder::new()
            .name(name.clone())
            .stack_size(STACK)
            .spawn(move || property(&data))
            .unwrap_or_else(|e| panic!("cannot start a thread: {e}"))
            .join();
        assert!(outcome.is_ok(), "{name} fails");
    }
    files.len()
}

#[test]
fn regressions() {
    let mut total = 0;
    for (target, property) in oracles::TARGETS {
        total += run(target, property);
    }
    assert!(total > 0, "no regression inputs found");
}
