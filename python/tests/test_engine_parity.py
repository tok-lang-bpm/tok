"""Parity of the Rust engine (module ``tok._engine``) with the pure-Python implementation.

Compared: diagnostics, canonical text, BPMN, SVG and exported files of every document; BPMN
import; decision table evaluation over a grid of typed input values; FEEL evaluation of the
expression corpus of the Rust fixtures (``rust/tok-core/tests/fixtures/eval/feel.json``); simulation
of every document and of the synthetic models of ``rust/tok-sim/tests/fixtures/sim``.

Also compared: the output and exit status of the ``tok`` command on the engine and with
``TOK_ENGINE=python``, and the functions of ``tok.api`` on both.

From 0.4 the automatic layout is done by the layout engine in Rust, which routes flows and
places labels differently from the frozen Python implementation (``bpmn.py``). So BPMN files
are compared without their diagram (``BPMNDiagram``: shapes, waypoints, label bounds), and
drawings by their structure (the same drawn elements, ``svg_structure``): the semantic BPMN
must stay identical. Decision tables and forms are compared exactly. The layout itself is
tested in Rust (``rust/tok-bpmn/tests/golden.rs`` and ``layout_quality.rs``).

The pure-Python implementation stays at language 0.3: a BPMN model that the engine imports
as language 0.5 (its text starts with ``tok 0.5``) is not compared; the Rust test
``rust/tok-core/tests/miwg.rs`` checks the semantic round trip of the whole import corpus.

The tests are skipped when the engine is not installed (a wheel of ``tok-lang``); see
``rust/tok-py/README.md``.
The pure-Python reference runs in a fresh interpreter for every input, so that the global
namespace registry of ElementTree does not carry over from one model to the next.
"""

import glob
import json
import os
import re
import subprocess
import sys
import unittest

try:
    from tok import _engine as tok_engine
except ImportError:  # pragma: no cover - depends on the environment
    tok_engine = None

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.normpath(os.path.join(HERE, "..", "src"))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
# examples of the companion course, when its repository is checked out next to this one
COURSE_EXAMPLES = os.path.normpath(os.path.join(REPO, "..", "tok-course", "przyklady"))


def language_version(path):
    """The language version in the header of a tok document (None without a header)."""
    with open(path, encoding="utf-8-sig") as f:
        for line in f:
            words = line.split("#", 1)[0].split()
            if words:
                return words[1] if words[0] == "tok" and len(words) > 1 else None
    return None


# The pure-Python implementation reads language 0.3; documents of newer language versions
# exist only for the engine and are checked by the Rust tests.
DOCUMENTS = sorted(
    path
    for path in glob.glob(os.path.join(REPO, "examples", "*.tok"))
    + glob.glob(os.path.join(REPO, "conformance", "cases", "*.tok"))
    + glob.glob(os.path.join(COURSE_EXAMPLES, "*.tok"))
    if language_version(path) in ("0.1", "0.3", None)
)
BPMN_FILES = sorted(glob.glob(os.path.join(REPO, "conformance", "import", "*.bpmn")))

REFERENCE_DOC = r'''
import json, os, sys, tempfile
import xml.etree.ElementTree as ET
from tok import bpmnlib, dmn, forms
from tok.bpmn_export import export_tree, semantic_xml
from tok.check import check
from tok.diagnostics import sort_key
from tok.io import write_bpmn
from tok.parser import parse
from tok.printer import fmt

with open(sys.argv[1], encoding="utf-8") as f:
    text = f.read()
out = {}

doc, diags = parse(text)
if not any(d.severity == "error" for d in diags):
    diags += check(doc, level="full", bpmn=True)
diags.sort(key=sort_key)
out["check_en"] = [d.as_dict("en") for d in diags]
out["check_pl"] = [d.as_dict("pl") for d in diags]
ok = not any(d.severity == "error" for d in diags)

fresh, pdiags = parse(text)
parse_ok = not any(d.severity == "error" for d in pdiags)
out["fmt"] = fmt(fresh) if parse_ok else None
out["semantic_xml"] = semantic_xml(parse(text)[0]) if parse_ok else None


def svg_text(title):
    tree = export_tree(doc)
    for prefix, uri in bpmnlib.NS.items():
        ET.register_namespace(prefix, uri)
    m = bpmnlib.Model(tree)
    bpmnlib.layout(m)
    return bpmnlib.render_svg(m, title=title)


def bpmn_text():
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "model.bpmn")
        write_bpmn(doc, path)
        with open(path, encoding="utf-8") as f:
            return f.read()


if ok:
    out["bpmn"] = bpmn_text()
    # a model without shapes (e.g. only a header) has no picture: bpmnlib raises LoadError,
    # the engine raises ValueError; both are recorded as None
    try:
        out["svg"] = svg_text(None)
        files = {"model.bpmn": bpmn_text(), "model.svg": svg_text("model.svg")}
        for dec in doc.decisions.values():
            files[dec.id + ".dmn"] = dmn.export_dmn(dec, doc)
        for form in doc.forms.values():
            files[form.id + ".form"] = forms.export_form(form)
        out["export"] = files
    except bpmnlib.LoadError:
        out["svg"] = out["export"] = None
else:
    out["bpmn"] = out["svg"] = out["export"] = None
sys.stdout.buffer.write(json.dumps(out, ensure_ascii=False).encode("utf-8"))
'''

