"""Seed corpora and dictionaries of the fuzz targets, generated from the files of the
repository: the conformance cases, the examples, the MIWG reference models and the
models of the DMN TCK.

    python3 rust/fuzz/seeds.py OUT [--tok PATH]

writes OUT/TARGET/ (one directory of seed inputs per target) and OUT/TARGET.dict (a
libFuzzer dictionary). With --tok (the tok program), the seeds of the sidecar target are
the layout sidecars that `tok import` writes for the drawn models of the target;
without it, sidecars are made from the diagrams of the MIWG files. Nothing is written
into the repository; see docs/fuzzing.md.
"""

import argparse
import hashlib
import html
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONF = ROOT / "conformance"

TARGETS = ["check", "roundtrip", "feel", "import_bpmn", "import_dmn", "sidecar", "render", "sim"]


def files(*patterns):
    out = []
    for p in patterns:
        out.extend(sorted(ROOT.glob(p)))
    return out


def write(out, target, data):
    if isinstance(data, str):
        data = data.encode("utf-8")
    d = out / target
    d.mkdir(parents=True, exist_ok=True)
    (d / hashlib.sha1(data).hexdigest()).write_bytes(data)


def documents():
    return files("conformance/cases/*.tok", "conformance/expected/*.tok", "examples/*.tok",
                 "skills/tok-modelling/references/*.tok")


def bpmn_files():
    return files("conformance/miwg/*.bpmn", "conformance/expected/*.bpmn", "conformance/import/*.bpmn",
                 "examples/*.bpmn")


def dmn_files():
    return files("conformance/dmn-tck/**/*.dmn", "conformance/expected/*.dmn")


def feel_texts():
    """FEEL found in the documents (quoted attribute values, table cells) and in the DMN
    models (text elements)."""
    seen = set()
    attr = re.compile(r'\b(?:when|expr|condition|correlation|collection|until|active|activation|'
                      r'while|default|allowed|output-item|with)="((?:[^"\\]|\\.)*)"')
    for f in documents():
        for line in f.read_text(encoding="utf-8").splitlines():
            for m in attr.finditer(line):
                seen.add(m.group(1).replace('\\"', '"').replace("\\\\", "\\"))
            if line.lstrip().startswith("|"):
                for cell in line.strip().strip("|").replace("||", "|").split("|"):
                    cell = cell.strip()
                    if cell and ":" not in cell:
                        seen.add(cell)
    text = re.compile(r"<(?:\w+:)?text>(.*?)</(?:\w+:)?text>", re.S)
    for f in dmn_files():
        for m in text.finditer(f.read_text(encoding="utf-8", errors="replace")):
            seen.add(html.unescape(m.group(1)).strip())
    return sorted(s for s in seen if s and len(s.encode("utf-8")) <= 512)


def keywords(target):
    """Dictionary entries: keywords and attribute keys of tok, the element and attribute
    names of BPMN and DMN, the keywords and functions of FEEL."""
    words = set()
    if target in ("check", "roundtrip", "render", "sim"):
        for f in documents():
            for line in f.read_text(encoding="utf-8").splitlines():
                s = line.strip()
                if s and not s.startswith(("#", "|")):
                    words.add(s.split()[0])
                words.update(re.findall(r"\b([a-z][a-z-]*)=", line))
        words = {w for w in words if re.fullmatch(r"[a-z][a-z0-9-]*", w)}
        words.update(["->", "||", "|", '"', "=", "  ", "tok 0.7", "tok 0.6", "tok 0.5", "tok 0.1"])
    elif target in ("import_bpmn", "import_dmn"):
        src = bpmn_files() if target == "import_bpmn" else dmn_files()
        for f in src:
            t = f.read_text(encoding="utf-8", errors="replace")
            words.update("<" + m for m in re.findall(r"<([A-Za-z][\w:.-]*)", t))
            words.update(m + "=" for m in re.findall(r"\s([A-Za-z][\w:.-]*)=", t))
    elif target == "feel":
        words.update(["for", "in", "return", "some", "every", "satisfies", "if", "then", "else",
                      "and", "or", "not", "between", "instance of", "function", "null", "true",
                      "false", "..", "[", "]", "{", "}", "(", ")", "@\"", "**", "<=", ">=", "!=",
                      "date(", "time(", "date and time(", "duration(", "years and months duration(",
                      "string(", "number(", "sum(", "count(", "sublist(", "substring(", "matches(",
                      "replace(", "list<", "context<", "range<", "->"])
    elif target == "sidecar":
        words.update(["tok-layout 1\n", "/label ", "\n", " -"])
    return sorted(words)


def dictionary(out, target):
    lines = []
    for w in keywords(target):
        if not all(32 <= ord(c) < 127 or c == "\n" for c in w):
            continue
        # libFuzzer dictionaries know the escapes \\, \" and \xHH only
        esc = w.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\x0A")
        lines.append(f'"{esc}"')
    (out / f"{target}.dict").write_text("\n".join(lines) + "\n", encoding="utf-8")


def sidecar_seeds(out, tok):
    oracles = (Path(__file__).parent / "src" / "oracles.rs").read_text(encoding="utf-8")
    block = oracles.split("pub const SIDECAR_MODELS", 1)[1].split("];", 1)[0]
    models = []
    for lit in re.findall(r'"((?:[^"\\]|\\.)*)"(?=,\s*(?:"|\n))', block, re.S):
        text = re.sub(r"\\\n\s*", "", lit)
        text = text.replace("\\x20", " ").replace('\\"', '"').replace("\\n", "\n")
        models.append(text)
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        for i, model in enumerate(models):
            (tmp / "m.tok").write_text(model, encoding="utf-8")
            subprocess.run([tok, "bpmn", str(tmp / "m.tok"), "-o", str(tmp / "m.bpmn")],
                           check=True, capture_output=True)
            subprocess.run([tok, "import", str(tmp / "m.bpmn"), "-o", str(tmp / "n.tok")],
                           check=True, capture_output=True)
            write(out, "sidecar", bytes([i]) + (tmp / "n.tok.layout").read_bytes())
        for f in bpmn_files()[:30]:
            r = subprocess.run([tok, "import", str(f), "-o", str(tmp / "x.tok")], capture_output=True)
            if r.returncode == 0 and (tmp / "x.tok.layout").exists():
                write(out, "sidecar", b"\0" + (tmp / "x.tok.layout").read_bytes())


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("out", type=Path)
    ap.add_argument("--tok", help="the tok program, for the seeds of the sidecar target")
    a = ap.parse_args()
    out = a.out
    out.mkdir(parents=True, exist_ok=True)
    for f in documents():
        data = f.read_bytes()
        for t in ("check", "roundtrip"):
            write(out, t, data)
        if len(data) <= 8 * 1024:
            for t in ("render", "sim"):
                write(out, t, data)
    for f in bpmn_files():
        write(out, "import_bpmn", f.read_bytes())
    for f in dmn_files():
        write(out, "import_dmn", f.read_bytes())
    for s in feel_texts():
        write(out, "feel", s)
    if a.tok:
        sidecar_seeds(out, a.tok)
    write(out, "sidecar", b"\0tok-layout 1\ns 100 100 36 36\na 200 80 100 80\nf-s-a 136 118 200 118\n")
    for t in TARGETS:
        dictionary(out, t)
    for t in TARGETS:
        n = len(list((out / t).iterdir())) if (out / t).exists() else 0
        print(f"{t}: {n} seeds", file=sys.stderr)


if __name__ == "__main__":
    main()
