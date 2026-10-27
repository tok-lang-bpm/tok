"""Decision tables: DMN 1.3 export, evaluation and checks (spec §9)."""

import xml.etree.ElementTree as ET
from decimal import Decimal

from . import feel

DMN = "https://www.omg.org/spec/DMN/20191111/MODEL/"
MODELER = "http://camunda.org/schema/modeler/1.0"
HIT_POLICIES = ("unique", "first", "any", "priority", "collect")


class DecisionError(Exception):
    """Raised when a hit policy is violated (e.g. two rules match under 'unique')."""


# ---------------------------------------------------------------------- export

def export_dmn(decision, doc=None):
    """Return DMN 1.3 XML text for one decision table (deployable to Camunda 8)."""
    ET.register_namespace("", DMN)
    ET.register_namespace("modeler", MODELER)
    did = decision.id
    ns = "http://camunda.org/schema/1.0/dmn"
    if doc is not None and doc.attrs.get("namespace"):
        ns = doc.attrs["namespace"]
    root = ET.Element("{%s}definitions" % DMN, {
        "id": "%s-definitions" % did,
        "name": decision.label or did,
        "namespace": ns,
        "exporter": "tok",
        "{%s}executionPlatform" % MODELER: "Camunda Cloud",
        "{%s}executionPlatformVersion" % MODELER: "8.10.0",
    })
    dec = ET.SubElement(root, "{%s}decision" % DMN, {"id": did, "name": decision.label or did})
    table = ET.SubElement(dec, "{%s}decisionTable" % DMN,
                          {"id": "%s-table" % did, "hitPolicy": decision.hit.upper()})
    for k, (expr, typ) in enumerate(decision.inputs, start=1):
        inp = ET.SubElement(table, "{%s}input" % DMN, {"id": "%s-input-%d" % (did, k), "label": expr})
        ie = ET.SubElement(inp, "{%s}inputExpression" % DMN, {"id": "%s-input-%d-expr" % (did, k), "typeRef": typ})
        ET.SubElement(ie, "{%s}text" % DMN).text = expr
    for k, (name, typ) in enumerate(decision.outputs, start=1):
        ET.SubElement(table, "{%s}output" % DMN,
                      {"id": "%s-output-%d" % (did, k), "label": name, "name": name, "typeRef": typ})
    for r, rule in enumerate(decision.rules, start=1):
        rel = ET.SubElement(table, "{%s}rule" % DMN, {"id": "%s-rule-%d" % (did, r)})
        if rule.annotation:
            ET.SubElement(rel, "{%s}description" % DMN).text = rule.annotation
        for k, cell in enumerate(rule.inputs, start=1):
            e = ET.SubElement(rel, "{%s}inputEntry" % DMN, {"id": "%s-rule-%d-in-%d" % (did, r, k)})
            ET.SubElement(e, "{%s}text" % DMN).text = cell
        for k, cell in enumerate(rule.outputs, start=1):
            e = ET.SubElement(rel, "{%s}outputEntry" % DMN, {"id": "%s-rule-%d-out-%d" % (did, r, k)})
            ET.SubElement(e, "{%s}text" % DMN).text = "" if cell == "-" else cell
    _indent(root)
    return '<?xml version="1.0" encoding="UTF-8"?>\n' + ET.tostring(root, encoding="unicode") + "\n"


def _indent(elem, level=0):
    pad = "\n" + "  " * level
    if len(elem):
        if not elem.text or not elem.text.strip():
            elem.text = pad + "  "
        for child in elem:
            _indent(child, level + 1)
        if not child.tail or not child.tail.strip():
            child.tail = pad
    if level and (not elem.tail or not elem.tail.strip()):
        elem.tail = pad


# ------------------------------------------------------------------ evaluation

def evaluate_decision(decision, context):
    """Evaluate a decision table against a context of variables.

    Returns, as plain Python values:
      - hit policy unique, first, any, priority: a dict {output name: value} or None if no rule matches;
      - collect: a list of such dicts (empty if nothing matches).
    Raises DecisionError if 'unique' matches several rules or 'any' matches rules with different outputs.
    'priority' has no output value lists in tok 0.1 and behaves like 'first'.
    """
    values = [feel.evaluate(feel.parse_expr(expr), context) for expr, _ in decision.inputs]
    matched = []
    for idx, rule in enumerate(decision.rules, start=1):
        ok = True
        for cell, v in zip(rule.inputs, values):
            if feel.test(feel.parse_unary_tests(cell), v, context) is not True:
                ok = False
                break
        if ok:
            out = {}
            for (name, _), cell in zip(decision.outputs, rule.outputs):
                out[name] = None if cell.strip() == "-" else feel.to_python(feel.eval_text(cell, context))
            matched.append((idx, out))
    hit = decision.hit
    if hit == "collect":
        return [o for _, o in matched]
    if not matched:
        return None
    if hit == "unique" and len(matched) > 1:
        raise DecisionError("rules %s all match (hit policy unique)" % ", ".join(str(i) for i, _ in matched))
    if hit == "any":
        first = matched[0][1]
        for i, o in matched[1:]:
            if o != first:
                raise DecisionError("rules %d and %d match with different outputs (hit policy any)"
                                    % (matched[0][0], i))
    return matched[0][1]


def single_result(decision, result):
    """For a decision with one output, unwrap {name: value} to value (what Camunda stores in resultVariable)."""
    if isinstance(result, dict) and len(decision.outputs) == 1:
        return result[decision.outputs[0][0]]
    if isinstance(result, list) and len(decision.outputs) == 1:
        return [r[decision.outputs[0][0]] for r in result]
    return result


