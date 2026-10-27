//! Bit-exact parity of `PyRandom` with CPython's `random.Random`.
//!
//! The fixtures in `tests/fixtures` are produced by `gen_python_expected.py`:
//! one file per platform, each a sequence of seedings and calls with the result
//! Python returned. The test replays every line on one generator and compares
//! the result bit for bit and as Python's `repr`. Results that pass through the
//! C library's `log`, `exp`, `sin` and `cos` are compared only against the
//! fixture of the platform the test runs on; the other platforms' fixtures are
//! still replayed and checked on the methods that do not use them.

use std::fs;
use std::path::{Path, PathBuf};

use tok_core::pyutil::float_repr;
use tok_sim::PyRandom;
use tok_sim::pyrandom::{NV_MAGICCONST, TWOPI};

const PLATFORM: &str = if cfg!(windows) {
    "windows"
} else if cfg!(target_os = "macos") {
    "macos"
} else {
    "linux"
};

/// Methods whose results depend on the platform's `log`, `exp`, `sin`, `cos`.
const LIBM: &[&str] = &["gauss", "normalvariate", "lognormvariate", "expovariate"];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}; run tests/gen_python_expected.py on this platform",
            p.display()
        )
    })
}

/// Absolute value of a decimal integer as little-endian 32-bit words.
fn decimal_magnitude(text: &str) -> Vec<u32> {
    let mut words: Vec<u32> = vec![0];
    for d in text.trim_start_matches('-').bytes() {
        assert!(d.is_ascii_digit(), "bad integer {text}");
        let mut carry = u64::from(d - b'0');
        for w in words.iter_mut() {
            let v = u64::from(*w) * 10 + carry;
            *w = v as u32;
            carry = v >> 32;
        }
        if carry > 0 {
            words.push(carry as u32);
        }
    }
    words
}

fn num(text: &str) -> f64 {
    text.parse()
        .unwrap_or_else(|e| panic!("bad number {text}: {e}"))
}

fn check_float(line_no: usize, line: &str, got: f64, expected: &str) {
    assert_eq!(
        got.to_bits(),
        num(expected).to_bits(),
        "line {line_no}: {line}: got {got:e}"
    );
    assert_eq!(float_repr(got), expected, "line {line_no}: {line}");
}

/// Replays one fixture and returns the number of compared values. With
/// `libm == false` the results of the methods in `LIBM` are not compared.
fn replay(text: &str, libm: bool) -> usize {
    let mut rng: Option<PyRandom> = None;
    let mut compared = 0;
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        let t: Vec<&str> = line.split_whitespace().collect();
        let Some((&op, rest)) = t.split_first() else {
            continue;
        };
        match op {
            "python" => continue,
            "const" => {
                let v = match rest[0] {
                    "NV_MAGICCONST" => NV_MAGICCONST,
                    "TWOPI" => TWOPI,
                    other => panic!("line {line_no}: unknown constant {other}"),
                };
                check_float(line_no, line, v, rest[1]);
                compared += 1;
                continue;
            }
            "seed" => {
                let words = decimal_magnitude(rest[0]);
                match rng.as_mut() {
                    Some(r) => r.seed_magnitude(&words),
                    None => rng = Some(PyRandom::from_magnitude(&words)),
                }
                continue;
            }
            "seedf" => {
                rng = Some(PyRandom::from_float_seed(num(rest[0])).expect("finite seed"));
                continue;
            }
            _ => {}
        }
        let r = rng
            .as_mut()
            .unwrap_or_else(|| panic!("line {line_no}: call before seed"));
        let (expected, args) = rest.split_last().expect("result");
        if op == "u32" {
            assert_eq!(r.next_u32().to_string(), *expected, "line {line_no}");
            compared += 1;
            continue;
        }
        let a: Vec<f64> = args
            .iter()
            .map(|s| if *s == "None" { f64::NAN } else { num(s) })
            .collect();
        let got = match op {
            "random" => r.random(),
            "uniform" => r.uniform(a[0], a[1]),
            "triangular" => {
                let mode = if args[2] == "None" { None } else { Some(a[2]) };
                r.triangular(a[0], a[1], mode)
            }
            "gauss" => r.gauss(a[0], a[1]),
            "normalvariate" => r.normalvariate(a[0], a[1]),
            "lognormvariate" => r.lognormvariate(a[0], a[1]),
            "expovariate" => r.expovariate(a[0]),
            other => panic!("line {line_no}: unknown method {other}"),
        };
        if libm || !LIBM.contains(&op) {
            check_float(line_no, line, got, expected);
            compared += 1;
        }
    }
    compared
}

#[test]
fn same_results_as_python_on_this_platform() {
    let path = fixtures().join(format!("pyrandom-{PLATFORM}.txt"));
    let compared = replay(&read(&path), true);
    assert!(compared > 36_000, "only {compared} values compared");
}

#[test]
fn platform_independent_results_of_every_fixture() {
    let mut files: Vec<PathBuf> = fs::read_dir(fixtures())
        .expect("fixtures directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".txt"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixtures");
    for f in files {
        let compared = replay(&read(&f), false);
        assert!(compared > 20_000, "{}: only {compared} values", f.display());
    }
}
