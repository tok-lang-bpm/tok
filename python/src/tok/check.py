"""Semantic checks of a tok document (``tok check``).

Order: references and structure (R), executable profile (C), decisions and
forms (D, M), FEEL (F), simulation (S), and finally the BPMN 2.0 rules of
bpmnlib on the exported model (E, W, L), which run only when the document has
no reference errors.
"""

import re

from . import model as M
from .diagnostics import Diag, LEVELS

BASIC_KINDS = {"start", "end", "task", "user", "service", "manual", "xor", "and"}
ANALYTIC_EXCLUDED_KEYS = {"form", "decision", "expr", "result", "type", "correlation"} | set(M.EXEC_KEYS)

ISO_ZONED = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:\d{2})(\[[^\]]+\])?$")
TARGET_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*$")
EXECUTION_EVENTS = ("start", "end")
TASK_EVENTS = ("creating", "assigning", "updating", "completing", "canceling")


def split_pairs(text, sep):
    """Split 'a = x; b = y' (sep '=') or 'k: v; …' (sep ':') into (left, right) pairs; None for a bad item."""
    out = []
    for item in text.split(";"):
        item = item.strip()
        if not item:
            continue
        if sep not in item:
            out.append((item, None))
            continue
        left, right = item.split(sep, 1)
        out.append((left.strip(), right.strip()))
    return out


def parse_listener(item):
    """'start: type' or 'start: type (5)' -> (event, type, retries) or None."""
    m = re.match(r"^([a-z]+)\s*:\s*(.+?)(?:\s*\((\d+)\))?$", item.strip())
    if not m or not m.group(2).strip():
        return None
    return m.group(1), m.group(2).strip(), m.group(3)


KIND_NAMES = {"node": ("flow node", "element przepływu"), "activity": ("activity", "czynność"),
              "data": ("data object or store", "obiekt lub magazyn danych"), "form": ("form", "formularz"),
              "decision": ("decision", "decyzja"), "resource": ("resource", "zasób"), "pool": ("pool", "basen")}


def kind_name(what):
    return KIND_NAMES.get(what, (what, what))


