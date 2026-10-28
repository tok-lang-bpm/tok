//! Lines and positions of a document. The lexer counts columns in characters; LSP 3.17
//! counts them in UTF-16 code units (the only position encoding this server offers), so a
//! Polish letter is one unit and a character outside the Basic Multilingual Plane two.

use serde_json::{Value, json};

/// The lines of a document as the editor holds them, split at `\n`, `\r\n` and `\r` like
/// the lexer splits them.
#[derive(Debug, Clone)]
pub struct Text {
    lines: Vec<Vec<char>>,
    /// The document starts with a byte order mark, which the lexer drops: the columns of
    /// the lexer on the first line are one character to the left of the editor's.
    bom: bool,
}

impl Text {
    pub fn new(text: &str) -> Self {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        Text {
            lines: normalized
                .split('\n')
                .map(|l| l.chars().collect())
                .collect(),
            bom: text.starts_with('\u{feff}'),
        }
    }

    /// Number of lines (a document always has at least one).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// The characters of line `line` (0-based), as the lexer sees them.
    pub fn chars(&self, line: usize) -> &[char] {
        let chars = self.lines.get(line).map(Vec::as_slice).unwrap_or(&[]);
        if line == 0 && self.bom {
            chars.get(1..).unwrap_or(&[])
        } else {
            chars
        }
    }

    fn shift(&self, line: usize) -> usize {
        usize::from(line == 0 && self.bom)
    }

    /// The UTF-16 column of lexer column `col` (characters) on line `line`.
    pub fn utf16(&self, line: usize, col: usize) -> u32 {
        let raw = self.lines.get(line).map(Vec::as_slice).unwrap_or(&[]);
        let end = (col + self.shift(line)).min(raw.len());
        let units: usize = raw[..end].iter().map(|c| c.len_utf16()).sum();
        u32::try_from(units).unwrap_or(u32::MAX)
    }

    /// The lexer column (characters) at UTF-16 column `units` of line `line`; a position
    /// inside a surrogate pair or beyond the end of the line is moved to the character.
    pub fn column(&self, line: usize, units: u32) -> usize {
        let raw = self.lines.get(line).map(Vec::as_slice).unwrap_or(&[]);
        let mut seen = 0usize;
        let mut col = 0usize;
        for c in raw {
            if seen >= units as usize {
                break;
            }
            seen += c.len_utf16();
            col += 1;
        }
        col.saturating_sub(self.shift(line))
    }

    /// The UTF-16 length of line `line`.
    pub fn end(&self, line: usize) -> u32 {
        self.utf16(line, usize::MAX / 2)
    }

    /// The range of lexer columns `start..end` on line `line` (0-based).
    pub fn range(&self, line: usize, start: usize, end: usize) -> Value {
        json!({
            "start": {"line": line, "character": self.utf16(line, start)},
            "end": {"line": line, "character": self.utf16(line, end)},
        })
    }

    /// The range of the text of line `line` without its indentation and trailing spaces.
    pub fn line_range(&self, line: usize) -> Value {
        let chars = self.chars(line);
        let start = chars.iter().take_while(|c| c.is_whitespace()).count();
        let end = chars.len() - chars.iter().rev().take_while(|c| c.is_whitespace()).count();
        self.range(line, start, end.max(start))
    }
}

/// A position of a request, `(line, character)` as the editor sends it.
pub fn position(params: &Value) -> Option<(usize, u32)> {
    let p = params.get("position")?;
    let line = usize::try_from(p.get("line")?.as_u64()?).ok()?;
    let character = u32::try_from(p.get("character")?.as_u64()?).ok()?;
    Some((line, character))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_count_utf16_units() {
        let t = Text::new("tok 0.1\r\nuser zł \"𝔸 ę\" x\n");
        assert_eq!(t.len(), 3);
        // `zł` is two characters and two units; `𝔸` is one character and two units
        assert_eq!(t.utf16(1, 7), 7);
        assert_eq!(t.utf16(1, 9), 9);
        assert_eq!(t.utf16(1, 10), 11);
        assert_eq!(t.column(1, 11), 10);
        assert_eq!(t.column(1, 10), 10);
        assert_eq!(t.end(1), 16);
    }

    #[test]
    fn a_byte_order_mark_shifts_the_first_line() {
        let t = Text::new("\u{feff}tok 0.1\n");
        assert_eq!(t.chars(0).len(), 7);
        assert_eq!(t.utf16(0, 0), 1);
        assert_eq!(t.column(0, 1), 0);
    }
}
