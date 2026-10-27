"""Canonical printer (``tok fmt``). See spec §14."""

import re

from . import model as M

_BARE = re.compile(r"^(?:[A-Za-z_][A-Za-z0-9_.:/+-]*|-?[0-9]+(?:\.[0-9]+)?|[0-9][A-Za-z0-9_.:/+-]*)$")

# values of these keys are free text and always printed in quotes
QUOTED_KEYS = {"when", "expr", "condition", "correlation", "duration", "assign", "delay", "arrival",
               "options", "members", "message", "signal", "error", "escalation", "link", "code",
               "start"}


def q(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def value(key, v):
    v = str(v)
    if key in QUOTED_KEYS or not _BARE.match(v) or "->" in v:
        return q(v)
    return v


def tail(el, keys, flags, quote_type=False):
    """Label, flags and attributes of an element in canonical order."""
    parts = []
    if el.label is not None:
        parts.append(q(el.label))
    for f in flags:
        if f in el.flags:
            parts.append(f)
    for f in el.flags:
        if f not in flags:
            parts.append(f)
    for k in keys:
        if k in el.attrs:
            parts.append("%s=%s" % (k, value(k, el.attrs[k]) if not (k == "type" and quote_type) else q(el.attrs[k])))
    for k in sorted(el.attrs):
        if k not in keys:
            parts.append("%s=%s" % (k, value(k, el.attrs[k])))
    return parts


class Printer:
    def __init__(self, doc):
        self.doc = doc
        self.out = []

    def emit(self, indent, text, el=None):
        pad = "  " * indent
        if el is not None:
            for c in el.comments:
                self.out.append(pad + ("# " + c if c else "#"))
        line = pad + text
        if el is not None and el.trailing:
            line += "  # " + el.trailing
        self.out.append(line)

    def blank(self):
        if self.out and self.out[-1] != "":
            self.out.append("")

    def run(self):
        d = self.doc
        for c in d.header_comments:
            self.out.append("# " + c if c else "#")
        head = ["tok", d.version or "0.1"]
        if d.title is not None:
            head.append(q(d.title))
        head += [f for f in M.HEADER_FLAGS if f in d.flags] + [f for f in d.flags if f not in M.HEADER_FLAGS]
        head += ["%s=%s" % (k, value(k, d.attrs[k])) for k in M.HEADER_KEYS if k in d.attrs]
        head += ["%s=%s" % (k, value(k, d.attrs[k])) for k in sorted(d.attrs) if k not in M.HEADER_KEYS]
        line = " ".join(head)
        if d.header_trailing:
            line += "  # " + d.header_trailing
        self.out.append(line)

        if d.top:
            self.blank()
            self.children(d.top, 0)
        for p in d.pools:
            self.blank()
            self.pool(p)
        if d.flows:
            self.blank()
            self.flows()
        if d.messages:
            self.blank()
            for m in d.messages:
                parts = ["message", m.source, "->", m.target] + tail(m, M.MESSAGE_KEYS, ())
                if m.explicit_id:
                    parts.append("id=%s" % m.id)
                self.emit(0, " ".join(parts), m)
        if d.assocs:
            self.blank()
            for a in d.assocs:
                self.emit(0, "%s %s %s" % (a.kind, a.activity, a.data), a)
        if d.notes or d.groups:
            self.blank()
            for n in d.notes:
                self.emit(0, " ".join(["note", n.id] + tail(n, M.NOTE_KEYS, ())), n)
            for g in d.groups:
                self.emit(0, " ".join(["group", g.id] + tail(g, M.GROUP_KEYS, ())), g)
        for dec in d.decisions.values():
            self.blank()
            self.decision(dec)
        for f in d.forms.values():
            self.blank()
            self.form(f)
        if d.simulate or d.resources or d.variables:
            self.blank()
            if d.simulate:
                self.emit(0, " ".join(["simulate"] + tail(d.simulate, M.SIMULATE_KEYS, ())), d.simulate)
            for r in d.resources.values():
                self.emit(0, " ".join(["resource", r.id] + tail(r, M.RESOURCE_KEYS, ())), r)
            for v in d.variables:
                self.emit(0, "variable %s %s" % (v.id, q(v.label)), v)
        if d.end_comments:
            self.blank()
            for c in d.end_comments:
                self.out.append("# " + c if c else "#")
        while self.out and self.out[-1] == "":
            self.out.pop()
        return "\n".join(self.out) + "\n"

    def node(self, n, indent):
        keys = M.NODE_KEYS.get(n.kind, ())
        flags = M.NODE_FLAGS.get(n.kind, ())
        self.emit(indent, " ".join([n.kind, n.id] + tail(n, keys, flags, quote_type=True)), n)
        if n.kind in M.SUB_KINDS:
            self.children(n.children, indent + 1)

    def data(self, x, indent):
        self.emit(indent, " ".join([x.kind, x.id] + tail(x, M.DATA_KEYS, M.DATA_FLAGS)), x)

    def children(self, ids, indent):
        d = self.doc
        datas = []
        for i in ids:
            if i in d.nodes:
                self.node(d.nodes[i], indent)
            elif i in d.lanes:
                self.lane(d.lanes[i], indent)
            elif i in d.data:
                datas.append(d.data[i])
        for x in datas:
            self.data(x, indent)

    def pool(self, p):
        self.emit(0, " ".join(["pool", p.id] + tail(p, M.POOL_KEYS, M.POOL_FLAGS)), p)
        self.children(p.children, 1)

    def lane(self, lane, indent):
        self.emit(indent, " ".join(["lane", lane.id] + tail(lane, (), ())), lane)
        self.children(lane.children, indent + 1)

    def flows(self):
        flows = self.doc.flows
        out_count = {}
        for f in flows:
            out_count[f.source] = out_count.get(f.source, 0) + 1
        chain, chain_el = None, None

        def plain(f):
            return f.label is None and not f.attrs and not f.flags and not f.explicit_id

        for f in flows:
            if chain is not None and plain(f) and f.source == chain[-1] and out_count.get(f.source, 0) == 1 \
                    and not f.comments and not f.trailing:
                chain.append(f.target)
                continue
            if chain is not None:
                self.emit(0, "flow " + " -> ".join(chain), chain_el)
                chain = None
            if plain(f):
                chain, chain_el = [f.source, f.target], f
            else:
                parts = ["flow", f.source, "->", f.target] + tail(f, M.FLOW_KEYS, M.FLOW_FLAGS)
                if f.explicit_id:
                    parts.append("id=%s" % f.id)
                self.emit(0, " ".join(parts), f)
        if chain is not None:
            self.emit(0, "flow " + " -> ".join(chain), chain_el)

    def decision(self, dec):
        self.emit(0, " ".join(["decision", dec.id] + tail(dec, M.DECISION_KEYS, ())), dec)
        header = ["%s: %s" % c for c in dec.inputs], ["%s: %s" % c for c in dec.outputs]
        rows = [(header[0], header[1], (dec.annotation_title or "") if dec.has_annotations else None, None)]
        ann = any(r.annotation for r in dec.rules) or dec.has_annotations
        for r in dec.rules:
            rows.append((r.inputs, r.outputs, (r.annotation or "") if ann else None, r))
        ncol = len(dec.inputs) + len(dec.outputs)
        widths = [0] * ncol
        awidth = 0
        for ins, outs, a, _ in rows:
            for k, c in enumerate(list(ins) + list(outs)):
                if k < ncol:
                    widths[k] = max(widths[k], len(c))
            if a is not None:
                awidth = max(awidth, len(a))
        for ins, outs, a, r in rows:
            cells = list(ins) + list(outs)
            left = " | ".join(cells[k].ljust(widths[k]) for k in range(len(dec.inputs)))
            right = " | ".join(cells[k].ljust(widths[k]) for k in range(len(dec.inputs), ncol))
            text = "| %s || %s |" % (left, right)
            if ann:
                text += "| %s |" % (a or "").ljust(awidth)
            if r is not None:
                for c in r.comments:
                    self.out.append("  # " + c)
                if r.trailing:
                    text += "  # " + r.trailing
            self.out.append("  " + text.rstrip())

    def form(self, f):
        self.emit(0, " ".join(["form", f.id] + tail(f, (), ())), f)
        for it in f.items:
            if it.kind == "text":
                self.emit(1, "text " + q(it.label), it)
            else:
                self.emit(1, " ".join(["field", it.id] + tail(it, M.FIELD_KEYS, M.FIELD_FLAGS)), it)


def fmt(doc):
    """Return the canonical text of a parsed document."""
    return Printer(doc).run()
