"""Generate the Python-reference fixtures for tests/feel_parity.rs and tok-cli/tests/eval_parity.rs.

Usage (from any directory):
    PYTHONPATH=tok/python/src python tok/rust/tok-core/tests/gen_python_eval.py [EXTRA_TOK_DIR ...]

Inputs: every document with a decision table among tok/examples/*.tok, tok/conformance/cases/*.tok
(copied as conformance-NAME.tok), the course examples (../tok-course/przyklady next to the tok
repository, or EXTRA_TOK_DIR) and the synthetic tables below (eval-tables.tok).
Outputs in fixtures/eval/:
    NAME.tok      input copies
    cases.json    `tok eval` runs: arguments, environment, standard output, standard error and
                  exit code of the reference command line, for a grid of input values per
                  decision (boundaries of intervals, literals of the cells and their
                  neighbours, missing inputs, wrong types, special numbers) and for argument
                  errors. An exception the command line does not catch is recorded as the last
                  line of its traceback, exit code 1.
    feel.json     FEEL expressions and unary tests with their contexts and results: the value
                  (numbers as str(Decimal)), the plain value of to_python as JSON, or the
                  exception as the last traceback line.
"""

import datetime
import decimal
import glob
import io
import itertools
import json
import os
import random
import shutil
import sys
import traceback
from decimal import Decimal

HERE = os.path.dirname(os.path.abspath(__file__))
TOK = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(TOK, "python", "src"))

from tok import cli, feel  # noqa: E402
from tok.check import check  # noqa: E402
from tok.parser import parse  # noqa: E402

FIX = os.path.join(HERE, "fixtures", "eval")
COURSE = os.path.normpath(os.path.join(TOK, "..", "tok-course", "przyklady"))
MISSING = object()

