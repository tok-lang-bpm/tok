"""Generate the expected outputs of the parity tests with the Python reference (`tok.bpmnlib`).

Usage:
    PYTHONPATH=python/src python rust/tok-bpmn/tests/gen_expected.py      (from the repository root)

For every fixture `fixtures/<group>/<name>.bpmn` it writes into `expected/<group>/`:
    <name>.validate.json   validate(m)                  (bpmn.py check)
    <name>.kroki.json      validate(m, kroki=True)      (bpmn.py check --kroki)
    <name>.camunda.json    validate_camunda(m, folder)  (bpmn.py check --camunda)
    <name>.summary.txt     summarize(m)
    <name>.opis.json       validate(m, kroki=True, opis_kroki=read_opis(<name>.opis.md)), when present
    <name>.layout.bpmn     layout(m) + save()           (bpmn.py layout)
    <name>.svg             render_svg(m, "model.bpmn")  (bpmn.py svg, laid out first if needed)
    <name>.svg.err         the error message when there is nothing to draw

ElementTree keeps a process-wide namespace registry; it is reset before each run so
that every output depends only on its own input, as in a fresh `bpmn.py` process.
Files are written with "\n" line endings (Python's text mode would use the platform's).
"""

import io
import json
import os
import tempfile
import xml.etree.ElementTree as ET

from tok import bpmnlib as b

HERE = os.path.dirname(os.path.abspath(__file__))
FIX = os.path.join(HERE, "fixtures")
EXP = os.path.join(HERE, "expected")
DEFAULT_NS_MAP = dict(ET._namespace_map)


def fresh():
    ET._namespace_map.clear()
    ET._namespace_map.update(DEFAULT_NS_MAP)


def write(path, text):
    with io.open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


def main():
    count = 0
    for group in sorted(os.listdir(FIX)):
        src_dir = os.path.join(FIX, group)
        out_dir = os.path.join(EXP, group)
        os.makedirs(out_dir, exist_ok=True)
        for name in sorted(os.listdir(src_dir)):
            if not name.endswith(".bpmn"):
                continue
            path = os.path.join(src_dir, name)
            stem = name[:-5]
            fresh()
            tree, in_map = b.load(path)
            m = b.Model(tree)
            write(os.path.join(out_dir, stem + ".validate.json"),
                  json.dumps(b.validate(m), ensure_ascii=False, indent=1) + "\n")
            write(os.path.join(out_dir, stem + ".kroki.json"),
                  json.dumps(b.validate(m, kroki=True), ensure_ascii=False, indent=1) + "\n")
            write(os.path.join(out_dir, stem + ".camunda.json"),
                  json.dumps(b.validate_camunda(m, os.path.dirname(path)), ensure_ascii=False, indent=1) + "\n")
            write(os.path.join(out_dir, stem + ".summary.txt"), b.summarize(m))
            opis = os.path.join(src_dir, stem + ".opis.md")
            if os.path.exists(opis):
                steps = b.read_opis(opis)
                write(os.path.join(out_dir, stem + ".opis.json"),
                      json.dumps(b.validate(m, kroki=True, opis_kroki=steps), ensure_ascii=False, indent=1) + "\n")

            fresh()
            tree, in_map = b.load(path)
            m = b.Model(tree)
            b.layout(m)
            fd, tmp = tempfile.mkstemp(suffix=".bpmn")
            os.close(fd)
            try:
                b.save(tree, tmp, in_map)
                with io.open(tmp, encoding="utf-8") as f:
                    write(os.path.join(out_dir, stem + ".layout.bpmn"), f.read())
            finally:
                os.remove(tmp)

            fresh()
            tree, _ = b.load(path)
            m = b.Model(tree)
            if not list(m.root.iter(b.BDI + "BPMNShape")):
                b.layout(m)
            try:
                write(os.path.join(out_dir, stem + ".svg"), b.render_svg(m, title="model.bpmn"))
            except b.LoadError as ex:
                write(os.path.join(out_dir, stem + ".svg.err"), str(ex))
            count += 1
    print("expected outputs for %d models" % count)


if __name__ == "__main__":
    main()