REFERENCE_IMPORT = r'''
import json, sys
from tok.bpmn_import import import_bpmn
from tok.printer import fmt

with open(sys.argv[1], "rb") as f:
    xml = f.read().decode("utf-8", "replace")
out = {}
for keep in (True, False):
    doc, warnings = import_bpmn(xml, keep_ids=keep)
    out[str(keep)] = [fmt(doc), list(warnings)]
sys.stdout.buffer.write(json.dumps(out, ensure_ascii=False).encode("utf-8"))
'''


REFERENCE_SIM = r'''
import json, sys
from tok import sim
from tok.check import check
from tok.parser import parse

with open(sys.argv[1], encoding="utf-8") as f:
    doc, diags = parse(f.read())
runs = json.loads(sys.stdin.buffer.read().decode("utf-8"))
out = []
for cases, seed in runs:
    try:
        r = sim.simulate(doc, cases=cases, seed=seed)
        kpi = r.kpi()
        res = {"kpi": kpi, "log": r.log, "cases": r.cases, "warnings": r.warnings,
               "seed": r.seed, "horizon": r.horizon, "json": r.to_json(include_log=True),
               "text_en": sim.format_kpi(kpi, "en"), "text_pl": sim.format_kpi(kpi, "pl")}
        out.append(["ok", res])
    except Exception as ex:
        out.append(["error", type(ex).__name__, str(ex)])
sys.stdout.buffer.write(json.dumps(out, ensure_ascii=False, default=str).encode("utf-8"))
'''

# synthetic simulation models of the Rust fixtures (every distribution, event, gateway and
# activity kind the simulator handles, and invalid parameters)
SIM_MODELS = sorted(glob.glob(os.path.join(REPO, "rust", "tok-sim", "tests", "fixtures", "sim",
                                           "sim-*.tok")))
# (cases, seed) of the simulation runs of a document with a simulate statement; documents
# without one run with three cases
SIM_RUNS = [[None, None], [5, 3], [12, -7], ["4", "11"], [2.9, 1e3]]


def engine_simulation(text, cases, seed):
    """The outcome of tok_engine.simulate in the form of REFERENCE_SIM."""
    try:
        r = tok_engine.simulate(text, cases, seed)
        kpi = r.kpi()
        res = {"kpi": kpi, "log": r.log, "cases": r.cases, "warnings": r.warnings,
               "seed": r.seed, "horizon": r.horizon, "json": r.to_json(include_log=True),
               "text_en": tok_engine.format_kpi(kpi, "en"), "text_pl": r.format_kpi("pl")}
        out = ["ok", res]
    except Exception as ex:
        out = ["error", type(ex).__name__, str(ex)]
    return json.loads(json.dumps(out, ensure_ascii=False, default=str))


EVAL_FIXTURES = os.path.join(REPO, "rust", "tok-core", "tests", "fixtures", "eval")
FEEL_CORPUS = os.path.join(EVAL_FIXTURES, "feel.json")
# synthetic decision tables (every hit policy, expressions in cells) of the Rust fixtures
EVAL_TABLES = os.path.join(EVAL_FIXTURES, "eval-tables.tok")

# shared by the reference script and the test: typed values in JSON, results as (repr) or error
VALUES_CODE = r'''
import datetime
from decimal import Decimal


def decode(v):
    if isinstance(v, dict):
        if "n" in v:
            return Decimal(v["n"])
        if "i" in v:
            return int(v["i"])
        if "f" in v:
            return float(v["f"])
        if "d" in v:
            return datetime.date.fromisoformat(v["d"])
        if "td" in v:
            return datetime.timedelta(days=v["td"][0], seconds=v["td"][1], microseconds=v["td"][2])
        if "c" in v:
            return dict((k, decode(x)) for k, x in v["c"])
    if isinstance(v, list):
        return [decode(x) for x in v]
    return v


def outcome(fn, *args):
    try:
        return ["ok", repr(fn(*args))]
    except Exception as ex:
        return ["error", type(ex).__name__, str(ex)]
'''

