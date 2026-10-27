"""Discrete-event simulation of tok models (spec §11).

The simulator runs one process: the process of the first pool that has one,
or the top-level process. Time is kept in seconds from the simulation start.
Every random draw comes from one ``random.Random(seed)``, so the same model
and seed always produce the same event log.
"""

import csv
import datetime
import heapq
import json
import math
import random
import re

from . import model as M

MAX_STEPS = 10000
DEFAULT_START = "2027-01-04T08:00"
DEFAULT_ARRIVAL = "exp(1h)"
DEFAULT_CASES = 100

UNITS = {"s": 1.0, "m": 60.0, "h": 3600.0, "d": 86400.0}
_DUR_RE = re.compile(r"^(\d+(?:\.\d+)?)\s*([smhd])$")
_NUM_RE = re.compile(r"^-?\d+(?:\.\d+)?$")
_CALL_RE = re.compile(r"^([a-z]+)\s*\((.*)\)$", re.S)
_ISO_RE = re.compile(r"^P(?:(\d+(?:\.\d+)?)Y)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)W)?(?:(\d+(?:\.\d+)?)D)?"
                     r"(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$")


# --------------------------------------------------------------- distributions

class DistError(Exception):
    def __init__(self, detail):
        Exception.__init__(self, detail)
        self.detail = detail


class Dist:
    """A parsed distribution. ``kind`` is 'duration', 'number', 'bool' or 'cat'."""

    def __init__(self, name, kind, args, labels=None):
        self.name = name
        self.kind = kind
        self.args = args
        self.labels = labels

    def sample(self, rng):
        n, a = self.name, self.args
        if n == "fixed":
            x = a[0]
        elif n == "unif":
            x = rng.uniform(a[0], a[1])
        elif n == "tri":
            x = rng.triangular(a[0], a[2], a[1])
        elif n == "norm":
            x = rng.gauss(a[0], a[1])
        elif n == "lnorm":
            mean, sd = a
            s2 = math.log(1.0 + (sd * sd) / (mean * mean))
            x = rng.lognormvariate(math.log(mean) - s2 / 2.0, math.sqrt(s2))
        elif n == "exp":
            x = rng.expovariate(1.0 / a[0])
        elif n == "bern":
            return rng.random() < a[0]
        elif n == "cat":
            u, acc = rng.random(), 0.0
            for label, p in zip(self.labels, a):
                acc += p
                if u < acc:
                    return label
            return self.labels[-1]
        else:
            raise DistError("unknown distribution " + n)
        if self.kind == "duration" and x < 0:
            x = 0.0
        return x

    def mean(self):
        """Expected value (numbers and durations only)."""
        n, a = self.name, self.args
        return {"fixed": lambda: a[0], "unif": lambda: (a[0] + a[1]) / 2.0,
                "tri": lambda: (a[0] + a[1] + a[2]) / 3.0, "norm": lambda: a[0],
                "lnorm": lambda: a[0], "exp": lambda: a[0]}.get(n, lambda: None)()


def _split_args(text):
    out, depth, buf = [], 0, []
    for c in text:
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        if c == "," and depth == 0:
            out.append("".join(buf).strip())
            buf = []
        else:
            buf.append(c)
    last = "".join(buf).strip()
    if last or out:
        out.append(last)
    return out


def _scalar(text):
    """Return ('duration', seconds) or ('number', value) or raise DistError."""
    t = text.strip()
    m = _DUR_RE.match(t)
    if m:
        return "duration", float(m.group(1)) * UNITS[m.group(2)]
    if _NUM_RE.match(t):
        return "number", float(t)
    raise DistError("'%s' is neither a number nor a duration like 30m" % t)


def _label_value(text):
    t = text.strip()
    if len(t) >= 2 and t[0] == t[-1] == '"':
        return t[1:-1]
    if t == "true":
        return True
    if t == "false":
        return False
    if _NUM_RE.match(t):
        return float(t)
    return t


ARITY = {"fixed": 1, "unif": 2, "tri": 3, "norm": 2, "lnorm": 2, "exp": 1, "bern": 1}


def parse_dist(text):
    """Parse a distribution: a duration (30m), a number, or a call such as tri(5m, 10m, 20m)."""
    if text is None or not str(text).strip():
        raise DistError("empty distribution")
    t = str(text).strip()
    m = _CALL_RE.match(t)
    if not m:
        kind, v = _scalar(t)
        return Dist("fixed", kind, [v])
    name, body = m.group(1), m.group(2)
    args = _split_args(body)
    if name == "cat":
        if not args:
            raise DistError("cat() needs at least one label=probability")
        labels, probs = [], []
        for a in args:
            if "=" not in a:
                raise DistError("cat() arguments must be label=probability, got '%s'" % a)
            label, p = a.rsplit("=", 1)
            kind, pv = _scalar(p)
            if kind != "number" or pv < 0 or pv > 1:
                raise DistError("probability of '%s' must be a number between 0 and 1" % label.strip())
            labels.append(_label_value(label))
            probs.append(pv)
        if abs(sum(probs) - 1.0) > 1e-6:
            raise DistError("cat() probabilities sum to %g, not 1" % sum(probs))
        return Dist("cat", "cat", probs, labels)
    if name not in ARITY:
        raise DistError("unknown distribution '%s'" % name)
    if len(args) != ARITY[name]:
        raise DistError("%s() takes %d argument(s), got %d" % (name, ARITY[name], len(args)))
    parsed = [_scalar(a) for a in args]
    kinds = set(k for k, _ in parsed)
    if len(kinds) > 1:
        raise DistError("mix of durations and numbers in %s()" % name)
    kind = kinds.pop()
    v = [x for _, x in parsed]
    if name == "bern":
        if kind != "number" or not 0 <= v[0] <= 1:
            raise DistError("bern(p) needs a probability between 0 and 1")
        return Dist("bern", "bool", v)
    if name == "unif" and v[0] > v[1]:
        raise DistError("unif(a, b) needs a <= b")
    if name == "tri" and not v[0] <= v[1] <= v[2]:
        raise DistError("tri(a, mode, b) needs a <= mode <= b")
    if name in ("norm", "lnorm") and v[1] < 0:
        raise DistError("standard deviation must not be negative")
    if name in ("lnorm", "exp") and v[0] <= 0:
        raise DistError("%s() needs a positive mean" % name)
    return Dist(name, kind, v)


