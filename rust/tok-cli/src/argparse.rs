//! The argument parsing of a `tok` subcommand as Python's `argparse` (3.11) does it:
//! option prefixes and their ambiguity, `--opt=value`, grouped single-dash flags (`-hX`),
//! negative numbers as values, options between positional arguments, `--`, type and
//! choice checks, and the same error messages.

use std::process::ExitCode;

use crate::{EXIT_OK, EXIT_USAGE, usage};

/// What an option does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opt {
    /// `-h`, `--help`: print the help text and exit.
    Help,
    /// `store_true` under this destination.
    Flag(&'static str),
    /// One argument stored under this destination, converted by [`Conv`].
    Value(&'static str, Conv),
}

/// The `type=` or `choices=` of an option with an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    Text,
    /// `type=int`; the value is kept as normalized decimal digits.
    Int,
    Choice(&'static [&'static str]),
}

/// A subcommand parser: its name, usage line, help text, options in declaration order
/// and positionals (name, takes any number of arguments).
pub struct Spec {
    pub name: &'static str,
    pub usage: &'static str,
    pub help: &'static str,
    pub options: &'static [(&'static str, Opt)],
    pub positionals: &'static [(&'static str, bool)],
}

/// Parsed arguments: option values by destination (in the order given), flags and
/// positionals.
#[derive(Debug, Default)]
pub struct Parsed {
    pub values: Vec<(&'static str, String)>,
    pub flags: Vec<&'static str>,
    pub positionals: Vec<(&'static str, Vec<String>)>,
}

impl Parsed {
    pub fn value(&self, dest: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(d, _)| *d == dest)
            .map(|(_, v)| v.as_str())
    }

    pub fn flag(&self, dest: &str) -> bool {
        self.flags.contains(&dest)
    }

    pub fn positional(&self, name: &str) -> Vec<String> {
        self.positionals
            .iter()
            .find(|t| t.0 == name)
            .map(|t| t.1.clone())
            .unwrap_or_default()
    }
}

/// An argument that looks like an option: (action, option string, separator before the
/// explicit argument: `=`, empty for `-hX`, or none; explicit argument).
type OptTuple = (Option<Opt>, String, Option<&'static str>, Option<String>);

/// argparse `_negative_number_matcher`: `^-\d+$|^-\d*\.\d+$`.
fn negative_number(s: &str) -> bool {
    let Some(body) = s.strip_prefix('-') else {
        return false;
    };
    let digit = |c: char| tok_core::pyutil::decimal_value(c).is_some();
    if !body.is_empty() && body.chars().all(digit) {
        return true;
    }
    match body.split_once('.') {
        Some((a, b)) => a.chars().all(digit) && !b.is_empty() && b.chars().all(digit),
        None => false,
    }
}

/// argparse's `int(text)` for `type=int`: normalized digits, or `None` for an invalid
/// literal (also for more than 4300 digits, Python's limit of integer conversion).
pub fn int_digits(text: &str) -> Option<String> {
    tok_core::pyutil::parse_int(text).ok()?;
    let t = tok_core::pyutil::strip(text);
    let (neg, body) = match t.chars().next() {
        Some('-') => (true, &t[1..]),
        Some('+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits: String = body
        .chars()
        .filter_map(tok_core::pyutil::decimal_value)
        .map(|d| char::from(b'0' + d as u8))
        .collect();
    if digits.len() > 4300 {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    Some(if neg && digits != "0" {
        format!("-{digits}")
    } else {
        digits.to_string()
    })
}

/// Greedy match of the remaining positionals (`false`: exactly one argument, `true`: any
/// number) against the argument pattern: the argument counts of the longest prefix of the
/// positionals that matches (argparse `_match_arguments_partial`).
fn match_positionals(kinds: &[bool], pattern: &[char]) -> Vec<usize> {
    for n in (1..=kinds.len()).rev() {
        let mut counts = Vec::new();
        let mut i = 0;
        let mut ok = true;
        for many in &kinds[..n] {
            let start = i;
            if *many {
                while i < pattern.len() && (pattern[i] == 'A' || pattern[i] == '-') {
                    i += 1;
                }
            } else {
                while i < pattern.len() && pattern[i] == '-' {
                    i += 1;
                }
                if i < pattern.len() && pattern[i] == 'A' {
                    i += 1;
                } else {
                    ok = false;
                    break;
                }
                while i < pattern.len() && pattern[i] == '-' {
                    i += 1;
                }
            }
            counts.push(i - start);
        }
        if ok {
            return counts;
        }
    }
    Vec::new()
}

/// State of the parser.
struct Parser<'a> {
    spec: &'a Spec,
    argv: &'a [String],
    pattern: Vec<char>,
    /// remaining positionals: (name, takes any number of arguments)
    positionals: Vec<(&'static str, bool)>,
    parsed: Parsed,
}

impl Parser<'_> {
    /// argparse `consume_positionals`: the index after the consumed arguments.
    fn consume_positionals(&mut self, start: usize) -> usize {
        let kinds: Vec<bool> = self.positionals.iter().map(|p| p.1).collect();
        let counts = match_positionals(&kinds, &self.pattern[start..]);
        let mut i = start;
        for (p, n) in self.positionals.iter().zip(&counts) {
            let mut args: Vec<String> = self.argv[i..i + n].to_vec();
            if let Some(k) = args.iter().position(|a| a == "--") {
                args.remove(k);
            }
            self.parsed.positionals.push((p.0, args));
            i += n;
        }
        self.positionals.drain(..counts.len());
        i
    }
}

impl Spec {
    fn option(&self, s: &str) -> Option<Opt> {
        self.options.iter().find(|o| o.0 == s).map(|o| o.1)
    }

    /// The name of an action in error messages (`-h/--help`, `--lang`).
    fn action_name(&self, action: Opt) -> String {
        let names: Vec<&str> = self
            .options
            .iter()
            .filter(|o| o.1 == action)
            .map(|o| o.0)
            .collect();
        names.join("/")
    }

    /// An error of this subcommand's parser.
    pub fn error(&self, msg: &str) -> ExitCode {
        errln!("{}", self.usage);
        errln!("tok {}: error: {msg}", self.name);
        ExitCode::from(EXIT_USAGE)
    }

    fn ignored_explicit(&self, name: &str, arg: &str) -> ExitCode {
        self.error(&format!(
            "argument {name}: ignored explicit argument {}",
            tok_core::pyutil::repr_str(arg)
        ))
    }

    /// argparse `_parse_optional`; `Err` is an ambiguity error.
    fn parse_optional(&self, arg: &str) -> Result<Option<OptTuple>, String> {
        if !arg.starts_with('-') {
            return Ok(None);
        }
        if let Some(o) = self.option(arg) {
            return Ok(Some((Some(o), arg.to_string(), None, None)));
        }
        if arg.chars().count() == 1 {
            return Ok(None);
        }
        if let Some((opt, val)) = arg.split_once('=')
            && let Some(o) = self.option(opt)
        {
            return Ok(Some((
                Some(o),
                opt.to_string(),
                Some("="),
                Some(val.to_string()),
            )));
        }
        let mut tuples: Vec<OptTuple> = Vec::new();
        if arg.starts_with("--") {
            let (prefix, sep, explicit) = match arg.split_once('=') {
                Some((p, e)) => (p, Some("="), Some(e.to_string())),
                None => (arg, None, None),
            };
            for (s, o) in self.options {
                if s.starts_with(prefix) {
                    tuples.push((Some(*o), s.to_string(), sep, explicit.clone()));
                }
            }
        } else {
            let short: String = arg.chars().take(2).collect();
            let short_explicit: String = arg.chars().skip(2).collect();
            for (s, o) in self.options {
                if *s == short {
                    tuples.push((
                        Some(*o),
                        s.to_string(),
                        Some(""),
                        Some(short_explicit.clone()),
                    ));
                } else if s.starts_with(arg) {
                    tuples.push((Some(*o), s.to_string(), None, None));
                }
            }
        }
        if tuples.len() > 1 {
            let names: Vec<&str> = tuples.iter().map(|t| t.1.as_str()).collect();
            return Err(format!(
                "ambiguous option: {arg} could match {}",
                names.join(", ")
            ));
        }
        if let Some(t) = tuples.pop() {
            return Ok(Some(t));
        }
        if negative_number(arg) || arg.contains(' ') {
            return Ok(None);
        }
        Ok(Some((None, arg.to_string(), None, None)))
    }

    /// Store the value of an option after argparse's type and choice checks.
    fn take_value(&self, parsed: &mut Parsed, action: Opt, value: String) -> Result<(), ExitCode> {
        let Opt::Value(dest, conv) = action else {
            return Ok(());
        };
        let name = self.action_name(action);
        let value = match conv {
            Conv::Text => value,
            Conv::Int => int_digits(&value).ok_or_else(|| {
                self.error(&format!(
                    "argument {name}: invalid int value: {}",
                    tok_core::pyutil::repr_str(&value)
                ))
            })?,
            Conv::Choice(choices) => {
                if !choices.contains(&value.as_str()) {
                    let list: Vec<String> = choices
                        .iter()
                        .map(|c| tok_core::pyutil::repr_str(c))
                        .collect();
                    return Err(self.error(&format!(
                        "argument {name}: invalid choice: {} (choose from {})",
                        tok_core::pyutil::repr_str(&value),
                        list.join(", ")
                    )));
                }
                value
            }
        };
        parsed.values.push((dest, value));
        Ok(())
    }

    /// Parse the arguments after the subcommand name. `Err` carries the exit code after a
    /// usage error or the help text.
    pub fn parse(&self, argv: &[String]) -> Result<Parsed, ExitCode> {
        let mut pattern: Vec<char> = Vec::new();
        let mut options: Vec<Option<OptTuple>> = Vec::new();
        let mut rest_positional = false;
        for arg in argv {
            if rest_positional {
                pattern.push('A');
                options.push(None);
            } else if arg == "--" {
                pattern.push('-');
                options.push(None);
                rest_positional = true;
            } else {
                if arg.starts_with("--=") {
                    // the main parser classifies the arguments first and cannot resolve this prefix
                    return Err(usage(&format!(
                        "ambiguous option: {arg} could match --help, --version"
                    )));
                }
                match self.parse_optional(arg).map_err(|m| self.error(&m))? {
                    Some(t) => {
                        pattern.push('O');
                        options.push(Some(t));
                    }
                    None => {
                        pattern.push('A');
                        options.push(None);
                    }
                }
            }
        }
        let mut p = Parser {
            spec: self,
            argv,
            pattern,
            positionals: self.positionals.to_vec(),
            parsed: Parsed::default(),
        };
        let mut extras: Vec<String> = Vec::new();
        let option_indices: Vec<usize> = (0..p.pattern.len())
            .filter(|i| p.pattern[*i] == 'O')
            .collect();
        let max_option = option_indices.last().copied();
        let mut start = 0usize;
        while max_option.is_some_and(|m| start <= m) {
            let next_option = option_indices
                .iter()
                .copied()
                .find(|i| *i >= start)
                .unwrap_or(p.pattern.len());
            if start != next_option {
                let end = p.consume_positionals(start);
                if end > start {
                    start = end;
                    continue;
                }
                start = end;
            }
            if p.pattern.get(start) != Some(&'O') {
                extras.extend(argv[start..next_option].iter().cloned());
                start = next_option;
            }
            // consume_optional
            let Some(Some((action, mut option_string, mut sep, mut explicit))) =
                options.get(start).cloned()
            else {
                break;
            };
            let Some(mut action) = action else {
                extras.push(argv[start].clone());
                start += 1;
                continue;
            };
            let mut actions: Vec<(Opt, Option<String>)> = Vec::new();
            let stop;
            loop {
                let name = p.spec.action_name(action);
                let takes_value = matches!(action, Opt::Value(..));
                if let Some(exp) = explicit.clone() {
                    if takes_value {
                        actions.push((action, Some(exp)));
                        stop = start + 1;
                        break;
                    }
                    // a single-dash flag with an explicit argument: `-hX` reads X as further flags
                    if option_string.chars().nth(1) != Some('-') && !exp.is_empty() {
                        if sep == Some("=") || exp.starts_with('-') {
                            return Err(self.ignored_explicit(&name, &exp));
                        }
                        actions.push((action, None));
                        let next = format!("-{}", exp.chars().next().unwrap_or('-'));
                        let rest: String = exp.chars().skip(1).collect();
                        if let Some(o) = self.option(&next) {
                            action = o;
                            option_string = next;
                            if rest.is_empty() {
                                sep = None;
                                explicit = None;
                            } else if let Some(r) = rest.strip_prefix('=') {
                                sep = Some("=");
                                explicit = Some(r.to_string());
                            } else {
                                sep = Some("");
                                explicit = Some(rest);
                            }
                            continue;
                        }
                        extras.push(format!("-{exp}"));
                        stop = start + 1;
                        break;
                    }
                    return Err(self.ignored_explicit(&name, &exp));
                }
                if takes_value {
                    if p.pattern.get(start + 1) != Some(&'A') {
                        return Err(self.error(&format!("argument {name}: expected one argument")));
                    }
                    actions.push((action, Some(argv[start + 1].clone())));
                    stop = start + 2;
                } else {
                    actions.push((action, None));
                    stop = start + 1;
                }
                break;
            }
            for (act, value) in actions {
                match act {
                    Opt::Help => {
                        out!("{}", self.help);
                        return Err(ExitCode::from(EXIT_OK));
                    }
                    Opt::Flag(dest) => {
                        if !p.parsed.flags.contains(&dest) {
                            p.parsed.flags.push(dest);
                        }
                    }
                    Opt::Value(..) => {
                        self.take_value(&mut p.parsed, act, value.unwrap_or_default())?
                    }
                }
            }
            start = stop;
        }
        let stop = p.consume_positionals(start);
        extras.extend(argv[stop..].iter().cloned());
        if !p.positionals.is_empty() {
            let names: Vec<&str> = p.positionals.iter().map(|q| q.0).collect();
            return Err(self.error(&format!(
                "the following arguments are required: {}",
                names.join(", ")
            )));
        }
        if !extras.is_empty() {
            return Err(usage(&format!(
                "unrecognized arguments: {}",
                extras.join(" ")
            )));
        }
        Ok(p.parsed)
    }
}