REFERENCE_EVAL = VALUES_CODE + r'''
import json, sys
from tok import dmn
from tok.parser import parse

with open(sys.argv[1], encoding="utf-8") as f:
    doc, _ = parse(f.read())
runs = json.loads(sys.stdin.buffer.read().decode("utf-8"))
out = [outcome(dmn.evaluate_decision, doc.decisions[d], decode(values)) for d, values in runs]
sys.stdout.buffer.write(json.dumps(out, ensure_ascii=False).encode("utf-8"))
'''

REFERENCE_FEEL = VALUES_CODE + r'''
import json, sys
from tok import feel

data = json.loads(sys.stdin.buffer.read().decode("utf-8"))
ctx = dict((k, decode(v)) for k, v in data["context"])
out = [outcome(feel.eval_text, e, ctx) for e in data["expressions"]]
sys.stdout.buffer.write(json.dumps(out, ensure_ascii=False).encode("utf-8"))
'''

_values = {}
exec(VALUES_CODE, _values)
decode_value, outcome = _values["decode"], _values["outcome"]


def reference(script, path, stdin=None):
    env = dict(os.environ, PYTHONPATH=SRC, PYTHONIOENCODING="utf-8")
    args = [sys.executable, "-c", script] + ([path] if path else [])
    data = None if stdin is None else json.dumps(stdin, ensure_ascii=False).encode("utf-8")
    res = subprocess.run(args, env=env, input=data, capture_output=True, check=True)
    return json.loads(res.stdout.decode("utf-8"))


def decision_grid(decision):
    """Typed input values for a decision: literals of the cells and their neighbours as
    Decimal, int and float, other strings and booleans, wrong types and missing inputs."""
    sys.path.insert(0, SRC)
    from tok import feel

    def walk(node, names, lits):
        if isinstance(node, tuple) and node:
            if node[0] == "name":
                names.append(node[1])
            elif node[0] in ("num", "str", "bool"):
                lits.append(node[1])
            else:
                for x in node[1:]:
                    walk(x, names, lits)
        elif isinstance(node, list):
            for x in node:
                walk(x, names, lits)

    names, lits = [], []
    for expr, _ in decision.inputs:
        walk(feel.parse_expr(expr), names, [])
    for rule in decision.rules:
        for cell in rule.inputs:
            walk(feel.parse_unary_tests(cell), names, lits)
        for cell in rule.outputs:
            if cell.strip() != "-":
                walk(feel.parse_expr(cell), names, [])
    values = [None, True, "", "x", {"i": 0}, {"f": 2.5}, {"n": "-1"}]
    for lit in lits:
        if isinstance(lit, bool) or isinstance(lit, str):
            values.append(lit)
        else:
            for d in (lit, lit - 1, lit + 1, lit + lit.__class__("0.5")):
                values.append({"n": str(d)})
            values.append({"i": int(lit)})
            values.append({"f": float(lit)})
    unique_values = []
    for v in values:
        if v not in unique_values:
            unique_values.append(v)
    names = list(dict.fromkeys(names))
    runs = []
    for k in range(max(len(unique_values), 1)):
        # each value once for every variable, the others shifted through the list
        combo = []
        for j, n in enumerate(names):
            v = unique_values[(k + 3 * j) % len(unique_values)]
            if v is not None or (k + j) % 2:
                combo.append([n, v])
        runs.append((decision.id, {"c": combo}))
    return runs


def run_tok(args, engine):
    """Output of ``python -m tok ARGS`` with the installed package: on the engine, or on the
    pure-Python implementation."""
    env = dict(os.environ, PYTHONIOENCODING="utf-8", COLUMNS="200")
    env.pop("PYTHONPATH", None)
    if engine:
        env.pop("TOK_ENGINE", None)
    else:
        env["TOK_ENGINE"] = "python"
    res = subprocess.run([sys.executable, "-m", "tok"] + args, env=env, capture_output=True, cwd=REPO)
    return (res.returncode, res.stdout.replace(b"\r\n", b"\n"),
            res.stderr.replace(b"\r\n", b"\n"))


def engine_additions_removed(result):
    """Output of the engine without what the frozen pure-Python command does not print: the
    BPMN conformance class of ``tok check --json`` and the ``--class`` option of ``tok check``
    (language 0.6)."""
    code, out, err = result
    def strip(b):
        b = re.sub(rb'\n  "bpmn_class": "[a-z-]+",', b"", b)
        return b.replace(b" [--class {descriptive,analytic,common-executable,full}]", b"")
    return code, strip(out), strip(err)


