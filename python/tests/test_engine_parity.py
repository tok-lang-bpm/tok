"""Parity of the Rust engine (Python module ``tok_engine``) with the pure-Python implementation.

The tests are skipped when ``tok_engine`` is not installed; see ``rust/tok-py/README.md``.
The pure-Python reference runs in a fresh interpreter for every input, so that the global
namespace registry of ElementTree does not carry over from one model to the next.
"""

import glob
import json
import os
import subprocess
import sys
import unittest

try:
    import tok_engine
except ImportError:  # pragma: no cover - depends on the environment
    tok_engine = None

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.normpath(os.path.join(HERE, "..", "src"))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
# examples of the companion course, when its repository is checked out next to this one
COURSE_EXAMPLES = os.path.normpath(os.path.join(REPO, "..", "tok-course", "przyklady"))

DOCUMENTS = sorted(
    glob.glob(os.path.join(REPO, "examples", "*.tok"))
    + glob.glob(os.path.join(REPO, "conformance", "cases", "*.tok"))
    + glob.glob(os.path.join(COURSE_EXAMPLES, "*.tok"))
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


def reference(script, path):
    env = dict(os.environ, PYTHONPATH=SRC, PYTHONIOENCODING="utf-8")
    res = subprocess.run([sys.executable, "-c", script, path], env=env, capture_output=True, check=True)
    return json.loads(res.stdout.decode("utf-8"))


def engine_or_none(fn, *args, **kw):
    try:
        return fn(*args, **kw)
    except ValueError:
        return None


@unittest.skipIf(tok_engine is None, "tok_engine (Rust engine) is not installed")
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
                self.assertEqual(engine_or_none(tok_engine.bpmn, text), ref["bpmn"])
            with self.subTest(document=name, output="svg"):
                self.assertEqual(engine_or_none(tok_engine.svg, text), ref["svg"])
            with self.subTest(document=name, output="export"):
                self.assertEqual(engine_or_none(tok_engine.export, text), ref["export"])
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
                    self.assertEqual([text, list(warnings)], ref[str(keep)])
        self.assertGreater(len(BPMN_FILES), 0)

    def test_errors_raise(self):
        with self.assertRaises(ValueError):
            tok_engine.fmt("start s\n")
        with self.assertRaises(ValueError):
            tok_engine.check("tok 0.1\n", lang="de")


if __name__ == "__main__":
    unittest.main()
