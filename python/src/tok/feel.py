"""FEEL subset used by tok: expressions and unary tests (spec §12, docs/feel.md).

The module has three parts: a tokenizer, a recursive-descent parser that
builds a small AST of tuples, and an evaluator. Numbers are decimal.Decimal.
``null`` is Python ``None``. Errors are raised as FeelSyntaxError (F002) or
FeelUnsupported (F001, valid FEEL outside the subset).

AST nodes (first item is the node type):
  ("num", Decimal) ("str", text) ("bool", True|False) ("null",)
  ("name", "a") ("path", node, "field") ("list", [nodes])
  ("neg", node) ("bin", op, left, right)       op in + - * / = != < <= > >=
  ("and", l, r) ("or", l, r) ("not", node) ("if", cond, then, else)
  ("between", x, low, high) ("in", x, target) ("call", fname, [args])
  ("range", low_closed, low, high, high_closed)
Unary tests:
  ("any",) ("tests", [positive]) ("nottests", [positive])
  positive: ("cmp", op, node) ("range", ...) ("eq", node)
"""

import datetime
import re
import decimal
from decimal import Decimal

D = Decimal


class FeelError(Exception):
    code = "F002"


class FeelSyntaxError(FeelError):
    code = "F002"


class FeelUnsupported(FeelError):
    code = "F001"


KEYWORDS = ("if", "then", "else", "and", "or", "between", "in", "true", "false", "null")
UNSUPPORTED_WORDS = ("for", "some", "every", "function", "instance", "satisfies", "return")

FUNCTIONS = {
    # name: (min args, max args)
    "string length": (1, 1), "upper case": (1, 1), "lower case": (1, 1),
    "contains": (2, 2), "starts with": (2, 2), "ends with": (2, 2), "substring": (2, 3),
    "abs": (1, 1), "floor": (1, 1), "ceiling": (1, 1), "round half up": (2, 2),
    "min": (1, 99), "max": (1, 99), "sum": (1, 99), "count": (1, 1),
    "string": (1, 1), "number": (1, 1), "date": (1, 1), "not": (1, 1),
}
MULTIWORD = sorted([f.split(" ") for f in FUNCTIONS if " " in f], key=len, reverse=True)

# ------------------------------------------------------------------ tokenizer

NUM, STR, NAME, OP, END = "num", "str", "name", "op", "end"
_OPS2 = ("..", "<=", ">=", "!=")
_OPS1 = "+-*/=<>()[],."


def tokenize(text):
    toks, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c in " \t\r\n":
            i += 1
        elif c.isdigit() or (c == "." and i + 1 < n and text[i + 1].isdigit()):
            j = i
            while j < n and text[j].isdigit():
                j += 1
            if j < n and text[j] == "." and j + 1 < n and text[j + 1].isdigit():
                j += 1
                while j < n and text[j].isdigit():
                    j += 1
            toks.append((NUM, text[i:j]))
            i = j
        elif c == '"':
            j, buf = i + 1, []
            while True:
                if j >= n:
                    raise FeelSyntaxError("unterminated string")
                ch = text[j]
                if ch == "\\" and j + 1 < n:
                    nxt = text[j + 1]
                    buf.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\"}.get(nxt, "\\" + nxt))
                    j += 2
                elif ch == '"':
                    break
                else:
                    buf.append(ch)
                    j += 1
            toks.append((STR, "".join(buf)))
            i = j + 1
        elif c.isalpha() or c == "_":
            j = i + 1
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            toks.append((NAME, text[i:j]))
            i = j
        elif text[i:i + 2] == "**":
            raise FeelUnsupported("exponentiation '**'")
        elif text[i:i + 2] in _OPS2:
            toks.append((OP, text[i:i + 2]))
            i += 2
        elif c in _OPS1:
            toks.append((OP, c))
            i += 1
        elif c in "{}:?@;":
            raise FeelUnsupported("'%s' (contexts, filters and temporal literals are not in the subset)" % c)
        else:
            raise FeelSyntaxError("unexpected character %r" % c)
    toks.append((END, None))
    return toks


# --------------------------------------------------------------------- parser

