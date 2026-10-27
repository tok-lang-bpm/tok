"""Command line interface: ``tok <command> FILE …``."""

import argparse
import json
import os
import sys

from . import __version__
from .check import check
from .diagnostics import format_diag, sort_key
from .parser import parse
from .printer import fmt

EXIT_OK, EXIT_ERRORS, EXIT_USAGE = 0, 1, 2


def _read(path):
    try:
        with open(path, encoding="utf-8") as f:
            return f.read()
    except OSError as ex:
        sys.stderr.write("tok: cannot read %s: %s\n" % (path, ex))
        sys.exit(EXIT_USAGE)


def _load(path, level="full", bpmn=True):
    doc, diags = parse(_read(path))
    if not any(d.severity == "error" for d in diags):
        diags += check(doc, level=level, bpmn=bpmn)
    diags.sort(key=sort_key)
    return doc, diags


def _report(path, diags, args, extra=None):
    errors = sum(1 for d in diags if d.severity == "error")
    if getattr(args, "json", False):
        out = {"file": path, "errors": errors, "diagnostics": [d.as_dict(args.lang) for d in diags]}
        if extra:
            out.update(extra)
        json.dump(out, sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    else:
        for d in diags:
            sys.stdout.write(format_diag(d, args.lang, path) + "\n")
        warnings = sum(1 for d in diags if d.severity == "warning")
        notes = len(diags) - errors - warnings
        if args.lang == "pl":
            sys.stdout.write("%s: błędy %d, ostrzeżenia %d, uwagi %d\n" % (path, errors, warnings, notes))
        else:
            sys.stdout.write("%s: %d error(s), %d warning(s), %d note(s)\n" % (path, errors, warnings, notes))
    return errors


def cmd_check(args):
    _, diags = _load(args.file, args.level)
    return EXIT_ERRORS if _report(args.file, diags, args) else EXIT_OK


def cmd_fmt(args):
    text = _read(args.file)
    doc, diags = parse(text)
    if any(d.severity == "error" for d in diags):
        _report(args.file, diags, args)
        return EXIT_ERRORS
    out = fmt(doc)
    if args.check:
        if out != text.replace("\r\n", "\n"):
            sys.stdout.write("%s: not in canonical form\n" % args.file)
            return EXIT_ERRORS
        return EXIT_OK
    if args.write:
        with open(args.file, "w", encoding="utf-8", newline="\n") as f:
            f.write(out)
    else:
        sys.stdout.write(out)
    return EXIT_OK


def _nothing_to_draw(doc, args):
    """A diagram needs at least one flow node or pool."""
    if not doc.nodes and not doc.pools:
        msg = ("nie ma czego narysować: model nie zawiera elementów" if args.lang == "pl"
               else "nothing to draw: the model has no elements")
        sys.stderr.write("tok: %s\n" % msg)
        return True
    return False


def _out(args, ext):
    return args.output or os.path.splitext(args.file)[0] + ext


def cmd_bpmn(args):
    from .io import write_bpmn
    doc, diags = _load(args.file)
    if any(d.severity == "error" for d in diags):
        _report(args.file, diags, args)
        return EXIT_ERRORS
    if _nothing_to_draw(doc, args):
        return EXIT_ERRORS
    path = _out(args, ".bpmn")
    write_bpmn(doc, path, layout=not args.no_layout)
    sys.stdout.write("BPMN: %s\n" % path)
    return EXIT_OK


def cmd_svg(args):
    from .io import write_svg
    doc, diags = _load(args.file)
    if any(d.severity == "error" for d in diags):
        _report(args.file, diags, args)
        return EXIT_ERRORS
    if _nothing_to_draw(doc, args):
        return EXIT_ERRORS
    path = _out(args, ".svg")
    write_svg(doc, path)
    sys.stdout.write("SVG: %s\n" % path)
    return EXIT_OK


def cmd_export(args):
    """BPMN, SVG, one .dmn per decision and one .form per form into a folder."""
    from .io import write_bpmn, write_svg
    from . import dmn, forms
    doc, diags = _load(args.file)
    if any(d.severity == "error" for d in diags):
        _report(args.file, diags, args)
        return EXIT_ERRORS
    if _nothing_to_draw(doc, args):
        return EXIT_ERRORS
    folder = args.output or os.path.dirname(os.path.abspath(args.file))
    os.makedirs(folder, exist_ok=True)
    base = os.path.splitext(os.path.basename(args.file))[0]
    written = [os.path.join(folder, base + ".bpmn"), os.path.join(folder, base + ".svg")]
    write_bpmn(doc, written[0])
    write_svg(doc, written[1])
    for dec in doc.decisions.values():
        p = os.path.join(folder, dec.id + ".dmn")
        with open(p, "w", encoding="utf-8", newline="\n") as f:
            f.write(dmn.export_dmn(dec, doc))
        written.append(p)
    for form in doc.forms.values():
        p = os.path.join(folder, form.id + ".form")
        with open(p, "w", encoding="utf-8", newline="\n") as f:
            f.write(forms.export_form(form))
        written.append(p)
    for p in written:
        sys.stdout.write(os.path.normpath(p) + "\n")
    return EXIT_OK


def cmd_import(args):
    from .bpmn_import import import_bpmn
    doc, warnings = import_bpmn(args.file, keep_ids=not args.no_flow_ids)
    for w in warnings:
        sys.stderr.write("%s: %s\n" % (args.file, w))
    out = fmt(doc)
    if args.output:
        with open(args.output, "w", encoding="utf-8", newline="\n") as f:
            f.write(out)
        sys.stdout.write("tok: %s\n" % args.output)
    else:
        sys.stdout.write(out)
    return EXIT_OK


def cmd_sim(args):
    from . import sim
    doc, diags = _load(args.file)
    if any(d.severity == "error" for d in diags):
        _report(args.file, diags, args)
        return EXIT_ERRORS
    result = sim.simulate(doc, cases=args.cases, seed=args.seed)
    if args.log or args.cases_csv:
        result.write_csv(args.log, args.cases_csv)
    if args.json:
        sys.stdout.write(result.to_json() + "\n")
    else:
        sys.stdout.write(sim.format_kpi(result.kpi(), args.lang) if hasattr(sim, "format_kpi")
                         else json.dumps(result.kpi(), ensure_ascii=False, indent=2) + "\n")
    return EXIT_OK


def _value(text):
    low = text.lower()
    if low in ("true", "false"):
        return low == "true"
    try:
        from decimal import Decimal
        return Decimal(text)
    except Exception:
        return text


def cmd_eval(args):
    from . import dmn
    doc, diags = _load(args.file, bpmn=False)
    if any(d.severity == "error" for d in diags):
        _report(args.file, diags, args)
        return EXIT_ERRORS
    dec = doc.decisions.get(args.decision)
    if dec is None:
        sys.stderr.write("tok: no decision '%s' in %s\n" % (args.decision, args.file))
        return EXIT_USAGE
    ctx = {}
    for pair in args.values:
        if "=" not in pair:
            sys.stderr.write("tok: expected NAME=VALUE, got '%s'\n" % pair)
            return EXIT_USAGE
        k, v = pair.split("=", 1)
        ctx[k] = _value(v)
    result = dmn.evaluate_decision(dec, ctx)
    sys.stdout.write(json.dumps(result, ensure_ascii=False, default=str) + "\n")
    return EXIT_OK


def cmd_cheatsheet(args):
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, "cheatsheet.md")
    sys.stdout.write(_read(path))
    return EXIT_OK