SYNTHETIC = r'''tok 0.3 "Decision tables for evaluation tests"

decision rabat "Rabat" hit=unique
  | kwota: number | klient: string        || rabat: number    | opis: string                   |
  | < 0           | -                     || null             | "błąd"                         |
  | [0..100)      | -                     || 0                | "brak"                         |
  | [100..500]    | "stały", "vip"        || kwota * 0.05     | "rabat " + string(kwota * 0.05) |
  | [100..500]    | not("stały", "vip")   || 0                | "brak"                         |
  | > 500         | "vip"                 || kwota / 3        | "trzecia część"                |
  | >= 500        | -                     || round half up(kwota * 0.123, 2) | "duży"        |

decision limit "Limit" hit=first
  | wiek: number | konto: string          || limit: number | uwagi: string |
  | < 18         | -                      || 5             | -             |
  | [18..26]     | "student", "doktorant" || 15            | "ulga"        |
  | >= 18        | "pracownik"            || 30            | upper case(konto) |
  | -            | -                      || 10            | "pozostali"   |

decision ocena "Ocena" hit=any
  | punkty: number | zaliczenie: boolean || ocena: string |
  | >= 90          | true                || "bardzo dobry" |
  | >= 80          | true                || "dobry"     |
  | >= 85          | -                   || "bardzo dobry" |
  | < 50           | -                   || "niedostateczny" |
  | -              | false               || "niedostateczny" |

decision kanaly "Kanały" hit=collect
  | pilne: boolean | kwota: number || kanal: string | koszt: number |
  | true           | -             || "sms"         | 0.25          |
  | -              | > 1000        || "list"        | kwota / 1000  |
  | -              | -             || "e-mail"      | 0             |
  | false          | <= 0          || "brak"        | -             |

decision tagi "Tagi" hit=collect
  | rodzaj: string || tag: string |
  | "książka", "e-book" || "publikacja" |
  | "e-book"            || "cyfrowe" |
  | starts with(rodzaj, "e") || "e-" + rodzaj |
  | -                   || lower case(rodzaj) |

decision priorytet "Priorytet" hit=priority
  | dni: number | typ: string || priorytet: number |
  | <= 1        | -           || 1 |
  | (1..7]      | "skarga"    || 2 |
  | ]1..7]      | -           || 3 |
  | > 7         | -           || 4 |

decision wyrazenia "Wyrażenia" hit=first
  | x: number | y: number || suma: number | iloraz: number | tekst: string | lista: number | data: date |
  | null      | -         || y            | null           | string(y)     | count([x, y]) | date("2027-01-15") |
  | -         | null      || x + 1        | x / 7          | string(x)     | sum([x, 1, 2]) | date("2027-02-30") |
  | -         | -         || x + y        | x / y          | string(x / y) | max(x, y)     | date("2027-01-15") + duration("P1DT2H") |

decision funkcje "Funkcje" hit=first
  | kod: string || dlugosc: number | fragment: string | koniec: string | od_konca: string | liczba: number |
  | ""          || string length(kod) | substring(kod, 1, 0) | substring(kod, 1) | substring(kod, -1) | number("12.50") |
  | -           || string length(kod) | substring(kod, 2, 3) | substring(kod, string length(kod)) | substring(kod, -3, 2) | number(kod) |

decision liczby "Liczby" hit=first
  | n: number || dol: number | gora: number | bezwzgl: number | zaokr: number | tekst: string | min_max: number |
  | -         || floor(n)    | ceiling(n)   | abs(n)          | round half up(n, 1) | string(n) | min([n, 0, -1]) + max(n, 2) |

decision zakresy "Zakresy" hit=collect
  | v: number | dolna: number | gorna: number || nazwa: string |
  | [dolna..gorna] | -        | -             || "w przedziale" |
  | < dolna        | -        | -             || "poniżej" |
  | > gorna        | -        | -             || "powyżej" |
  | not([dolna..gorna]) | -   | -             || "poza" |
  | dolna, gorna  | -            | -             || "na granicy" |

decision wejscia "Wejścia" hit=first
  | x + y: number | string length(s): number | osoba.wiek: number || wynik: string |
  | > 10          | -                        | -                  || "duża suma"   |
  | -             | >= 3                     | -                  || "długi tekst" |
  | -             | -                        | null               || "inne"        |

decision teksty "Teksty" hit=first
  | s: string || porownanie: boolean | zawiera: boolean | konczy: boolean | duze: string |
  | > "m"     || s > "m"             | contains(s, "a") | ends with(s, "z") | upper case(s) |
  | -         || s < "m"             | contains(s, "") | ends with(s, "") | lower case(s) |

decision logika "Logika" hit=first
  | a: boolean | b: boolean || i: boolean | lub: boolean | nie: boolean | jesli: string |
  | -          | -          || a and b    | a or b       | not(a)       | if a then "tak" else "nie" |

decision brak "Brak reguł pasujących" hit=unique
  | kod: number || wynik: string |
  | 1           || "jeden" |
  | 2           || "dwa" |
  | 2           || "znowu dwa" |
'''

# numbers as the command line reads them: digits of other scripts, underscores, exponents,
# signed zero, more digits than the precision, integers longer than Python converts to text
# (4300 digits), NaN and infinities
EXOTIC_NUMBERS = ["NaN", "sNaN", "Infinity", "-Infinity", "1_000", " 7", "١٢", "1E+30",
                  "1e-30", "-0", "0.000", "12345678901234567890123456789.5", "1E+5000",
                  "0.1e1", "-1E-5000", "5."]
GENERIC = ["0", "7", "-2.5", "abc", "true", ""]
BOOLEANS = ["true", "false", "TRUE", "nie"]
GRID_LIMIT = 120


def write(path, text):
    with io.open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def read_text(path):
    with io.open(path, encoding="utf-8") as f:
        return f.read()


def last_traceback_line(ex):
    return traceback.format_exception_only(type(ex), ex)[-1].rstrip("\n")