def parse_assign(text):
    """'a=bern(0.8); b=cat(x=0.5, y=0.5)' -> [(name, Dist), ...]."""
    out = []
    for part in str(text).split(";"):
        part = part.strip()
        if not part:
            continue
        if "=" not in part:
            raise DistError("assign needs NAME=DIST, got '%s'" % part)
        name, dist = part.split("=", 1)
        name = name.strip()
        if not re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
            raise DistError("'%s' is not a variable name" % name)
        out.append((name, parse_dist(dist)))
    return out


def iso_duration(text):
    """Seconds of an ISO 8601 duration (P14D, PT2H30M, P1W). None if not a duration."""
    m = _ISO_RE.match(str(text).strip())
    if not m or str(text).strip() in ("P", "PT"):
        return None
    y, mo, w, d, h, mi, s = [float(x) if x else 0.0 for x in m.groups()]
    return (((y * 365 + mo * 30 + w * 7 + d) * 24 + h) * 60 + mi) * 60 + s


def timer_seconds(value, now_dt):
    """Waiting time of a timer: duration, cycle R…/period (first period) or a date-time."""
    v = str(value).strip()
    if v.startswith("R") and "/" in v:
        return iso_duration(v.split("/")[-1])
    d = iso_duration(v)
    if d is not None:
        return d
    try:
        target = datetime.datetime.fromisoformat(v)
    except ValueError:
        return None
    return max(0.0, (target - now_dt).total_seconds())


# ----------------------------------------------------------------- expressions

def eval_expr(text, ctx):
    """Evaluate a FEEL expression with tok.feel; fall back to a small built-in subset."""
    try:
        from . import feel
    except ImportError:
        feel = None
    if feel is not None and hasattr(feel, "parse_expr") and hasattr(feel, "evaluate"):
        try:
            v = feel.evaluate(feel.parse_expr(text), dict(ctx))
            if hasattr(feel, "to_python"):
                v = feel.to_python(v)
            return v
        except Exception:
            pass
    return _Fallback(text).parse(ctx)


def eval_decision(doc, decision_id, ctx):
    """Evaluate a decision table: tok.dmn when available, else a built-in subset (unique/first/collect)."""
    dec = doc.decisions.get(decision_id)
    if dec is None:
        return None
    try:
        from . import dmn
    except ImportError:
        dmn = None
    if dmn is not None and hasattr(dmn, "evaluate_decision"):
        try:
            return dmn.evaluate_decision(dec, dict(ctx))
        except Exception:
            pass
    matches = []
    for r in dec.rules:
        ok = True
        for (expr, _typ), cell in zip(dec.inputs, r.inputs):
            if not _fallback_test(cell, eval_expr(expr, ctx), ctx):
                ok = False
                break
        if ok:
            out = {name: eval_expr(cell, ctx) for (name, _t), cell in zip(dec.outputs, r.outputs)}
            matches.append(out[dec.outputs[0][0]] if len(out) == 1 else out)
            if dec.hit in ("unique", "first", "any", "priority"):
                break
    if dec.hit == "collect":
        return matches
    return matches[0] if matches else None


def _fallback_test(cell, value, ctx):
    c = cell.strip()
    if c == "-":
        return True
    for part in _split_args(c):
        p = part.strip()
        m = re.match(r"^([\[\(\]])(.+)\.\.(.+)([\]\)\[])$", p)
        if m:
            lo, hi = eval_expr(m.group(2), ctx), eval_expr(m.group(3), ctx)
            lo_ok = value >= lo if m.group(1) == "[" else value > lo
            hi_ok = value <= hi if m.group(4) == "]" else value < hi
            if value is not None and lo_ok and hi_ok:
                return True
            continue
        m = re.match(r"^(<=|>=|<|>|!=|=)\s*(.+)$", p)
        if m:
            if _Fallback("x %s y" % m.group(1)).compare(m.group(1), value, eval_expr(m.group(2), ctx)):
                return True
            continue
        if value == eval_expr(p, ctx):
            return True
    return False


