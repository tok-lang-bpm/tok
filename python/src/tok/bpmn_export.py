"""Export a tok Document to BPMN 2.0 XML (semantic part; layout is added by bpmnlib)."""

import re
import unicodedata
import xml.etree.ElementTree as ET

from . import model as M

BPMN = "http://www.omg.org/spec/BPMN/20100524/MODEL"
XSI = "http://www.w3.org/2001/XMLSchema-instance"
ZEEBE = "http://camunda.org/schema/zeebe/1.0"
MODELER = "http://camunda.org/schema/modeler/1.0"
B = "{%s}" % BPMN
Z = "{%s}" % ZEEBE

TAGS = {
    "start": "startEvent", "end": "endEvent", "catch": "intermediateCatchEvent",
    "throw": "intermediateThrowEvent", "boundary": "boundaryEvent",
    "task": "task", "user": "userTask", "service": "serviceTask", "manual": "manualTask",
    "script": "scriptTask", "rule": "businessRuleTask", "send": "sendTask", "receive": "receiveTask",
    "call": "callActivity", "sub": "subProcess", "eventsub": "subProcess", "adhoc": "adHocSubProcess",
    "xor": "exclusiveGateway", "and": "parallelGateway", "or": "inclusiveGateway", "eventgw": "eventBasedGateway",
}
DEFINITION = {
    "message": "messageEventDefinition", "timer": "timerEventDefinition", "signal": "signalEventDefinition",
    "error": "errorEventDefinition", "escalation": "escalationEventDefinition",
    "condition": "conditionalEventDefinition", "link": "linkEventDefinition",
    "terminate": "terminateEventDefinition", "compensate": "compensateEventDefinition",
}


def slug(text):
    """ASCII identifier fragment from a label: 'Zamrożenie budżetu' -> 'Zamrozenie_budzetu'."""
    t = text.replace("ł", "l").replace("Ł", "L")
    t = unicodedata.normalize("NFKD", t).encode("ascii", "ignore").decode("ascii")
    t = re.sub(r"[^A-Za-z0-9]+", "_", t).strip("_")
    if not t or not re.match(r"[A-Za-z_]", t[0]):
        t = "_" + t
    return t


REST_CONNECTOR_TYPE = "io.camunda:http-json:1"
BINDINGS = {"latest": "latest", "deployment": "deployment", "version-tag": "versionTag"}


def split_pairs(text, sep):
    out = []
    for item in (text or "").split(";"):
        item = item.strip()
        if item and sep in item:
            left, right = item.split(sep, 1)
            out.append((left.strip(), right.strip()))
    return out


def parse_listeners(text):
    import re
    out = []
    for item in (text or "").split(";"):
        m = re.match(r"^([a-z]+)\s*:\s*(.+?)(?:\s*\((\d+)\))?$", item.strip())
        if m:
            out.append((m.group(1), m.group(2).strip(), m.group(3)))
    return out


def timer_kind(value):
    v = value.strip()
    if v.startswith("="):
        return "timeDuration"
    if v.startswith("R"):
        return "timeCycle"
    if v.startswith("P"):
        return "timeDuration"
    return "timeDate"


