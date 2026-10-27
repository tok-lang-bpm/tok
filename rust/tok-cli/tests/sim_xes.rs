//! `tok sim --xes PATH`: the event log as XES, an option the Python command does not have.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn xes_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tok-sim")
        .join("tests")
        .join("fixtures")
        .join("xes")
}

fn tok(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tok"))
        .args(args)
        .current_dir(xes_fixtures())
        .env_remove("TOK_LANG")
        .output()
        .expect("run tok")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tok-cli-xes-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temporary folder");
    dir
}

#[test]
fn writes_the_xes_file_next_to_the_other_outputs() {
    let dir = temp_dir("write");
    let xes = dir.join("log.xes");
    let csv = dir.join("log.csv");
    let (xes_s, csv_s) = (xes.to_string_lossy(), csv.to_string_lossy());
    let with = tok(&["sim", "repair-desk.tok", "--log", &csv_s, "--xes", &xes_s]);
    let without = tok(&["sim", "repair-desk.tok"]);
    assert_eq!(with.status.code(), Some(0), "{}", text(&with.stderr));
    assert_eq!(text(&with.stdout), text(&without.stdout));
    let written = std::fs::read_to_string(&xes).expect("XES file");
    let golden = std::fs::read_to_string(xes_fixtures().join("repair-desk.xes"))
        .expect("golden file")
        .replace("\r\n", "\n");
    assert_eq!(written, golden);
    assert!(!written.contains('\r'), "LF line endings on every platform");
    assert!(csv.exists());
    // an abbreviation, as argparse accepts it
    let short = dir.join("short.xes");
    let out = tok(&[
        "sim",
        "repair-desk.tok",
        "--x",
        &short.to_string_lossy(),
        "--json",
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(std::fs::read_to_string(&short).expect("XES file"), golden);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_path_writes_nothing() {
    let out = tok(&["sim", "repair-desk.tok", "--xes", ""]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).starts_with("Simulation (seed 3)"));
}

#[test]
fn errors() {
    let out = tok(&["sim", "repair-desk.tok", "--xes"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).ends_with("tok sim: error: argument --xes: expected one argument\n"),
        "{}",
        text(&out.stderr)
    );
    let dir = temp_dir("missing");
    let missing = dir.join("no-such-folder").join("log.xes");
    let missing_s = missing.to_string_lossy().to_string();
    let _ = std::fs::remove_dir_all(&dir);
    let out = tok(&["sim", "repair-desk.tok", "--xes", &missing_s]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).starts_with("FileNotFoundError: [Errno 2] No such file or directory"),
        "{}",
        text(&out.stderr)
    );
    assert!(text(&out.stdout).is_empty());
}

#[test]
fn help_lists_the_option() {
    let out = tok(&["sim", "-h"]);
    let help = text(&out.stdout);
    assert!(help.starts_with(
        "usage: tok sim [-h] [--cases CASES] [--seed SEED] [--log LOG] \
         [--cases-csv CASES_CSV] [--xes XES] [--lang {en,pl}] [--json] file\n"
    ));
    assert!(help.contains(
        "  --cases-csv CASES_CSV\n                        write one row per case (CSV)\n  \
         --xes XES             write the event log (XES, IEEE 1849-2016)\n  --lang {en,pl}"
    ));
}