def build_parser():
    ap = argparse.ArgumentParser(prog="tok", description="tok: plain-text information-process models "
                                 "(BPMN 2.0, DMN, forms, simulation).")
    ap.add_argument("--version", action="version", version="tok " + __version__)
    sub = ap.add_subparsers(dest="cmd")

    def common(p, json_flag=True):
        p.add_argument("--lang", choices=("en", "pl"), default=os.environ.get("TOK_LANG", "en"),
                       help="language of messages (default: en, or TOK_LANG)")
        if json_flag:
            p.add_argument("--json", action="store_true", help="machine-readable output")

    p = sub.add_parser("check", help="check a document and list diagnostics")
    p.add_argument("file")
    p.add_argument("--level", choices=("basic", "analytic", "full"), default="full")
    common(p)
    p.set_defaults(fn=cmd_check)

    p = sub.add_parser("fmt", help="print the canonical form")
    p.add_argument("file")
    p.add_argument("-w", "--write", action="store_true", help="rewrite the file in place")
    p.add_argument("--check", action="store_true", help="only report whether the file is canonical")
    common(p)
    p.set_defaults(fn=cmd_fmt)

    for name, fn, ext in (("bpmn", cmd_bpmn, ".bpmn"), ("svg", cmd_svg, ".svg")):
        p = sub.add_parser(name, help="write %s next to the file" % ext)
        p.add_argument("file")
        p.add_argument("-o", "--output")
        if name == "bpmn":
            p.add_argument("--no-layout", action="store_true", help="semantic model only, no BPMNDI")
        common(p)
        p.set_defaults(fn=fn)

    p = sub.add_parser("export", help="write BPMN, SVG, DMN and form files into a folder")
    p.add_argument("file")
    p.add_argument("-o", "--output", help="output folder (default: folder of the file)")
    common(p)
    p.set_defaults(fn=cmd_export)

    p = sub.add_parser("import", help="convert a BPMN 2.0 XML file to tok")
    p.add_argument("file")
    p.add_argument("-o", "--output")
    p.add_argument("--no-flow-ids", action="store_true",
                   help="drop the identifiers of sequence flows so they print as compact chains")
    common(p, json_flag=False)
    p.set_defaults(fn=cmd_import)

    p = sub.add_parser("sim", help="simulate the process and print indicators")
    p.add_argument("file")
    p.add_argument("--cases", type=int)
    p.add_argument("--seed", type=int)
    p.add_argument("--log", help="write the event log (CSV)")
    p.add_argument("--cases-csv", help="write one row per case (CSV)")
    common(p)
    p.set_defaults(fn=cmd_sim)

    p = sub.add_parser("eval", help="evaluate a decision table: tok eval FILE DECISION name=value …")
    p.add_argument("file")
    p.add_argument("decision")
    p.add_argument("values", nargs="*")
    common(p, json_flag=False)
    p.set_defaults(fn=cmd_eval)

    p = sub.add_parser("cheatsheet", help="print the one-page language summary")
    p.set_defaults(fn=cmd_cheatsheet)
    return ap


def main(argv=None):
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8")
            sys.stderr.reconfigure(encoding="utf-8")
        except Exception:
            pass
    ap = build_parser()
    args = ap.parse_args(argv)
    if not getattr(args, "fn", None):
        ap.print_help()
        return EXIT_USAGE
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