def run_cli(args, env_lang=None):
    """Run the reference command line in this interpreter: (stdout, stderr, exit code)."""
    out, err = io.StringIO(), io.StringIO()
    saved = sys.stdout, sys.stderr
    if env_lang is None:
        os.environ.pop("TOK_LANG", None)
    else:
        os.environ["TOK_LANG"] = env_lang
    sys.stdout, sys.stderr = out, err
    try:
        code = cli.main(args)
    except SystemExit as ex:
        code = ex.code if isinstance(ex.code, int) else (0 if ex.code is None else 1)
    except Exception as ex:  # an exception the command line does not catch
        err.write(last_traceback_line(ex) + "\n")
        code = 1
    finally:
        sys.stdout, sys.stderr = saved
        os.environ.pop("TOK_LANG", None)
    return out.getvalue(), err.getvalue(), code


# ------------------------------------------------------------------ value grids

def walk(node, names, numbers, strings):
    """Collect variable names and literals of a parsed FEEL tree."""
    if isinstance(node, tuple):
        if node and node[0] == "name":
            names.append(node[1])
            return
        if node and node[0] == "num":
            numbers.append(node[1])
            return
        if node and node[0] == "str":
            strings.append(node[1])
            return
        if node and node[0] == "neg" and node[1][0] == "num":
            numbers.append(-node[1][1])
            return
        for x in node[1:]:
            walk(x, names, numbers, strings)
    elif isinstance(node, list):
        for x in node:
            walk(x, names, numbers, strings)


def unique(seq):
    out = []
    for x in seq:
        if x not in out:
            out.append(x)
    return out


def number_text(d):
    return format(d.normalize(), "f") if d != d.to_integral_value() else str(d.quantize(Decimal(1)))


def candidates(decision):
    """Candidate command-line values (MISSING = argument left out) for every variable."""
    column = {}
    for k, (expr, typ) in enumerate(decision.inputs):
        names, numbers, strings = [], [], []
        for rule in decision.rules:
            if k < len(rule.inputs):
                walk(feel.parse_unary_tests(rule.inputs[k]), names, numbers, strings)
        try:
            node = feel.parse_expr(expr)
        except feel.FeelError:
            continue
        if node[0] == "name":
            column[node[1]] = (typ, numbers, strings)
    names = []
    for expr, _ in decision.inputs:
        try:
            walk(feel.parse_expr(expr), names, [], [])
        except feel.FeelError:
            pass
    for rule in decision.rules:
        for cell in rule.inputs:
            try:
                walk(feel.parse_unary_tests(cell), names, [], [])
            except feel.FeelError:
                pass
        for cell in rule.outputs:
            if cell.strip() != "-":
                try:
                    walk(feel.parse_expr(cell), names, [], [])
                except feel.FeelError:
                    pass
    result = {}
    for name in unique(names):
        typ, numbers, strings = column.get(name, (None, [], []))
        if typ == "number":
            values = []
            for n in unique(numbers):
                for d in (n, n - 1, n + 1, n - Decimal("0.5"), n + Decimal("0.5")):
                    values.append(number_text(d))
            values += ["0", "-1", "1.5", "abc", ""]
        elif typ == "string":
            values = unique(strings) + ["zzz", "", "42"]
        elif typ == "boolean":
            values = list(BOOLEANS)
        elif typ == "date":
            values = ["2027-01-15", "x"]
        else:
            values = list(GENERIC)
        result[name] = unique(values) + [MISSING]
    return result


def grid(decision, exotic):
    """Lists of NAME=VALUE arguments covering the candidates of a decision."""
    cands = candidates(decision)
    names = list(cands)
    rng = random.Random(decision.id)
    combos = []
    if not names:
        combos.append({})
    else:
        total = 1
        for n in names:
            total *= len(cands[n])
        if total <= GRID_LIMIT:
            for values in itertools.product(*(cands[n] for n in names)):
                combos.append(dict(zip(names, values)))
        else:
            # every candidate once with the others at their first value, then random combinations
            for n in names:
                for v in cands[n]:
                    combo = {m: cands[m][0] for m in names}
                    combo[n] = v
                    combos.append(combo)
            while len(combos) < GRID_LIMIT:
                combos.append({n: rng.choice(cands[n]) for n in names})
        if exotic:
            for n in names:
                for v in EXOTIC_NUMBERS:
                    combo = {m: cands[m][0] for m in names}
                    combo[n] = v
                    combos.append(combo)
    out = []
    for combo in combos:
        args = ["%s=%s" % (n, v) for n, v in combo.items() if v is not MISSING]
        if args not in out:
            out.append(args)
    return out


