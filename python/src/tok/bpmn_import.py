"""Import BPMN 2.0 XML into a tok Document (the inverse of bpmn_export).

    doc, warnings = import_bpmn("model.bpmn")
    print(printer.fmt(doc))

Diagram interchange (BPMNDI) is ignored: tok has no coordinates. Elements
that tok cannot express are skipped or simplified, and each such decision is
reported as a warning string ``"id: explanation"``.
"""

import os
import re
import unicodedata
import xml.etree.ElementTree as ET

from . import model as M

BPMN = "http://www.omg.org/spec/BPMN/20100524/MODEL"
ZEEBE = "http://camunda.org/schema/zeebe/1.0"
B = "{%s}" % BPMN
Z = "{%s}" % ZEEBE

ID_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")

KINDS = {
    "startEvent": "start", "endEvent": "end", "intermediateCatchEvent": "catch",
    "intermediateThrowEvent": "throw", "boundaryEvent": "boundary",
    "task": "task", "userTask": "user", "serviceTask": "service", "manualTask": "manual",
    "scriptTask": "script", "businessRuleTask": "rule", "sendTask": "send", "receiveTask": "receive",
    "callActivity": "call", "subProcess": "sub", "adHocSubProcess": "adhoc",
    "exclusiveGateway": "xor", "parallelGateway": "and", "inclusiveGateway": "or",
    "eventBasedGateway": "eventgw",
}
# tok has no counterpart; imported as the nearest kind with a warning
APPROXIMATE = {"transaction": "sub"}
REST_CONNECTOR_TYPE = "io.camunda:http-json:1"
BINDINGS = {"latest": "latest", "deployment": "deployment", "versionTag": "version-tag"}
CONNECTOR_INPUTS = {"method": "method", "url": "url", "headers": "headers-in", "queryParameters": "query",
                    "body": "body"}
UNSUPPORTED_NODES = {"complexGateway", "choreographyTask", "subChoreography", "callChoreography"}
DEFINITIONS = {
    "messageEventDefinition": "message", "timerEventDefinition": "timer",
    "signalEventDefinition": "signal", "errorEventDefinition": "error",
    "escalationEventDefinition": "escalation", "conditionalEventDefinition": "condition",
    "linkEventDefinition": "link", "terminateEventDefinition": "terminate",
    "compensateEventDefinition": "compensate",
}
IGNORED_CHILDREN = {"incoming", "outgoing", "documentation", "extensionElements", "property",
                    "dataInputAssociation", "dataOutputAssociation", "standardLoopCharacteristics",
                    "multiInstanceLoopCharacteristics", "script", "laneSet", "ioSpecification",
                    "completionCondition"}


def local(tag):
    return tag.split("}", 1)[1] if "}" in tag else tag


def is_bpmn(e):
    return e.tag.startswith(B)


def slug(text):
    t = (text or "").replace("ł", "l").replace("Ł", "L")
    t = unicodedata.normalize("NFKD", t).encode("ascii", "ignore").decode("ascii")
    t = re.sub(r"[^A-Za-z0-9_-]+", "_", t).strip("_")
    if not t:
        t = "id"
    if not re.match(r"[A-Za-z_]", t[0]):
        t = "_" + t
    return t


def clean_label(text):
    if text is None:
        return None
    t = " ".join(text.split())
    return t or None


def strip_feel(text):
    t = (text or "").strip()
    return t[1:].strip() if t.startswith("=") else t


