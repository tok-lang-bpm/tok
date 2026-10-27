"""Intermediate representation of a tok document.

Every element is a plain object with an ``id``, the ``line`` it was declared
on and the comments attached to it. Containers refer to each other by id, so
the structure is easy to port to languages without object references.
"""

EVENT_KINDS = ("start", "end", "catch", "throw", "boundary")
TASK_KINDS = ("task", "user", "service", "manual", "script", "rule", "send", "receive", "call")
SUB_KINDS = ("sub", "eventsub", "adhoc")
GATEWAY_KINDS = ("xor", "and", "or", "eventgw")
NODE_KINDS = EVENT_KINDS + TASK_KINDS + SUB_KINDS + GATEWAY_KINDS
ACTIVITY_KINDS = TASK_KINDS + SUB_KINDS

TRIGGERS = ("message", "timer", "signal", "error", "escalation", "condition", "link")
TRIGGER_FLAGS = ("terminate", "compensate")
# catch-all flags (no name): catch every error or escalation (language 0.3)
CATCH_ALL_FLAGS = ("error", "escalation")

# attribute keys and flags per statement kind, in canonical print order
EVENT_KEYS = ("on", "message", "timer", "signal", "error", "escalation", "code", "condition", "link",
              "correlation", "form", "binding", "version", "activity", "events",
              "type", "retries", "headers", "outputs", "listeners", "delay", "assign")
EVENT_FLAGS = ("noninterrupting", "terminate", "compensate", "error", "escalation")
TASK_KEYS = ("message", "process", "form", "decision", "expr", "result", "type",
             "binding", "version", "child-vars", "parent-vars", "business-id",
             "collection", "item", "output", "output-item", "until", "active",
             "assignee", "candidate-users", "candidate-groups", "due", "follow-up", "priority",
             "retries", "headers", "connector", "method", "url", "query", "headers-in", "body",
             "inputs", "outputs", "compensate", "listeners",
             "by", "duration", "assign")
TASK_FLAGS = ("loop", "parallel", "sequential", "compensation", "wait")

# The executable layer of language 0.3 (spec §10.3): meaningful only in executable documents,
# rejected at the levels basic and analytic.
EXEC_KEYS = ("binding", "version", "child-vars", "parent-vars", "business-id",
             "collection", "item", "output", "output-item", "until", "active",
             "assignee", "candidate-users", "candidate-groups", "due", "follow-up", "priority",
             "retries", "headers", "connector", "method", "url", "query", "headers-in", "body",
             "inputs", "outputs", "listeners", "events", "version-tag")
EXEC_FLAGS = ("wait",)
USER_TASK_KEYS = ("assignee", "candidate-users", "candidate-groups", "due", "follow-up", "priority")
CONNECTOR_KEYS = ("method", "url", "query", "headers-in", "body")
# attributes whose value is always a FEEL expression (written without "=")
EXPR_KEYS = ("when", "expr", "condition", "correlation", "collection", "output-item", "until", "active")
LANGUAGE_VERSIONS = ("0.1", "0.3")
GATEWAY_KEYS = ()
GATEWAY_FLAGS = ()

NODE_KEYS = {}
NODE_FLAGS = {}
for _k in EVENT_KINDS:
    NODE_KEYS[_k], NODE_FLAGS[_k] = EVENT_KEYS, EVENT_FLAGS
for _k in TASK_KINDS + SUB_KINDS:
    NODE_KEYS[_k], NODE_FLAGS[_k] = TASK_KEYS, TASK_FLAGS
for _k in GATEWAY_KINDS:
    NODE_KEYS[_k], NODE_FLAGS[_k] = GATEWAY_KEYS, GATEWAY_FLAGS

HEADER_KEYS = ("id", "process", "target", "namespace", "collaboration", "version-tag")
HEADER_FLAGS = ("executable",)
POOL_KEYS = ("process", "version-tag")
POOL_FLAGS = ("executable",)
FLOW_KEYS = ("id", "when", "p")
FLOW_FLAGS = ("default",)
MESSAGE_KEYS = ("id", "message", "delay")
DATA_KEYS = ("state",)
DATA_FLAGS = ("collection",)
NOTE_KEYS = ("on",)
GROUP_KEYS = ("members",)
DECISION_KEYS = ("hit",)
FIELD_KEYS = ("type", "options", "min", "max", "default")
FIELD_FLAGS = ("required",)
SIMULATE_KEYS = ("cases", "seed", "arrival", "start")
RESOURCE_KEYS = ("count",)

HIT_POLICIES = ("unique", "first", "any", "priority", "collect")
FIELD_TYPES = ("textfield", "textarea", "number", "checkbox", "select", "radio", "datetime")