class _Parser:
    def __init__(self, text):
        self.toks = tokenize(text)
        self.i = 0
        self.in_interval = False

    def peek(self, k=0):
        return self.toks[min(self.i + k, len(self.toks) - 1)]

    def next(self):
        t = self.toks[self.i]
        self.i += 1
        return t

    def at_op(self, op):
        t = self.peek()
        return t[0] == OP and t[1] == op

    def at_word(self, w):
        t = self.peek()
        return t[0] == NAME and t[1] == w

    def expect_op(self, op):
        t = self.next()
        if t[0] != OP or t[1] != op:
            raise FeelSyntaxError("expected '%s', found %s" % (op, _show(t)))

    def expect_word(self, w):
        t = self.next()
        if t[0] != NAME or t[1] != w:
            raise FeelSyntaxError("expected '%s', found %s" % (w, _show(t)))

    def expect_end(self):
        if self.peek()[0] != END:
            raise FeelSyntaxError("unexpected %s" % _show(self.peek()))

    # expression grammar, lowest precedence first
    def expr(self):
        if self.at_word("if"):
            self.next()
            c = self.expr()
            self.expect_word("then")
            a = self.expr()
            self.expect_word("else")
            b = self.expr()
            return ("if", c, a, b)
        t = self.peek()
        if t[0] == NAME and t[1] in UNSUPPORTED_WORDS:
            raise FeelUnsupported("'%s' expressions" % t[1])
        return self.disjunction()

    def disjunction(self):
        left = self.conjunction()
        while self.at_word("or"):
            self.next()
            left = ("or", left, self.conjunction())
        return left

    def conjunction(self):
        left = self.comparison()
        while self.at_word("and"):
            self.next()
            left = ("and", left, self.comparison())
        return left

    def comparison(self):
        left = self.additive()
        t = self.peek()
        if t[0] == OP and t[1] in ("=", "!=", "<", "<=", ">", ">="):
            self.next()
            return ("bin", t[1], left, self.additive())
        if self.at_word("between"):
            self.next()
            low = self.additive()
            self.expect_word("and")
            high = self.additive()
            return ("between", left, low, high)
        if self.at_word("in"):
            self.next()
            return ("in", left, self.in_target())
        if t[0] == NAME and t[1] == "instance":
            raise FeelUnsupported("'instance of'")
        return left

    def in_target(self):
        if self.at_op("(") or self.at_op("]"):
            return self.interval()
        if self.at_op("["):
            save = self.i
            self.next()
            first = self.expr()
            if self.at_op(".."):
                self.i = save
                return self.interval()
            self.i = save
        return self.additive()

    def interval(self):
        t = self.next()
        if t[0] != OP or t[1] not in ("[", "(", "]"):
            raise FeelSyntaxError("expected an interval")
        low_closed = t[1] == "["
        low = self.additive()
        self.expect_op("..")
        self.in_interval = True
        try:
            high = self.additive()
        finally:
            self.in_interval = False
        t = self.next()
        if t[0] != OP or t[1] not in ("]", ")", "["):
            raise FeelSyntaxError("expected ']' or ')' closing the interval")
        return ("range", low_closed, low, high, t[1] == "]")

    def additive(self):
        left = self.multiplicative()
        while self.at_op("+") or self.at_op("-"):
            op = self.next()[1]
            left = ("bin", op, left, self.multiplicative())
        return left

    def multiplicative(self):
        left = self.unary()
        while self.at_op("*") or self.at_op("/"):
            op = self.next()[1]
            left = ("bin", op, left, self.unary())
        return left

    def unary(self):
        if self.at_op("-"):
            self.next()
            return ("neg", self.unary())
        return self.postfix()

    def postfix(self):
        node = self.primary()
        while self.at_op("."):
            self.next()
            t = self.next()
            if t[0] != NAME:
                raise FeelSyntaxError("expected a name after '.'")
            node = ("path", node, t[1])
        if self.at_op("[") and not self.in_interval:
            raise FeelUnsupported("filter expressions")
        return node

    def primary(self):
        t = self.peek()
        if t[0] == NUM:
            self.next()
            return ("num", D(t[1]))
        if t[0] == STR:
            self.next()
            return ("str", t[1])
        if t[0] == OP and t[1] == "(":
            self.next()
            e = self.expr()
            self.expect_op(")")
            return e
        if t[0] == OP and t[1] == "[":
            self.next()
            items = []
            if not self.at_op("]"):
                items.append(self.expr())
                if self.at_op(".."):
                    raise FeelSyntaxError("an interval is allowed only after 'in' or as a unary test")
                while self.at_op(","):
                    self.next()
                    items.append(self.expr())
            self.expect_op("]")
            return ("list", items)
        if t[0] == NAME:
            fname = self.function_name()
            if fname is not None:
                return self.call(fname)
            self.next()
            w = t[1]
            if w == "true":
                return ("bool", True)
            if w == "false":
                return ("bool", False)
            if w == "null":
                return ("null",)
            if w in UNSUPPORTED_WORDS:
                raise FeelUnsupported("'%s' expressions" % w)
            if w in KEYWORDS:
                raise FeelSyntaxError("unexpected keyword '%s'" % w)
            if self.at_op("("):
                raise FeelUnsupported("function '%s'" % w)
            return ("name", w)
        raise FeelSyntaxError("unexpected %s" % _show(t))

    def function_name(self):
        """Return a known function name starting at the current token if a '(' follows it."""
        for words in MULTIWORD:
            ok = True
            for k, w in enumerate(words):
                t = self.peek(k)
                if t[0] != NAME or t[1] != w:
                    ok = False
                    break
            if ok:
                t = self.peek(len(words))
                if t[0] == OP and t[1] == "(":
                    return " ".join(words)
        t = self.peek()
        if t[1] in FUNCTIONS and " " not in t[1]:
            nxt = self.peek(1)
            if nxt[0] == OP and nxt[1] == "(":
                return t[1]
        return None

    def call(self, fname):
        self.i += len(fname.split(" "))
        self.expect_op("(")
        args = []
        if not self.at_op(")"):
            args.append(self.expr())
            while self.at_op(","):
                self.next()
                args.append(self.expr())
        self.expect_op(")")
        lo, hi = FUNCTIONS[fname]
        if not lo <= len(args) <= hi:
            raise FeelSyntaxError("%s() takes %s arguments" % (fname, lo if lo == hi else "%d-%d" % (lo, hi)))
        if fname == "not":
            return ("not", args[0])
        return ("call", fname, args)

    # unary tests
    def unary_tests(self):
        if self.peek()[0] == OP and self.peek()[1] == "-" and self.peek(1)[0] == END:
            self.next()
            return ("any",)
        if self.at_word("not") and self.peek(1) == (OP, "("):
            save = self.i
            self.next()
            self.next()
            tests = self.positive_list()
            if self.at_op(")") and self.peek(1)[0] == END:
                self.next()
                return ("nottests", tests)
            self.i = save
        return ("tests", self.positive_list())

    def positive_list(self):
        tests = [self.positive()]
        while self.at_op(","):
            self.next()
            tests.append(self.positive())
        return tests

    def positive(self):
        t = self.peek()
        if t[0] == OP and t[1] in ("<", "<=", ">", ">=", "=", "!="):
            self.next()
            return ("cmp", t[1], self.endpoint())
        if t[0] == OP and t[1] in ("(", "]"):
            save = self.i
            try:
                return self.interval()
            except FeelSyntaxError:
                self.i = save
        if t[0] == OP and t[1] == "[":
            save = self.i
            try:
                return self.interval()
            except FeelSyntaxError:
                self.i = save
        return ("eq", self.additive())

    def endpoint(self):
        return self.additive()


