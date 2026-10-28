"""Functions for using tok from Python programs and notebooks.

Every function takes the text of a document. They run on the Rust engine (``tok._engine``)
when it is installed, as in the wheels of ``tok-lang``, and on the pure-Python
implementation otherwise (or when the environment variable ``TOK_ENGINE=python`` is set).
Both give the same results for language 0.3; language features added after 0.3 exist only
in the engine.

    >>> from tok import api
    >>> api.check('tok 0.3 "x"\\nstart s\\nend e\\nflow s -> e')
    []
"""

import os
import tempfile

from .cli import _engine

__all__ = [
    "engine", "version", "check", "fmt", "bpmn", "svg", "export", "import_bpmn",
    "eval_decision", "simulate", "format_kpi",
]


def engine():
    """``"rust"`` when the Rust engine is used, ``"python"`` otherwise."""
    return "python" if _engine() is None else "rust"


def version():
    """Version of the engine in use."""
    e = _engine()
    if e is not None:
        return e.version()
    from . import __version__
    return __version__


def _normalize(text):
    return text.replace("\r\n", "\n").replace("\r", "\n")


def _errors(diags):
    from .diagnostics import format_diag
    return ValueError("\n".join(format_diag(d, "en") for d in diags if d.severity == "error"))


def _parse_ok(text):
    from .parser import parse
    doc, diags = parse(_normalize(text))
    if any(d.severity == "error" for d in diags):
        raise _errors(diags)
    return doc


def _load(text, level="full", bpmn=True):
    from .check import check as run_check
    from .diagnostics import sort_key
    from .parser import parse
    doc, diags = parse(_normalize(text))
    if not any(d.severity == "error" for d in diags):
        diags += run_check(doc, level=level, bpmn=bpmn)
    diags.sort(key=sort_key)
    return doc, diags


def _load_ok(text):
    doc, diags = _load(text)
    if any(d.severity == "error" for d in diags):
        raise _errors(diags)
    return doc


def check(text, lang="en", level="full", bpmn=True):
    """Diagnostics of ``tok check``: a list of dicts with the keys ``code``, ``severity``,
    ``line``, ``id`` and ``message``, sorted by line and code."""
    e = _engine()
    if e is not None:
        return e.check(text, lang, level, bpmn)
    if lang not in ("en", "pl"):
        raise ValueError("lang must be 'en' or 'pl', not %r" % (lang,))
    if level not in ("basic", "analytic", "full"):
        raise ValueError("level must be 'basic', 'analytic' or 'full', not %r" % (level,))
    return [d.as_dict(lang) for d in _load(text, level, bpmn)[1]]


def fmt(text):
    """Canonical text (``tok fmt``); raises ``ValueError`` on syntax errors."""
    e = _engine()
    if e is not None:
        return e.fmt(text)
    from .printer import fmt as print_doc
    return print_doc(_parse_ok(text))


def _bpmn_file(doc):
    from .io import write_bpmn
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "model.bpmn")
        write_bpmn(doc, path)
        with open(path, encoding="utf-8") as f:
            return f.read()


def _svg_text(doc, title):
    import xml.etree.ElementTree as ET
    from . import bpmnlib
    from .bpmn_export import export_tree
    tree = export_tree(doc)
    for prefix, uri in bpmnlib.NS.items():
        ET.register_namespace(prefix, uri)
    m = bpmnlib.Model(tree)
    bpmnlib.layout(m)
    return bpmnlib.render_svg(m, title=title)


def bpmn(text):
    """BPMN 2.0 XML with diagram layout (``tok bpmn``); raises ``ValueError`` on errors."""
    e = _engine()
    if e is not None:
        return e.bpmn(text)
    return _bpmn_file(_load_ok(text))


def svg(text, title=None):
    """SVG picture of the diagram (``tok svg``); raises ``ValueError`` on errors."""
    e = _engine()
    if e is not None:
        return e.svg(text, title)
    return _svg_text(_load_ok(text), title)


def export(text, name="model"):
    """Every file of ``tok export`` as a dict file name -> content: ``NAME.bpmn``,
    ``NAME.svg``, one ``.dmn`` per decision table and one ``.form`` per form."""
    e = _engine()
    if e is not None:
        return e.export(text, name)
    from . import dmn, forms
    doc = _load_ok(text)
    files = {name + ".bpmn": _bpmn_file(doc), name + ".svg": _svg_text(doc, name + ".svg")}
    for dec in doc.decisions.values():
        files[dec.id + ".dmn"] = dmn.export_dmn(dec, doc)
    for form in doc.forms.values():
        files[form.id + ".form"] = forms.export_form(form)
    return files


def import_bpmn(xml, keep_ids=True):
    """BPMN 2.0 XML to tok (``tok import``): ``(text, warnings)``."""
    e = _engine()
    if e is not None:
        return e.import_bpmn(xml, keep_ids)
    from .bpmn_import import import_bpmn as read_bpmn
    from .printer import fmt as print_doc
    doc, warnings = read_bpmn(xml, keep_ids=keep_ids)
    return print_doc(doc), list(warnings)


def eval_decision(text, decision, values=None):
    """Evaluate a decision (``tok eval``) with a dict of inputs. In a document of language
    0.7 (Rust engine) the decision is evaluated with the decisions, input data and business
    knowledge models it requires, and the result is its value. In earlier documents the
    result follows the hit policy of the table: a dict output -> value, a list of them for
    ``collect``, or ``None``."""
    e = _engine()
    if e is not None:
        return e.eval_decision(text, decision, values)
    from . import dmn
    doc, diags = _load(text, bpmn=False)
    if any(d.severity == "error" for d in diags):
        raise _errors(diags)
    return dmn.evaluate_decision(doc.decisions[decision], dict(values or {}))


def simulate(text, cases=None, seed=None):
    """Simulate the process (``tok sim``). The result has ``kpi()``, ``log``, ``cases``,
    ``warnings``, ``seed``, ``horizon``, ``to_json(include_log=False)`` and
    ``write_csv(path_log, path_cases=None)``; :func:`format_kpi` prints the report.
    On the Rust engine only, it also has ``to_xes()`` and ``write_xes(path)``: the event
    log as an XES document (IEEE 1849-2016) for process-mining tools."""
    e = _engine()
    if e is not None:
        return e.simulate(text, cases, seed)
    from . import sim
    return sim.simulate(_load_ok(text), cases=cases, seed=seed)


def format_kpi(kpi, lang="en"):
    """The text report of ``tok sim`` for the indicators ``simulate(...).kpi()``."""
    e = _engine()
    if e is not None:
        return e.format_kpi(kpi, lang)
    from . import sim
    return sim.format_kpi(kpi, lang)