class _Fallback:
    """Tiny FEEL subset: literals, names (a.b), + - * /, comparisons, and, or, not(), parentheses."""
    _TOK = re.compile(r'\s*(?:(\d+(?:\.\d+)?)|("(?:[^"\\]|\\.)*")|(<=|>=|!=|[-+*/()=<>.,])|([A-Za-z_][A-Za-z0-9_]*))')

    def __init__(self, text):
        self.toks, pos = [], 0
        text = text.strip()
        while pos < len(text):
            m = self._TOK.match(text, pos)
            if not m or m.end() == pos:
                raise ValueError("cannot read expression: " + text)
            pos = m.end()
            num, s, op, name = m.groups()
            if num is not None:
                self.toks.append(("num", float(num)))
            elif s is not None:
                self.toks.append(("str", s[1:-1].replace('\\"', '"').replace("\\\\", "\\")))
            elif op is not None:
                self.toks.append(("op", op))
            elif name is not None:
                self.toks.append(("name", name))
        self.i = 0

    def peek(self):
        return self.toks[self.i] if self.i < len(self.toks) else (None, None)

    def take(self):
        t = self.peek()
        self.i += 1
        return t

    def parse(self, ctx):
        self.ctx = ctx
        v = self.or_()
        if self.i != len(self.toks):
            raise ValueError("unexpected token")
        return v

    def or_(self):
        v = self.and_()
        while self.peek() == ("name", "or"):
            self.take()
            r = self.and_()
            v = bool(v) or bool(r)
        return v

    def and_(self):
        v = self.cmp()
        while self.peek() == ("name", "and"):
            self.take()
            r = self.cmp()
            v = bool(v) and bool(r)
        return v

    @staticmethod
    def compare(op, a, b):
        if op == "=":
            return a == b
        if op == "!=":
            return a != b
        if a is None or b is None:
            return False
        try:
            return {"<": a < b, "<=": a <= b, ">": a > b, ">=": a >= b}[op]
        except TypeError:
            return False

    def cmp(self):
        v = self.add()
        k, op = self.peek()
        if k == "op" and op in ("=", "!=", "<", "<=", ">", ">="):
            self.take()
            return self.compare(op, v, self.add())
        return v

    def add(self):
        v = self.mul()
        while self.peek() in (("op", "+"), ("op", "-")):
            op = self.take()[1]
            r = self.mul()
            v = (v + r) if op == "+" else (v - r)
        return v

    def mul(self):
        v = self.unary()
        while self.peek() in (("op", "*"), ("op", "/")):
            op = self.take()[1]
            r = self.unary()
            v = (v * r) if op == "*" else (v / r)
        return v

    def unary(self):
        if self.peek() == ("op", "-"):
            self.take()
            return -self.unary()
        return self.primary()

    def primary(self):
        k, v = self.take()
        if k in ("num", "str"):
            return v
        if k == "op" and v == "(":
            r = self.or_()
            self.take()
            return r
        if k == "name":
            if v == "true":
                return True
            if v == "false":
                return False
            if v == "null":
                return None
            if v == "not" and self.peek() == ("op", "("):
                self.take()
                r = self.or_()
                self.take()
                return not r
            val = self.ctx.get(v)
            while self.peek() == ("op", "."):
                self.take()
                key = self.take()[1]
                val = val.get(key) if isinstance(val, dict) else None
            return val
        raise ValueError("unexpected token")


# ---------------------------------------------------------------- validation

def _dist_check(out, text, line, id, assign=False):
    try:
        if assign:
            parse_assign(text)
        else:
            parse_dist(text)
    except DistError as ex:
        out.append(("S001", line, id, {"text": text, "detail": ex.detail}))


def check_sim(doc):
    """Simulation diagnostics as (code, line, id, args) tuples."""
    out = []
    if doc.simulate is not None and "arrival" in doc.simulate.attrs:
        _dist_check(out, doc.simulate.attrs["arrival"], doc.simulate.line, None)
    for v in doc.variables:
        _dist_check(out, v.label, v.line, v.id)
    for n in doc.nodes.values():
        for key in ("duration", "delay"):
            if key in n.attrs:
                _dist_check(out, n.attrs[key], n.line, n.id)
        if "assign" in n.attrs:
            _dist_check(out, n.attrs["assign"], n.line, n.id, assign=True)
        if "by" in n.attrs and n.attrs["by"] not in doc.resources:
            out.append(("R002", n.line, n.id, {"what": "resource", "id": n.attrs["by"]}))
    for m in doc.messages:
        if "delay" in m.attrs:
            _dist_check(out, m.attrs["delay"], m.line, m.id)
    outgoing = {}
    for f in doc.flows:
        outgoing.setdefault(f.source, []).append(f)
        if "p" in f.attrs:
            try:
                p = float(f.attrs["p"])
                if not 0 <= p <= 1:
                    raise ValueError
            except ValueError:
                out.append(("S001", f.line, f.id, {"text": "p=" + str(f.attrs["p"]),
                                                   "detail": "a probability must be between 0 and 1"}))
    for n in doc.nodes.values():
        if n.kind not in ("xor", "or"):
            continue
        flows = outgoing.get(n.id, [])
        ps = [f for f in flows if "p" in f.attrs]
        if n.kind == "xor" and ps:
            try:
                total = sum(float(f.attrs["p"]) for f in ps)
            except ValueError:
                total = None
            if total is not None and abs(total - 1.0) > 1e-6:
                out.append(("S002", n.line, n.id, {"id": n.id, "total": round(total, 6)}))
        if doc.simulate is not None and len(flows) >= 2 and not ps \
                and not any("when" in f.attrs for f in flows):
            out.append(("S003", n.line, n.id, {"id": n.id}))
    return out


# ------------------------------------------------------------------ simulation

class SimError(Exception):
    pass


class _Resource:
    def __init__(self, id, label, count):
        self.id, self.label, self.count = id, label, count
        self.busy = 0
        self.queue = []
        self.busy_time = 0.0


class _Frame:
    """Tokens of one process level of a case: the case itself or one embedded sub-process instance."""

    def __init__(self, case, scope, parent=None, ai=None):
        self.case, self.scope, self.parent, self.ai = case, scope, parent, ai
        self.active = 0
        self.alive = True
        self.joins = {}
        self.or_stack = []
        self.ais = []
        self.waiters = []
        self.children = []
        self.last_end = None
        self.until = None         # ad-hoc sub-process: (expression, wait flag)