# ------------------------------------------------------------------- FEEL corpus

def enc(v):
    """A FEEL value as JSON: numbers as str(Decimal), strings as JSON strings."""
    if v is None or isinstance(v, (bool, str)):
        return v
    if isinstance(v, Decimal):
        return {"n": str(v)}
    if isinstance(v, datetime.datetime):
        return {"dt": v.isoformat()}
    if isinstance(v, datetime.date):
        return {"d": v.isoformat()}
    if isinstance(v, datetime.timedelta):
        return {"td": [v.days, v.seconds, v.microseconds]}
    if isinstance(v, list):
        return [enc(x) for x in v]
    if isinstance(v, dict):
        return {"c": [[k, enc(x)] for k, x in v.items()]}
    raise TypeError(type(v))


CONTEXT = {
    "n_nan": Decimal("NaN"), "n_snan": Decimal("sNaN"), "n_inf": Decimal("Infinity"),
    "n_ninf": Decimal("-Infinity"), "n_big": Decimal("1E+999999"), "n_tiny": Decimal("1E-999999"),
    "n_negzero": Decimal("-0"), "n_long": Decimal("1.234567890123456789012345678901234567890"),
    "n_e3": Decimal("1E+3"), "n_frac": Decimal("2.50"), "n_int": Decimal("42"),
    "n_huge": Decimal("1E+40"), "n_payload": Decimal("NaN123"), "n_e5000": Decimal("1E+5000"),
    "n_small": Decimal("-1.50E-50"),
    "s": "Zażółć gęślą jaźń", "s_empty": "", "s_quote": "it's \"x\"\n",
    "b": True, "nul": None,
    "lst": [Decimal("1"), Decimal("2.5"), Decimal("-3")], "lst_s": ["a", "b"], "lst_empty": [],
    "lst_mixed": [Decimal("1"), "a", None, True],
    "d1": datetime.date(2027, 1, 15), "d2": datetime.date(2024, 2, 29),
    "dmax": datetime.date(9999, 12, 31),
    "td1": datetime.timedelta(days=1, hours=2), "tdneg": datetime.timedelta(hours=-1),
    "osoba": {"wiek": Decimal("30"), "imie": "Anna", "adres": {"miasto": "Kraków"}},
}

OPERANDS = ["0", "1", "-1", "0.1", "0.2", "1.50", "3", "7", "-0.5", "100",
            "12345678901234567890.123456789", "0.000001", "n_nan", "n_snan", "n_inf",
            "n_ninf", "n_negzero", "n_long", "n_e3", "n_huge",
            "\"a\"", "\"b\"", "\"\"", "true", "false", "null", "[1, 2]", "[]", "d1", "d2",
            "td1", "tdneg", "osoba", "nieznana"]
BINARY = ["+", "-", "*", "/", "=", "!=", "<", "<=", ">", ">="]
UNARY_FUNCS = ["string length", "upper case", "lower case", "abs", "floor", "ceiling", "count",
               "string", "number", "date", "duration", "not", "-"]
FUNC_ARGS = ["0", "1.5", "-2.5", "-0.5", "2.675", "n_nan", "n_snan", "n_inf", "n_negzero", "n_long",
             "n_e3", "n_huge", "n_e5000", "n_small", "\"12.50\"", "\" 7 \"", "\"1e3\"", "\"x\"", "\"\"",
             "\"NaN\"", "\"2027-01-15\"", "\"2027-02-30\"", "\" 202-1-01\"", "\"+202-01-01\"",
             "\"P1DT2H30M\"", "\"PT1.5S\"", "\"-P1D\"", "\"P\"", "\"PT\"", "\"P1DT\"",
             "\"PT0.0000005S\"", "\"P999999999D\"", "\"P1000000000D\"", "\"PT1M2H\"",
             "s", "s_quote", "true", "null", "lst", "lst_s", "lst_mixed", "lst_empty", "d1", "td1",
             "osoba", "[1, [2, \"x\"], d1, td1, null, true]"]