def _show(t):
    if t[0] == END:
        return "end of expression"
    return "'%s'" % t[1]


def parse_expr(text):
    """Parse a FEEL expression. Raises FeelSyntaxError or FeelUnsupported."""
    if text is None or not text.strip():
        raise FeelSyntaxError("empty expression")
    p = _Parser(text)
    node = p.expr()
    p.expect_end()
    return node


def parse_unary_tests(text):
    """Parse FEEL unary tests (a decision table input cell)."""
    if text is None or not text.strip():
        raise FeelSyntaxError("empty cell; use '-' for any value")
    p = _Parser(text)
    node = p.unary_tests()
    p.expect_end()
    return node


def check_expr(text):
    """None if the expression is in the subset, else (code, detail)."""
    try:
        parse_expr(text)
    except FeelError as ex:
        return (ex.code, str(ex))
    return None


def check_unary(text):
    try:
        parse_unary_tests(text)
    except FeelError as ex:
        return (ex.code, str(ex))
    return None


def _pl_token(text):
    """'x' -> „x”, end of expression -> koniec wyrażenia."""
    text = text.strip()
    if text == "end of expression":
        return "koniec wyrażenia"
    if len(text) >= 2 and text[0] == text[-1] and text[0] in "'\"":
        return "„%s”" % text[1:-1]
    return text


