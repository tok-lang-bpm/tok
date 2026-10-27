//! Simulation parameter checks (S-codes): the validation half of `sim.py`
//! (`parse_dist`, `parse_assign`, `check_sim`). The simulator itself comes later.

use crate::model::Document;
use crate::pyutil::{fmt_g, parse_float, round6_repr, strip};

/// (code, line, element id, arguments)
pub type SimCheck = (
    &'static str,
    usize,
    Option<String>,
    Vec<(&'static str, String)>,
);

fn is_dur_unit(c: char) -> bool {
    matches!(c, 's' | 'm' | 'h' | 'd')
}

/// `\d+(?:\.\d+)?`
fn is_plain_number(t: &str) -> bool {
    let (int, frac) = match t.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (t, None),
    };
    !int.is_empty()
        && int.chars().all(|c| c.is_ascii_digit())
        && frac
            .map(|f| !f.is_empty() && f.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(true)
}

/// ('duration' | 'number', value) or an error detail.
fn scalar(text: &str) -> Result<(&'static str, f64), String> {
    let t = strip(text);
    if let Some(unit) = t.chars().last()
        && is_dur_unit(unit)
    {
        let num = strip(&t[..t.len() - 1]);
        let num_raw = &t[..t.len() - 1];
        // `^(\d+(?:\.\d+)?)\s*([smhd])$`: whitespace only between number and unit
        if is_plain_number(num)
            && num_raw.trim_end_matches(crate::pyutil::is_space) == num
            && !num.is_empty()
        {
            let mult = match unit {
                's' => 1.0,
                'm' => 60.0,
                'h' => 3600.0,
                _ => 86400.0,
            };
            return Ok(("duration", num.parse::<f64>().unwrap_or(0.0) * mult));
        }
    }
    let body = t.strip_prefix('-').unwrap_or(t);
    if is_plain_number(body) {
        return Ok(("number", t.parse::<f64>().unwrap_or(0.0)));
    }
    Err(format!("'{t}' is neither a number nor a duration like 30m"))
}

fn split_args(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i64;
    let mut buf = String::new();
    for c in text.chars() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
        }
        if c == ',' && depth == 0 {
            out.push(strip(&buf).to_string());
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    let last = strip(&buf).to_string();
    if !last.is_empty() || !out.is_empty() {
        out.push(last);
    }
    out
}

/// `^([a-z]+)\s*\((.*)\)$` (dot matches newlines).
fn call(t: &str) -> Option<(&str, &str)> {
    let name_end = t.find(|c: char| !c.is_ascii_lowercase()).unwrap_or(t.len());
    if name_end == 0 {
        return None;
    }
    let rest = t[name_end..].trim_start_matches(crate::pyutil::is_space);
    let body = rest.strip_prefix('(')?.strip_suffix(')')?;
    Some((&t[..name_end], body))
}

fn arity(name: &str) -> Option<usize> {
    match name {
        "fixed" | "exp" | "bern" => Some(1),
        "unif" | "norm" | "lnorm" => Some(2),
        "tri" => Some(3),
        _ => None,
    }
}

/// Validate a distribution text (`sim.parse_dist`); the error is the detail text.
pub fn parse_dist(text: &str) -> Result<(), String> {
    if strip(text).is_empty() {
        return Err("empty distribution".into());
    }
    let t = strip(text);
    let Some((name, body)) = call(t) else {
        scalar(t)?;
        return Ok(());
    };
    let args = split_args(body);
    if name == "cat" {
        if args.is_empty() {
            return Err("cat() needs at least one label=probability".into());
        }
        let mut total = 0.0;
        for a in &args {
            let Some((label, p)) = a.rsplit_once('=') else {
                return Err(format!(
                    "cat() arguments must be label=probability, got '{a}'"
                ));
            };
            let (kind, pv) = scalar(p)?;
            if kind != "number" || !(0.0..=1.0).contains(&pv) {
                return Err(format!(
                    "probability of '{}' must be a number between 0 and 1",
                    strip(label)
                ));
            }
            total += pv;
        }
        if (total - 1.0).abs() > 1e-6 {
            return Err(format!(
                "cat() probabilities sum to {}, not 1",
                fmt_g(total)
            ));
        }
        return Ok(());
    }
    let Some(n) = arity(name) else {
        return Err(format!("unknown distribution '{name}'"));
    };
    if args.len() != n {
        return Err(format!(
            "{name}() takes {n} argument(s), got {}",
            args.len()
        ));
    }
    let mut parsed = Vec::new();
    for a in &args {
        parsed.push(scalar(a)?);
    }
    let kind = parsed[0].0;
    if parsed.iter().any(|p| p.0 != kind) {
        return Err(format!("mix of durations and numbers in {name}()"));
    }
    let v: Vec<f64> = parsed.iter().map(|p| p.1).collect();
    if name == "bern" {
        if kind != "number" || !(0.0..=1.0).contains(&v[0]) {
            return Err("bern(p) needs a probability between 0 and 1".into());
        }
        return Ok(());
    }
    if name == "unif" && v[0] > v[1] {
        return Err("unif(a, b) needs a <= b".into());
    }
    if name == "tri" && !(v[0] <= v[1] && v[1] <= v[2]) {
        return Err("tri(a, mode, b) needs a <= mode <= b".into());
    }
    if (name == "norm" || name == "lnorm") && v[1] < 0.0 {
        return Err("standard deviation must not be negative".into());
    }
    if (name == "lnorm" || name == "exp") && v[0] <= 0.0 {
        return Err(format!("{name}() needs a positive mean"));
    }
    Ok(())
}

fn is_var_name(s: &str) -> bool {
    let mut ch = s.chars();
    matches!(ch.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && ch.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate `assign="a=bern(0.8); b=cat(x=0.5, y=0.5)"`.
pub fn parse_assign(text: &str) -> Result<(), String> {
    for part in text.split(';') {
        let part = strip(part);
        if part.is_empty() {
            continue;
        }
        let Some((name, dist)) = part.split_once('=') else {
            return Err(format!("assign needs NAME=DIST, got '{part}'"));
        };
        let name = strip(name);
        if !is_var_name(name) {
            return Err(format!("'{name}' is not a variable name"));
        }
        parse_dist(dist)?;
    }
    Ok(())
}

fn dist_check(out: &mut Vec<SimCheck>, text: &str, line: usize, id: Option<&str>, assign: bool) {
    let res = if assign {
        parse_assign(text)
    } else {
        parse_dist(text)
    };
    if let Err(detail) = res {
        out.push((
            "S001",
            line,
            id.map(str::to_string),
            vec![("text", text.to_string()), ("detail", detail)],
        ));
    }
}

fn prob(text: &str) -> Option<f64> {
    parse_float(text)
}

/// Simulation diagnostics (`sim.check_sim`).
pub fn check_sim(doc: &Document) -> Vec<SimCheck> {
    let mut out = Vec::new();
    if let Some(s) = &doc.simulate
        && let Some(a) = s.attrs.get("arrival")
    {
        dist_check(&mut out, a, s.line, None, false);
    }
    for v in &doc.variables {
        dist_check(
            &mut out,
            v.label.as_deref().unwrap_or(""),
            v.line,
            v.id.as_deref(),
            false,
        );
    }
    for n in doc.nodes.values() {
        for key in ["duration", "delay"] {
            if let Some(t) = n.base.attrs.get(key) {
                dist_check(&mut out, t, n.base.line, n.base.id.as_deref(), false);
            }
        }
        if let Some(t) = n.base.attrs.get("assign") {
            dist_check(&mut out, t, n.base.line, n.base.id.as_deref(), true);
        }
        if let Some(by) = n.base.attrs.get("by")
            && !doc.resources.contains_key(by)
        {
            out.push((
                "R002",
                n.base.line,
                n.base.id.clone(),
                vec![("what", "resource".into()), ("id", by.clone())],
            ));
        }
    }
    for m in &doc.messages {
        if let Some(t) = m.base.attrs.get("delay") {
            dist_check(&mut out, t, m.base.line, m.base.id.as_deref(), false);
        }
    }
    let mut outgoing: indexmap::IndexMap<&str, Vec<&crate::model::Flow>> = Default::default();
    for f in &doc.flows {
        outgoing.entry(f.source.as_str()).or_default().push(f);
        if let Some(p) = f.base.attrs.get("p") {
            let ok = prob(p).map(|x| (0.0..=1.0).contains(&x)).unwrap_or(false);
            if !ok {
                out.push((
                    "S001",
                    f.base.line,
                    f.base.id.clone(),
                    vec![
                        ("text", format!("p={p}")),
                        ("detail", "a probability must be between 0 and 1".into()),
                    ],
                ));
            }
        }
    }
    for n in doc.nodes.values() {
        let kind = n.base.kind.as_str();
        if kind != "xor" && kind != "or" {
            continue;
        }
        let flows = outgoing.get(n.base.id()).cloned().unwrap_or_default();
        let ps: Vec<_> = flows
            .iter()
            .filter(|f| f.base.attrs.contains_key("p"))
            .collect();
        if kind == "xor" && !ps.is_empty() {
            let mut total = Some(0.0);
            for f in &ps {
                match prob(&f.base.attrs["p"]) {
                    Some(x) => total = total.map(|t| t + x),
                    None => {
                        total = None;
                        break;
                    }
                }
            }
            if let Some(t) = total
                && (t - 1.0).abs() > 1e-6
            {
                out.push((
                    "S002",
                    n.base.line,
                    n.base.id.clone(),
                    vec![("id", n.base.id().to_string()), ("total", round6_repr(t))],
                ));
            }
        }
        if doc.simulate.is_some()
            && flows.len() >= 2
            && ps.is_empty()
            && !flows.iter().any(|f| f.base.attrs.contains_key("when"))
        {
            out.push((
                "S003",
                n.base.line,
                n.base.id.clone(),
                vec![("id", n.base.id().to_string())],
            ));
        }
    }
    out
}