def written(args):
    """Structure of the drawing written by ``tok svg ... -o FILE`` (removed after reading)."""
    if args[0] != "svg" or not os.path.exists(args[-1]):
        return None
    with open(args[-1], encoding="utf-8") as f:
        svg = f.read()
    os.remove(args[-1])
    return svg_structure(svg)


def engine_or_none(fn, *args, **kw):
    try:
        return fn(*args, **kw)
    except ValueError:
        return None


def without_diagram(xml):
    """BPMN XML without its ``BPMNDiagram`` element (the layout)."""
    if xml is None:
        return None
    return re.sub(r"\s*<(\w+):BPMNDiagram\b.*?</\1:BPMNDiagram>", "", xml, flags=re.S)


def svg_structure(svg):
    """The drawn elements of an SVG (tag names and how often they occur; lines of wrapped
    text not counted), which do not depend on the layout."""
    if svg is None:
        return None
    tags = re.findall(r"<([A-Za-z]+)", svg)
    return sorted((t, tags.count(t)) for t in set(tags) if t != "tspan")


def comparable_files(files):
    """Exported files with BPMN compared without the diagram and SVG by structure."""
    if files is None:
        return None
    out = {}
    for name, text in files.items():
        if name.endswith(".bpmn"):
            out[name] = without_diagram(text)
        elif name.endswith(".svg"):
            out[name] = svg_structure(text)
        else:
            out[name] = text
    return out