_PL_FIXED = {
    "unterminated string": "niezamknięty tekst",
    "exponentiation '**'": "potęgowanie „**”",
    "'instance of'": "„instance of”",
    "expected an interval": "oczekiwano przedziału",
    "expected ']' or ')' closing the interval": "oczekiwano „]” albo „)” zamykającego przedział",
    "expected a name after '.'": "oczekiwano nazwy po „.”",
    "filter expressions": "wyrażenia filtrujące",
    "an interval is allowed only after 'in' or as a unary test":
        "przedział jest dozwolony tylko po „in” albo jako test jednoargumentowy",
    "empty expression": "puste wyrażenie",
    "empty cell; use '-' for any value": "pusta komórka; dla dowolnej wartości wpisz „-”",
    "an interval is not a value": "przedział nie jest wartością",
}

_PL_PATTERNS = (
    (re.compile(r"^'(.*)' \(contexts, filters and temporal literals are not in the subset\)$"),
     lambda m: "„%s” (konteksty, filtry i literały czasowe nie należą do podzbioru)" % m.group(1)),
    (re.compile(r"^unexpected character (.+)$"), lambda m: "nieoczekiwany znak %s" % _pl_token(m.group(1))),
    (re.compile(r"^expected (.+?), found (.+)$"),
     lambda m: "oczekiwano %s, znaleziono %s" % (_pl_token(m.group(1)), _pl_token(m.group(2)))),
    (re.compile(r"^unexpected keyword (.+)$"), lambda m: "nieoczekiwane słowo kluczowe %s" % _pl_token(m.group(1))),
    (re.compile(r"^unexpected end of expression$"), lambda m: "nieoczekiwany koniec wyrażenia"),
    (re.compile(r"^unexpected (.+)$"), lambda m: "nieoczekiwany element %s" % _pl_token(m.group(1))),
    (re.compile(r"^'(.*)' expressions$"), lambda m: "wyrażenia „%s”" % m.group(1)),
    (re.compile(r"^function (.+)$"), lambda m: "funkcja %s" % _pl_token(m.group(1))),
    (re.compile(r"^(.+)\(\) takes (.+) arguments$"), lambda m: "%s(): liczba argumentów: %s" % (m.group(1), m.group(2))),
    (re.compile(r"^unknown node (.+)$"), lambda m: "nieznany węzeł %s" % m.group(1)),
)


def detail_pl(text):
    """Polish version of an F001/F002 detail produced by this module."""
    if text in _PL_FIXED:
        return _PL_FIXED[text]
    for rx, fn in _PL_PATTERNS:
        m = rx.match(text)
        if m:
            return fn(m)
    return text


# ------------------------------------------------------------------ evaluator

def to_feel(v):
    """Convert a Python value from a context into a FEEL value."""
    if v is None or isinstance(v, (bool, str, Decimal, datetime.date)):
        return v
    if isinstance(v, int):
        return D(v)
    if isinstance(v, float):
        return D(repr(v))
    if isinstance(v, (list, tuple)):
        return [to_feel(x) for x in v]
    if isinstance(v, dict):
        return dict((k, to_feel(x)) for k, x in v.items())
    return v


def _is_num(v):
    return isinstance(v, Decimal)


def _same_type(a, b):
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool)
    if _is_num(a) and _is_num(b):
        return True
    if isinstance(a, str) and isinstance(b, str):
        return True
    if isinstance(a, datetime.date) and isinstance(b, datetime.date):
        return True
    if isinstance(a, list) and isinstance(b, list):
        return True
    return False