class Checker:
    def __init__(self, doc, level="full", bpmn=True):
        self.doc = doc
        self.level = level if level in LEVELS else "full"
        self.bpmn = bpmn
        self.diags = []

    def add(self, code, line, eid=None, **args):
        self.diags.append(Diag(code, line, eid, **args))

    def ref(self, id, line, what, ok):
        """Check that `id` names an element; `ok` is a predicate on that element."""
        el = self.doc.ids.get(id)
        if el is None:
            en, pl = kind_name(what)
            self.add("R002", line, id, what=en, what_pl=pl, id=id)
            return None
        if not ok(el):
            en, pl = kind_name(what)
            self.add("R003", line, id, id=id, actual=el.kind, expected=en, expected_pl=pl)
            return None
        return el

    # ------------------------------------------------------------------ run
    def run(self):
        d = self.doc
        self.attributes()
        self.structure()
        self.references()
        self.flows()
        if d.executable_any:
            self.executable()
        self.layer_03()
        self.decisions_and_forms()
        self.feel()
        self.simulation()
        self.levels()
        if self.bpmn and not any(x.severity == "error" for x in self.diags):
            self.bpmn_rules()
        return self.diags

    def attributes(self):
        d = self.doc
        for n in d.nodes.values():
            keys = M.NODE_KEYS.get(n.kind, ())
            flags = M.NODE_FLAGS.get(n.kind, ())
            for k in n.attrs:
                if k not in keys:
                    self.add("R010", n.line, n.id, key=k, kind=n.kind)
            for f in n.flags:
                if f not in flags:
                    self.add("R011", n.line, n.id, flag=f, kind=n.kind)
            if n.kind in M.EVENT_KINDS:
                trig = [k for k in M.TRIGGERS if k in n.attrs] + \
                    [f for f in M.TRIGGER_FLAGS + M.CATCH_ALL_FLAGS if f in n.flags]
                if len(trig) > 1:
                    self.add("R012", n.line, n.id, triggers=", ".join(trig))
        checks = [(d.flows, M.FLOW_KEYS, M.FLOW_FLAGS), (d.messages, M.MESSAGE_KEYS, ()),
                  (list(d.data.values()), M.DATA_KEYS, M.DATA_FLAGS), (d.notes, M.NOTE_KEYS, ()),
                  (d.groups, M.GROUP_KEYS, ()), (d.pools, M.POOL_KEYS, M.POOL_FLAGS),
                  (list(d.lanes.values()), (), ()), (list(d.decisions.values()), M.DECISION_KEYS, ()),
                  (list(d.resources.values()), M.RESOURCE_KEYS, ())]
        if d.simulate is not None:
            checks.append(([d.simulate], M.SIMULATE_KEYS, ()))
        for f in d.forms.values():
            checks.append(([x for x in f.items if x.kind == "field"], M.FIELD_KEYS, M.FIELD_FLAGS))
        for items, keys, flags in checks:
            for el in items:
                for k in el.attrs:
                    if k not in keys and k != "id":
                        self.add("R010", el.line, el.id, key=k, kind=el.kind)
                for f in el.flags:
                    if f not in flags:
                        self.add("R011", el.line, el.id, flag=f, kind=el.kind)
        for k in d.attrs:
            if k not in M.HEADER_KEYS:
                self.add("R010", d.header_line, None, key=k, kind="tok")
        for f in d.flags:
            if f not in M.HEADER_FLAGS:
                self.add("R011", d.header_line, None, flag=f, kind="tok")

    def structure(self):
        d = self.doc
        if d.pools:
            for i in d.top:
                if i in d.nodes:
                    self.add("R031", d.nodes[i].line, i, id=i)
        for p in d.pools:
            if any(c in d.lanes for c in p.children):
                for c in p.children:
                    if c in d.nodes:
                        self.add("R030", d.nodes[c].line, c, id=c)

    def references(self):
        d = self.doc
        is_activity = lambda e: e.kind in M.ACTIVITY_KINDS
        for n in d.nodes.values():
            a = n.attrs
            if n.kind == "boundary":
                if "on" not in a:
                    self.add("R032", n.line, n.id, id=n.id)
                else:
                    act = self.ref(a["on"], n.line, "activity", is_activity)
                    if act is not None:
                        n.scope = act.scope
            elif "on" in a:
                self.add("R013", n.line, n.id, key="on", requirement="a boundary event")
            if "form" in a:
                self.ref(a["form"], n.line, "form", lambda e: e.kind == "form")
                if n.kind not in ("user", "start"):
                    self.add("R013", n.line, n.id, key="form", requirement="a user task or a start event")
            if "decision" in a:
                self.ref(a["decision"], n.line, "decision", lambda e: e.kind == "decision")
                if n.kind != "rule":
                    self.add("R013", n.line, n.id, key="decision", requirement="a rule task")
            if "by" in a:
                self.ref(a["by"], n.line, "resource", lambda e: e.kind == "resource")
            if "expr" in a and n.kind != "script":
                self.add("R013", n.line, n.id, key="expr", requirement="a script task")
            if "process" in a and n.kind != "call":
                self.add("R013", n.line, n.id, key="process", requirement="a call activity")
            if "compensate" in a:
                h = self.ref(a["compensate"], n.line, "activity", is_activity)
                if h is not None and not h.has("compensation"):
                    self.add("R051", n.line, n.id,
                             detail="compensation handler '%s' needs the flag compensation" % h.id,
                             detail_pl="procedura kompensacji „%s” wymaga flagi compensation" % h.id)
            if "activity" in a:
                if not (n.kind in ("throw", "end") and n.has("compensate")):
                    self.add("R013", n.line, n.id, key="activity", requirement="a compensation throw or end event")
                else:
                    self.ref(a["activity"], n.line, "activity", is_activity)
        for x in d.assocs:
            self.ref(x.activity, x.line, "activity", is_activity)
            self.ref(x.data, x.line, "data", lambda e: e.kind in ("data", "store"))
        for n in d.notes:
            if "on" in n.attrs:
                self.ref(n.attrs["on"], n.line, "node", lambda e: True)
        for g in d.groups:
            for m in g.attrs.get("members", "").split():
                self.ref(m, g.line, "node", lambda e: e.kind in M.NODE_KINDS)

    def flows(self):
        d = self.doc
        is_node = lambda e: e.kind in M.NODE_KINDS
        for f in d.flows:
            s = self.ref(f.source, f.line, "node", is_node)
            t = self.ref(f.target, f.line, "node", is_node)
            if s is not None and t is not None and s.scope != t.scope:
                self.add("R021", f.line, f.id, source=f.source, target=f.target)
        for m in d.messages:
            ok = lambda e: e.kind in M.NODE_KINDS or e.kind == "pool"
            s = self.ref(m.source, m.line, "node", ok)
            t = self.ref(m.target, m.line, "node", ok)
            if s is not None and t is not None:
                ps = s.id if s.kind == "pool" else s.pool
                pt = t.id if t.kind == "pool" else t.pool
                if ps is None or pt is None or ps == pt:
                    self.add("R022", m.line, m.id, source=m.source, target=m.target)

    def executable(self):
        """Executable profile for Camunda 8 (codes shared with bpmn.py --camunda)."""
        d = self.doc
        outs = {}
        for f in d.flows:
            outs.setdefault(f.source, []).append(f)
        for n in d.nodes.values():
            a = n.attrs
            if n.kind == "user" and "form" not in a:
                self.diags.append(Diag("C011", n.line, n.id, severity="note",
                                       text_pl="Zadanie użytkownika „%s” nie ma formularza (form=)." % n.id,
                                       text_en="user task '%s' has no form (form=)" % n.id))
            elif n.kind in ("service", "send") and "type" not in a and a.get("connector") != "rest":
                self.diags.append(Diag("C020", n.line, n.id, severity="error",
                                       text_pl="%s „%s” wymaga type=\"…\" (typ zadania dla workera)." %
                                       ("Zadanie usługowe" if n.kind == "service" else "Zadanie wysyłania", n.id),
                                       text_en="%s task '%s' needs type=\"…\" (job type for a worker)" % (n.kind, n.id)))
            elif n.kind == "rule" and "decision" not in a:
                self.diags.append(Diag("C030", n.line, n.id, severity="error",
                                       text_pl="Zadanie reguły biznesowej „%s” wymaga decision=ID." % n.id,
                                       text_en="rule task '%s' needs decision=ID" % n.id))
            elif n.kind == "script" and "expr" not in a:
                self.diags.append(Diag("C040", n.line, n.id, severity="error",
                                       text_pl="Zadanie skryptowe „%s” wymaga expr=\"…\"." % n.id,
                                       text_en="script task '%s' needs expr=\"…\"" % n.id))
            if (n.kind in ("catch", "boundary") and "message" in a) or n.kind == "receive":
                if "correlation" not in a:
                    self.diags.append(Diag("C052", n.line, n.id, severity="error",
                                           text_pl="„%s” czeka na wiadomość i wymaga correlation=\"…\"." % n.id,
                                           text_en="'%s' waits for a message and needs correlation=\"…\"" % n.id))
            if n.kind in ("xor", "or"):
                fl = outs.get(n.id, [])
                if len(fl) > 1:
                    for f in fl:
                        if "when" not in f.attrs and not f.has("default"):
                            self.diags.append(Diag("C070", f.line, f.id, severity="error",
                                                   text_pl="Przepływ %s -> %s z bramki „%s” wymaga when=\"…\" albo "
                                                           "flagi default." % (f.source, f.target, n.id),
                                                   text_en="flow %s -> %s leaving gateway '%s' needs when=\"…\" or the "
                                                           "default flag" % (f.source, f.target, n.id)))

    # ------------------------------------------------------- language 0.3
    def node_executable(self, n):
        d = self.doc
        if d.executable:
            return True
        for p in d.pools:
            if p.id == n.pool and p.has("executable"):
                return True
        return False

    def layer_03(self):
        """Ad-hoc blocks, compensation handlers and the executable layer (spec §5.3, §10.3)."""
        d = self.doc
        incoming, outgoing = {}, {}
        for f in d.flows:
            outgoing.setdefault(f.source, []).append(f)
            incoming.setdefault(f.target, []).append(f)
        for n in d.nodes.values():
            a = n.attrs
            if n.kind == "adhoc":
                inner = [d.nodes[c] for c in n.children if c in d.nodes]
                if not any(x.kind in M.ACTIVITY_KINDS for x in inner) or \
                        any(x.kind in ("start", "end") for x in inner):
                    self.add("R050", n.line, n.id, id=n.id)
            if n.has("compensation") and (incoming.get(n.id) or outgoing.get(n.id)):
                self.add("R051", n.line, n.id,
                         detail="compensation handler '%s' must not have sequence flows" % n.id,
                         detail_pl="procedura kompensacji „%s” nie może mieć przepływów sekwencji" % n.id)
            if "events" in a and n.kind == "start" and n.scope in self.process_ids():
                self.add("R052", n.line, n.id, id=n.id)
            if "collection" in a and not (n.has("parallel") or n.has("sequential")):
                self.add("R054", n.line, n.id,
                         detail="collection= on '%s' needs the flag parallel or sequential" % n.id,
                         detail_pl="collection= w „%s” wymaga flagi parallel albo sequential" % n.id)
            if ("output" in a) != ("output-item" in a):
                self.add("R054", n.line, n.id,
                         detail="'%s' needs both output= and output-item=" % n.id,
                         detail_pl="„%s” wymaga jednocześnie output= i output-item=" % n.id)
            for key in ("inputs", "outputs"):
                if key in a:
                    for left, right in split_pairs(a[key], "="):
                        if right is None or not TARGET_RE.match(left) or not right:
                            self.add("R053", n.line, n.id, key=key, item=left if right is None else
                                     "%s = %s" % (left, right), expected="target = expression",
                                     expected_pl="cel = wyrażenie")
            if "headers" in a:
                for left, right in split_pairs(a["headers"], ":"):
                    if right is None or not left:
                        self.add("R053", n.line, n.id, key="headers", item=left, expected="key: value",
                                 expected_pl="klucz: wartość")
            if "listeners" in a:
                allowed = EXECUTION_EVENTS + (TASK_EVENTS if n.kind == "user" else ())
                for item in a["listeners"].split(";"):
                    if not item.strip():
                        continue
                    lst = parse_listener(item)
                    if lst is None or lst[0] not in allowed:
                        self.add("R053", n.line, n.id, key="listeners", item=item.strip(),
                                 expected="event: type with event one of " + ", ".join(allowed),
                                 expected_pl="zdarzenie: typ, gdzie zdarzenie to " + ", ".join(allowed))
            for key in M.USER_TASK_KEYS:
                if key in a and n.kind != "user":
                    self.add("R013", n.line, n.id, key=key, requirement="a user task")
            if "connector" in a:
                if n.kind != "service":
                    self.add("R013", n.line, n.id, key="connector", requirement="a service task")
                elif a["connector"] != "rest":
                    self.add("R013", n.line, n.id, key="connector", requirement="the value rest")
                elif "url" not in a or "method" not in a:
                    self.add("R013", n.line, n.id, key="connector=rest", requirement="method= and url=")
            for key in M.CONNECTOR_KEYS:
                if key in a and "connector" not in a:
                    self.add("R013", n.line, n.id, key=key, requirement="connector=rest")
            for key in ("active", "wait"):
                if (key in a or n.has(key)) and n.kind != "adhoc":
                    self.add("R013", n.line, n.id, key=key, requirement="an ad-hoc sub-process")
            if n.kind == "adhoc" and n.has("loop"):
                pass
            # executable-layer attributes outside executable documents
            if not self.node_executable(n):
                for key in M.EXEC_KEYS:
                    if key in a:
                        self.add("R055", n.line, n.id, key=key)
                for f in M.EXEC_FLAGS:
                    if n.has(f):
                        self.add("R055", n.line, n.id, key=f)
                if n.kind in M.EVENT_KINDS and "type" in a:
                    self.add("R055", n.line, n.id, key="type")
            else:
                self.executable_03(n)
        if "version-tag" in d.attrs and not d.executable:
            self.add("R055", d.header_line, None, key="version-tag")
        for p in d.pools:
            if "version-tag" in p.attrs and not (p.has("executable") or d.executable):
                self.add("R055", p.line, p.id, key="version-tag")

    def process_ids(self):
        d = self.doc
        ids = {d.default_process_id}
        for p in d.pools:
            ids.add(p.process_id)
        return ids

    def executable_03(self, n):
        a = n.attrs
        if n.kind in ("throw", "end") and "message" in a and "type" not in a:
            self.add("C022", n.line, n.id, kind=n.kind, id=n.id,
                     kind_pl={"throw": "rzucające", "end": "końcowe"}[n.kind])
        for key in ("due", "follow-up"):
            v = a.get(key)
            if v is not None and not v.startswith("=") and not ISO_ZONED.match(v):
                self.add("C090", n.line, n.id, key=key, value=v)
        v = a.get("priority")
        if v is not None and not v.startswith("="):
            ok = re.match(r"^\d+$", v) is not None and 0 <= int(v) <= 100
            if not ok:
                self.add("C091", n.line, n.id, value=v)

    def decisions_and_forms(self):
        d = self.doc
        try:
            from . import dmn
        except ImportError:
            dmn = None
        for dec in d.decisions.values():
            if dec.hit not in M.HIT_POLICIES:
                self.add("D003", dec.line, dec.id, hit=dec.hit)
            if not dec.inputs and not dec.outputs:
                self.add("D001", dec.line, dec.id, id=dec.id)
            elif not dec.rules:
                self.add("D005", dec.line, dec.id, id=dec.id)
            if dmn is not None and hasattr(dmn, "check_decision"):
                for code, line, args in dmn.check_decision(dec):
                    if code not in ("D001", "D003", "D005"):
                        self.add(code, line, dec.id, **args)
        try:
            from . import forms
        except ImportError:
            forms = None
        for f in d.forms.values():
            for it in f.items:
                if it.kind != "field":
                    continue
                t = it.attrs.get("type", "textfield")
                if t not in M.FIELD_TYPES:
                    self.add("M001", it.line, it.id, type=t)
                elif t in ("select", "radio") and "options" not in it.attrs:
                    self.add("M002", it.line, it.id, id=it.id, type=t)

    def feel(self):
        try:
            from . import feel
        except ImportError:
            return
        d = self.doc
        exprs = []
        for n in d.nodes.values():
            for k in ("expr", "condition", "correlation", "collection", "output-item", "until", "active"):
                if k in n.attrs:
                    exprs.append((n.attrs[k], n.line, n.id))
            for k in ("inputs", "outputs"):
                for left, right in split_pairs(n.attrs.get(k, ""), "="):
                    if right:
                        exprs.append((right, n.line, n.id))
            for k in ("assignee", "candidate-users", "candidate-groups", "due", "follow-up", "priority",
                      "process", "type", "timer", "business-id", "url", "body", "query", "headers-in"):
                v = n.attrs.get(k)
                if v is not None and v.startswith("=") and v[1:].strip():
                    exprs.append((v[1:].strip(), n.line, n.id))
        for f in d.flows:
            if "when" in f.attrs:
                exprs.append((f.attrs["when"], f.line, f.id))
        for text, line, id in exprs:
            res = feel.check_expr(text)
            if res is not None:
                code, detail = res
                self.add(code, line, id, detail=detail, detail_pl=feel.detail_pl(detail))

    def simulation(self):
        try:
            from . import sim
        except ImportError:
            return
        if not hasattr(sim, "check_sim"):
            return
        for code, line, id, args in sim.check_sim(self.doc):
            if code == "R002":
                if any(x.code == "R002" and x.line == line for x in self.diags):
                    continue
                args = dict(args, what="resource", what_pl="zasób")
            self.add(code, line, id, **args)

    def levels(self):
        if self.level == "full":
            return
        d = self.doc
        for n in d.nodes.values():
            exec_layer = any(k in n.attrs for k in M.EXEC_KEYS) or any(n.has(f) for f in M.EXEC_FLAGS) \
                or (n.kind in M.EVENT_KINDS and "type" in n.attrs)
            if self.level == "basic":
                bad = n.kind not in BASIC_KINDS or any(k in n.attrs for k in M.TRIGGERS) \
                    or any(f in n.flags for f in M.TRIGGER_FLAGS + M.CATCH_ALL_FLAGS + M.TASK_FLAGS) \
                    or "compensate" in n.attrs or exec_layer
            else:
                bad = any(k in n.attrs for k in ANALYTIC_EXCLUDED_KEYS) or exec_layer
            if bad:
                self.add("R040", n.line, n.id, keyword=n.kind, level=self.level)
        if self.level == "basic" and (d.decisions or d.forms or d.simulate):
            el = next(iter(list(d.decisions.values()) + list(d.forms.values()) + [d.simulate]), None)
            self.add("R040", el.line if el else 0, None, keyword=el.kind if el else "?", level=self.level)
        for p in d.pools:
            if "version-tag" in p.attrs:
                self.add("R040", p.line, p.id, keyword="version-tag", level=self.level)
        if "version-tag" in d.attrs:
            self.add("R040", d.header_line, None, keyword="version-tag", level=self.level)
        if self.level == "analytic" and (d.decisions or d.forms or d.executable_any):
            el = next(iter(list(d.decisions.values()) + list(d.forms.values())), None)
            self.add("R040", el.line if el else d.header_line, None,
                     keyword=el.kind if el else "executable", level=self.level)

    def bpmn_rules(self):
        from . import bpmnlib
        from .bpmn_export import export_tree
        d = self.doc
        try:
            tree = export_tree(d)
        except Exception as ex:  # pragma: no cover - exporter bug guard
            self.add("P008", 0, None, keyword="export", detail=str(ex))
            return
        m = bpmnlib.Model(tree)
        lines = {}
        for el in d.ids.values():
            lines[el.id] = el.line
        for issue in bpmnlib.validate(m):
            code = issue["kod"]
            if code.startswith(("D0", "K")):
                continue
            sev = {"błąd": "error", "ostrzeżenie": "warning", "uwaga": "note"}[issue["poziom"]]
            eid = issue.get("id")
            self.diags.append(Diag(code, lines.get(eid, 0), eid, text_pl=issue["komunikat"], severity=sev))


def check(doc, level="full", bpmn=True):
    """Return the list of diagnostics for a parsed document (parser diagnostics not included)."""
    return Checker(doc, level, bpmn).run()