class Exporter:
    def __init__(self, doc, target=None):
        self.doc = doc
        self.camunda = doc.executable_any and (target or doc.attrs.get("target", "camunda")) == "camunda"
        self.decls = {"message": {}, "signal": {}, "error": {}, "escalation": {}}
        self.stores = {}
        self.containers = {}     # scope id -> XML element (process or subProcess)

    # ---------------------------------------------------------------- helpers
    def sub(self, parent, tag, **attrs):
        e = ET.SubElement(parent, B + tag)
        for k, v in attrs.items():
            if v is not None:
                e.set(k.rstrip("_"), str(v))
        return e

    def ext(self, el):
        x = el.find(B + "extensionElements")
        if x is None:
            x = ET.Element(B + "extensionElements")
            el.insert(0, x)
        return x

    def feel(self, expr):
        return "=" + expr if self.camunda else expr

    def declare(self, kind, name, code=None):
        reg = self.decls[kind]
        if name not in reg:
            prefix = {"message": "Message", "signal": "Signal", "error": "Error", "escalation": "Escalation"}[kind]
            reg[name] = {"id": "%s_%s" % (prefix, slug(name)), "code": code, "correlation": None}
        elif code and not reg[name]["code"]:
            reg[name]["code"] = code
        return reg[name]

    # ------------------------------------------------------------------ main
    def run(self):
        d = self.doc
        ET.register_namespace("bpmn", BPMN)
        ET.register_namespace("xsi", XSI)
        if self.camunda:
            ET.register_namespace("zeebe", ZEEBE)
            ET.register_namespace("modeler", MODELER)
        root = ET.Element(B + "definitions")
        root.set("id", d.attrs.get("id", "definitions"))
        if d.title:
            root.set("name", d.title)
        root.set("targetNamespace", d.attrs.get("namespace", "http://bpmn.io/schema/bpmn"))
        root.set("exporter", "tok")
        root.set("exporterVersion", d.version or "0.1")
        if self.camunda:
            root.set("{%s}executionPlatform" % MODELER, "Camunda Cloud")
            root.set("{%s}executionPlatformVersion" % MODELER, "8.10.0")
        self.root = root
        decl_slot = len(root)        # declarations are inserted here at the end

        processes = []
        if d.pools:
            collab = self.sub(root, "collaboration", id=d.attrs.get("collaboration", "collaboration"))
            inner = [p for p in d.pools if not p.blackbox]
            for p in d.pools:
                part = self.sub(collab, "participant", id=p.id, name=p.label)
                if not p.blackbox:
                    part.set("processRef", p.process_id)
                    pname = d.title if len(inner) == 1 and d.title else p.label
                    processes.append((p.process_id, pname, p.has("executable") or d.executable, p))
            for m in d.messages:
                mf = self.sub(collab, "messageFlow", id=m.id, name=m.label, sourceRef=m.source, targetRef=m.target)
                name = m.attrs.get("message") or m.label
                if name:
                    mf.set("messageRef", self.declare("message", name)["id"])
            self.collab = collab
        else:
            self.collab = None
            processes.append((d.default_process_id, d.title, d.executable, None))

        for pid, name, executable, pool in processes:
            proc = self.sub(root, "process", id=pid, name=name, isExecutable="true" if executable else "false")
            self.containers[pid] = proc
            tag = (pool.attrs.get("version-tag") if pool is not None else None) or \
                (d.attrs.get("version-tag") if pool is None or len(processes) == 1 else None)
            if self.camunda and executable and tag:
                ET.SubElement(self.ext(proc), Z + "versionTag", {"value": tag})
            if pool is not None:
                lanes = [d.lanes[c] for c in pool.children if c in d.lanes]
                if lanes:
                    ls = self.sub(proc, "laneSet", id="%s-lanes" % pool.id)
                    for lane in lanes:
                        self.lane(ls, lane)
                self.block(proc, self.ordered_nodes(pool.children))
            else:
                self.block(proc, d.top)

        self.flows()
        self.data()
        self.artifacts()
        self.compensation_links()
        self.order_adhoc()

        # declarations go before the collaboration and processes
        decl = []
        for kind in ("message", "signal", "error", "escalation"):
            for name, info in self.decls[kind].items():
                e = ET.Element(B + kind, {"id": info["id"], "name": name})
                if kind == "error" and info["code"]:
                    e.set("errorCode", info["code"])
                if kind == "escalation" and info["code"]:
                    e.set("escalationCode", info["code"])
                if kind == "message" and self.camunda and info["correlation"]:
                    x = ET.SubElement(e, B + "extensionElements")
                    ET.SubElement(x, Z + "subscription", {"correlationKey": self.feel(info["correlation"])})
                decl.append(e)
        for name, sid in self.stores.items():
            decl.append(ET.Element(B + "dataStore", {"id": sid, "name": name}))
        for k, e in enumerate(decl):
            root.insert(decl_slot + k, e)
        return ET.ElementTree(root)

    def ordered_nodes(self, children):
        """Node ids of a pool in declaration order, walking lanes."""
        d, out = self.doc, []

        def walk(ids):
            for i in ids:
                if i in d.lanes:
                    walk(d.lanes[i].children)
                elif i in d.nodes:
                    out.append(i)
        walk(children)
        return out

    def lane(self, parent, lane):
        d = self.doc
        e = self.sub(parent, "lane", id=lane.id, name=lane.label)
        sublanes = [d.lanes[c] for c in lane.children if c in d.lanes]
        for c in lane.children:
            if c in d.nodes:
                self.sub(e, "flowNodeRef").text = c
                if "compensate" in d.nodes[c].attrs:
                    self.sub(e, "flowNodeRef").text = self.compensation_id(c)
        if sublanes:
            cls = self.sub(e, "childLaneSet", id="%s-lanes" % lane.id)
            for s in sublanes:
                self.lane(cls, s)

    def block(self, container, ids):
        d = self.doc
        for i in ids:
            n = d.nodes.get(i)
            if n is not None:
                self.node(container, n)

    def node(self, container, n):
        tag = TAGS[n.kind]
        e = self.sub(container, tag, id=n.id, name=n.label)
        a = n.attrs
        if n.kind == "boundary":
            e.set("attachedToRef", a.get("on", ""))
            if n.has("noninterrupting"):
                e.set("cancelActivity", "false")
        if n.kind == "start" and n.has("noninterrupting"):
            e.set("isInterrupting", "false")
        if n.kind == "eventsub":
            e.set("triggeredByEvent", "true")
        if n.kind == "call" and "process" in a and not a["process"].startswith("="):
            e.set("calledElement", a["process"])
        if n.kind == "adhoc":
            if n.has("sequential"):
                e.set("ordering", "Sequential")
            if self.camunda and n.has("wait"):
                e.set("cancelRemainingInstances", "false")
        if n.has("compensation"):
            e.set("isForCompensation", "true")
        if n.kind in ("send", "receive") and "message" in a:
            e.set("messageRef", self.declare("message", a["message"])["id"])
        if n.kind == "script" and not self.camunda and "expr" in a:
            e.set("scriptFormat", "feel")
            self.sub(e, "script").text = a["expr"]

        if self.camunda:
            self.zeebe(e, n)

        if n.kind in M.EVENT_KINDS:
            self.event_definition(e, n)
        if n.has("loop"):
            self.sub(e, "standardLoopCharacteristics")
        elif n.kind != "adhoc" and (n.has("parallel") or n.has("sequential")):
            mi = self.sub(e, "multiInstanceLoopCharacteristics", isSequential="true" if n.has("sequential") else "false")
            if self.camunda and "collection" in a:
                lc = {"inputCollection": self.feel(a["collection"])}
                if "item" in a:
                    lc["inputElement"] = a["item"]
                if "output" in a:
                    lc["outputCollection"] = a["output"]
                if "output-item" in a:
                    lc["outputElement"] = self.feel(a["output-item"])
                ET.SubElement(self.ext(mi), Z + "loopCharacteristics", lc)
            if self.camunda and "until" in a:
                c = self.sub(mi, "completionCondition")
                c.set("{%s}type" % XSI, "bpmn:tFormalExpression")
                c.text = self.feel(a["until"])
        if n.kind in M.SUB_KINDS:
            self.containers[n.id] = e
            self.block(e, n.children)
            if n.kind == "adhoc" and self.camunda and "until" in a:
                c = self.sub(e, "completionCondition")
                c.set("{%s}type" % XSI, "bpmn:tFormalExpression")
                c.text = self.feel(a["until"])
        if "compensate" in a:
            b = self.sub(container, "boundaryEvent", id=self.compensation_id(n.id), attachedToRef=n.id)
            self.sub(b, "compensateEventDefinition", id="%s-def" % self.compensation_id(n.id))

    @staticmethod
    def compensation_id(activity_id):
        return "%s-compensate" % activity_id

    def binding(self, attrs, a):
        if "binding" in a:
            attrs["bindingType"] = BINDINGS.get(a["binding"], a["binding"])
        if "version" in a:
            attrs["versionTag"] = a["version"]
        return attrs

    def zeebe(self, e, n):
        a = n.attrs
        if not self.node_executable(n):
            # only the 0.1 executable profile applies to processes that are not executable
            return self.zeebe_01(e, n)
        x = None
        if n.kind == "user":
            x = self.ext(e)
            ET.SubElement(x, Z + "userTask")
            if "form" in a:
                ET.SubElement(x, Z + "formDefinition", self.binding({"formId": a["form"]}, a))
            assign = {}
            for key, attr in (("assignee", "assignee"), ("candidate-users", "candidateUsers"),
                              ("candidate-groups", "candidateGroups")):
                if key in a:
                    assign[attr] = a[key]
            if assign:
                ET.SubElement(x, Z + "assignmentDefinition", assign)
            sched = {}
            for key, attr in (("due", "dueDate"), ("follow-up", "followUpDate")):
                if key in a:
                    sched[attr] = a[key]
            if sched:
                ET.SubElement(x, Z + "taskSchedule", sched)
            if "priority" in a:
                ET.SubElement(x, Z + "priorityDefinition", {"priority": a["priority"]})
        elif n.kind == "start" and "form" in a:
            ET.SubElement(self.ext(e), Z + "formDefinition", self.binding({"formId": a["form"]}, a))
        elif n.kind == "rule" and "decision" in a:
            res = a.get("result") or self.default_result(a["decision"])
            ET.SubElement(self.ext(e), Z + "calledDecision",
                          self.binding({"decisionId": a["decision"], "resultVariable": res}, a))
        elif n.kind == "script" and "expr" in a:
            ET.SubElement(self.ext(e), Z + "script",
                          {"expression": self.feel(a["expr"]), "resultVariable": a.get("result", "result")})
        elif n.kind == "call" and "process" in a:
            ce = self.binding({"processId": a["process"]}, a)
            if a.get("child-vars") == "none":
                ce["propagateAllChildVariables"] = "false"
            if a.get("parent-vars") == "none":
                ce["propagateAllParentVariables"] = "false"
            if "business-id" in a:
                ce["businessId"] = a["business-id"]
            ET.SubElement(self.ext(e), Z + "calledElement", ce)
        elif n.kind == "adhoc" and "active" in a:
            ET.SubElement(self.ext(e), Z + "adHoc", {"activeElementsCollection": self.feel(a["active"])})
        job_kinds = ("service", "send") + (("throw", "end") if "message" in a else ())
        inputs = list(split_pairs(a.get("inputs"), "="))
        headers = list(split_pairs(a.get("headers"), ":"))
        if n.kind == "service" and a.get("connector") == "rest":
            td = {"type": REST_CONNECTOR_TYPE}
            if "retries" in a:
                td["retries"] = a["retries"]
            ET.SubElement(self.ext(e), Z + "taskDefinition", td)
            conn = [("authentication.type", "noAuth"), ("method", a.get("method", "GET")), ("url", a.get("url", ""))]
            for key, target in (("headers-in", "headers"), ("query", "queryParameters"), ("body", "body")):
                if key in a:
                    conn.append((target, a[key]))
            self.io_mapping(e, conn, [], static_sources=True)
            if "result" in a:
                headers = [("resultVariable", a["result"])] + headers
        elif n.kind in job_kinds and "type" in a:
            td = {"type": a["type"]}
            if "retries" in a:
                td["retries"] = a["retries"]
            ET.SubElement(self.ext(e), Z + "taskDefinition", td)
        if inputs or "outputs" in a:
            self.io_mapping(e, [(t, self.feel(src)) for t, src in inputs],
                            [(t, self.feel(src)) for t, src in split_pairs(a.get("outputs"), "=")])
        if headers:
            th = ET.SubElement(self.ext(e), Z + "taskHeaders")
            for k, v in headers:
                ET.SubElement(th, Z + "header", {"key": k, "value": v})
        if "events" in a and n.kind in ("start", "catch", "boundary"):
            ET.SubElement(self.ext(e), Z + "conditionalFilter", {"variableEvents": a["events"]})
        if "listeners" in a:
            execl, taskl = [], []
            for ev, typ, retries in parse_listeners(a["listeners"]):
                item = {"eventType": ev, "type": typ}
                if retries:
                    item["retries"] = retries
                (execl if ev in ("start", "end") else taskl).append(item)
            if execl:
                box = ET.SubElement(self.ext(e), Z + "executionListeners")
                for item in execl:
                    ET.SubElement(box, Z + "executionListener", item)
            if taskl:
                box = ET.SubElement(self.ext(e), Z + "taskListeners")
                for item in taskl:
                    ET.SubElement(box, Z + "taskListener", item)

    def io_mapping(self, e, inputs, outputs, static_sources=False):
        x = self.ext(e)
        box = x.find(Z + "ioMapping")
        if box is None:
            box = ET.SubElement(x, Z + "ioMapping")
        for target, source in inputs:
            ET.SubElement(box, Z + "input", {"source": source, "target": target})
        for target, source in outputs:
            ET.SubElement(box, Z + "output", {"source": source, "target": target})

    def zeebe_01(self, e, n):
        """The executable profile of language 0.1 (used where the process itself is not executable)."""
        a = n.attrs
        if n.kind == "user":
            x = self.ext(e)
            ET.SubElement(x, Z + "userTask")
            if "form" in a:
                ET.SubElement(x, Z + "formDefinition", {"formId": a["form"]})
        elif n.kind == "start" and "form" in a:
            ET.SubElement(self.ext(e), Z + "formDefinition", {"formId": a["form"]})
        elif n.kind == "rule" and "decision" in a:
            res = a.get("result") or self.default_result(a["decision"])
            ET.SubElement(self.ext(e), Z + "calledDecision", {"decisionId": a["decision"], "resultVariable": res})
        elif n.kind == "script" and "expr" in a:
            ET.SubElement(self.ext(e), Z + "script",
                          {"expression": self.feel(a["expr"]), "resultVariable": a.get("result", "result")})
        elif n.kind in ("service", "send") and "type" in a:
            ET.SubElement(self.ext(e), Z + "taskDefinition", {"type": a["type"]})

    def node_executable(self, n):
        d = self.doc
        if d.executable:
            return True
        return any(p.id == n.pool and p.has("executable") for p in d.pools)

    def compensation_links(self):
        """Associations from generated compensation boundary events to their handlers."""
        d = self.doc
        for n in d.nodes.values():
            h = n.attrs.get("compensate")
            if h is None or h not in d.nodes:
                continue
            cont = self.containers.get(n.scope)
            if cont is None:
                continue
            self.sub(cont, "association", id="%s-assoc" % self.compensation_id(n.id),
                     associationDirection="One", sourceRef=self.compensation_id(n.id), targetRef=h)

    def order_adhoc(self):
        """completionCondition is the last child of an ad-hoc sub-process (BPMN schema order)."""
        for e in self.root.iter(B + "adHocSubProcess"):
            for c in e.findall(B + "completionCondition"):
                e.remove(c)
                e.append(c)

    def default_result(self, decision_id):
        dec = self.doc.decisions.get(decision_id)
        if dec is not None and len(dec.outputs) == 1:
            return dec.outputs[0][0]
        return "result"

    def event_definition(self, e, n):
        a = n.attrs
        for key in M.TRIGGERS:
            if key not in a:
                continue
            d = self.sub(e, DEFINITION[key], id="%s-def" % n.id)
            if key == "message":
                info = self.declare("message", a["message"])
                if "correlation" in a:
                    info["correlation"] = a["correlation"]
                d.set("messageRef", info["id"])
            elif key == "signal":
                d.set("signalRef", self.declare("signal", a["signal"])["id"])
            elif key == "error":
                d.set("errorRef", self.declare("error", a["error"], a.get("code"))["id"])
            elif key == "escalation":
                d.set("escalationRef", self.declare("escalation", a["escalation"], a.get("code"))["id"])
            elif key == "timer":
                t = self.sub(d, timer_kind(a["timer"]))
                t.set("{%s}type" % XSI, "bpmn:tFormalExpression")
                t.text = a["timer"]
            elif key == "condition":
                c = self.sub(d, "condition")
                c.set("{%s}type" % XSI, "bpmn:tFormalExpression")
                c.text = self.feel(a["condition"])
            elif key == "link":
                d.set("name", a["link"])
            return
        for flag in M.TRIGGER_FLAGS + M.CATCH_ALL_FLAGS:
            if n.has(flag):
                d = self.sub(e, DEFINITION[flag], id="%s-def" % n.id)
                if flag == "compensate" and "activity" in a:
                    d.set("activityRef", a["activity"])
                return

    def flows(self):
        d = self.doc
        for f in d.flows:
            src, tgt = d.nodes.get(f.source), d.nodes.get(f.target)
            if src is None or tgt is None or src.scope != tgt.scope:
                continue
            cont = self.containers.get(src.scope)
            if cont is None:
                continue
            e = self.sub(cont, "sequenceFlow", id=f.id, name=f.label, sourceRef=f.source, targetRef=f.target)
            if "when" in f.attrs:
                c = self.sub(e, "conditionExpression")
                c.set("{%s}type" % XSI, "bpmn:tFormalExpression")
                c.text = self.feel(f.attrs["when"])
            if f.has("default"):
                srcel = self.find(src.id)
                if srcel is not None:
                    srcel.set("default", f.id)
        # sequence flows must follow the nodes they connect inside each container: already true

    def find(self, id):
        for e in self.root.iter():
            if e.get("id") == id:
                return e
        return None

    def data_scope(self, x):
        d = self.doc
        if x.scope:
            return x.scope
        for a in d.assocs:
            if a.data == x.id and a.activity in d.nodes:
                return d.nodes[a.activity].scope
        if d.pools:
            for p in d.pools:
                if not p.blackbox:
                    return p.process_id
        return d.default_process_id

    def data(self):
        d = self.doc
        for x in d.data.values():
            cont = self.containers.get(self.data_scope(x))
            if cont is None:
                continue
            if x.kind == "data":
                obj = self.sub(cont, "dataObject", id="%s-object" % x.id, name=x.label,
                               isCollection="true" if x.has("collection") else None)
                ref = self.sub(cont, "dataObjectReference", id=x.id, name=x.label, dataObjectRef=obj.get("id"))
                if "state" in x.attrs:
                    self.sub(ref, "dataState", id="%s-state" % x.id, name=x.attrs["state"])
            else:
                name = x.label or x.id
                sid = self.stores.setdefault(name, "%s-store" % x.id)
                self.sub(cont, "dataStoreReference", id=x.id, name=x.label, dataStoreRef=sid)
        n = 0
        for a in d.assocs:
            act = self.find(a.activity)
            if act is None or a.data not in d.data:
                continue
            n += 1
            # property and data associations go after extensionElements, before event definitions and loops
            pos = 1 if act.find(B + "extensionElements") is not None else 0
            if a.kind == "reads":
                prop = ET.Element(B + "property", {"id": "%s-in-%d" % (a.activity, n), "name": "__targetRef_placeholder"})
                assoc = ET.Element(B + "dataInputAssociation", {"id": "%s-dia-%d" % (a.activity, n)})
                ET.SubElement(assoc, B + "sourceRef").text = a.data
                ET.SubElement(assoc, B + "targetRef").text = prop.get("id")
                act.insert(pos, prop)
                act.insert(self.after_props(act), assoc)
            else:
                assoc = ET.Element(B + "dataOutputAssociation", {"id": "%s-doa-%d" % (a.activity, n)})
                ET.SubElement(assoc, B + "targetRef").text = a.data
                act.insert(self.after_props(act), assoc)

    @staticmethod
    def after_props(act):
        pos = 0
        for k, c in enumerate(act):
            if c.tag in (B + "extensionElements", B + "property", B + "dataInputAssociation",
                         B + "dataOutputAssociation"):
                pos = k + 1
        return pos

    def artifacts(self):
        d = self.doc
        for note in d.notes:
            target = note.attrs.get("on")
            cont = None
            if target in d.nodes:
                cont = self.containers.get(d.nodes[target].scope)
            if cont is None:
                cont = self.collab if self.collab is not None else next(iter(self.containers.values()), None)
            if cont is None:
                continue
            t = self.sub(cont, "textAnnotation", id=note.id)
            self.sub(t, "text").text = note.label or ""
            if target:
                self.sub(cont, "association", id="%s-assoc" % note.id, sourceRef=note.id, targetRef=target)
        if d.groups:
            cat = ET.Element(B + "category", {"id": "categories"})
            for g in d.groups:
                cv = ET.SubElement(cat, B + "categoryValue", {"id": "%s-value" % g.id, "value": g.label or g.id})
                cont = self.collab if self.collab is not None else next(iter(self.containers.values()))
                self.sub(cont, "group", id=g.id, categoryValueRef=cv.get("id"))
            self.root.insert(0, cat)


def export_tree(doc, target=None):
    """Return an ElementTree with the semantic BPMN model (no BPMNDI)."""
    return Exporter(doc, target).run()


def semantic_xml(doc, target=None):
    """Canonical semantic XML text, used by the conformance corpus (attributes sorted, no layout)."""
    tree = export_tree(doc, target)
    root = tree.getroot()
    for e in root.iter():
        items = sorted(e.attrib.items())
        e.attrib.clear()
        for k, v in items:
            e.set(k, v)
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