@unittest.skipIf(tok_engine is None, "the Rust engine (tok._engine) is not installed")
class EngineParity(unittest.TestCase):
    def test_documents(self):
        compared = 0
        for path in DOCUMENTS:
            name = os.path.relpath(path, os.path.dirname(REPO))
            ref = reference(REFERENCE_DOC, path)
            with open(path, encoding="utf-8") as f:
                text = f.read()
            with self.subTest(document=name, output="check"):
                self.assertEqual(tok_engine.check(text, "en"), ref["check_en"])
                self.assertEqual(tok_engine.check(text, "pl"), ref["check_pl"])
            with self.subTest(document=name, output="fmt"):
                self.assertEqual(engine_or_none(tok_engine.fmt, text), ref["fmt"])
            with self.subTest(document=name, output="semantic_xml"):
                self.assertEqual(engine_or_none(tok_engine.semantic_xml, text), ref["semantic_xml"])
            with self.subTest(document=name, output="bpmn"):
                self.assertEqual(without_diagram(engine_or_none(tok_engine.bpmn, text)),
                                 without_diagram(ref["bpmn"]))
            with self.subTest(document=name, output="svg"):
                self.assertEqual(svg_structure(engine_or_none(tok_engine.svg, text)),
                                 svg_structure(ref["svg"]))
            with self.subTest(document=name, output="export"):
                self.assertEqual(comparable_files(engine_or_none(tok_engine.export, text)),
                                 comparable_files(ref["export"]))
            compared += 1
        self.assertEqual(compared, len(DOCUMENTS))
        self.assertGreater(compared, 0)

    def test_import(self):
        for path in BPMN_FILES:
            ref = reference(REFERENCE_IMPORT, path)
            with open(path, "rb") as f:
                xml = f.read().decode("utf-8", "replace")
            for keep in (True, False):
                with self.subTest(model=os.path.basename(path), keep_ids=keep):
                    text, warnings = tok_engine.import_bpmn(xml, keep)
                    if text.startswith("tok 0.5"):
                        continue
                    self.assertEqual([text, list(warnings)], ref[str(keep)])
        self.assertGreater(len(BPMN_FILES), 0)

    def test_eval_decisions(self):
        sys.path.insert(0, SRC)
        from tok.check import check
        from tok.parser import parse
        compared = 0
        for path in DOCUMENTS + ([EVAL_TABLES] if os.path.exists(EVAL_TABLES) else []):
            with open(path, encoding="utf-8") as f:
                text = f.read()
            doc, diags = parse(text)
            if not doc.decisions or any(d.severity == "error" for d in diags):
                continue
            if any(d.severity == "error" for d in diags + check(doc, bpmn=False)):
                continue
            runs = [r for dec in doc.decisions.values() for r in decision_grid(dec)]
            ref = reference(REFERENCE_EVAL, path, stdin=runs)
            for (dec_id, values), expected in zip(runs, ref):
                with self.subTest(document=os.path.basename(path), decision=dec_id, values=values):
                    got = outcome(tok_engine.eval_decision, text, dec_id, decode_value(values))
                    self.assertEqual(got, expected)
                compared += 1
        self.assertGreater(compared, 0)

    @unittest.skipUnless(os.path.exists(FEEL_CORPUS), "FEEL corpus not generated")
    def test_feel_eval(self):
        with open(FEEL_CORPUS, encoding="utf-8") as f:
            corpus = json.load(f)
        data = {"context": corpus["context"], "expressions": [e["expr"] for e in corpus["expressions"]]}
        ref = reference(REFERENCE_FEEL, None, stdin=data)
        ctx = dict((k, decode_value(v)) for k, v in corpus["context"])
        for expr, expected in zip(data["expressions"], ref):
            with self.subTest(expr=expr):
                self.assertEqual(outcome(tok_engine.feel_eval, expr, ctx), expected)
        self.assertGreater(len(ref), 10000)

    def test_simulation(self):
        sys.path.insert(0, SRC)
        from tok.check import check
        from tok.parser import parse
        compared = 0
        for path in DOCUMENTS + SIM_MODELS:
            with open(path, encoding="utf-8") as f:
                text = f.read()
            doc, diags = parse(text)
            if any(d.severity == "error" for d in diags):
                continue
            if any(d.severity == "error" for d in diags + check(doc, level="full", bpmn=True)):
                with self.assertRaises(ValueError):
                    tok_engine.simulate(text)
                continue
            runs = SIM_RUNS if doc.simulate is not None else [[3, None]]
            ref = reference(REFERENCE_SIM, path, stdin=runs)
            for (cases, seed), expected in zip(runs, ref):
                with self.subTest(document=os.path.basename(path), cases=cases, seed=seed):
                    self.assertEqual(engine_simulation(text, cases, seed), expected)
                compared += 1
        self.assertGreater(compared, 100)

    def test_command_line(self):
        """``tok`` on the engine prints exactly what the pure-Python command prints; the
        drawing written by ``tok svg`` has the same structure."""
        import tempfile
        compared = 0
        with tempfile.TemporaryDirectory() as tmp:
            for path in DOCUMENTS:
                commands = [["check", path], ["check", path, "--json", "--lang", "pl"],
                            ["check", path, "--level", "basic"], ["fmt", path],
                            ["svg", path, "-o", os.path.join(tmp, "m.svg")]]
                for args in commands:
                    got = engine_additions_removed(run_tok(args, engine=True)) + (written(args),)
                    want = run_tok(args, engine=False) + (written(args),)
                    with self.subTest(command=" ".join(args[:1] + [os.path.basename(path)] + args[2:])):
                        self.assertEqual(got, want)
                    compared += 1
        # The engine writes argparse's messages as Python 3.11 does; Python 3.12 and newer
        # print the choices of an invalid command without quotes.
        usage_cases = [["--version"], [], ["check"]]
        if sys.version_info < (3, 12):
            usage_cases.append(["nonsense"])
        for args in usage_cases:
            with self.subTest(command=" ".join(args)):
                self.assertEqual(engine_additions_removed(run_tok(args, engine=True)),
                                 run_tok(args, engine=False))
        self.assertGreater(compared, 100)

    def test_api_on_both_implementations(self):
        from tok import api
        text = open(os.path.join(REPO, "examples", "propozycja-zakupu.tok"), encoding="utf-8").read()
        on_engine = (api.check(text, "pl"), api.fmt(text), comparable_files(api.export(text, "m")),
                     api.eval_decision(text, "tryb-akceptacji", {"rodzaj": "drukowana", "cena": 99}),
                     api.format_kpi(api.simulate(text, 20, 7).kpi(), "pl"))
        self.assertEqual(api.engine(), "rust")
        os.environ["TOK_ENGINE"] = "python"
        try:
            self.assertEqual(api.engine(), "python")
            on_python = (api.check(text, "pl"), api.fmt(text), comparable_files(api.export(text, "m")),
                         api.eval_decision(text, "tryb-akceptacji", {"rodzaj": "drukowana", "cena": 99}),
                         api.format_kpi(api.simulate(text, 20, 7).kpi(), "pl"))
        finally:
            del os.environ["TOK_ENGINE"]
        self.assertEqual(on_engine, on_python)

    def test_errors_raise(self):
        with self.assertRaises(ValueError):
            tok_engine.fmt("start s\n")
        with self.assertRaises(ValueError):
            tok_engine.check("tok 0.1\n", lang="de")


if __name__ == "__main__":
    unittest.main()