class _Case:
    def __init__(self, n, start):
        self.id = n
        self.start = start
        self.end = None
        self.end_event = None
        self.vars = {}
        self.steps = 0
        self.top = None
        self.compensable = []     # (activity id, handler id) of completed activities, in completion order


class _AI:
    """An activity instance (task or sub-process)."""

    def __init__(self, frame, node, t):
        self.frame, self.node = frame, node
        self.enqueue, self.start, self.end = t, None, None
        self.resource = None
        self.alive = True
        self.timers = []
        self.child = None
        self.local = None         # multi-instance: variables of one instance
        self.body = None          # multi-instance: the body this instance belongs to
        self.instances = None     # multi-instance body: list of instance AIs
        self.silent = False       # not written to the event log (compensation throw)


class _Waiter:
    def __init__(self):
        self.alive = True


class Simulator:
    def __init__(self, doc, cases=None, seed=None):
        self.doc = doc
        attrs = doc.simulate.attrs if doc.simulate is not None else {}
        self.n_cases = int(float(cases if cases is not None else attrs.get("cases", DEFAULT_CASES)))
        self.seed = int(float(seed if seed is not None else attrs.get("seed", 1)))
        self.rng = random.Random(self.seed)
        self.t0 = datetime.datetime.fromisoformat(str(attrs.get("start", DEFAULT_START)))
        self.arrival = parse_dist(attrs.get("arrival", DEFAULT_ARRIVAL))
        self.variables = [(v.id, parse_dist(v.label)) for v in doc.variables]
        self.resources = {}
        for r in doc.resources.values():
            self.resources[r.id] = _Resource(r.id, r.label, int(float(r.attrs.get("count", 1))))
        self.scope = self._main_scope()
        self.out = {}
        self.inc = {}
        for f in doc.flows:
            s, tg = doc.nodes.get(f.source), doc.nodes.get(f.target)
            if s is None or tg is None:
                continue
            self.out.setdefault(f.source, []).append(f)
            self.inc[f.target] = self.inc.get(f.target, 0) + 1
        self.boundaries = {}
        for n in doc.nodes.values():
            if n.kind == "boundary" and "on" in n.attrs:
                self.boundaries.setdefault(n.attrs["on"], []).append(n)
        self.dist_cache = {}
        self.heap = []
        self.seq = 0
        self.t = 0.0
        self.cases = []
        self.rows = []
        self.warnings = []

    def _main_scope(self):
        for p in self.doc.pools:
            if not p.blackbox:
                return p.process_id
        return self.doc.default_process_id

    def dist(self, text):
        d = self.dist_cache.get(text)
        if d is None:
            d = self.dist_cache[text] = parse_dist(text)
        return d

    def start_of(self, scope):
        starts = [n for n in self.doc.nodes.values() if n.scope == scope and n.kind == "start"]
        plain = [n for n in starts if not any(k in n.attrs for k in M.TRIGGERS)]
        pick = plain or starts
        return pick[0].id if pick else None

    def schedule(self, dt, fn, *args):
        self.seq += 1
        heapq.heappush(self.heap, (self.t + max(0.0, dt), self.seq, fn, args))

    def now_dt(self):
        return self.t0 + datetime.timedelta(seconds=self.t)

    # --------------------------------------------------------------- driver
    def run(self):
        start = self.start_of(self.scope)
        if start is None:
            raise SimError("the process '%s' has no start event" % self.scope)
        self.main_start = start
        if self.n_cases > 0:
            self.schedule(0.0, self.arrive, 0)
        while self.heap:
            t, _s, fn, args = heapq.heappop(self.heap)
            self.t = t
            fn(*args)
        self.horizon = self.t
        return Result(self)

    def arrive(self, k):
        case = _Case(k + 1, self.t)
        for name, d in self.variables:
            case.vars[name] = d.sample(self.rng)
        self.cases.append(case)
        frame = _Frame(case, self.scope)
        case.top = frame
        frame.active = 1
        self.enter(frame, self.main_start)
        if k + 1 < self.n_cases:
            self.schedule(self.arrival.sample(self.rng), self.arrive, k + 1)

    # --------------------------------------------------------------- tokens
    def consume(self, frame):
        frame.active -= 1
        if frame.until is not None and frame.alive and frame.active > 0:
            expr, wait = frame.until
            if not wait and self.truth(expr, frame.case.vars, "until of '%s'" % frame.scope):
                self.kill(frame)
                frame.alive = True
                frame.active = 0
        if frame.active <= 0 and frame.alive:
            self.frame_done(frame)

    def frame_done(self, frame):
        frame.alive = False
        if frame.parent is None:
            case = frame.case
            if case.end is None:
                case.end = self.t
                pending = any(v for v in frame.joins.values())
                case.end_event = frame.last_end if frame.last_end else ("deadlock" if pending else None)
        else:
            self.finish_ai(frame.ai)

    def kill(self, frame):
        frame.alive = False
        for w in frame.waiters:
            w.alive = False
        for ai in frame.ais:
            if ai.alive:
                ai.alive = False
                self.release(ai)
                for w in ai.timers:
                    w.alive = False
        for ch in frame.children:
            if ch.alive:
                self.kill(ch)

    def enter(self, frame, node_id):
        if not frame.alive:
            return
        case = frame.case
        case.steps += 1
        if case.steps > MAX_STEPS:
            top = case.top
            self.kill(top)
            if case.end is None:
                case.end, case.end_event = self.t, "loop-limit"
            return
        n = self.doc.nodes[node_id]
        k = n.kind
        if k in ("throw", "end") and n.has("compensate") and self.start_compensation(frame, n):
            return
        if k == "end":
            frame.last_end = n.id
            if n.has("terminate"):
                self.kill(frame)
                frame.alive = True
                frame.active = 0
                self.frame_done(frame)
            else:
                self.consume(frame)
        elif k in ("start", "throw", "boundary"):
            self.leave(frame, n.id)
        elif k == "catch":
            wait = self.event_wait(n)
            if wait is None or wait <= 0:
                self.leave(frame, n.id)
            else:
                w = _Waiter()
                frame.waiters.append(w)
                self.schedule(wait, self.wake, frame, w, n.id)
        elif k in M.GATEWAY_KINDS:
            self.gateway(frame, n)
        elif k == "sub":
            self.start_sub(frame, n)
        elif k == "adhoc":
            self.start_adhoc(frame, n)
        elif k == "eventsub":
            self.consume(frame)
        elif (n.has("parallel") or n.has("sequential")) and "collection" in n.attrs:
            self.start_multi(frame, n)
        else:
            self.start_activity(frame, n)

    def wake(self, frame, waiter, node_id):
        if waiter.alive and frame.alive:
            waiter.alive = False
            self.leave(frame, node_id)

    def event_wait(self, n):
        a = n.attrs
        if "timer" in a:
            w = timer_seconds(a["timer"], self.now_dt())
            if w is None:
                self.warnings.append("timer of '%s' is not understood: %s" % (n.id, a["timer"]))
            return w
        if "delay" in a:
            return self.dist(a["delay"]).sample(self.rng)
        if "duration" in a:
            return self.dist(a["duration"]).sample(self.rng)
        return 0.0

    def leave(self, frame, node_id):
        """Move the token that is at node_id along its outgoing flows."""
        flows = self.out.get(node_id, [])
        if not flows:
            self.consume(frame)
            return
        if len(flows) == 1:
            self.enter(frame, flows[0].target)
            return
        ctx = frame.case.vars
        taken = [f for f in flows if not f.has("default") and ("when" not in f.attrs or self.cond(f, ctx))]
        if not taken:
            taken = [f for f in flows if f.has("default")] or flows[:1]
        self.route(frame, [f.target for f in taken])

    def route(self, frame, targets):
        if not targets:
            self.consume(frame)
            return
        frame.active += len(targets) - 1
        for tg in targets:
            self.enter(frame, tg)

    def cond(self, f, ctx):
        try:
            return bool(eval_expr(f.attrs["when"], ctx))
        except Exception as ex:
            self.warnings.append("condition of flow '%s' could not be evaluated: %s" % (f.id, ex))
            return False

    # ------------------------------------------------------------- gateways
    def gateway(self, frame, n):
        nin = self.inc.get(n.id, 0)
        ctx = frame.case.vars
        if nin > 1 and n.kind in ("and", "or"):
            if n.kind == "and":
                expected = nin
            else:
                expected = frame.or_stack[-1] if frame.or_stack else nin
            got = frame.joins.get(n.id, 0) + 1
            if got < expected:
                frame.joins[n.id] = got
                self.consume(frame)
                return
            frame.joins[n.id] = 0
            if n.kind == "or" and frame.or_stack:
                frame.or_stack.pop()
        flows = self.out.get(n.id, [])
        if len(flows) <= 1:
            self.leave(frame, n.id)
            return
        if n.kind == "and":
            self.route(frame, [f.target for f in flows])
        elif n.kind == "xor":
            self.route(frame, [self.xor_choice(flows, ctx)])
        elif n.kind == "or":
            targets = self.or_choice(flows, ctx)
            frame.or_stack.append(len(targets))
            self.route(frame, targets)
        else:
            self.race(frame, n, flows)

    def xor_choice(self, flows, ctx):
        for f in flows:
            if "when" in f.attrs and not f.has("default") and self.cond(f, ctx):
                return f.target
        ps = [f for f in flows if "p" in f.attrs]
        if ps:
            u, acc = self.rng.random(), 0.0
            for f in ps:
                acc += float(f.attrs["p"])
                if u < acc:
                    return f.target
        for f in flows:
            if f.has("default"):
                return f.target
        if ps:
            return ps[-1].target
        return flows[0].target

    def or_choice(self, flows, ctx):
        chosen = []
        for f in flows:
            if f.has("default"):
                continue
            if "when" in f.attrs:
                if self.cond(f, ctx):
                    chosen.append(f.target)
            elif "p" in f.attrs:
                if self.rng.random() < float(f.attrs["p"]):
                    chosen.append(f.target)
        if not chosen:
            chosen = [f.target for f in flows if f.has("default")] or [flows[0].target]
        return chosen

    def race(self, frame, n, flows):
        best, best_t = None, None
        for f in flows:
            target = self.doc.nodes[f.target]
            if target.kind == "receive":
                a = target.attrs
                w = self.dist(a["delay"]).sample(self.rng) if "delay" in a else (
                    self.dist(a["duration"]).sample(self.rng) if "duration" in a else None)
            else:
                a = target.attrs
                w = self.event_wait(target) if ("timer" in a or "delay" in a) else None
            if w is not None and (best_t is None or w < best_t):
                best, best_t = target, w
        if best is None:
            self.warnings.append("event-based gateway '%s': no event can happen (add delay= to message events)" % n.id)
            return
        w = _Waiter()
        frame.waiters.append(w)
        if best.kind == "receive":
            ai = _AI(frame, best, self.t)
            frame.ais.append(ai)
            self.schedule(best_t, self.race_receive, frame, w, ai)
        else:
            self.schedule(best_t, self.wake, frame, w, best.id)

    def race_receive(self, frame, waiter, ai):
        if waiter.alive and frame.alive:
            ai.start = self.t
            self.complete(ai)

    # ----------------------------------------------------------- activities
    def start_activity(self, frame, n):
        ai = _AI(frame, n, self.t)
        frame.ais.append(ai)
        self.arm_boundaries(ai)
        rid = n.attrs.get("by")
        if rid and rid in self.resources:
            res = self.resources[rid]
            ai.resource = res
            if res.busy < res.count:
                self.begin(ai)
            else:
                res.queue.append(ai)
        else:
            self.begin(ai)

    def begin(self, ai):
        ai.start = self.t
        if ai.resource is not None:
            ai.resource.busy += 1
        a = ai.node.attrs
        dur = self.dist(a["duration"]).sample(self.rng) if "duration" in a else 0.0
        self.schedule(dur, self.done, ai)

    def done(self, ai):
        if ai.alive and ai.frame.alive:
            self.complete(ai)

    def complete(self, ai):
        n = ai.node
        case = ai.frame.case
        ai.alive = False
        ai.end = self.t
        self.release(ai)
        for w in ai.timers:
            w.alive = False
        a = n.attrs
        # an instance of a multi-instance activity writes into its own variables
        target = ai.local if ai.local is not None else case.vars
        ctx = dict(case.vars, **ai.local) if ai.local is not None else case.vars
        if "assign" in a:
            for name, d in parse_assign(a["assign"]):
                target[name] = d.sample(self.rng)
        if n.kind == "rule" and "decision" in a:
            dec = self.doc.decisions.get(a["decision"])
            res = a.get("result") or (dec.outputs[0][0] if dec is not None and len(dec.outputs) == 1 else "result")
            try:
                target[res] = eval_decision(self.doc, a["decision"], ctx)
            except Exception as ex:
                self.warnings.append("decision '%s' could not be evaluated: %s" % (a["decision"], ex))
        if n.kind == "script" and "expr" in a:
            try:
                target[a.get("result", "result")] = eval_expr(a["expr"], ctx)
            except Exception as ex:
                self.warnings.append("expression of '%s' could not be evaluated: %s" % (n.id, ex))
        if not ai.silent:
            self.log(ai, "completed")
        if ai.body is not None:
            self.instance_done(ai)
            return
        if "compensate" in a and ai.instances is None:
            case.compensable.append((n.id, a["compensate"]))
        self.leave(ai.frame, n.id)

    def release(self, ai):
        res = ai.resource
        if res is None or ai.start is None or getattr(ai, "_released", False):
            if res is not None and ai.start is None and ai in res.queue:
                res.queue.remove(ai)
            return
        ai._released = True
        res.busy -= 1
        res.busy_time += self.t - ai.start
        while res.queue and res.busy < res.count:
            nxt = res.queue.pop(0)
            if nxt.alive and nxt.frame.alive:
                self.begin(nxt)

    def log(self, ai, status):
        res = ai.resource.id if ai.resource is not None else ""
        self.rows.append((ai.frame.case.id, ai.node.id, ai.node.label or "", res,
                          ai.enqueue, ai.start, ai.end, status))

    def arm_boundaries(self, ai):
        for b in self.boundaries.get(ai.node.id, []):
            a = b.attrs
            if "timer" in a:
                w = timer_seconds(a["timer"], self.now_dt())
            elif "delay" in a:
                w = self.dist(a["delay"]).sample(self.rng)
            else:
                w = None
            if w is None:
                continue
            wt = _Waiter()
            ai.timers.append(wt)
            self.schedule(w, self.boundary_fire, ai, b, wt)

    def boundary_fire(self, ai, b, waiter):
        frame = ai.frame
        if not waiter.alive or not ai.alive or not frame.alive:
            return
        waiter.alive = False
        if b.has("noninterrupting"):
            frame.active += 1
            self.leave(frame, b.id)
            return
        ai.alive = False
        ai.end = self.t
        self.release(ai)
        for w in ai.timers:
            w.alive = False
        if ai.child is not None:
            self.kill(ai.child)
        self.log(ai, "interrupted")
        self.leave(frame, b.id)

    def start_sub(self, frame, n):
        start = self.start_of(n.id)
        if start is None:
            self.start_activity(frame, n)
            return
        ai = _AI(frame, n, self.t)
        ai.start = self.t
        frame.ais.append(ai)
        self.arm_boundaries(ai)
        child = _Frame(frame.case, n.id, frame, ai)
        ai.child = child
        frame.children.append(child)
        child.active = 1
        self.enter(child, start)

    def finish_ai(self, ai):
        if ai.alive and ai.frame.alive:
            self.complete(ai)

    # ------------------------------------------------------ language 0.3
    def truth(self, expr, ctx, what):
        try:
            return bool(eval_expr(expr, ctx))
        except Exception as ex:
            self.warnings.append("%s could not be evaluated: %s" % (what, ex))
            return False

    def start_multi(self, frame, n):
        """Multi-instance activity with an input collection: one instance per element."""
        a = n.attrs
        try:
            items = eval_expr(a["collection"], frame.case.vars)
        except Exception as ex:
            self.warnings.append("collection of '%s' could not be evaluated: %s" % (n.id, ex))
            items = None
        if not isinstance(items, list):
            self.start_activity(frame, n)
            return
        body = _AI(frame, n, self.t)
        body.start = self.t
        body.instances = []
        body.items = items
        body.results = [None] * len(items)
        body.completed = 0
        body.next = 0
        frame.ais.append(body)
        self.arm_boundaries(body)
        if not items:
            self.finish_body(body)
            return
        k = len(items) if n.has("parallel") else 1
        for _ in range(k):
            self.next_instance(body)

    def next_instance(self, body):
        n = body.node
        i = body.next
        body.next += 1
        inst = _AI(body.frame, n, self.t)
        inst.body = body
        inst.index = i
        inst.local = {}
        if "item" in n.attrs:
            inst.local[n.attrs["item"]] = body.items[i]
        inst.local["loopCounter"] = i + 1
        body.instances.append(inst)
        body.frame.ais.append(inst)
        rid = n.attrs.get("by")
        if rid and rid in self.resources:
            res = self.resources[rid]
            inst.resource = res
            if res.busy < res.count:
                self.begin(inst)
            else:
                res.queue.append(inst)
        else:
            self.begin(inst)

    def instance_done(self, inst):
        body = inst.body
        if not body.alive or not body.frame.alive:
            return
        n = body.node
        a = n.attrs
        body.completed += 1
        ctx = dict(body.frame.case.vars, **inst.local)
        if "output-item" in a:
            try:
                body.results[inst.index] = eval_expr(a["output-item"], ctx)
            except Exception as ex:
                self.warnings.append("output-item of '%s' could not be evaluated: %s" % (n.id, ex))
        total = len(body.items)
        active = sum(1 for x in body.instances if x.alive)
        if "until" in a:
            uctx = dict(ctx, numberOfInstances=total, numberOfCompletedInstances=body.completed,
                        numberOfActiveInstances=active, numberOfTerminatedInstances=0)
            if self.truth(a["until"], uctx, "until of '%s'" % n.id):
                for x in body.instances:
                    if x.alive:
                        x.alive = False
                        self.release(x)
                        for w in x.timers:
                            w.alive = False
                self.finish_body(body)
                return
        if body.completed >= total:
            self.finish_body(body)
        elif n.has("sequential") and body.next < total:
            self.next_instance(body)

    def finish_body(self, body):
        n = body.node
        a = n.attrs
        body.alive = False
        body.end = self.t
        for w in body.timers:
            w.alive = False
        if "output" in a:
            body.frame.case.vars[a["output"]] = body.results
        if "compensate" in a:
            body.frame.case.compensable.append((n.id, a["compensate"]))
        self.leave(body.frame, n.id)

    def start_adhoc(self, frame, n):
        """Ad-hoc sub-process: the listed activities start at once (all inner activities without
        incoming flows when no list is given); it completes when their paths have finished."""
        inner = [self.doc.nodes[c] for c in n.children if c in self.doc.nodes]
        chosen = None
        if "active" in n.attrs:
            try:
                val = eval_expr(n.attrs["active"], frame.case.vars)
                chosen = [x for x in val if isinstance(x, str)] if isinstance(val, list) else None
            except Exception as ex:
                self.warnings.append("active of '%s' could not be evaluated: %s" % (n.id, ex))
        if chosen is None:
            chosen = [x.id for x in inner if x.kind in M.ACTIVITY_KINDS and not self.inc.get(x.id)]
        chosen = [c for c in chosen if c in {x.id for x in inner}]
        ai = _AI(frame, n, self.t)
        ai.start = self.t
        frame.ais.append(ai)
        self.arm_boundaries(ai)
        if not chosen:
            self.complete(ai)
            return
        child = _Frame(frame.case, n.id, frame, ai)
        if "until" in n.attrs:
            child.until = (n.attrs["until"], n.has("wait"))
        ai.child = child
        frame.children.append(child)
        child.active = len(chosen)
        for c in chosen:
            self.enter(child, c)

    def start_compensation(self, frame, n):
        """Run the compensation handlers of completed activities; continue at n when they finish.
        Returns False when there is nothing to compensate."""
        case = frame.case
        target = n.attrs.get("activity")
        todo = [h for act, h in reversed(case.compensable) if target is None or act == target]
        todo = [h for h in todo if h in self.doc.nodes]
        if not todo:
            return False
        case.compensable = [x for x in case.compensable if not (target is None or x[0] == target)]
        if n.kind == "end":
            frame.last_end = n.id
        ai = _AI(frame, n, self.t)
        ai.start = self.t
        ai.silent = True
        frame.ais.append(ai)
        child = _Frame(case, frame.scope, frame, ai)
        ai.child = child
        frame.children.append(child)
        child.active = len(todo)
        for h in todo:
            self.start_activity(child, self.doc.nodes[h])
        return True


