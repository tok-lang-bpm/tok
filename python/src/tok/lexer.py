"""Line-based tokenizer for tok documents.

A document is split into logical lines. Each line has an indentation width,
a list of tokens and an optional trailing comment. Decision table rows
(lines starting with ``|``) are kept as raw text in a single ROW token.
"""

WORD, NUMBER, STRING, ARROW, EQUALS, ROW = "WORD", "NUMBER", "STRING", "ARROW", "EQUALS", "ROW"

_WORD_START = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_")
_WORD_CHARS = _WORD_START | set("0123456789-.:/+")
_DIGITS = set("0123456789")


class Token:
    __slots__ = ("kind", "value", "col")

    def __init__(self, kind, value, col):
        self.kind = kind
        self.value = value
        self.col = col

    def __repr__(self):
        return "Token(%s, %r)" % (self.kind, self.value)


class Line:
    """One logical line: ``indent`` spaces, ``tokens`` and a trailing ``comment``.

    ``tokens`` is empty for comment-only lines (``comment`` is then set and
    ``standalone`` is True).
    """
    __slots__ = ("number", "indent", "tokens", "comment", "standalone")

    def __init__(self, number, indent, tokens, comment, standalone):
        self.number = number
        self.indent = indent
        self.tokens = tokens
        self.comment = comment
        self.standalone = standalone


class LexError(Exception):
    def __init__(self, code, line, message):
        Exception.__init__(self, message)
        self.code = code
        self.line = line
        self.message = message


def tokenize(text):
    """Return (lines, errors). Errors are (code, line, message) tuples."""
    lines, errors = [], []
    if text.startswith("\ufeff"):
        text = text[1:]
    for number, raw in enumerate(text.replace("\r\n", "\n").replace("\r", "\n").split("\n"), start=1):
        raw = raw.rstrip()
        if not raw.strip():
            continue
        stripped = raw.lstrip(" ")
        lead = raw[: len(raw) - len(stripped)]
        if stripped.startswith("\t") or "\t" in lead:
            errors.append(("T002", number, "tab character in indentation"))
            stripped = stripped.lstrip("\t ")
        indent = len(lead)
        if stripped.startswith("#"):
            lines.append(Line(number, indent, [], stripped[1:].strip(), True))
            continue
        if stripped.startswith("|"):
            body, comment = _split_row_comment(stripped)
            lines.append(Line(number, indent, [Token(ROW, body, indent)], comment, False))
            continue
        try:
            tokens, comment = _scan(stripped, indent)
        except LexError as ex:
            errors.append((ex.code, number, ex.message))
            continue
        lines.append(Line(number, indent, tokens, comment, False))
    return lines, errors


def _split_row_comment(s):
    in_str = False
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and in_str:
            i += 2
            continue
        if c == '"':
            in_str = not in_str
        elif c == "#" and not in_str:
            return s[:i].rstrip(), s[i + 1:].strip()
        i += 1
    return s, None


def _scan(s, offset):
    tokens = []
    i, n = 0, len(s)
    while i < n:
        c = s[i]
        col = offset + i + 1
        if c == " ":
            i += 1
        elif c == "#":
            return tokens, s[i + 1:].strip()
        elif c == '"':
            j, buf = i + 1, []
            while True:
                if j >= n:
                    raise LexError("T001", 0, "unterminated string")
                ch = s[j]
                if ch == "\\" and j + 1 < n and s[j + 1] in '"\\n':
                    buf.append("\n" if s[j + 1] == "n" else s[j + 1])
                    j += 2
                elif ch == '"':
                    break
                else:
                    buf.append(ch)
                    j += 1
            tokens.append(Token(STRING, "".join(buf), col))
            i = j + 1
        elif c == "-" and i + 1 < n and s[i + 1] == ">":
            tokens.append(Token(ARROW, "->", col))
            i += 2
        elif c == "=":
            tokens.append(Token(EQUALS, "=", col))
            i += 1
        elif c in _DIGITS or (c == "-" and i + 1 < n and s[i + 1] in _DIGITS):
            j = i + 1
            while j < n and s[j] in _DIGITS:
                j += 1
            if j < n and s[j] == "." and j + 1 < n and s[j + 1] in _DIGITS:
                j += 1
                while j < n and s[j] in _DIGITS:
                    j += 1
            if j < n and s[j] in _WORD_CHARS and s[j] not in "-":
                # something like 2027-01-04T08:00 or 30m: read as a word
                while j < n and (s[j] in _WORD_CHARS or s[j] in _DIGITS):
                    j += 1
                tokens.append(Token(WORD, s[i:j], col))
            elif j < n and s[j] == "-" and j + 1 < n and s[j + 1] in _DIGITS:
                while j < n and (s[j] in _WORD_CHARS or s[j] in _DIGITS):
                    j += 1
                tokens.append(Token(WORD, s[i:j], col))
            else:
                tokens.append(Token(NUMBER, s[i:j], col))
            i = j
        elif c in _WORD_START or ord(c) > 127:
            j = i + 1
            while j < n and (s[j] in _WORD_CHARS or ord(s[j]) > 127):
                if s[j] == "-" and j + 1 < n and s[j + 1] == ">":
                    break
                j += 1
            tokens.append(Token(WORD, s[i:j], col))
            i = j
        else:
            raise LexError("T001", 0, "unexpected character %r" % c)
    return tokens, None
