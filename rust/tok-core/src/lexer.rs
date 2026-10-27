//! Line-based tokenizer (spec §2). Mirrors `tok/python/src/tok/lexer.py`.

use crate::pyutil::{repr_char, rstrip, strip};

/// Kind of a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Word,
    Number,
    Str,
    Arrow,
    Equals,
    Row,
}

/// One token with its text value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub value: String,
}

/// One logical line: indentation, tokens and a trailing comment.
#[derive(Debug, Clone)]
pub struct Line {
    pub number: usize,
    pub indent: usize,
    pub tokens: Vec<Token>,
    pub comment: Option<String>,
    /// A comment-only line.
    pub standalone: bool,
}

/// A tokenizer error: (code, line, detail).
pub type LexError = (&'static str, usize, String);

fn word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn word_char(c: char) -> bool {
    word_start(c) || c.is_ascii_digit() || matches!(c, '-' | '.' | ':' | '/' | '+')
}

fn tok(kind: Kind, value: impl Into<String>) -> Token {
    Token {
        kind,
        value: value.into(),
    }
}

/// Split text into lines and tokens. Returns lines and tokenizer errors.
pub fn tokenize(text: &str) -> (Vec<Line>, Vec<LexError>) {
    let mut lines = Vec::new();
    let mut errors = Vec::new();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    for (idx, raw) in normalized.split('\n').enumerate() {
        let number = idx + 1;
        let raw = rstrip(raw);
        if strip(raw).is_empty() {
            continue;
        }
        let mut stripped = raw.trim_start_matches(' ');
        let indent = raw.len() - stripped.len();
        if stripped.starts_with('\t') {
            errors.push(("T002", number, "tab character in indentation".to_string()));
            stripped = stripped.trim_start_matches(['\t', ' ']);
        }
        if let Some(rest) = stripped.strip_prefix('#') {
            lines.push(Line {
                number,
                indent,
                tokens: Vec::new(),
                comment: Some(strip(rest).to_string()),
                standalone: true,
            });
            continue;
        }
        if stripped.starts_with('|') {
            let (body, comment) = split_row_comment(stripped);
            lines.push(Line {
                number,
                indent,
                tokens: vec![tok(Kind::Row, body)],
                comment,
                standalone: false,
            });
            continue;
        }
        match scan(stripped) {
            Ok((tokens, comment)) => lines.push(Line {
                number,
                indent,
                tokens,
                comment,
                standalone: false,
            }),
            Err(msg) => errors.push(("T001", number, msg)),
        }
    }
    (lines, errors)
}

fn split_row_comment(s: &str) -> (String, Option<String>) {
    let chars: Vec<char> = s.chars().collect();
    let mut in_str = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && in_str {
            i += 2;
            continue;
        }
        if c == '"' {
            in_str = !in_str;
        } else if c == '#' && !in_str {
            let body: String = chars[..i].iter().collect();
            let rest: String = chars[i + 1..].iter().collect();
            return (rstrip(&body).to_string(), Some(strip(&rest).to_string()));
        }
        i += 1;
    }
    (s.to_string(), None)
}

fn scan(s: &str) -> Result<(Vec<Token>, Option<String>), String> {
    let ch: Vec<char> = s.chars().collect();
    let n = ch.len();
    let mut tokens = Vec::new();
    let mut i = 0;
    let slice = |a: usize, b: usize| -> String { ch[a..b].iter().collect() };
    while i < n {
        let c = ch[i];
        if c == ' ' {
            i += 1;
        } else if c == '#' {
            return Ok((tokens, Some(strip(&slice(i + 1, n)).to_string())));
        } else if c == '"' {
            let mut j = i + 1;
            let mut buf = String::new();
            loop {
                if j >= n {
                    return Err("unterminated string".into());
                }
                let x = ch[j];
                if x == '\\' && j + 1 < n && matches!(ch[j + 1], '"' | '\\' | 'n') {
                    buf.push(if ch[j + 1] == 'n' { '\n' } else { ch[j + 1] });
                    j += 2;
                } else if x == '"' {
                    break;
                } else {
                    buf.push(x);
                    j += 1;
                }
            }
            tokens.push(tok(Kind::Str, buf));
            i = j + 1;
        } else if c == '-' && i + 1 < n && ch[i + 1] == '>' {
            tokens.push(tok(Kind::Arrow, "->"));
            i += 2;
        } else if c == '=' {
            tokens.push(tok(Kind::Equals, "="));
            i += 1;
        } else if c.is_ascii_digit() || (c == '-' && i + 1 < n && ch[i + 1].is_ascii_digit()) {
            let mut j = i + 1;
            while j < n && ch[j].is_ascii_digit() {
                j += 1;
            }
            if j < n && ch[j] == '.' && j + 1 < n && ch[j + 1].is_ascii_digit() {
                j += 1;
                while j < n && ch[j].is_ascii_digit() {
                    j += 1;
                }
            }
            // something like 2027-01-04T08:00 or 30m: read as a word
            let word_follows = j < n && word_char(ch[j]) && ch[j] != '-';
            let date_follows = j < n && ch[j] == '-' && j + 1 < n && ch[j + 1].is_ascii_digit();
            if word_follows || date_follows {
                while j < n && word_char(ch[j]) {
                    j += 1;
                }
                tokens.push(tok(Kind::Word, slice(i, j)));
            } else {
                tokens.push(tok(Kind::Number, slice(i, j)));
            }
            i = j;
        } else if word_start(c) || (c as u32) > 127 {
            let mut j = i + 1;
            while j < n && (word_char(ch[j]) || (ch[j] as u32) > 127) {
                if ch[j] == '-' && j + 1 < n && ch[j + 1] == '>' {
                    break;
                }
                j += 1;
            }
            tokens.push(tok(Kind::Word, slice(i, j)));
            i = j;
        } else {
            return Err(format!("unexpected character {}", repr_char(c)));
        }
    }
    Ok((tokens, None))
}