def equal(a, b):
    """FEEL '=': null = null is true, null = x is false, different types give null."""
    if a is None or b is None:
        return a is None and b is None
    if not _same_type(a, b):
        return None
    if isinstance(a, list):
        if len(a) != len(b):
            return False
        for x, y in zip(a, b):
            if equal(x, y) is not True:
                return False
        return True
    return a == b


def compare(op, a, b):
    if op == "=":
        return equal(a, b)
    if op == "!=":
        r = equal(a, b)
        return None if r is None else not r
    if a is None or b is None or not _same_type(a, b) or isinstance(a, (bool, list)):
        return None
    if op == "<":
        return a < b
    if op == "<=":
        return a <= b
    if op == ">":
        return a > b
    if op == ">=":
        return a >= b
    return None


def in_range(v, rng, ctx):
    _, low_closed, low, high, high_closed = rng
    lo, hi = evaluate(low, ctx), evaluate(high, ctx)
    a = compare(">=" if low_closed else ">", v, lo)
    b = compare("<=" if high_closed else "<", v, hi)
    if a is None or b is None:
        return None
    return a and b


def evaluate(node, ctx):
    """Evaluate an AST against a context (dict of variables). Returns a FEEL value."""
    t = node[0]
    if t == "num":
        return node[1]
    if t == "str":
        return node[1]
    if t == "bool":
        return node[1]
    if t == "null":
        return None
    if t == "name":
        return to_feel(ctx.get(node[1])) if ctx else None
    if t == "path":
        base = evaluate(node[1], ctx)
        if isinstance(base, dict):
            return to_feel(base.get(node[2]))
        return None
    if t == "list":
        return [evaluate(x, ctx) for x in node[1]]
    if t == "neg":
        v = evaluate(node[1], ctx)
        return -v if _is_num(v) else None
    if t == "bin":
        op, a, b = node[1], evaluate(node[2], ctx), evaluate(node[3], ctx)
        if op in ("=", "!=", "<", "<=", ">", ">="):
            return compare(op, a, b)
        return _arith(op, a, b)
    if t == "and":
        a, b = _bool(evaluate(node[1], ctx)), _bool(evaluate(node[2], ctx))
        if a is False or b is False:
            return False
        if a is True and b is True:
            return True
        return None
    if t == "or":
        a, b = _bool(evaluate(node[1], ctx)), _bool(evaluate(node[2], ctx))
        if a is True or b is True:
            return True
        if a is False and b is False:
            return False
        return None
    if t == "not":
        v = evaluate(node[1], ctx)
        return (not v) if isinstance(v, bool) else None
    if t == "if":
        return evaluate(node[2] if evaluate(node[1], ctx) is True else node[3], ctx)
    if t == "between":
        v = evaluate(node[1], ctx)
        a = compare(">=", v, evaluate(node[2], ctx))
        b = compare("<=", v, evaluate(node[3], ctx))
        if a is None or b is None:
            return None
        return a and b
    if t == "in":
        v, target = evaluate(node[1], ctx), node[2]
        if target[0] == "range":
            return in_range(v, target, ctx)
        tv = evaluate(target, ctx)
        if isinstance(tv, list):
            return any(equal(v, x) is True for x in tv)
        return equal(v, tv)
    if t == "range":
        raise FeelSyntaxError("an interval is not a value")
    if t == "call":
        return _call(node[1], [evaluate(a, ctx) for a in node[2]])
    raise FeelSyntaxError("unknown node %r" % (t,))


def _bool(v):
    return v if isinstance(v, bool) else None


def _arith(op, a, b):
    if a is None or b is None:
        return None
    if op == "+" and isinstance(a, str) and isinstance(b, str):
        return a + b
    if not (_is_num(a) and _is_num(b)):
        return None
    try:
        if op == "+":
            return a + b
        if op == "-":
            return a - b
        if op == "*":
            return a * b
        if op == "/":
            if b == 0:
                return None
            return a / b
    except decimal.InvalidOperation:
        return None
    return None


def number_to_string(v):
    """12.50 -> '12.5', 3.0 -> '3', 1E+2 -> '100'."""
    if v == v.to_integral_value():
        return str(v.quantize(D(1)))
    s = format(v.normalize(), "f")
    return s


