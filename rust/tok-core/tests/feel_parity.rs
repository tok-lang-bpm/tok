//! FEEL evaluation parity with the Python reference implementation.
//!
//! `tests/fixtures/eval/feel.json` is produced by `gen_python_eval.py`: expressions
//! (arithmetic, comparisons, built-in functions, dates and durations, null handling,
//! special numbers, errors) evaluated by `feel.eval_text` against one context, and unary
//! tests applied by `feel.test`. Each result is compared exactly: the value with numbers as
//! `str(Decimal)` (so exponents count), the plain value of `to_python` as JSON, or the
//! exception as the last line of a Python traceback.

use std::fs;
use std::path::Path;

use serde_json::{Value as J, json};
use tok_core::feel;
use tok_core::feel_eval::{self as fe, Context, Value};
use tok_core::pydec::Decimal;
use tok_core::pytime::{Date, Duration};
use tok_core::pyutil::PyError;

fn fixture() -> J {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("eval")
        .join("feel.json");
    let text = fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}; run gen_python_eval.py", p.display()));
    serde_json::from_str(&text).expect("feel.json is JSON")
}

fn decode(v: &J) -> Value {
    match v {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(*b),
        J::String(s) => Value::Str(s.clone()),
        J::Array(items) => Value::List(items.iter().map(decode).collect()),
        J::Object(m) => {
            if let Some(J::String(n)) = m.get("n") {
                Value::Num(Decimal::parse(n).expect("decimal in fixture"))
            } else if let Some(J::String(d)) = m.get("d") {
                let part = |a: usize, b: usize| d[a..b].parse::<i128>().expect("date digits");
                Value::Date(Date::new(part(0, 4), part(5, 7), part(8, 10)).expect("date"))
            } else if let Some(J::Array(t)) = m.get("td") {
                let n = |k: usize| t[k].as_i64().expect("timedelta field") as i128;
                Value::Duration(
                    Duration::from_micros(n(0) * 86_400_000_000 + n(1) * 1_000_000 + n(2))
                        .expect("timedelta"),
                )
            } else if let Some(J::Array(pairs)) = m.get("c") {
                let mut c = Context::new();
                for p in pairs {
                    c.insert(p[0].as_str().expect("key").to_string(), decode(&p[1]));
                }
                Value::Context(c)
            } else {
                panic!("unknown value in fixture: {v}")
            }
        }
        J::Number(_) => panic!("plain number in fixture: {v}"),
    }
}

fn encode(v: &Value) -> J {
    match v {
        Value::Null => J::Null,
        Value::Bool(b) => J::Bool(*b),
        Value::Str(s) => J::String(s.clone()),
        Value::Num(d) => json!({ "n": d.to_sci_string() }),
        Value::Date(d) => json!({ "d": d.isoformat() }),
        Value::DateTime(d) => json!({ "dt": d.isoformat() }),
        Value::Duration(d) => json!({ "td": [d.days, d.seconds, d.microseconds] }),
        Value::List(items) => J::Array(items.iter().map(encode).collect()),
        Value::Context(m) => json!({
            "c": m.iter().map(|(k, x)| json!([k, encode(x)])).collect::<Vec<_>>()
        }),
    }
}

fn text(e: &PyError) -> J {
    J::String(e.to_string())
}

#[test]
fn feel_matches_python() {
    let fx = fixture();
    let mut ctx = Context::new();
    for pair in fx["context"].as_array().expect("context") {
        ctx.insert(
            pair[0].as_str().expect("name").to_string(),
            decode(&pair[1]),
        );
    }
    let mut failures = Vec::new();
    let exprs = fx["expressions"].as_array().expect("expressions");
    for item in exprs {
        let expr = item["expr"].as_str().expect("expr");
        let (got, expected) = match fe::eval_text(expr, &ctx) {
            Ok(v) => {
                let plain = match fe::to_python(&v).and_then(|p| p.to_json()) {
                    Ok(p) => json!({ "plain": p.compact() }),
                    Err(e) => json!({ "plain_error": text(&e) }),
                };
                let mut got = json!({ "value": encode(&v) });
                got.as_object_mut()
                    .expect("object")
                    .extend(plain.as_object().expect("object").clone());
                let mut expected = item.clone();
                expected.as_object_mut().expect("object").remove("expr");
                (got, expected)
            }
            Err(e) => (json!({ "error": text(&e) }), {
                let mut x = item.clone();
                x.as_object_mut().expect("object").remove("expr");
                x
            }),
        };
        if got != expected {
            failures.push(format!("{expr}\n  python: {expected}\n  rust:   {got}"));
        }
    }
    let tests = fx["unary"].as_array().expect("unary tests");
    let unary_ctx = ctx.clone();
    for item in tests {
        let cell = item["cell"].as_str().expect("cell");
        let input = decode(&item["input"]);
        let got = match feel::parse_unary_tests(cell)
            .map_err(fe::feel_error)
            .and_then(|u| fe::test(&u, &input, &unary_ctx))
        {
            Ok(r) => json!({ "result": r }),
            Err(e) => json!({ "error": text(&e) }),
        };
        let mut expected = item.clone();
        let obj = expected.as_object_mut().expect("object");
        obj.remove("cell");
        obj.remove("input");
        if got != expected {
            failures.push(format!(
                "unary {cell} with {}\n  python: {expected}\n  rust:   {got}",
                item["input"]
            ));
        }
    }
    assert!(
        exprs.len() > 10_000 && tests.len() > 50,
        "fixtures missing; run gen_python_eval.py"
    );
    assert!(
        failures.is_empty(),
        "{} of {} FEEL results differ from Python:\n{}",
        failures.len(),
        exprs.len() + tests.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
