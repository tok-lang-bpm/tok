"""Parser: tokens -> Document.

The parser reads one statement per line. A statement that accepts a block
(pool, lane, sub, eventsub, decision, form) owns the following lines that
are indented deeper than itself.
"""

import re

from . import lexer as L
from .diagnostics import Diag
from .model import (Association, Data, Decision, Document, Element, Field, Flow, Form, FormText, Lane,
                    MessageFlow, Node, Pool, Rule, NODE_KINDS, SUB_KINDS)

ID_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")

BLOCK_KEYWORDS = ("pool", "lane", "sub", "eventsub", "decision", "form")

# which statements may appear in which context
_NODE_LIKE = set(NODE_KINDS) | {"flow", "message", "data", "store", "reads", "writes", "note"}
ALLOWED = {
    "top": _NODE_LIKE | {"pool", "group", "decision", "form", "simulate", "resource", "variable"},
    "pool": _NODE_LIKE | {"lane"},
    "lane": _NODE_LIKE | {"lane"},
    "sub": _NODE_LIKE,
    "eventsub": _NODE_LIKE,
    "decision": set(),
    "form": {"text", "field"},
}
KEYWORDS = set().union(*ALLOWED.values()) | {"tok"}


class Ctx:
    """Where a statement is: block kind, process/sub-process scope, lane, pool, owner element."""

    def __init__(self, kind, scope=None, lane=None, pool=None, owner=None):
        self.kind = kind
        self.scope = scope
        self.lane = lane
        self.pool = pool
        self.owner = owner