# --------------------------------------------------------------------- results

def _iso(t0, s):
    if s is None:
        return ""
    return (t0 + datetime.timedelta(seconds=s)).isoformat(timespec="seconds")


def human(seconds):
    """Short human-readable duration: 45 s, 12.5 min, 3.2 h, 1.8 d."""
    if seconds is None:
        return ""
    if seconds >= 86400:
        return "%.1f d" % (seconds / 86400.0)
    if seconds >= 3600:
        return "%.1f h" % (seconds / 3600.0)
    if seconds >= 60:
        return "%.1f min" % (seconds / 60.0)
    return "%.0f s" % seconds


def _quantile(values, q):
    if not values:
        return None
    v = sorted(values)
    k = max(0, int(math.ceil(q * len(v))) - 1)
    return v[k]


class Result:
    def __init__(self, sim):
        self.t0 = sim.t0
        self.seed = sim.seed
        self.horizon = sim.horizon
        self._rows = sim.rows
        self._cases = sim.cases
        self._resources = sim.resources
        self.warnings = sim.warnings
        self.doc = sim.doc

    @property
    def log(self):
        t0 = self.t0
        return [{"case": c, "activity": a, "label": lab, "resource": r, "enqueue": _iso(t0, e),
                 "start": _iso(t0, s), "end": _iso(t0, x), "status": st}
                for c, a, lab, r, e, s, x, st in self._rows]

    @property
    def cases(self):
        out = []
        for c in self._cases:
            ct = (c.end - c.start) if c.end is not None else None
            out.append({"case": c.id, "start": _iso(self.t0, c.start), "end": _iso(self.t0, c.end),
                        "end_event": c.end_event or "", "cycle_time_s": round(ct, 3) if ct is not None else None,
                        "variables": {k: v for k, v in c.vars.items()}})
        return out

    def kpi(self):
        finished = [c for c in self._cases if c.end is not None]
        by_end = {}
        for c in finished:
            key = c.end_event or ""
            by_end[key] = by_end.get(key, 0) + 1
        ct = [c.end - c.start for c in finished]
        mean = sum(ct) / len(ct) if ct else None
        acts = {}
        for c, a, lab, r, e, s, x, st in self._rows:
            if st != "completed":
                continue
            d = acts.setdefault(a, {"label": lab, "count": 0, "_w": 0.0, "_p": 0.0})
            d["count"] += 1
            d["_w"] += (s - e)
            d["_p"] += (x - s)
        for a, d in acts.items():
            n = d["count"]
            d["mean_wait_s"] = round(d.pop("_w") / n, 3)
            d["mean_processing_s"] = round(d.pop("_p") / n, 3)
            d["mean_wait"] = human(d["mean_wait_s"])
            d["mean_processing"] = human(d["mean_processing_s"])
        res = {}
        for r in self._resources.values():
            cap = r.count * self.horizon
            res[r.id] = {"label": r.label or r.id, "count": r.count,
                         "utilization": round(r.busy_time / cap, 4) if cap > 0 else 0.0}
        return {
            "seed": self.seed,
            "cases": {"started": len(self._cases), "finished": len(finished), "by_end_event": by_end},
            "cycle_time": {
                "mean_s": round(mean, 3) if mean is not None else None,
                "median_s": round(_quantile(ct, 0.5), 3) if ct else None,
                "p90_s": round(_quantile(ct, 0.9), 3) if ct else None,
                "mean": human(mean), "median": human(_quantile(ct, 0.5)), "p90": human(_quantile(ct, 0.9)),
            },
            "activities": acts,
            "resources": res,
            "horizon_s": round(self.horizon, 3),
            "warnings": list(dict.fromkeys(self.warnings)),
        }

    def write_csv(self, path_log, path_cases=None):
        cols = ["case", "activity", "label", "resource", "enqueue", "start", "end", "status"]
        with open(path_log, "w", newline="", encoding="utf-8") as f:
            w = csv.DictWriter(f, fieldnames=cols)
            w.writeheader()
            for row in self.log:
                w.writerow(row)
        if path_cases:
            with open(path_cases, "w", newline="", encoding="utf-8") as f:
                w = csv.writer(f)
                w.writerow(["case", "start", "end", "end_event", "cycle_time_s", "variables"])
                for c in self.cases:
                    w.writerow([c["case"], c["start"], c["end"], c["end_event"], c["cycle_time_s"],
                                json.dumps(c["variables"], ensure_ascii=False, default=str)])

    def to_json(self, include_log=False):
        data = {"kpi": self.kpi(), "cases": self.cases}
        if include_log:
            data["log"] = self.log
        return json.dumps(data, ensure_ascii=False, indent=2, default=str)