# ---------------------------------------------------------------------- checks

def check_decision(decision):
    """Return a list of (code, line, args) diagnostics for one decision."""
    out = []
    if not decision.inputs and not decision.outputs:
        out.append(("D001", decision.line, {"id": decision.id}))
    if decision.hit not in HIT_POLICIES:
        out.append(("D003", decision.line, {"hit": decision.hit}))
    if not decision.rules:
        out.append(("D005", decision.line, {"id": decision.id}))
    hline = getattr(decision, "header_line", decision.line)
    for expr, _ in decision.inputs:
        r = feel.check_expr(expr)
        if r:
            out.append((r[0], hline, {"detail": "%s (%s)" % (r[1], expr)}))
    for rule in decision.rules:
        for cell in rule.inputs:
            r = feel.check_unary(cell)
            if r:
                out.append((r[0], rule.line, {"detail": "%s (%s)" % (r[1], cell)}))
        for cell in rule.outputs:
            if cell.strip() == "-":
                continue
            r = feel.check_expr(cell)
            if r:
                out.append((r[0], rule.line, {"detail": "%s (%s)" % (r[1], cell)}))
    if decision.hit == "unique" and decision.inputs:
        out.extend(_overlaps(decision))
    return out


# Overlap detection for 'unique'. Each input cell becomes a list of atoms:
#   ("any",)  ("str", text)  ("bool", b)  ("iv", low, low_closed, high, high_closed)  (None = unbounded)
# A cell that cannot be described this way (variables, not(), functions) makes the pair undecidable.

def _atoms(cell):
    try:
        u = feel.parse_unary_tests(cell)
    except feel.FeelError:
        return None
    if u[0] == "any":
        return [("any",)]
    if u[0] != "tests":
        return None
    atoms = []
    for p in u[1]:
        k = p[0]
        if k == "eq":
            lit = _literal(p[1])
            if lit is None:
                return None
            if isinstance(lit, Decimal):
                atoms.append(("iv", lit, True, lit, True))
            elif isinstance(lit, bool):
                atoms.append(("bool", lit))
            else:
                atoms.append(("str", lit))
        elif k == "cmp":
            lit = _literal(p[2])
            if not isinstance(lit, Decimal) or isinstance(lit, bool):
                return None
            op = p[1]
            if op == "<":
                atoms.append(("iv", None, False, lit, False))
            elif op == "<=":
                atoms.append(("iv", None, False, lit, True))
            elif op == ">":
                atoms.append(("iv", lit, False, None, False))
            elif op == ">=":
                atoms.append(("iv", lit, True, None, False))
            elif op == "=":
                atoms.append(("iv", lit, True, lit, True))
            else:
                return None
        elif k == "range":
            lo, hi = _literal(p[2]), _literal(p[3])
            if not isinstance(lo, Decimal) or not isinstance(hi, Decimal):
                return None
            atoms.append(("iv", lo, p[1], hi, p[4]))
        else:
            return None
    return atoms


def _literal(node):
    if node[0] in ("num", "str", "bool"):
        return node[1]
    if node[0] == "neg" and node[1][0] == "num":
        return -node[1][1]
    return None


def _meet(a, b):
    """Intersection of two atoms: None if empty, else an example value (or '-' for any)."""
    if a[0] == "any":
        return _example(b)
    if b[0] == "any":
        return _example(a)
    if a[0] != b[0]:
        return None
    if a[0] in ("str", "bool"):
        return a[1] if a[1] == b[1] else None
    lo, loc = a[1], a[2]
    if b[1] is not None and (lo is None or b[1] > lo or (b[1] == lo and not b[2])):
        lo, loc = b[1], b[2]
    hi, hic = a[3], a[4]
    if b[3] is not None and (hi is None or b[3] < hi or (b[3] == hi and not b[4])):
        hi, hic = b[3], b[4]
    if lo is not None and hi is not None:
        if lo > hi or (lo == hi and not (loc and hic)):
            return None
    return _example(("iv", lo, loc, hi, hic))


def _example(atom):
    if atom[0] == "any":
        return "-"
    if atom[0] in ("str", "bool"):
        return atom[1]
    _, lo, loc, hi, hic = atom
    if lo is not None and hi is not None:
        return lo if loc else (hi if hic and hi != lo else (lo + hi) / 2)
    if lo is not None:
        return lo if loc else lo + 1
    if hi is not None:
        return hi if hic else hi - 1
    return Decimal(0)


def _show(v):
    if isinstance(v, str) and v != "-":
        return '"%s"' % v
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, Decimal):
        return feel.number_to_string(v)
    return str(v)


def _overlaps(decision):
    cells = [[_atoms(c) for c in r.inputs] for r in decision.rules]
    out = []
    for i in range(len(decision.rules)):
        for j in range(i + 1, len(decision.rules)):
            example = []
            ok = True
            for col in range(len(decision.inputs)):
                a, b = cells[i][col], cells[j][col]
                if a is None or b is None:
                    ok = False
                    break
                found = None
                for x in a:
                    for y in b:
                        found = _meet(x, y)
                        if found is not None:
                            break
                    if found is not None:
                        break
                if found is None:
                    ok = False
                    break
                example.append("%s=%s" % (decision.inputs[col][0], _show(found)))
            if ok:
                out.append(("D010", decision.rules[j].line,
                            {"a": i + 1, "b": j + 1, "example": ", ".join(example)}))
    return out