class Element:
    """Common fields: id, label, attrs (dict key -> value), flags (list), line, comments."""

    def __init__(self, kind, id, label=None, attrs=None, flags=None, line=0):
        self.kind = kind
        self.id = id
        self.label = label
        self.attrs = attrs if attrs is not None else {}
        self.flags = flags if flags is not None else []
        self.line = line
        self.comments = []        # comment lines printed above the statement
        self.trailing = None      # comment at the end of the statement line

    def has(self, flag):
        return flag in self.flags

    def __repr__(self):
        return "<%s %s>" % (self.kind, self.id)


class Node(Element):
    """A flow node. ``scope`` is the id of its process or sub-process,
    ``lane`` the id of its innermost lane (or None), ``pool`` the id of its pool (or None)."""

    def __init__(self, kind, id, label=None, attrs=None, flags=None, line=0):
        Element.__init__(self, kind, id, label, attrs, flags, line)
        self.scope = None
        self.lane = None
        self.pool = None
        self.children = []        # ids of statements declared in the block of a sub/eventsub, in order


class Flow(Element):
    def __init__(self, id, source, target, label=None, attrs=None, flags=None, line=0):
        Element.__init__(self, "flow", id, label, attrs, flags, line)
        self.source = source
        self.target = target
        self.explicit_id = False
        self.chain_start = True   # False when the flow continued a chain on the same source line


class MessageFlow(Element):
    def __init__(self, id, source, target, label=None, attrs=None, line=0):
        Element.__init__(self, "message", id, label, attrs, [], line)
        self.source = source
        self.target = target
        self.explicit_id = False


class Data(Element):
    """``kind`` is 'data' or 'store'; ``scope`` is set by the parser or by the first association."""

    def __init__(self, kind, id, label=None, attrs=None, flags=None, line=0):
        Element.__init__(self, kind, id, label, attrs, flags, line)
        self.scope = None
        self.block = None         # id of the block it was declared in (None = top level)


class Association(Element):
    """``kind`` is 'reads' or 'writes'."""

    def __init__(self, kind, activity, data, line=0):
        Element.__init__(self, kind, "%s-%s-%s" % (kind, activity, data), None, {}, [], line)
        self.activity = activity
        self.data = data


class Pool(Element):
    def __init__(self, id, label=None, attrs=None, flags=None, line=0):
        Element.__init__(self, "pool", id, label, attrs, flags, line)
        self.blackbox = True
        self.children = []        # ids of lanes, nodes and data declared directly in the pool block

    @property
    def process_id(self):
        return self.attrs.get("process") or (self.id + "-process")


class Lane(Element):
    def __init__(self, id, label=None, attrs=None, flags=None, line=0):
        Element.__init__(self, "lane", id, label, attrs, flags, line)
        self.pool = None
        self.parent = None        # parent lane id or None
        self.children = []        # ids of lanes and nodes in order


class Decision(Element):
    def __init__(self, id, label=None, attrs=None, line=0):
        Element.__init__(self, "decision", id, label, attrs, [], line)
        self.inputs = []          # list of (expression, type)
        self.outputs = []         # list of (name, type)
        self.rules = []           # list of Rule
        self.has_annotations = False
        self.annotation_title = None   # text of the annotation column in the header row

    @property
    def hit(self):
        return self.attrs.get("hit", "unique")


class Rule:
    def __init__(self, inputs, outputs, annotation, line):
        self.inputs = inputs
        self.outputs = outputs
        self.annotation = annotation
        self.line = line


class Form(Element):
    def __init__(self, id, label=None, attrs=None, line=0):
        Element.__init__(self, "form", id, label, attrs, [], line)
        self.items = []           # FormText or Field


class FormText(Element):
    def __init__(self, text, line=0):
        Element.__init__(self, "text", None, text, {}, [], line)


class Field(Element):
    def __init__(self, key, label=None, attrs=None, flags=None, line=0):
        Element.__init__(self, "field", key, label, attrs, flags, line)


class Document:
    def __init__(self):
        self.version = None
        self.title = None
        self.attrs = {}
        self.flags = []
        self.header_line = 0
        self.header_comments = []
        self.header_trailing = None
        self.top = []             # ids of top-level nodes and data in order
        self.nodes = {}           # id -> Node (insertion order = declaration order)
        self.flows = []
        self.messages = []
        self.data = {}            # id -> Data
        self.assocs = []
        self.notes = []
        self.groups = []
        self.pools = []
        self.lanes = {}           # id -> Lane
        self.decisions = {}       # id -> Decision
        self.forms = {}           # id -> Form
        self.simulate = None
        self.resources = {}       # id -> Element(kind='resource')
        self.variables = []       # Element(kind='variable', id=name, label=dist)
        self.end_comments = []
        self.ids = {}             # every declared id -> element (for duplicate and kind checks)

    @property
    def executable(self):
        return "executable" in self.flags

    @property
    def executable_any(self):
        return self.executable or any(p.has("executable") for p in self.pools)

    @property
    def default_process_id(self):
        return self.attrs.get("process", "process")

    def element(self, id):
        return self.ids.get(id)
