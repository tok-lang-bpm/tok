"""Generate the Python-reference fixtures for tests/python_parity.rs.

Usage (from any directory):
    PYTHONPATH=tok/python/src python tok/rust/tok-core/tests/gen_python_expected.py [EXTRA_TOK_DIR ...]

Inputs: tok/examples/*.tok, tok/conformance/cases/*.tok (copied as conformance-NAME.tok), the .tok
files already in fixtures/tok (refreshed from EXTRA_TOK_DIR when given, e.g. the course examples),
and tok/conformance/import/*.bpmn.
Outputs in fixtures/:
    tok/NAME.tok                 input copy
    tok/NAME.en.json, .pl.json   diagnostics of `tok check` (as in --json, without the file name)
    tok/NAME.bpmn, .svg          `tok bpmn` / `tok svg` output (documents without errors);
                                 .svg.err holds the message when there is nothing to draw
    tok/NAME.DECISION.dmn        `tok export` decision tables
    tok/NAME.FORM.form           `tok export` forms
    import/NAME.tok, .noids.tok  `tok import` output with and without flow identifiers
    import/NAME.warn             import warnings, one per line
"""

import glob
import io
import json
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
TOK = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(TOK, "python", "src"))

from tok import bpmn_import, bpmnlib, dmn, forms  # noqa: E402
from tok.check import check  # noqa: E402
from tok.diagnostics import sort_key  # noqa: E402
from tok.io import write_bpmn, write_svg  # noqa: E402
from tok.parser import parse  # noqa: E402
from tok.printer import fmt  # noqa: E402

FIX = os.path.join(HERE, "fixtures")


def write(path, text):
    with io.open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def read_text(path):
    with io.open(path, encoding="utf-8") as f:   # universal newlines, like the CLI
        return f.read()


def gen_tok(path, out_dir):
    name = os.path.splitext(os.path.basename(path))[0]
    text = read_text(path)
    dst = os.path.join(out_dir, name + ".tok")
    if os.path.abspath(path) != os.path.abspath(dst):
        shutil.copyfile(path, dst)
    doc, diags = parse(text)
    if not any(d.severity == "error" for d in diags):
        diags += check(doc)
    diags.sort(key=sort_key)
    for lang in ("en", "pl"):
        data = [d.as_dict(lang) for d in diags]
        write(os.path.join(out_dir, "%s.%s.json" % (name, lang)), json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    if any(d.severity == "error" for d in diags):
        return
    tmp = tempfile.mkdtemp()
    try:
        b = os.path.join(tmp, name + ".bpmn")
        s = os.path.join(tmp, name + ".svg")
        write_bpmn(doc, b)
        write(os.path.join(out_dir, name + ".bpmn"), read_text(b))
        try:
            write_svg(doc, s)
            write(os.path.join(out_dir, name + ".svg"), read_text(s))
        except bpmnlib.LoadError as e:     # nothing to draw
            write(os.path.join(out_dir, name + ".svg.err"), str(e) + "\n")
    finally:
        shutil.rmtree(tmp)
    for dec in doc.decisions.values():
        write(os.path.join(out_dir, "%s.%s.dmn" % (name, dec.id)), dmn.export_dmn(dec, doc))
    for form in doc.forms.values():
        write(os.path.join(out_dir, "%s.%s.form" % (name, form.id)), forms.export_form(form))


def gen_import(path, out_dir):
    name = os.path.splitext(os.path.basename(path))[0]
    doc, warnings = bpmn_import.import_bpmn(path, keep_ids=True)
    write(os.path.join(out_dir, name + ".tok"), fmt(doc))
    write(os.path.join(out_dir, name + ".warn"), "".join(w + "\n" for w in warnings))
    doc, _ = bpmn_import.import_bpmn(path, keep_ids=False)
    write(os.path.join(out_dir, name + ".noids.tok"), fmt(doc))


def main():
    tok_dir = os.path.join(FIX, "tok")
    imp_dir = os.path.join(FIX, "import")
    os.makedirs(tok_dir, exist_ok=True)
    os.makedirs(imp_dir, exist_ok=True)
    for extra in sys.argv[1:]:
        for p in glob.glob(os.path.join(extra, "*.tok")):
            shutil.copyfile(p, os.path.join(tok_dir, os.path.basename(p)))
    for p in glob.glob(os.path.join(TOK, "examples", "*.tok")):
        shutil.copyfile(p, os.path.join(tok_dir, os.path.basename(p)))
    for p in glob.glob(os.path.join(TOK, "conformance", "cases", "*.tok")):
        shutil.copyfile(p, os.path.join(tok_dir, "conformance-" + os.path.basename(p)))
    for f in glob.glob(os.path.join(tok_dir, "*")):
        if not f.endswith(".tok"):
            os.remove(f)
    inputs = sorted(glob.glob(os.path.join(tok_dir, "*.tok")))
    for p in inputs:
        gen_tok(p, tok_dir)
    for f in glob.glob(os.path.join(imp_dir, "*")):
        os.remove(f)
    imports = sorted(glob.glob(os.path.join(TOK, "conformance", "import", "*.bpmn")))
    for p in imports:
        gen_import(p, imp_dir)
    print("tok inputs: %d, import inputs: %d" % (len(inputs), len(imports)))


if __name__ == "__main__":
    main()