EXPRESSIONS = [
    "0.1 + 0.2", "1 / 3", "2 / 3", "1 / 3 * 3", "10 / 4", "100 / 10", "1.0 / 2", "12 / 0.4",
    "1.20 * 2", "1E2", "1 / 0", "0 / 0", "-0", "-(0)", "0 * -1", "-0.0 + 0", "n_negzero + n_negzero",
    "n_negzero - 0", "string(0 * -1)", "string(n_negzero)", "string(n_long)", "string(1 / 3)",
    "string(n_e3)", "string(100)", "string(1.000)", "string(n_huge)", "string(n_big)",
    "string(n_small)", "string(n_nan)", "string(n_payload)", "string(-n_nan)", "n_long + 0",
    "n_long * 1", "-n_long", "abs(n_long)", "n_long - n_long", "n_huge + 1", "n_huge * n_huge",
    "n_big * 10", "n_big - n_big", "n_e5000", "n_e5000 + 1", "-n_e5000", "n_tiny / 1E+30",
    "n_tiny * n_tiny", "n_tiny / 3",
    "123456789012345678901234567890 + 1", "0.1 * 0.1 * 0.1", "7 / 7", "2.50 * 4",
    "round half up(2.345, 2)", "round half up(2.5, 0)", "round half up(-2.5, 0)",
    "round half up(1.005, 2)", "round half up(123.456, -1)", "round half up(1, 30)",
    "round half up(1, 2.7)", "round half up(n_inf, 2)", "round half up(n_nan, 2)",
    "round half up(1, n_nan)", "round half up(1, n_inf)", "round half up(1, 99999999)",
    "round half up(n_huge, 0)", "floor(-0.5)", "ceiling(-0.5)", "floor(n_huge)", "ceiling(n_e3)",
    "substring(\"abcdef\", 2)", "substring(\"abcdef\", 0)", "substring(\"abcdef\", -2)",
    "substring(\"abcdef\", -10)", "substring(\"abcdef\", -10, 9)", "substring(\"abcdef\", 2, -1)",
    "substring(\"abcdef\", 1.9, 2.9)", "substring(\"abcdef\", n_nan)", "substring(\"abcdef\", n_inf)",
    "substring(\"abcdef\", 2, n_inf)", "substring(\"abcdef\", n_huge)", "substring(s, 3, 4)",
    "substring(\"abc\", 1, \"x\")", "min(3, 1, 2)", "max(3, 1, 2)", "min([3, 1, 2])", "min([])",
    "max([])", "sum([])", "sum(1, 2, 3)", "sum([0.1, 0.2])", "sum(n_inf, n_ninf)",
    "sum(n_e5000, n_e5000)", "min(1, n_nan)", "max(n_nan)", "min(1, \"a\")", "min(n_negzero, 0)",
    "max(0, n_negzero)", "min([1, 1.0])", "max(1.0, 1)", "count([1, 2, null])", "count(\"ab\")",
    "string(lst)", "string(lst_s)", "string(lst_mixed)", "string(osoba)", "string(s_quote)",
    "string([\"it's\"])", "string([\"a\\\"b\"])", "string([d1, td1, tdneg])", "string(td1)",
    "string(tdneg)", "string(duration(\"P2D\"))", "string(date(\"2027-01-15\"))",
    "number(\" 12.50 \")", "number(\"1_000\")", "number(\"abc\")", "number(\"sNaN\")",
    "number(\"-Infinity\")", "number(\"1e999999999999999999999\")",
    "date(\"2027-01-15\") + duration(\"P1D\")", "date(\"2027-01-15\") - duration(\"PT1H\")",
    "date(\"2027-01-15\") + duration(\"-PT1H\")", "duration(\"P1D\") + date(\"2027-01-15\")",
    "duration(\"P1D\") - duration(\"PT1S\")", "dmax + duration(\"P1D\")",
    "date(\"0001-01-01\") - duration(\"P1D\")", "duration(\"P999999999D\") + duration(\"P1D\")",
    "-duration(\"P1D\")", "d1 - d2", "d1 > d2", "d1 = date(\"2027-01-15\")", "d1 in [d2..d1]",
    "td1 = td1", "td1 < td1", "\"a\" + \"b\"", "\"a\" + 1", "\"a\" < \"b\"", "\"B\" < \"a\"",
    "\"ä\" > \"z\"", "upper case(\"straße\")", "lower case(\"İSTANBUL\")", "lower case(\"ΟΔΟΣ\")",
    "string length(\"zażółć\")", "contains(s, \"gęślą\")", "starts with(s, \"\")",
    "ends with(\"abc\", \"bc\")", "osoba.wiek", "osoba.adres.miasto", "osoba.brak",
    "osoba.wiek.x", "nieznana.pole", "[1, 2] = [1, 2]", "[1, 2] = [1, 2, 3]", "[1, \"a\"] = [1, 2]",
    "[1, 2] < [1, 3]", "[] = []", "[null] = [null]", "1 in [1, 2]", "3 in [1, 2]", "null in [1, null]",
    "1 in 1", "\"a\" in [\"a\", 1]", "5 in [1..10]", "10 in [1..10)", "10 in ]1..10]",
    "1 in (1..10)", "1 in ]1..10[", "\"b\" in [\"a\"..\"c\"]", "5 in [x..10]", "n_nan in [1, 2]",
    "n_nan in [1..2]", "n_snan in [1, 2]", "n_snan = 1", "n_nan = n_nan", "n_nan != 1",
    "n_inf > n_huge", "n_ninf < n_big", "n_inf = n_inf", "5 between 1 and 10",
    "5 between 10 and 1", "null between 1 and 2", "\"b\" between \"a\" and \"c\"",
    "n_nan between 1 and 2", "true and null", "false and null", "null or true", "null or false",
    "1 and true", "not(null)", "not(1)", "not(false)", "if null then 1 else 2",
    "if 1 then 1 else 2", "if true then n_snan else 1", "if false then 1 else -n_snan",
    "-n_snan", "abs(n_snan)", "-n_inf", "-n_nan", "abs(n_ninf)", "n_inf - n_inf", "n_inf * 0",
    "n_inf / n_inf", "n_inf / 2", "1 / n_inf", "n_snan + 1", "n_snan / 1", "1 / n_snan",
    "n_nan + 1", "n_payload * 2", "true = 1", "true < false", "null = null", "null != null",
    "null < 1", "1 = null", "x ** 2", "{a: 1}", "for x in [1] return x", "some x in [1] satisfies x",
    "f(1)", "1 +", "\"open", "a[1]", "@\"2027-01-01\"", "1 in [1..2] in true", "", "  ",
]
UNARY_TESTS = [
    ("-", [None, Decimal("5"), "x"]),
    ("< 18", [Decimal("17"), Decimal("18"), Decimal("17.999"), None, "a", True, Decimal("NaN")]),
    ("<= 150", [Decimal("150"), Decimal("150.0001"), Decimal("-0")]),
    (">= 18", [Decimal("18"), Decimal("1E+1")]),
    ("[18..65]", [Decimal("18"), Decimal("65"), Decimal("65.5"), None, "x"]),
    ("]18..65[", [Decimal("18"), Decimal("18.01"), Decimal("65")]),
    ("(0..10)", [Decimal("0"), Decimal("10"), Decimal("5")]),
    ("[1..5[", [Decimal("5"), Decimal("1")]),
    ("\"student\", \"doktorant\"", ["student", "Student", None, Decimal("1")]),
    ("not(\"student\", \"doktorant\")", ["student", "x", None, Decimal("1")]),
    ("not(< 0)", [Decimal("-1"), Decimal("1"), None]),
    ("not([1..5], 7)", [Decimal("3"), Decimal("7"), Decimal("8")]),
    ("150", [Decimal("150"), Decimal("150.00"), "150"]),
    ("true", [True, False, None, "true"]),
    ("null", [None, Decimal("0")]),
    ("= null", [None, Decimal("0")]),
    ("!= 5", [Decimal("5"), Decimal("6"), None]),
    ("< limit", [Decimal("1"), Decimal("100")]),
    ("[1, 2]", [Decimal("1"), Decimal("3"), [Decimal("1"), Decimal("2")]]),
    ("lst", [Decimal("2.5"), Decimal("4")]),
    ("d1", [datetime.date(2027, 1, 15), datetime.date(2027, 1, 16)]),
    ("[d2..d1]", [datetime.date(2025, 1, 1), datetime.date(2028, 1, 1)]),
    ("> \"m\"", ["n", "a", None]),
    ("< 5, > 10", [Decimal("3"), Decimal("7"), Decimal("11")]),
    ("n_nan", [Decimal("1")]),
    ("< n_snan", [Decimal("1")]),
    ("1, n_snan", [Decimal("1"), Decimal("2")]),
    ("[1, n_snan]", [Decimal("1"), Decimal("2")]),
    ("x ** 2", [Decimal("1")]),
    ("", [Decimal("1")]),
]