class Importer:
    def __init__(self, root, keep_ids=True):
        self.root = root
        self.keep_ids = keep_ids
        self.doc = M.Document()
        self.warnings = []
        self.idmap = {}           # XML id -> tok id
        self.used = set()
        self.by_id = {}           # XML id -> element
        self.kind_of = {}         # tok id -> kind of tok element
        self.parent = {}
        for e in root.iter():
            for c in e:
                self.parent[c] = e
            if e.get("id"):
                self.by_id[e.get("id")] = e
        self.warned = set()

    # --------------------------------------------------------------- helpers
    def warn(self, id, text, once=None):
        if once is not None:
            if once in self.warned:
                return
            self.warned.add(once)
        self.warnings.append("%s: %s" % (id or "-", text))

    def tid(self, xml_id):
        """tok identifier for an XML id (converted and made unique on first use)."""
        if xml_id is None:
            return None
        if xml_id in self.idmap:
            return self.idmap[xml_id]
        new = xml_id if ID_RE.match(xml_id) else slug(xml_id)
        base, n = new, 2
        while new in self.used:
            new = "%s-%d" % (base, n)
            n += 1
        if new != xml_id:
            self.warn(xml_id, "identifier changed to '%s'" % new)
        self.idmap[xml_id] = new
        self.used.add(new)
        return new

    def reserve_ids(self):
        """Reserve unchanged XML ids first so that converted ids never take them."""
        for xml_id in self.by_id:
            if ID_RE.match(xml_id):
                self.used.add(xml_id)
                self.idmap[xml_id] = xml_id

    def named(self, ref, kind):
        """Name of a declared message/signal/error/escalation referenced by id."""
        e = self.by_id.get(ref)
        if e is None:
            self.warn(ref, "unknown %s reference" % kind)
            return ref
        return clean_label(e.get("name")) or e.get("id")

    # ------------------------------------------------------------------ main
    def uses_03(self):
        self.doc.version = "0.3"

    def compensation_boundaries(self):
        """Compensation boundary events with an association to a handler: tok writes them as compensate=."""
        handlers = {}
        for a in self.root.iter(B + "association"):
            src, tgt = self.by_id.get(a.get("sourceRef")), self.by_id.get(a.get("targetRef"))
            if src is None or tgt is None:
                continue
            if local(src.tag) == "boundaryEvent" and src.find(B + "compensateEventDefinition") is not None \
                    and tgt.get("isForCompensation") == "true" and src.get("attachedToRef"):
                handlers[src.get("id")] = (src.get("attachedToRef"), tgt.get("id"), a.get("id"))
        return handlers

    def run(self):
        root, d = self.root, self.doc
        self.compensation = self.compensation_boundaries()
        self.skip_ids = set(self.compensation) | {v[2] for v in self.compensation.values()}
        if root.tag != B + "definitions":
            raise ValueError("not a BPMN 2.0 document: root element is %s" % root.tag)
        self.reserve_ids()
        d.version = "0.1"
        d.title = clean_label(root.get("name"))
        if root.get("id") and root.get("id") != "definitions":
            d.attrs["id"] = self.tid(root.get("id"))
        ns = root.get("targetNamespace")
        if ns and ns != "http://bpmn.io/schema/bpmn":
            d.attrs["namespace"] = ns

        processes = [e for e in root if e.tag == B + "process"]
        collabs = [e for e in root if e.tag == B + "collaboration"]
        for e in root:
            if local(e.tag) in ("choreography", "globalTask", "globalUserTask", "globalManualTask",
                                "globalScriptTask", "globalBusinessRuleTask"):
                self.warn(e.get("id"), "%s is not supported and was skipped" % local(e.tag))
        if len(collabs) > 1:
            self.warn(collabs[1].get("id"), "only the first collaboration is imported")
        collab = collabs[0] if collabs else None
        if collab is not None and collab.get("id") != "collaboration":
            d.attrs["collaboration"] = self.tid(collab.get("id"))

        executable = [p for p in processes if p.get("isExecutable") == "true"]
        all_exec = processes and len(executable) == len(processes)
        if all_exec:
            d.flags.append("executable")

        proc_by_id = {p.get("id"): p for p in processes}
        participants = collab.findall(B + "participant") if collab is not None else []
        if collab is None and len(processes) == 1:
            p = processes[0]
            if p.get("id") != "process":
                d.attrs["process"] = self.tid(p.get("id"))
            if d.title is None:
                d.title = clean_label(p.get("name"))
            self.process(p, None)
        else:
            referenced = set()
            for part in participants:
                pool = M.Pool(self.tid(part.get("id")), clean_label(part.get("name")))
                ref = part.get("processRef")
                d.pools.append(pool)
                self.kind_of[pool.id] = "pool"
                d.ids[pool.id] = pool
                if ref and ref in proc_by_id:
                    referenced.add(ref)
                    self.attach_process(pool, proc_by_id[ref], all_exec)
                elif ref:
                    self.warn(part.get("id"), "process '%s' not found; imported as a black-box pool" % ref)
            for p in processes:
                if p.get("id") in referenced:
                    continue
                # a process without a participant: give it its own pool
                pid = self.tid(p.get("id"))
                pool_id = pid + "-pool"
                while pool_id in self.used:
                    pool_id += "_"
                self.used.add(pool_id)
                pool = M.Pool(pool_id, clean_label(p.get("name")) or pid)
                d.pools.append(pool)
                self.kind_of[pool.id] = "pool"
                d.ids[pool.id] = pool
                self.attach_process(pool, p, all_exec)
                self.warn(p.get("id"), "process without a participant was placed in pool '%s'" % pool_id)

        inner = [p for p in d.pools if not p.blackbox]
        if d.title is None and len(inner) == 1:
            # tok names a single process after the document title
            proc = proc_by_id.get(inner[0].process_id) or next(
                (p for p in processes if self.tid(p.get("id")) == inner[0].process_id), None)
            if proc is not None and clean_label(proc.get("name")):
                d.title = clean_label(proc.get("name"))
        if collab is not None:
            for mf in collab.findall(B + "messageFlow"):
                self.message_flow(mf)
            for e in collab:
                t = local(e.tag)
                if t in ("conversation", "subConversation", "callConversation", "conversationLink",
                         "participantAssociation", "correlationKey"):
                    self.warn(e.get("id"), "%s is not supported and was skipped" % t)

        self.flows_and_artifacts()
        self.derive_flow_ids()
        return d, self.warnings

    def attach_process(self, pool, p, all_exec):
        pool.blackbox = False
        if p.get("id") != pool.id + "-process":
            pool.attrs["process"] = self.tid(p.get("id"))
        if p.get("isExecutable") == "true" and not all_exec:
            pool.flags.append("executable")
        self.process(p, pool)

    # ------------------------------------------------------------- processes
    def process(self, p, pool):
        d = self.doc
        vt = p.find(B + "extensionElements/" + Z + "versionTag")
        if vt is not None and vt.get("value"):
            if pool is not None:
                pool.attrs["version-tag"] = vt.get("value")
            else:
                d.attrs["version-tag"] = vt.get("value")
            self.uses_03()
        scope = pool.process_id if pool is not None else d.default_process_id
        lane_of = {}
        lanes = []
        for ls in p.findall(B + "laneSet"):
            for lane in ls.findall(B + "lane"):
                lanes.append(self.lane(lane, None, pool, lane_of))
        order = self.container(p, scope, pool, lane_of)
        if pool is None:
            d.top.extend(order["children"])
            return
        if not lanes:
            pool.children.extend(order["children"] + order["data"])
            return
        # place nodes into lanes, then order lanes and nodes by first appearance in the process
        pos = {nid: k for k, nid in enumerate(order["nodes"])}
        outside = []
        for nid in order["nodes"]:
            lid = lane_of.get(nid)
            if lid is None:
                outside.append(nid)
                continue
            d.nodes[nid].lane = lid
            d.lanes[lid].children.append(nid)
        self.sort_lanes(lanes, pos)
        pool.children.extend(self.merge(lanes, [], pos))
        for nid in outside:
            pool.children.append(nid)
            self.warn(nid, "node is in a pool with lanes but in no lane")
        pool.children.extend(order["data"])

    def lane(self, e, parent, pool, lane_of):
        d = self.doc
        lane = M.Lane(self.tid(e.get("id")), clean_label(e.get("name")))
        lane.pool = pool.id
        lane.parent = parent
        d.lanes[lane.id] = lane
        d.ids[lane.id] = lane
        self.kind_of[lane.id] = "lane"
        lane._sub = []
        for ref in e.findall(B + "flowNodeRef"):
            xid = (ref.text or "").strip()
            if xid:
                # the deepest lane wins (some tools list a node in parent and child lanes)
                lane_of[self.tid(xid)] = lane.id
        for cls in e.findall(B + "childLaneSet"):
            for c in cls.findall(B + "lane"):
                lane._sub.append(self.lane(c, lane.id, pool, lane_of))
        return lane

    def sort_lanes(self, lanes, pos):
        for lane in lanes:
            self.sort_lanes(lane._sub, pos)
            nodes = [c for c in lane.children if c in self.doc.nodes]
            lane.children = self.merge(lane._sub, nodes, pos)

    def first_pos(self, lane, pos):
        best = None
        for c in lane.children:
            k = pos.get(c) if c in self.doc.nodes else self.first_pos(self.doc.lanes[c], pos)
            if k is not None and (best is None or k < best):
                best = k
        return best

    def merge(self, lanes, nodes, pos):
        """Order lanes (kept in XML order) and nodes by their first position in the process."""
        items = []
        last = -1.0
        for k, lane in enumerate(lanes):
            p = self.first_pos(lane, pos)
            key = float(p) if p is not None else last + 0.001
            last = key
            items.append((key, 1, k, lane.id))
        for nid in nodes:
            items.append((float(pos.get(nid, 1e9)), 0, 0, nid))
        items.sort()
        return [x[3] for x in items]

    def container(self, e, scope, pool, lane_of, sub=None):
        """Import flow nodes and data of a process or sub-process. Return ids in order."""
        d = self.doc
        children, nodes, data = [], [], []
        for c in e:
            if not is_bpmn(c):
                continue
            t = local(c.tag)
            if c.get("id") in getattr(self, "skip_ids", ()):
                continue
            if t in KINDS or t in APPROXIMATE:
                n = self.node(c, t, scope, pool)
                children.append(n.id)
                nodes.append(n.id)
                if n.kind in M.SUB_KINDS:
                    inner = self.container(c, n.id, pool, lane_of, n)
                    n.children = inner["children"] + inner["data"]
            elif t in UNSUPPORTED_NODES:
                self.warn(c.get("id"), "%s is not supported in tok and was skipped" % t)
            elif t in ("dataObjectReference", "dataStoreReference"):
                x = self.data(c, t, scope, sub.id if sub is not None else (pool.id if pool else None))
                data.append(x.id)
            elif t == "laneSet" and sub is not None:
                self.warn(c.get("id"), "lanes inside a sub-process are not supported and were ignored")
            elif t == "ioSpecification":
                self.warn(e.get("id"), "process input/output specification was skipped", once="ioSpec")
        if sub is None and pool is None:
            children = children + data
        return {"children": children, "nodes": nodes, "data": data}

    def node(self, e, t, scope, pool):
        d = self.doc
        kind = KINDS.get(t) or APPROXIMATE[t]
        if t in APPROXIMATE:
            self.warn(e.get("id"), "%s imported as an embedded sub-process" % t)
        if t == "subProcess" and e.get("triggeredByEvent") == "true":
            kind = "eventsub"
        n = M.Node(kind, self.tid(e.get("id")), clean_label(e.get("name")))
        n.scope = scope
        n.pool = pool.id if pool is not None else None
        a = n.attrs
        if kind == "boundary":
            a["on"] = self.tid(e.get("attachedToRef"))
            if e.get("cancelActivity") == "false":
                n.flags.append("noninterrupting")
        if kind == "start" and e.get("isInterrupting") == "false":
            n.flags.append("noninterrupting")
        if kind == "call" and e.get("calledElement"):
            a["process"] = e.get("calledElement")
        if kind == "adhoc":
            self.uses_03()
            if e.get("ordering") == "Sequential":
                n.flags.append("sequential")
            if e.get("cancelRemainingInstances") == "false":
                n.flags.append("wait")
            cc = e.find(B + "completionCondition")
            if cc is not None and (cc.text or "").strip():
                a["until"] = strip_feel(cc.text)
        for xml_id, (act, handler, _) in getattr(self, "compensation", {}).items():
            if act == e.get("id"):
                a["compensate"] = self.tid(handler)
                self.uses_03()
        if e.get("isForCompensation") == "true":
            n.flags.append("compensation")
        if kind in ("send", "receive") and e.get("messageRef"):
            a["message"] = self.named(e.get("messageRef"), "message")
            if kind == "receive":
                corr = self.correlation(e.get("messageRef"))
                if corr:
                    self.warn(n.id, "correlation key of a receive task cannot be expressed in tok; use a "
                                    "message catch event")
        if kind == "script":
            s = e.find(B + "script")
            if s is not None and (s.text or "").strip():
                a["expr"] = strip_feel(s.text)
        if kind in M.EVENT_KINDS:
            self.event_definitions(e, n)
        if e.find(B + "standardLoopCharacteristics") is not None:
            n.flags.append("loop")
        mi = e.find(B + "multiInstanceLoopCharacteristics")
        if mi is not None:
            n.flags.append("sequential" if mi.get("isSequential") == "true" else "parallel")
            lc = mi.find(B + "extensionElements/" + Z + "loopCharacteristics")
            if lc is not None:
                self.uses_03()
                if lc.get("inputCollection"):
                    a["collection"] = strip_feel(lc.get("inputCollection"))
                if lc.get("inputElement"):
                    a["item"] = lc.get("inputElement")
                if lc.get("outputCollection"):
                    a["output"] = lc.get("outputCollection")
                if lc.get("outputElement"):
                    a["output-item"] = strip_feel(lc.get("outputElement"))
            cc = mi.find(B + "completionCondition")
            if cc is not None and (cc.text or "").strip():
                a["until"] = strip_feel(cc.text)
                self.uses_03()
        self.extensions(e, n)
        if e.find(B + "ioSpecification") is not None and kind != "sub":
            self.warn(n.id, "input/output specification was skipped", once="io-" + n.id)
        d.nodes[n.id] = n
        d.ids[n.id] = n
        self.kind_of[n.id] = kind
        return n

    @staticmethod
    def is_rest(ext):
        td = ext.find(Z + "taskDefinition")
        return td is not None and td.get("type") == REST_CONNECTOR_TYPE

    def read_binding(self, z, a):
        if z.get("bindingType"):
            a["binding"] = BINDINGS.get(z.get("bindingType"), z.get("bindingType"))
            self.uses_03()
        if z.get("versionTag"):
            a["version"] = z.get("versionTag")
            self.uses_03()

    def correlation(self, msg_ref):
        msg = self.by_id.get(msg_ref)
        if msg is None:
            return None
        sub = msg.find(B + "extensionElements/" + Z + "subscription")
        if sub is not None and sub.get("correlationKey"):
            return strip_feel(sub.get("correlationKey"))
        return None

    def event_definitions(self, e, n):
        defs = [c for c in e if is_bpmn(c) and local(c.tag) in DEFINITIONS]
        for c in e:
            if is_bpmn(c) and local(c.tag) == "eventDefinitionRef":
                self.warn(n.id, "reusable event definitions (eventDefinitionRef) are not supported")
        if not defs:
            return
        if len(defs) > 1:
            self.warn(n.id, "multiple event definitions: only the first one was imported")
        c = defs[0]
        key = DEFINITIONS[local(c.tag)]
        a = n.attrs
        if key in M.TRIGGER_FLAGS:
            n.flags.append(key)
            if key == "compensate" and c.get("activityRef"):
                a["activity"] = self.tid(c.get("activityRef"))
                self.uses_03()
        elif key == "message":
            ref = c.get("messageRef")
            a["message"] = self.named(ref, "message") if ref else (n.label or n.id)
            if not ref:
                self.warn(n.id, "message event without messageRef; the event label was used as message name")
            corr = self.correlation(ref) if ref else None
            if corr:
                a["correlation"] = corr
        elif key in ("signal", "error", "escalation"):
            ref = c.get(key + "Ref")
            if ref:
                a[key] = self.named(ref, key)
                decl = self.by_id.get(ref)
                code = decl.get(key + "Code") if decl is not None else None
                if code:
                    a["code"] = code
            elif key in M.CATCH_ALL_FLAGS and n.kind in ("boundary", "start"):
                n.flags.append(key)
                self.uses_03()
            else:
                a[key] = n.label or n.id
                self.warn(n.id, "%s event without a reference; the event label was used as name" % key)

        elif key == "timer":
            value = None
            for part in ("timeDuration", "timeCycle", "timeDate"):
                x = c.find(B + part)
                if x is not None and (x.text or "").strip():
                    value = x.text.strip()
                    break
            if value is None:
                self.warn(n.id, "timer without a value; P1D was used")
                value = "P1D"
            a["timer"] = value
        elif key == "condition":
            x = c.find(B + "condition")
            a["condition"] = strip_feel(x.text if x is not None else "") or "true"
        elif key == "link":
            a["link"] = c.get("name") or n.label or n.id

    def extensions(self, e, n):
        x = e.find(B + "extensionElements")
        if x is None:
            return
        a = n.attrs
        for z in x:
            t = local(z.tag)
            if not z.tag.startswith(Z):
                self.warn(n.id, "extension element %s was skipped" % z.tag, once="ext-" + z.tag)
                continue
            if t == "userTask" and n.kind == "user":
                continue
            if t == "formDefinition" and n.kind in ("user", "start"):
                if z.get("formId"):
                    a["form"] = z.get("formId")
                    self.read_binding(z, a)
                else:
                    self.warn(n.id, "form reference without formId (formKey or external form) was skipped")
            elif t == "calledDecision" and n.kind == "rule":
                if z.get("decisionId"):
                    a["decision"] = z.get("decisionId")
                if z.get("resultVariable"):
                    a["result"] = z.get("resultVariable")
                self.read_binding(z, a)
            elif t == "calledElement" and n.kind == "call":
                if z.get("processId"):
                    a["process"] = z.get("processId")
                self.read_binding(z, a)
                if z.get("propagateAllChildVariables") == "false":
                    a["child-vars"] = "none"
                if z.get("propagateAllParentVariables") == "false":
                    a["parent-vars"] = "none"
                if z.get("businessId"):
                    a["business-id"] = z.get("businessId")
                if len(z.attrib) > 1:
                    self.uses_03()
            elif t == "adHoc" and n.kind == "adhoc":
                if z.get("activeElementsCollection"):
                    a["active"] = strip_feel(z.get("activeElementsCollection"))
            elif t == "assignmentDefinition" and n.kind == "user":
                for attr, key in (("assignee", "assignee"), ("candidateUsers", "candidate-users"),
                                  ("candidateGroups", "candidate-groups")):
                    if z.get(attr):
                        a[key] = z.get(attr)
                self.uses_03()
            elif t == "taskSchedule" and n.kind == "user":
                for attr, key in (("dueDate", "due"), ("followUpDate", "follow-up")):
                    if z.get(attr):
                        a[key] = z.get(attr)
                self.uses_03()
            elif t == "priorityDefinition" and n.kind == "user":
                if z.get("priority"):
                    a["priority"] = z.get("priority")
                self.uses_03()
            elif t == "ioMapping":
                ins = [(i.get("target"), i.get("source") or "") for i in z.findall(Z + "input")]
                outs = [(o.get("target"), o.get("source") or "") for o in z.findall(Z + "output")]
                if a.get("connector") == "rest" or a.get("type") == REST_CONNECTOR_TYPE:
                    rest = []
                    for target, source in ins:
                        if target in CONNECTOR_INPUTS:
                            a[CONNECTOR_INPUTS[target]] = source
                        elif target == "authentication.type" and source == "noAuth":
                            continue
                        else:
                            rest.append((target, source))
                    ins = rest
                    a["connector"] = "rest"
                    a.pop("type", None)
                if ins:
                    a["inputs"] = "; ".join("%s = %s" % (tg, strip_feel(src)) for tg, src in ins)
                if outs:
                    a["outputs"] = "; ".join("%s = %s" % (tg, strip_feel(src)) for tg, src in outs)
                self.uses_03()
            elif t == "taskHeaders":
                hs = [(h.get("key"), h.get("value") or "") for h in z.findall(Z + "header")]
                if a.get("connector") == "rest" or self.is_rest(x):
                    keep = []
                    for k, v in hs:
                        if k == "resultVariable":
                            a["result"] = v
                        else:
                            keep.append((k, v))
                    hs = keep
                if hs:
                    a["headers"] = "; ".join("%s: %s" % (k, v) for k, v in hs)
                self.uses_03()
            elif t == "conditionalFilter":
                if z.get("variableEvents"):
                    a["events"] = z.get("variableEvents")
                    self.uses_03()
            elif t in ("executionListeners", "taskListeners"):
                items = []
                for li in z:
                    ev, typ = li.get("eventType"), li.get("type")
                    if ev and typ:
                        items.append("%s: %s%s" % (ev, typ, " (%s)" % li.get("retries") if li.get("retries") else ""))
                if items:
                    prev = a.get("listeners")
                    a["listeners"] = "; ".join(([prev] if prev else []) + items)
                    self.uses_03()
            elif t == "script" and n.kind == "script":
                if z.get("expression"):
                    a["expr"] = strip_feel(z.get("expression"))
                if z.get("resultVariable"):
                    a["result"] = z.get("resultVariable")
            elif t == "taskDefinition" and (n.kind in ("service", "send") or
                                            (n.kind in ("throw", "end") and "message" in a)):
                if z.get("type"):
                    a["type"] = z.get("type")
                    if a["type"] == REST_CONNECTOR_TYPE and n.kind == "service":
                        a["connector"] = "rest"
                        del a["type"]
                        self.uses_03()
                if z.get("retries"):
                    a["retries"] = z.get("retries")
                    self.uses_03()
                if n.kind in ("throw", "end"):
                    self.uses_03()
            else:
                self.warn(n.id, "Camunda extension zeebe:%s was skipped" % t, once="zeebe-%s-%s" % (t, n.kind))

    # ------------------------------------------------------------------ data
    def data(self, e, t, scope, block):
        d = self.doc
        did = self.tid(e.get("id"))
        if t == "dataObjectReference":
            obj = self.by_id.get(e.get("dataObjectRef"))
            label = clean_label(e.get("name")) or (clean_label(obj.get("name")) if obj is not None else None)
            x = M.Data("data", did, label)
            st = e.find(B + "dataState")
            if st is not None and st.get("name"):
                x.attrs["state"] = st.get("name")
            if obj is not None and obj.get("isCollection") == "true":
                x.flags.append("collection")
        else:
            store = self.by_id.get(e.get("dataStoreRef"))
            label = clean_label(e.get("name")) or (clean_label(store.get("name")) if store is not None else None)
            x = M.Data("store", did, label)
        # top-level data of a single process document has no explicit scope (as written by the parser)
        if block is None:
            x.scope, x.block = None, None
        else:
            x.scope, x.block = scope, block
        d.data[did] = x
        d.ids[did] = x
        self.kind_of[did] = x.kind
        return x

    # ------------------------------------------------- flows and artifacts
    def flows_and_artifacts(self):
        d = self.doc
        flows, assocs, notes, groups = [], [], [], []
        for e in self.root.iter():
            if not is_bpmn(e):
                continue
            t = local(e.tag)
            if t == "sequenceFlow":
                flows.append(e)
            elif t in ("dataInputAssociation", "dataOutputAssociation"):
                assocs.append(e)
            elif t == "textAnnotation":
                notes.append(e)
            elif t == "group":
                groups.append(e)
        for e in flows:
            self.sequence_flow(e)
        self.associations(assocs)
        annotated = {}
        for a in self.root.iter(B + "association"):
            src, tgt = a.get("sourceRef"), a.get("targetRef")
            if src in self.by_id and local(self.by_id[src].tag) == "textAnnotation":
                annotated.setdefault(src, tgt)
            elif tgt in self.by_id and local(self.by_id[tgt].tag) == "textAnnotation":
                annotated.setdefault(tgt, src)
        for e in notes:
            text = e.find(B + "text")
            note = M.Element("note", self.tid(e.get("id")), clean_label(text.text if text is not None else "") or "")
            target = annotated.get(e.get("id"))
            if target:
                note.attrs["on"] = self.tid(target)
            d.notes.append(note)
            d.ids[note.id] = note
        for e in groups:
            cv = self.by_id.get(e.get("categoryValueRef"))
            label = clean_label(cv.get("value")) if cv is not None else None
            g = M.Element("group", self.tid(e.get("id")), label or None)
            d.groups.append(g)
            d.ids[g.id] = g
            self.warn(g.id, "group membership is not stored in BPMN; add members=\"…\" by hand", once="groups")

    def sequence_flow(self, e):
        d = self.doc
        src, tgt = self.tid(e.get("sourceRef")), self.tid(e.get("targetRef"))
        if src not in d.nodes or tgt not in d.nodes:
            self.warn(e.get("id"), "sequence flow to a skipped element was dropped")
            return
        f = M.Flow(self.tid(e.get("id")), src, tgt, clean_label(e.get("name")))
        cond = e.find(B + "conditionExpression")
        if cond is not None and (cond.text or "").strip():
            f.attrs["when"] = strip_feel(cond.text)
        srcel = self.by_id.get(e.get("sourceRef"))
        if srcel is not None and srcel.get("default") == e.get("id"):
            f.flags.append("default")
        d.flows.append(f)

    def associations(self, assocs):
        d = self.doc
        items = []
        for k, e in enumerate(assocs):
            owner = self.parent.get(e)
            if owner is None or not owner.get("id"):
                continue
            act = self.tid(owner.get("id"))
            if local(e.tag) == "dataInputAssociation":
                refs = [r.text.strip() for r in e.findall(B + "sourceRef") if r.text]
                kind = "reads"
            else:
                refs = [r.text.strip() for r in e.findall(B + "targetRef") if r.text]
                kind = "writes"
            data_ids = [self.tid(r) for r in refs if self.tid(r) in d.data]
            if not data_ids:
                self.warn(e.get("id"), "data association without a data object or store was skipped")
                continue
            # keep the order of the original tok document when ids were written by tok (…-dia-N / …-doa-N)
            m = re.search(r"-(?:dia|doa)-(\d+)$", e.get("id") or "")
            key = (0, int(m.group(1))) if m else (1, k)
            for x in data_ids:
                items.append((key, k, M.Association(kind, act, x)))
        items.sort(key=lambda t: (t[0], t[1]))
        d.assocs.extend(a for _, _, a in items)

    def message_flow(self, e):
        d = self.doc
        src, tgt = self.tid(e.get("sourceRef")), self.tid(e.get("targetRef"))
        m = M.MessageFlow(self.tid(e.get("id")), src, tgt, clean_label(e.get("name")))
        ref = e.get("messageRef")
        if ref:
            name = self.named(ref, "message")
            if name != m.label:
                m.attrs["message"] = name
        d.messages.append(m)

    def derive_flow_ids(self):
        """Mark flow ids that the parser would not derive as explicit (spec §6.1)."""
        d = self.doc
        known = set(d.nodes) | set(d.lanes) | set(d.data) | {p.id for p in d.pools}
        counter = {}

        def peek(src, tgt):
            base = "f-%s-%s" % (src, tgt)
            n = counter.get(base, 0) + 1
            fid = base if n == 1 else "%s-%d" % (base, n)
            while fid in known:
                n += 1
                fid = "%s-%d" % (base, n)
            return base, fid, n

        for f in d.flows:
            base, expected, n = peek(f.source, f.target)
            if self.keep_ids and f.id != expected:
                f.explicit_id = True
            else:
                f.id = expected
                counter[base] = n
            known.add(f.id)
            d.ids[f.id] = f
        for m in d.messages:
            base, expected, n = peek("m-" + m.source, m.target)
            expected = expected[2:]
            if self.keep_ids and m.id != expected:
                m.explicit_id = True
            else:
                m.id = expected
                counter[base] = n
            known.add(m.id)
            d.ids[m.id] = m
        for x in d.notes + d.groups:
            if x.id in known:
                old = x.id
                x.id = old + "-note" if x.kind == "note" else old + "-group"
                self.warn(old, "renamed to '%s' to avoid a clash with a derived flow identifier" % x.id)


def import_bpmn(path_or_text, keep_ids=True):
    """Read BPMN 2.0 XML (a path, XML text or bytes). Return (Document, list of warning strings).

    With ``keep_ids=False`` sequence and message flows get tok's derived identifiers.
    """
    src = path_or_text
    if isinstance(src, bytes):
        root = ET.fromstring(src)
    elif isinstance(src, str) and src.lstrip().startswith("<"):
        root = ET.fromstring(src.encode("utf-8"))
    else:
        root = ET.parse(src).getroot()
    return Importer(root, keep_ids).run()


def import_file(path, keep_ids=True):
    """Convenience wrapper returning (tok text, warnings)."""
    from .printer import fmt
    doc, warnings = import_bpmn(path, keep_ids)
    return fmt(doc), warnings


__all__ = ["import_bpmn", "import_file"]