def simulate(doc, cases=None, seed=None):
    """Run the simulation of a parsed document and return a Result."""
    return Simulator(doc, cases, seed).run()


def format_kpi(kpi, lang="en"):
    """Plain-text report of the indicators returned by Result.kpi()."""
    pl = lang == "pl"
    t = (lambda en, p: p if pl else en)
    out = []
    c = kpi["cases"]
    out.append(t("Simulation (seed %s): %d cases started, %d finished",
                 "Symulacja (ziarno %s): rozpoczęte sprawy %d, zakończone %d") % (kpi.get("seed"), c["started"], c["finished"]))
    out.append("")
    out.append(t("End events", "Zakończenia"))
    for k, v in c["by_end_event"].items():
        out.append("  %-30s %6d  %5.1f%%" % (k, v, 100.0 * v / max(c["finished"], 1)))
    ct = kpi["cycle_time"]
    out.append("")
    out.append(t("Cycle time: mean %s, median %s, 90th percentile %s",
                 "Czas cyklu: średnio %s, mediana %s, 90. percentyl %s") % (ct["mean"], ct["median"], ct["p90"]))
    out.append("")
    out.append(t("Activities", "Czynności") + "  (" + t("count, mean wait, mean processing",
                                                         "liczba, średnie oczekiwanie, średnie wykonanie") + ")")
    for k, a in kpi["activities"].items():
        name = a.get("label") or k
        out.append("  %-30s %6d  %10s  %10s" % (name[:30], a["count"], a["mean_wait"], a["mean_processing"]))
    if kpi.get("resources"):
        out.append("")
        out.append(t("Resources (utilization)", "Zasoby (wykorzystanie)"))
        for k, r in kpi["resources"].items():
            out.append("  %-30s x%-3d %5.1f%%" % ((r.get("label") or k)[:30], r["count"], 100.0 * r["utilization"]))
    for w in kpi.get("warnings", []):
        out.append(t("warning: ", "ostrzeżenie: ") + str(w))
    return "\n".join(out) + "\n"