class Parser:
    def __init__(self, text):
        self.doc = Document()
        self.diags = []
        self.lines, errors = L.tokenize(text)
        for code, line, msg in errors:
            self.diags.append(Diag(code, line, detail=msg))
        self.pending = []
        self.flow_counter = {}

    # ----------------------------------------------------------------- helpers
    def err(self, code, line, eid=None, **args):
        self.diags.append(Diag(code, line, eid, **args))

    def take_comments(self, el, line):
        el.comments = self.pending
        el.trailing = line.comment
        self.pending = []

    def register(self, el, line_no):
        if el.id is None:
            return
        if not ID_RE.match(el.id):
            self.err("R004", line_no, el.id, id=el.id)
        other = self.doc.ids.get(el.id)
        if other is not None:
            self.err("R001", line_no, el.id, id=el.id, other=other.line)
            return False
        self.doc.ids[el.id] = el
        return True

    def parse_tail(self, toks, start, line_no, allow_label=True):
        """Parse [STRING] attrs from toks[start:]. Return (label, attrs, flags, ok)."""
        label, attrs, flags = None, {}, []
        i = start
        if allow_label and i < len(toks) and toks[i].kind == L.STRING:
            label = toks[i].value
            i += 1
        while i < len(toks):
            t = toks[i]
            if t.kind == L.WORD and i + 1 < len(toks) and toks[i + 1].kind == L.EQUALS:
                if i + 2 >= len(toks) or toks[i + 2].kind not in (L.WORD, L.NUMBER, L.STRING):
                    self.err("P007", line_no, near=t.value)
                    return label, attrs, flags, False
                attrs[t.value] = toks[i + 2].value
                i += 3
            elif t.kind == L.WORD:
                flags.append(t.value)
                i += 1
            else:
                self.err("P007", line_no, near=t.value)
                return label, attrs, flags, False
        return label, attrs, flags, True

    # ------------------------------------------------------------------ driver
    def parse(self):
        lines = self.lines
        i = 0
        while i < len(lines) and lines[i].standalone:
            self.pending.append(lines[i].comment)
            i += 1
        if i >= len(lines) or not lines[i].tokens or lines[i].tokens[0].value != "tok":
            self.err("P005", lines[i].number if i < len(lines) else 1)
        else:
            self.header(lines[i])
            i += 1
        self.block(i, -1, Ctx("top", scope=self.doc.default_process_id))
        self.doc.end_comments = self.pending
        self.pending = []
        return self.doc, self.diags

    def block(self, i, parent_indent, ctx):
        """Parse sibling statements indented deeper than parent_indent. Return next line index."""
        lines = self.lines
        indent = None
        prev = None          # (element, accepts_block, child ctx) of the previous statement
        while i < len(lines):
            line = lines[i]
            if line.standalone:
                self.pending.append(line.comment)
                i += 1
                continue
            if line.indent <= parent_indent:
                return i
            if indent is None:
                indent = line.indent
            if line.indent > indent:
                if prev is not None and prev[1] and prev[2] is None:
                    i = self.skip_block(i, indent)
                    prev = (prev[0], False, None)
                elif prev is not None and prev[1]:
                    prev[0].blackbox = False if prev[0].kind == "pool" else getattr(prev[0], "blackbox", None)
                    i = self.block(i, indent, prev[2])
                    prev = (prev[0], False, None)
                else:
                    kw = prev[0].kind if prev else "?"
                    self.err("P003", line.number, keyword=kw)
                    i = self.skip_block(i, indent)
                continue
            if line.indent < indent:
                self.err("T003", line.number)
                return i
            prev = self.statement(line, ctx)
            i += 1
        return i

    def skip_block(self, i, indent):
        while i < len(self.lines) and (self.lines[i].standalone or self.lines[i].indent > indent):
            i += 1
        return i

    # -------------------------------------------------------------- statements
    def header(self, line):
        toks = line.tokens
        d = self.doc
        d.header_line = line.number
        d.header_comments = self.pending
        d.header_trailing = line.comment
        self.pending = []
        if len(toks) < 2 or toks[1].kind != L.NUMBER:
            self.err("P005", line.number)
            return
        d.version = toks[1].value
        if d.version != "0.1":
            self.err("P006", line.number, version=d.version)
        d.title, d.attrs, d.flags, _ = self.parse_tail(toks, 2, line.number)

    def statement(self, line, ctx):
        """Parse one statement. Return (element, accepts_block, child_ctx)."""
        toks = line.tokens
        first = toks[0]
        kw = first.value if first.kind == L.WORD else None
        if first.kind == L.ROW:
            if ctx.kind == "decision":
                self.row(line, ctx.owner)
            else:
                self.err("P004", line.number, keyword="|", context=ctx.kind)
            return (Element("row", None), False, None)
        if kw == "tok":
            self.err("P010", line.number)
            return (Element("tok", None), False, None)
        if kw not in KEYWORDS:
            self.err("P001", line.number, word=first.value)
            return (Element("?", None), False, None)
        if kw not in ALLOWED[ctx.kind]:
            self.err("P004", line.number, keyword=kw, context=ctx.kind)
            return (Element(kw, None), False, None)
        handler = getattr(self, "st_" + kw, None)
        if handler is None:
            return self.st_node(line, ctx)
        return handler(line, ctx)

    def need_id(self, line, pos=1):
        toks = line.tokens
        if len(toks) <= pos or toks[pos].kind not in (L.WORD, L.NUMBER):
            self.err("P002", line.number, keyword=toks[0].value)
            return None
        return toks[pos].value

    def st_node(self, line, ctx):
        kind = line.tokens[0].value
        nid = self.need_id(line)
        if nid is None:
            return (Element(kind, None), kind in SUB_KINDS, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        node = Node(kind, nid, label, attrs, flags, line.number)
        self.take_comments(node, line)
        node.scope = ctx.scope
        node.pool = ctx.pool
        node.lane = ctx.lane if ctx.kind in ("lane",) else None
        if not self.register(node, line.number):
            return (node, kind in SUB_KINDS, None)
        self.doc.nodes[node.id] = node
        self.add_child(ctx, node.id)
        if kind in SUB_KINDS:
            return (node, True, Ctx(kind, scope=node.id, lane=None, pool=ctx.pool, owner=node))
        return (node, False, None)

    def add_child(self, ctx, id):
        if ctx.kind == "top":
            self.doc.top.append(id)
        elif ctx.kind in ("pool", "lane", "sub", "eventsub"):
            ctx.owner.children.append(id)

    def st_pool(self, line, ctx):
        pid = self.need_id(line)
        if pid is None:
            return (Element("pool", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        pool = Pool(pid, label, attrs, flags, line.number)
        self.take_comments(pool, line)
        if not self.register(pool, line.number):
            return (pool, True, None)
        self.doc.pools.append(pool)
        return (pool, True, Ctx("pool", scope=pool.process_id, lane=None, pool=pool.id, owner=pool))

    def st_lane(self, line, ctx):
        lid = self.need_id(line)
        if lid is None:
            return (Element("lane", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        lane = Lane(lid, label, attrs, flags, line.number)
        self.take_comments(lane, line)
        lane.pool = ctx.pool
        lane.parent = ctx.lane
        if not self.register(lane, line.number):
            return (lane, True, None)
        self.doc.lanes[lid] = lane
        ctx.owner.children.append(lid)
        return (lane, True, Ctx("lane", scope=ctx.scope, lane=lid, pool=ctx.pool, owner=lane))

    def st_flow(self, line, ctx):
        toks = line.tokens
        ids, i = [], 1
        while True:
            if i >= len(toks) or toks[i].kind not in (L.WORD, L.NUMBER):
                self.err("P008", line.number, keyword="flow", detail="expected an identifier")
                return (Element("flow", None), False, None)
            ids.append(toks[i].value)
            i += 1
            if i < len(toks) and toks[i].kind == L.ARROW:
                i += 1
                continue
            break
        if len(ids) < 2:
            self.err("P008", line.number, keyword="flow", detail="expected '->'")
            return (Element("flow", None), False, None)
        label, attrs, flags, _ = self.parse_tail(toks, i, line.number)
        if (label is not None or attrs or flags) and len(ids) > 2:
            self.err("R020", line.number)
            label, attrs, flags = None, {}, []
        first = None
        for k in range(len(ids) - 1):
            src, tgt = ids[k], ids[k + 1]
            explicit = attrs.get("id") if k == 0 else None
            fid = explicit or self.derive_flow_id(src, tgt)
            f = Flow(fid, src, tgt, label, dict(attrs), list(flags), line.number)
            f.explicit_id = explicit is not None
            f.attrs.pop("id", None)
            f.chain_start = k == 0
            if k == 0:
                self.take_comments(f, line)
                first = f
            self.register(f, line.number)
            self.doc.flows.append(f)
        return (first, False, None)

    def derive_flow_id(self, src, tgt):
        return self.derive_id("f-%s-%s" % (src, tgt))

    def derive_id(self, base):
        n = self.flow_counter.get(base, 0) + 1
        self.flow_counter[base] = n
        fid = base if n == 1 else "%s-%d" % (base, n)
        while fid in self.doc.ids:
            n += 1
            self.flow_counter[base] = n
            fid = "%s-%d" % (base, n)
        return fid

    def st_message(self, line, ctx):
        toks = line.tokens
        if len(toks) < 4 or toks[1].kind not in (L.WORD, L.NUMBER) or toks[2].kind != L.ARROW \
                or toks[3].kind not in (L.WORD, L.NUMBER):
            self.err("P008", line.number, keyword="message", detail="expected 'message A -> B'")
            return (Element("message", None), False, None)
        label, attrs, flags, _ = self.parse_tail(toks, 4, line.number)
        src, tgt = toks[1].value, toks[3].value
        mid = attrs.pop("id", None)
        m = MessageFlow(mid or self.derive_id("m-%s-%s" % (src, tgt)), src, tgt, label, attrs, line.number)
        m.explicit_id = mid is not None
        for fl in flags:
            self.err("R011", line.number, flag=fl, kind="message")
        self.take_comments(m, line)
        self.register(m, line.number)
        self.doc.messages.append(m)
        return (m, False, None)

    def st_data(self, line, ctx, kind="data"):
        did = self.need_id(line)
        if did is None:
            return (Element(kind, None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        d = Data(kind, did, label, attrs, flags, line.number)
        self.take_comments(d, line)
        if ctx.kind == "top":
            d.block = None
            d.scope = None
        else:
            d.scope = ctx.scope
            d.block = ctx.pool if ctx.kind == "lane" else ctx.owner.id
        if not self.register(d, line.number):
            return (d, False, None)
        self.doc.data[did] = d
        if ctx.kind == "lane":
            self.find_pool(ctx.pool).children.append(did)
        else:
            self.add_child(ctx, did)
        return (d, False, None)

    def st_store(self, line, ctx):
        return self.st_data(line, ctx, "store")

    def find_pool(self, pid):
        for p in self.doc.pools:
            if p.id == pid:
                return p
        return None

    def st_reads(self, line, ctx, kind="reads"):
        toks = line.tokens
        if len(toks) != 3 or toks[1].kind != L.WORD or toks[2].kind != L.WORD:
            self.err("P008", line.number, keyword=kind, detail="expected '%s ACTIVITY DATA'" % kind)
            return (Element(kind, None), False, None)
        a = Association(kind, toks[1].value, toks[2].value, line.number)
        self.take_comments(a, line)
        self.doc.assocs.append(a)
        return (a, False, None)

    def st_writes(self, line, ctx):
        return self.st_reads(line, ctx, "writes")

    def st_note(self, line, ctx):
        nid = self.need_id(line)
        if nid is None:
            return (Element("note", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        n = Element("note", nid, label, attrs, flags, line.number)
        self.take_comments(n, line)
        self.register(n, line.number)
        self.doc.notes.append(n)
        return (n, False, None)

    def st_group(self, line, ctx):
        gid = self.need_id(line)
        if gid is None:
            return (Element("group", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        g = Element("group", gid, label, attrs, flags, line.number)
        self.take_comments(g, line)
        self.register(g, line.number)
        self.doc.groups.append(g)
        return (g, False, None)

    def st_decision(self, line, ctx):
        did = self.need_id(line)
        if did is None:
            return (Element("decision", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        d = Decision(did, label, attrs, line.number)
        for fl in flags:
            self.err("R011", line.number, flag=fl, kind="decision")
        self.take_comments(d, line)
        self.register(d, line.number)
        self.doc.decisions[did] = d
        d._header_done = False
        return (d, True, Ctx("decision", owner=d))

    def row(self, line, d):
        cells = split_row(line.tokens[0].value)
        if cells is None:
            self.err("P009", line.number, detail="unbalanced quotes")
            return
        groups = [[]]
        for c in cells:
            if c == "":
                groups.append([])
            else:
                groups[-1].append(c.strip())
        if len(groups) < 2 or len(groups) > 3:
            self.err("P009", line.number, detail="expected inputs || outputs [|| annotation]")
            return
        ins, outs = groups[0], groups[1]
        ann = " ".join(x for x in groups[2] if x) if len(groups) == 3 else None
        if not d._header_done:
            d._header_done = True
            for cell in ins:
                p = split_header_cell(cell)
                if p is None:
                    self.err("D004", line.number, d.id, cell=cell)
                else:
                    d.inputs.append(p)
            for cell in outs:
                p = split_header_cell(cell)
                if p is None:
                    self.err("D004", line.number, d.id, cell=cell)
                else:
                    d.outputs.append(p)
            d.has_annotations = len(groups) == 3
            d.annotation_title = ann
            d.header_line = line.number
            return
        if len(ins) != len(d.inputs) or len(outs) != len(d.outputs):
            self.err("D002", line.number, d.id, got=len(ins) + len(outs), expected=len(d.inputs) + len(d.outputs))
            return
        r = Rule(ins, outs, ann or None, line.number)
        r.comments, r.trailing = self.pending, line.comment
        self.pending = []
        d.rules.append(r)

    def st_form(self, line, ctx):
        fid = self.need_id(line)
        if fid is None:
            return (Element("form", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        f = Form(fid, label, attrs, line.number)
        self.take_comments(f, line)
        self.register(f, line.number)
        self.doc.forms[fid] = f
        return (f, True, Ctx("form", owner=f))

    def st_text(self, line, ctx):
        toks = line.tokens
        if len(toks) != 2 or toks[1].kind != L.STRING:
            self.err("P008", line.number, keyword="text", detail='expected text "…"')
            return (Element("text", None), False, None)
        t = FormText(toks[1].value, line.number)
        self.take_comments(t, line)
        ctx.owner.items.append(t)
        return (t, False, None)

    def st_field(self, line, ctx):
        key = self.need_id(line)
        if key is None:
            return (Element("field", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        f = Field(key, label, attrs, flags, line.number)
        self.take_comments(f, line)
        ctx.owner.items.append(f)
        return (f, False, None)

    def st_simulate(self, line, ctx):
        label, attrs, flags, _ = self.parse_tail(line.tokens, 1, line.number, allow_label=False)
        s = Element("simulate", None, None, attrs, flags, line.number)
        self.take_comments(s, line)
        self.doc.simulate = s
        return (s, False, None)

    def st_resource(self, line, ctx):
        rid = self.need_id(line)
        if rid is None:
            return (Element("resource", None), False, None)
        label, attrs, flags, _ = self.parse_tail(line.tokens, 2, line.number)
        r = Element("resource", rid, label, attrs, flags, line.number)
        self.take_comments(r, line)
        self.register(r, line.number)
        self.doc.resources[rid] = r
        return (r, False, None)

    def st_variable(self, line, ctx):
        toks = line.tokens
        if len(toks) != 3 or toks[1].kind != L.WORD or toks[2].kind != L.STRING:
            self.err("P008", line.number, keyword="variable", detail='expected variable NAME "DIST"')
            return (Element("variable", None), False, None)
        v = Element("variable", toks[1].value, toks[2].value, {}, [], line.number)
        self.take_comments(v, line)
        self.doc.variables.append(v)
        return (v, False, None)


def split_row(text):
    """Split a table row into cells by '|' outside quotes. Empty string cells mark '||'."""
    s = text.strip()
    if s.startswith("|"):
        s = s[1:]
    if s.endswith("|") and not s.endswith("\\|"):
        s = s[:-1]
    cells, buf, in_str, i = [], [], False, 0
    while i < len(s):
        c = s[i]
        if c == "\\" and in_str and i + 1 < len(s):
            buf.append(c + s[i + 1])
            i += 2
            continue
        if c == '"':
            in_str = not in_str
        if c == "|" and not in_str:
            cells.append("".join(buf))
            buf = []
        else:
            buf.append(c)
        i += 1
    if in_str:
        return None
    cells.append("".join(buf))
    return cells


def split_header_cell(cell):
    if ":" not in cell:
        return None
    name, typ = cell.rsplit(":", 1)
    name, typ = name.strip(), typ.strip()
    if not name or typ not in ("string", "number", "boolean", "date"):
        return None
    return (name, typ)


def parse(text):
    """Parse tok text. Return (Document, list of Diag)."""
    return Parser(text).parse()