def _call(f, args):
    if f in ("min", "max", "sum") and len(args) == 1 and isinstance(args[0], list):
        args = args[0]
    if f == "string length":
        return D(len(args[0])) if isinstance(args[0], str) else None
    if f == "upper case":
        return args[0].upper() if isinstance(args[0], str) else None
    if f == "lower case":
        return args[0].lower() if isinstance(args[0], str) else None
    if f in ("contains", "starts with", "ends with"):
        s, x = args
        if not (isinstance(s, str) and isinstance(x, str)):
            return None
        if f == "contains":
            return x in s
        if f == "starts with":
            return s.startswith(x)
        return s.endswith(x)
    if f == "substring":
        s, start = args[0], args[1]
        if not isinstance(s, str) or not _is_num(start):
            return None
        st = int(start)
        idx = st - 1 if st > 0 else len(s) + st
        if len(args) == 3:
            if not _is_num(args[2]):
                return None
            return s[idx: idx + int(args[2])]
        return s[idx:]
    if f == "abs":
        return abs(args[0]) if _is_num(args[0]) else None
    if f == "floor":
        return args[0].to_integral_value(rounding=decimal.ROUND_FLOOR) if _is_num(args[0]) else None
    if f == "ceiling":
        return args[0].to_integral_value(rounding=decimal.ROUND_CEILING) if _is_num(args[0]) else None
    if f == "round half up":
        n, scale = args
        if not (_is_num(n) and _is_num(scale)):
            return None
        return n.quantize(D(1).scaleb(-int(scale)), rounding=decimal.ROUND_HALF_UP)
    if f in ("min", "max"):
        if not args or not all(_is_num(a) for a in args):
            return None
        return min(args) if f == "min" else max(args)
    if f == "sum":
        if not all(_is_num(a) for a in args):
            return None
        total = D(0)
        for a in args:
            total += a
        return total
    if f == "count":
        return D(len(args[0])) if isinstance(args[0], list) else None
    if f == "string":
        v = args[0]
        if v is None:
            return None
        if isinstance(v, bool):
            return "true" if v else "false"
        if _is_num(v):
            return number_to_string(v)
        if isinstance(v, datetime.date):
            return v.isoformat()
        return str(v)
    if f == "number":
        v = args[0]
        if not isinstance(v, str):
            return None
        try:
            return D(v.strip())
        except decimal.InvalidOperation:
            return None
    if f == "date":
        v = args[0]
        if not isinstance(v, str):
            return None
        try:
            return datetime.date(int(v[0:4]), int(v[5:7]), int(v[8:10])) if len(v) == 10 else None
        except ValueError:
            return None
    return None


def test(unary, value, ctx=None):
    """Apply parsed unary tests to a value. Returns True, False or None."""
    value = to_feel(value)
    kind = unary[0]
    if kind == "any":
        return True
    results = [_positive(p, value, ctx or {}) for p in unary[1]]
    hit = any(r is True for r in results)
    if kind == "nottests":
        if hit:
            return False
        if any(r is None for r in results):
            return None
        return True
    return hit


def _positive(p, value, ctx):
    k = p[0]
    if k == "cmp":
        return compare(p[1], value, evaluate(p[2], ctx))
    if k == "range":
        return in_range(value, p, ctx)
    if k == "eq":
        target = evaluate(p[1], ctx)
        if isinstance(target, list) and not isinstance(value, list):
            return any(equal(value, x) is True for x in target)
        return equal(value, target)
    return None


def eval_text(text, ctx):
    """Parse and evaluate an expression in one call."""
    return evaluate(parse_expr(text), ctx)


def to_python(v):
    """FEEL value -> plain Python/JSON value (Decimal -> int or float, date -> ISO text)."""
    if isinstance(v, Decimal):
        return int(v) if v == v.to_integral_value() else float(v)
    if isinstance(v, datetime.date):
        return v.isoformat()
    if isinstance(v, list):
        return [to_python(x) for x in v]
    if isinstance(v, dict):
        return dict((k, to_python(x)) for k, x in v.items())
    return v