def feel_corpus():
    exprs = list(EXPRESSIONS)
    for a in OPERANDS:
        for op in BINARY:
            for b in OPERANDS:
                exprs.append("%s %s %s" % (a, op, b))
    for f in UNARY_FUNCS:
        for a in FUNC_ARGS:
            exprs.append("-%s" % a if f == "-" else "%s(%s)" % (f, a))
    for f in ("contains", "starts with", "ends with", "round half up", "min", "max", "sum"):
        for a in FUNC_ARGS[:16]:
            for b in ("\"\"", "\"a\"", "1", "2", "-1", "n_nan", "s"):
                exprs.append("%s(%s, %s)" % (f, a, b))
    ctx_json = {k: enc(v) for k, v in CONTEXT.items()}
    items = []
    for text in unique(exprs):
        item = {"expr": text}
        try:
            v = feel.eval_text(text, CONTEXT)
            item["value"] = enc(v)
            try:
                item["plain"] = json.dumps(feel.to_python(v), ensure_ascii=False, default=str)
            except Exception as ex:
                item["plain_error"] = last_traceback_line(ex)
        except Exception as ex:
            item["error"] = last_traceback_line(ex)
        items.append(item)
    tests = []
    for cell, values in UNARY_TESTS:
        for value in values:
            item = {"cell": cell, "input": enc(value)}
            try:
                item["result"] = feel.test(feel.parse_unary_tests(cell), value, dict(CONTEXT, limit=Decimal(50)))
            except Exception as ex:
                item["error"] = last_traceback_line(ex)
            tests.append(item)
    ctx_json["limit"] = enc(Decimal(50))
    return {"context": [[k, v] for k, v in ctx_json.items()], "expressions": items, "unary": tests}


