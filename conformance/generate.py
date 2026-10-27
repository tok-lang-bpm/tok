"""Generate the expected outputs of the conformance corpus with the Python reference implementation.

Usage (from any directory):
    python tok/conformance/generate.py

For every cases/NAME.tok it writes:
    expected/NAME.tok   canonical text (tok fmt)
    expected/NAME.diag  diagnostic codes of the core level, sorted, one per line
    expected/NAME.bpmn  semantic BPMN XML (only when the document has no errors)
    expected/NAME.levels  all diagnostic codes of `tok check` at the levels basic, analytic and full
                          (full conformance level; only when the document has no parse errors)
"""

import io
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "python", "src"))

from tok.bpmn_export import semantic_xml  # noqa: E402
from tok.parser import parse  # noqa: E402
from tok.printer import fmt  # noqa: E402
from tok.check import check  # noqa: E402

# diagnostic codes every core implementation must report
CORE_CODES = {"T001", "T002", "T003", "P001", "P002", "P003", "P004", "P005", "P006", "P007", "P008",
              "P009", "P010", "R001", "R004", "R020", "D002", "D004"}


def write(path, text):
    with io.open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def main():
    cases = os.path.join(HERE, "cases")
    expected = os.path.join(HERE, "expected")
    os.makedirs(expected, exist_ok=True)
    for name in sorted(os.listdir(cases)):
        if not name.endswith(".tok"):
            continue
        stem = name[:-4]
        with io.open(os.path.join(cases, name), encoding="utf-8", newline="") as f:
            text = f.read()
        doc, diags = parse(text)
        codes = sorted(d.code for d in diags if d.code in CORE_CODES)
        write(os.path.join(expected, stem + ".tok"), fmt(doc))
        write(os.path.join(expected, stem + ".diag"), "".join(c + "\n" for c in codes))
        bpmn_path = os.path.join(expected, stem + ".bpmn")
        if any(d.severity == "error" for d in diags):
            if os.path.exists(bpmn_path):
                os.remove(bpmn_path)
        else:
            write(bpmn_path, semantic_xml(doc))
        levels_path = os.path.join(expected, stem + ".levels")
        if any(d.severity == "error" for d in diags):
            if os.path.exists(levels_path):
                os.remove(levels_path)
        else:
            out = []
            for level in ("basic", "analytic", "full"):
                doc_l, _ = parse(text)
                out.append("%s: %s\n" % (level, " ".join(sorted(d.code for d in check(doc_l, level=level)))))
            write(levels_path, "".join(out))
        print("%-24s diag=%-40s bpmn=%s" % (stem, ",".join(codes) or "-", os.path.exists(bpmn_path)))


if __name__ == "__main__":
    main()