# ------------------------------------------------------------------------- main

def documents(extra_dirs):
    """(source path, fixture name) of every document with a decision table."""
    found = []
    for p in sorted(glob.glob(os.path.join(TOK, "examples", "*.tok"))):
        found.append((p, os.path.basename(p)))
    for p in sorted(glob.glob(os.path.join(TOK, "conformance", "cases", "*.tok"))):
        found.append((p, "conformance-" + os.path.basename(p)))
    dirs = list(extra_dirs) or ([COURSE] if os.path.isdir(COURSE) else [])
    for d in dirs:
        for p in sorted(glob.glob(os.path.join(d, "*.tok"))):
            found.append((p, os.path.basename(p)))
    out = []
    for p, name in found:
        if any(line.startswith("decision") for line in read_text(p).splitlines()):
            out.append((p, name))
    return out


def main():
    os.makedirs(FIX, exist_ok=True)
    # the Rust command line prints the usage line of the main parser unwrapped, as argparse
    # does on a terminal wider than that line
    os.environ["COLUMNS"] = "200"
    for f in glob.glob(os.path.join(FIX, "*")):
        os.remove(f)
    docs = documents(sys.argv[1:])
    for p, name in docs:
        shutil.copyfile(p, os.path.join(FIX, name))
    write(os.path.join(FIX, "eval-tables.tok"), SYNTHETIC)
    names = [name for _, name in docs] + ["eval-tables.tok"]
    cwd = os.getcwd()
    os.chdir(FIX)
    cases = []
    try:
        def add(args, env_lang=None):
            out, err, code = run_cli(args, env_lang)
            case = {"args": args, "stdout": out, "stderr": err, "code": code}
            if env_lang:
                case["env_lang"] = env_lang
            cases.append(case)

        for name in names:
            doc, diags = parse(read_text(name))
            if not any(d.severity == "error" for d in diags):
                diags += check(doc, bpmn=False)
            decisions = list(doc.decisions.values())
            if any(d.severity == "error" for d in diags):
                first = decisions[0].id if decisions else "d"
                add(["eval", name, first])
                add(["eval", name, first, "--lang", "pl"])
                add(["eval", name, first], env_lang="pl")
                continue
            for dec in decisions:
                for k, values in enumerate(grid(dec, exotic=name == "eval-tables.tok")):
                    args = ["eval", name, dec.id] + values
                    if k % 7 == 3:
                        args = ["eval", "--lang", "pl", name, dec.id] + values
                    add(args)
        f, d = "eval-tables.tok", "limit"
        for args in ([], ["-h"], ["--help"], [f], [f, "nieznana"], [f, d, "wiek"], [f, d, "--json"],
                     [f, d, "wiek=20", "--lang", "pl", "konto=student"],
                     [f, d, "--lang", "pl", "wiek=20"], ["--lang", "pl", f, d, "wiek=20"],
                     [f, "--lang", "pl", d, "wiek=20", "konto=student"],
                     [f, d, "--", "-wiek=20"], [f, d, "-5"], [f, d, "-.5"], [f, d, "-5e3"],
                     [f, d, "--lang", "de"], [f, d, "--lang"], [f, d, "--lang="], [f, d, "--la=pl"],
                     [f, d, "--l", "pl"], [f, d, "-hx"], [f, d, "-hh"], [f, d, "-h=x"], [f, d, "-h-"],
                     [f, d, "--help=x"], [f, d, "--help="], [f, d, "--h"], [f, d, "--=x"], [f, d, "-l"],
                     [f, d, "-"], [f, d, "a b"], [f, d, "=5"], [f, d, "wiek=20", "wiek=10"],
                     [f, d, "wiek=b=c"], [f, d, "WIEK=20"], [f, d, "wiek=20", "x", "y"],
                     [f, d, "-x=1", "wiek=3"], ["--", f, d, "wiek=3"], [f, "--", d, "wiek=3"],
                     [f, d, "wiek=3", "--", "konto=student"], [f, d, "--lang", "pl", "--lang", "en"]):
            add(["eval"] + args)
    finally:
        os.chdir(cwd)
    write(os.path.join(FIX, "cases.json"), json.dumps(cases, ensure_ascii=False, indent=1) + "\n")
    corpus = feel_corpus()
    write(os.path.join(FIX, "feel.json"), json.dumps(corpus, ensure_ascii=False, indent=1) + "\n")
    print("documents: %d, eval cases: %d, expressions: %d, unary tests: %d"
          % (len(names), len(cases), len(corpus["expressions"]), len(corpus["unary"])))


if __name__ == "__main__":
    main()
