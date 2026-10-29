#!/usr/bin/env python3
"""The catalogue of BPMN symbols, drawn by the tok engine.

Every symbol is a small tok model: the program ``tok`` exports it to BPMN (for the bounds of
its elements in the diagram) and to SVG (for the drawing), and the drawing is cut to the
element the symbol shows, without labels. The symbols are therefore exactly what ``tok svg``
draws in a diagram.

    python tools/symbols.py [-o docs/symbols] [--tok PATH] [--doc docs/symbols.md]

writes one ``ID.svg`` per symbol and ``symbols.json`` (identifier, family, BPMN element, tok
syntax, execution on Camunda 8, short descriptions in English and Polish) to the output
folder, and with ``--doc`` the reference page of the symbols. The program is taken from
``--tok``, the environment variable ``TOK``, or the ``PATH``.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

HEADER = 'tok 1.0 "Symbol"\n'
BPMNDI = "{http://www.omg.org/spec/BPMN/20100524/DI}"
DC = "{http://www.omg.org/spec/DD/20100524/DC}"
DI = "{http://www.omg.org/spec/DD/20100524/DI}"

EXECUTED = "executed"
MODELLING = "modelling only"


# --- models -----------------------------------------------------------------------------

def activity(stmt):
    return "start s0\n%s\nend e0\nflow s0 -> x -> e0\n" % stmt


def start(attrs):
    return "start x %s\ntask t\nend e0\nflow x -> t -> e0\n" % attrs


def end(attrs):
    return "start s0\ntask t\nend x %s\nflow s0 -> t -> x\n" % attrs


def end_in_transaction(attrs):
    return ("start s0\ntransaction tr\n  start s1\n  task t\n  end x %s\n  flow s1 -> t -> x\n"
            "boundary b on=tr cancel\nend e0\nend e1\nflow s0 -> tr -> e0\nflow b -> e1\n" % attrs)


def catch(attrs):
    return "start s0\ncatch x %s\nend e0\nflow s0 -> x -> e0\n" % attrs


def throw(attrs):
    return "start s0\nthrow x %s\nend e0\nflow s0 -> x -> e0\n" % attrs


def link_pair():
    return ("start s0\nthrow l1 link=\"A\"\ncatch x link=\"A\"\nend e0\n"
            "flow s0 -> l1\nflow x -> e0\n")


def boundary(attrs):
    return ("start s0\ntask t\nend e0\nend e1\nboundary x on=t %s\n"
            "flow s0 -> t -> e0\nflow x -> e1\n" % attrs)


def boundary_cancel():
    return ("start s0\ntransaction t\n  start s1\n  task a\n  end c1 cancel\n  flow s1 -> a -> c1\n"
            "boundary x on=t cancel\nend e0\nend e1\nflow s0 -> t -> e0\nflow x -> e1\n")


def boundary_compensation():
    return ("start s0\nservice t compensate=h\nservice h compensation\nend e0\n"
            "flow s0 -> t -> e0\n")


def event_sub(attrs):
    return ("start s0\ntask t\nend e0\nflow s0 -> t -> e0\n"
            "eventsub es\n  start x %s\n  task h\n  end e1\n  flow x -> h -> e1\n" % attrs)


def gateway(stmt):
    return ("start s0\n%s\ntask a\ntask b\nend e0\nend e1\n"
            "flow s0 -> x\nflow x -> a -> e0\nflow x -> b -> e1\n" % stmt)


def event_gateway(stmt):
    return ("start s0\n%s\ncatch c1 message=\"A\"\ncatch c2 timer=PT1H\nend e0\nend e1\n"
            "flow s0 -> x\nflow x -> c1 -> e0\nflow x -> c2 -> e1\n" % stmt)


def instantiating_gateway(flags):
    return ("pool p \"P\"\n  eventgw x instantiate %s\n  catch c1 message=\"A\"\n"
            "  catch c2 message=\"B\"\n  end e0\n  end e1\n"
            "  flow x -> c1 -> e0\n  flow x -> c2 -> e1\n" % flags).replace("  \n", "\n")


def with_data(decl, link):
    return "start s0\ntask t\nend e0\nflow s0 -> t -> e0\n%s\n%s\n" % (decl, link)


SEQ = "start s0\ntask a\ntask b\nend e0\nflow s0 -> a\nflow a -> b\nflow b -> e0\n"


# --- catalogue --------------------------------------------------------------------------
# (id, family, BPMN element, tok syntax, Camunda 8, English, Polish, model, focus)
# focus: element ids whose bounds make the symbol; a flow given as "edge:SOURCE>TARGET".

S = []


def sym(id_, family, bpmn, tok, camunda, en, pl, model, focus=("x",), labels=False):
    S.append({"id": id_, "family": family, "bpmn": bpmn, "tok": tok, "camunda": camunda,
              "en": en, "pl": pl, "model": model, "focus": list(focus), "labels": labels})


TRIGGERS = {
    # key: (tok attributes, English, Polish)
    "message": ('message="M"', "message", "wiadomość"),
    "timer": ("timer=P1D", "timer", "czas"),
    "conditional": ('condition="x > 1"', "conditional", "warunek"),
    "signal": ('signal="S"', "signal", "sygnał"),
    "error": ('error="E"', "error", "błąd"),
    "escalation": ('escalation="X"', "escalation", "eskalacja"),
    "compensation": ("compensate", "compensation", "kompensacja"),
    "multiple": ('message="A" signal="B"', "multiple", "wielokrotne"),
    "parallel-multiple": ('message="A" message="B" parallel', "parallel multiple",
                          "równoległe wielokrotne"),
}


def trig(key):
    return TRIGGERS[key]


# events: start
sym("event-start-none", "Start events", "StartEvent", "start ID", EXECUTED,
    "start event", "zdarzenie początkowe", start(""))
for k in ("message", "timer", "conditional", "signal", "multiple", "parallel-multiple"):
    a, en, pl = trig(k)
    cam = EXECUTED
    if k == "conditional":
        cam = "executed in event sub-processes"
    if k in ("multiple", "parallel-multiple"):
        cam = "not executed (C104)" if k == "multiple" else "not executed (C105)"
    sym("event-start-" + k, "Start events", "StartEvent with %s trigger" % en, "start ID " + a,
        cam, "%s start event" % en, "zdarzenie początkowe: %s" % pl, start(a))

# events: start of event sub-processes
for k in ("message", "timer", "conditional", "signal", "error", "escalation", "compensation",
          "multiple", "parallel-multiple"):
    a, en, pl = trig(k)
    cam = EXECUTED
    if k == "multiple":
        cam = "not executed (C104)"
    if k == "parallel-multiple":
        cam = "not executed (C105)"
    sym("event-subprocess-start-" + k, "Start events of event sub-processes",
        "StartEvent with %s trigger in an event sub-process (interrupting)" % en,
        "eventsub … / start ID " + a, cam,
        "interrupting %s start event of an event sub-process" % en,
        "przerywające zdarzenie początkowe podprocesu zdarzeniowego: %s" % pl, event_sub(a))
for k in ("message", "timer", "conditional", "signal", "escalation", "multiple",
          "parallel-multiple"):
    a, en, pl = trig(k)
    cam = EXECUTED
    if k == "multiple":
        cam = "not executed (C104)"
    if k == "parallel-multiple":
        cam = "not executed (C105)"
    sym("event-subprocess-start-%s-noninterrupting" % k, "Start events of event sub-processes",
        "StartEvent with %s trigger in an event sub-process (non-interrupting)" % en,
        "eventsub … / start ID %s noninterrupting" % a, cam,
        "non-interrupting %s start event of an event sub-process" % en,
        "nieprzerywające zdarzenie początkowe podprocesu zdarzeniowego: %s" % pl,
        event_sub(a + " noninterrupting"))

# events: intermediate catching
for k in ("message", "timer", "conditional", "signal", "multiple", "parallel-multiple"):
    a, en, pl = trig(k)
    cam = EXECUTED
    if k == "multiple":
        cam = "not executed (C104)"
    if k == "parallel-multiple":
        cam = "not executed (C105)"
    sym("event-catch-" + k, "Intermediate events",
        "IntermediateCatchEvent with %s trigger" % en, "catch ID " + a, cam,
        "catching %s intermediate event" % en,
        "zdarzenie pośrednie przechwytujące: %s" % pl, catch(a))
sym("event-catch-link", "Intermediate events", "IntermediateCatchEvent with link trigger",
    'catch ID link="L"', EXECUTED, "catching link intermediate event",
    "zdarzenie pośrednie przechwytujące: łącznik", link_pair())

# events: intermediate throwing
sym("event-throw-none", "Intermediate events", "IntermediateThrowEvent", "throw ID", EXECUTED,
    "intermediate event (none)", "zdarzenie pośrednie", throw(""))
for k in ("message", "signal", "escalation", "compensation", "multiple"):
    a, en, pl = trig(k)
    cam = "not executed (C104)" if k == "multiple" else EXECUTED
    model = throw(a)
    if k == "message":
        model = throw(a + ' type="job"')
    if k == "compensation":
        model = ("start s0\nservice a compensate=h\nservice h compensation\nthrow x compensate\n"
                 "end e0\nflow s0 -> a -> x -> e0\n")
    sym("event-throw-" + k, "Intermediate events",
        "IntermediateThrowEvent with %s trigger" % en, "throw ID " + a, cam,
        "throwing %s intermediate event" % en, "zdarzenie pośrednie rzucające: %s" % pl, model)
sym("event-throw-link", "Intermediate events", "IntermediateThrowEvent with link trigger",
    'throw ID link="L"', EXECUTED, "throwing link intermediate event",
    "zdarzenie pośrednie rzucające: łącznik", link_pair(), focus=("l1",))

# events: boundary
for k in ("message", "timer", "conditional", "signal", "error", "escalation", "multiple",
          "parallel-multiple"):
    a, en, pl = trig(k)
    cam = EXECUTED
    if k == "multiple":
        cam = "not executed (C104)"
    if k == "parallel-multiple":
        cam = "not executed (C105)"
    sym("event-boundary-" + k, "Boundary events",
        "BoundaryEvent with %s trigger (interrupting)" % en, "boundary ID on=ACTIVITY " + a, cam,
        "interrupting %s boundary event" % en,
        "przerywające zdarzenie graniczne: %s" % pl, boundary(a), focus=("t", "x"))
sym("event-boundary-cancel", "Boundary events", "BoundaryEvent with cancel trigger",
    "boundary ID on=TRANSACTION cancel", "not executed (C101)", "cancel boundary event",
    "zdarzenie graniczne: anulowanie", boundary_cancel(), focus=("t", "x"))
sym("event-boundary-compensation", "Boundary events",
    "BoundaryEvent with compensation trigger", "service ID compensate=HANDLER", EXECUTED,
    "compensation boundary event", "zdarzenie graniczne: kompensacja", boundary_compensation(),
    focus=("t", "t-compensate"))
for k in ("message", "timer", "conditional", "signal", "escalation", "multiple",
          "parallel-multiple"):
    a, en, pl = trig(k)
    cam = EXECUTED
    if k == "multiple":
        cam = "not executed (C104)"
    if k == "parallel-multiple":
        cam = "not executed (C105)"
    sym("event-boundary-%s-noninterrupting" % k, "Boundary events",
        "BoundaryEvent with %s trigger (non-interrupting)" % en,
        "boundary ID on=ACTIVITY %s noninterrupting" % a, cam,
        "non-interrupting %s boundary event" % en,
        "nieprzerywające zdarzenie graniczne: %s" % pl, boundary(a + " noninterrupting"),
        focus=("t", "x"))

# events: end
sym("event-end-none", "End events", "EndEvent", "end ID", EXECUTED, "end event",
    "zdarzenie końcowe", end(""))
for k in ("message", "error", "escalation", "signal", "compensation", "multiple"):
    a, en, pl = trig(k)
    cam = "not executed (C104)" if k == "multiple" else EXECUTED
    model = end(a)
    if k == "message":
        model = end(a + ' type="job"')
    if k == "compensation":
        model = ("start s0\nservice a compensate=h\nservice h compensation\nend x compensate\n"
                 "flow s0 -> a -> x\n")
    sym("event-end-" + k, "End events", "EndEvent with %s trigger" % en, "end ID " + a, cam,
        "%s end event" % en, "zdarzenie końcowe: %s" % pl, model)
sym("event-end-cancel", "End events", "EndEvent with cancel trigger", "end ID cancel",
    "not executed (C101)", "cancel end event", "zdarzenie końcowe: anulowanie",
    end_in_transaction("cancel"))
sym("event-end-terminate", "End events", "EndEvent with terminate trigger", "end ID terminate",
    EXECUTED, "terminate end event", "zdarzenie końcowe: zakończenie wszystkiego",
    end("terminate"))

# activities
TASKS = [
    ("task", "Task", "task", "task", "zadanie"),
    ("user", "UserTask", "user", "user task", "zadanie użytkownika"),
    ("manual", "ManualTask", "manual", "manual task", "zadanie ręczne"),
    ("service", "ServiceTask", "service", "service task", "zadanie usługowe"),
    ("script", "ScriptTask", "script", "script task", "zadanie skryptowe"),
    ("rule", "BusinessRuleTask", "rule", "business rule task", "zadanie reguły biznesowej"),
    ("send", "SendTask", "send", "send task", "zadanie wysyłania"),
    ("receive", "ReceiveTask", "receive", "receive task", "zadanie odbioru"),
]
for key, bpmn, kw, en, pl in TASKS:
    sym("task-" + key, "Activities", bpmn, kw + " ID", EXECUTED, en, pl, activity(kw + " x"))
sym("task-receive-instantiating", "Activities", "ReceiveTask (instantiate)",
    'receive ID message="M" instantiate', "not executed (C106)",
    "instantiating receive task", "zadanie odbioru rozpoczynające proces",
    'pool p "P"\n  receive x message="M" instantiate\n  task t\n  end e0\n  flow x -> t -> e0\n')
sym("subprocess", "Activities", "SubProcess", "sub ID (block)", EXECUTED,
    "embedded sub-process", "podproces osadzony",
    "start s0\nsub x\n  start s1\n  task a\n  end e1\n  flow s1 -> a -> e1\nend e0\n"
    "flow s0 -> x -> e0\n")
sym("subprocess-event", "Activities", "SubProcess (triggeredByEvent)", "eventsub ID (block)",
    EXECUTED, "event sub-process", "podproces zdarzeniowy", event_sub('message="M"'),
    focus=("es",))
sym("subprocess-transaction", "Activities", "Transaction", "transaction ID (block)",
    "not executed (C100)", "transaction", "transakcja",
    "start s0\ntransaction x\n  start s1\n  task a\n  end e1\n  flow s1 -> a -> e1\nend e0\n"
    "flow s0 -> x -> e0\n")
sym("subprocess-adhoc", "Activities", "AdHocSubProcess", "adhoc ID (block)", EXECUTED,
    "ad-hoc sub-process", "podproces ad hoc",
    "start s0\nadhoc x\n  task a\n  task b\nend e0\nflow s0 -> x -> e0\n")
sym("call-activity", "Activities", "CallActivity (process)", "call ID process=P", EXECUTED,
    "call activity", "czynność wywołująca", activity('call x process=other'))
sym("call-activity-global-task", "Activities", "CallActivity (global task)",
    "call ID task=GLOBAL", "not executed (C108)", "call activity of a global user task",
    "czynność wywołująca zadanie globalne",
    'global g "G" kind=user\n' + activity("call x task=g"))
sym("marker-loop", "Activity markers", "standard loop", "task ID loop", "not executed (C103)",
    "task with the loop marker", "zadanie ze znacznikiem pętli", activity("task x loop"))
sym("marker-multi-instance-parallel", "Activity markers", "parallel multi-instance",
    "task ID parallel", EXECUTED, "task with the parallel multi-instance marker",
    "zadanie wielokrotne równoległe", activity("task x parallel"))
sym("marker-multi-instance-sequential", "Activity markers", "sequential multi-instance",
    "task ID sequential", EXECUTED, "task with the sequential multi-instance marker",
    "zadanie wielokrotne sekwencyjne", activity("task x sequential"))
sym("marker-compensation", "Activity markers", "compensation activity",
    "service ID compensation", EXECUTED, "task with the compensation marker",
    "zadanie kompensujące", boundary_compensation(), focus=("h",))

# gateways
sym("gateway-exclusive", "Gateways", "ExclusiveGateway", "xor ID", EXECUTED,
    "exclusive gateway", "bramka wykluczająca", gateway('xor x'))
sym("gateway-inclusive", "Gateways", "InclusiveGateway", "or ID", EXECUTED,
    "inclusive gateway", "bramka inkluzywna", gateway('or x'))
sym("gateway-parallel", "Gateways", "ParallelGateway", "and ID", EXECUTED,
    "parallel gateway", "bramka równoległa", gateway("and x"))
sym("gateway-complex", "Gateways", "ComplexGateway", "complex ID", "not executed (C102)",
    "complex gateway", "bramka złożona", gateway("complex x"))
sym("gateway-event-based", "Gateways", "EventBasedGateway", "eventgw ID", EXECUTED,
    "event-based gateway", "bramka oparta na zdarzeniach", event_gateway("eventgw x"))
sym("gateway-event-based-instantiating", "Gateways", "EventBasedGateway (instantiate)",
    "eventgw ID instantiate", "not executed (C107)",
    "instantiating exclusive event-based gateway",
    "bramka oparta na zdarzeniach rozpoczynająca proces", instantiating_gateway(""))
sym("gateway-event-based-instantiating-parallel", "Gateways",
    "EventBasedGateway (instantiate, parallel)", "eventgw ID instantiate parallel",
    "not executed (C107)", "instantiating parallel event-based gateway",
    "równoległa bramka oparta na zdarzeniach rozpoczynająca proces",
    instantiating_gateway("parallel"))

# data
sym("data-object", "Data", "DataObjectReference", "data ID", MODELLING, "data object",
    "obiekt danych", with_data('data x "D"', "writes t x"))
sym("data-object-collection", "Data", "DataObjectReference (collection)",
    "data ID collection", MODELLING, "collection data object", "kolekcja obiektów danych",
    with_data('data x "D" collection', "writes t x"))
sym("data-input", "Data", "DataInput", "input ID", MODELLING, "data input", "dane wejściowe",
    with_data('input x "I"', "reads t x"))
sym("data-output", "Data", "DataOutput", "output ID", MODELLING, "data output",
    "dane wyjściowe", with_data('output x "O"', "writes t x"))
sym("data-store", "Data", "DataStoreReference", "store ID", MODELLING, "data store",
    "składnica danych", with_data('store x "S"', "writes t x"))

# connections
sym("flow-sequence", "Connections", "SequenceFlow", "flow A -> B", EXECUTED, "sequence flow",
    "przepływ sekwencji", SEQ, focus=("edge:a>b",))
sym("flow-conditional", "Connections", "SequenceFlow with condition from an activity",
    'flow A -> B when="feel"', EXECUTED, "conditional sequence flow",
    "przepływ warunkowy",
    'start s0\ntask a\ntask b\nend e0\nend e1\nflow s0 -> a\nflow a -> b when="x > 1"\n'
    'flow a -> e1 default\nflow b -> e0\n', focus=("edge:a>b",))
sym("flow-default", "Connections", "SequenceFlow (default)", "flow G -> B default", EXECUTED,
    "default sequence flow", "przepływ domyślny",
    'start s0\nxor g\ntask a\ntask b\nend e0\nend e1\nflow s0 -> g\nflow g -> b default\n'
    'flow g -> a when="x > 1"\nflow a -> e0\nflow b -> e1\n', focus=("edge:g>b",))
sym("flow-message", "Connections", "MessageFlow", "message A -> B", EXECUTED, "message flow",
    "przepływ wiadomości",
    'pool p "P"\n  start s0\n  send a message="M"\n  end e0\n  flow s0 -> a -> e0\n'
    'pool q "Q"\nmessage a -> q\n', focus=("edge:a>q",))
sym("association", "Connections", "Association", "note ID \"Text\" on=ID", MODELLING,
    "association", "powiązanie",
    'start s0\ntask t\nend e0\nflow s0 -> t -> e0\nnote n "Note" on=t\n', focus=("edge:n>t",))
sym("association-data", "Connections", "DataOutputAssociation", "writes ACTIVITY DATA",
    MODELLING, "data association", "powiązanie danych",
    with_data('data d "D"', "writes t d"), focus=("edge:t>d",))

# artifacts
sym("annotation", "Artifacts", "TextAnnotation", 'note ID "Text"', MODELLING,
    "text annotation", "adnotacja tekstowa",
    'start s0\ntask t\nend e0\nflow s0 -> t -> e0\nnote x "Text" on=t\n', labels=True)

# swimlanes
sym("pool", "Pools and lanes", "Participant (pool) with a process", 'pool ID "Label" (block)',
    EXECUTED, "pool", "basen",
    'pool x "P"\n  start s0\n  task t\n  end e0\n  flow s0 -> t -> e0\n')
sym("lane", "Pools and lanes", "Lane", 'lane ID "Label"', EXECUTED, "pool with two lanes",
    "basen z dwiema liniami",
    'pool x "P"\n  lane l1 "A"\n    start s0\n    task t\n  lane l2 "B"\n    task u\n'
    '    end e0\n  flow s0 -> t -> u -> e0\n')
sym("pool-black-box", "Pools and lanes", "Participant without a process (black-box pool)",
    'pool ID "Label" (no block)', MODELLING, "black-box pool", "basen zamknięty",
    'pool p "P"\n  start s0\n  send a message="M"\n  end e0\n  flow s0 -> a -> e0\n'
    'pool x "Q"\nmessage a -> x\n')
sym("pool-multiplicity", "Pools and lanes", "Participant multiplicity",
    'pool ID "Label" min=2 max=5', "not executed (C123)", "pool with participant multiplicity",
    "basen z krotnością uczestników",
    'pool p "P"\n  start s0\n  send a message="M"\n  end e0\n  flow s0 -> a -> e0\n'
    'pool x "Q" min=2 max=5\nmessage a -> x\n')

# conversations
CONV = ('pool p "P"\n  start s0\n  send a message="M"\n  end e0\n  flow s0 -> a -> e0\n'
        'pool q "Q"\n  start s1 message="M"\n  end e1\n  flow s1 -> e1\n'
        'message a -> s1 id=m1\n')
sym("conversation", "Conversations", "Conversation", 'conversation ID "Label" pools="P Q"',
    "not executed (C120)", "conversation", "konwersacja",
    CONV + 'conversation x "C" pools="p q" messages="m1"\n')
sym("conversation-sub", "Conversations", "SubConversation",
    'subconversation ID "Label" pools="P Q" (block)', "not executed (C120)",
    "sub-conversation", "podkonwersacja",
    CONV + 'subconversation x "S" pools="p q"\n  conversation c1 "C" pools="p q" messages="m1"\n')
sym("conversation-call", "Conversations", "CallConversation",
    'callconversation ID "Label" pools="P Q" collaboration=ID', "not executed (C121)",
    "call conversation", "wywołanie konwersacji",
    CONV + 'callconversation x "Call" pools="p q" collaboration=other\n')
sym("conversation-link", "Conversations", "ConversationLink", "pools= of a conversation",
    "not executed (C120)", "conversation link", "łącze konwersacji",
    CONV + 'conversation c "C" pools="p q" messages="m1"\n', focus=("edge:c>p",))

# choreographies
CHOREO = 'pool p "P"\npool q "Q"\n'
sym("choreography-task", "Choreographies", "ChoreographyTask",
    'choreo ID "Label" from=P to=Q message="M"', "not executed (C122)", "choreography task",
    "zadanie choreografii",
    CHOREO + 'choreography ch "Ch"\n  start s0\n  choreo x "Order" from=p to=q message="M"\n'
    '  end e0\n  flow s0 -> x -> e0\n')
sym("choreography-sub", "Choreographies", "SubChoreography",
    'subchoreo ID "Label" pools="P Q" from=P (block)', "not executed (C122)",
    "sub-choreography", "podchoreografia",
    CHOREO + 'choreography ch "Ch"\n  start s0\n  subchoreo x "Sub" pools="p q" from=p\n'
    '    start s1\n    choreo c1 "Order" from=p to=q\n    end e1\n    flow s1 -> c1 -> e1\n'
    '  end e0\n  flow s0 -> x -> e0\n')
sym("choreography-call", "Choreographies", "CallChoreography",
    'callchoreo ID "Label" pools="P Q" from=P choreography=ID', "not executed (C122)",
    "call choreography", "wywołanie choreografii",
    CHOREO + 'choreography ch "Ch"\n  start s0\n  callchoreo x "Call" pools="p q" from=p '
    'choreography=other\n  end e0\n  flow s0 -> x -> e0\n'
    'choreography other "Other"\n  start s1\n  choreo c1 "Order" from=p to=q\n  end e1\n'
    '  flow s1 -> c1 -> e1\n')


# --- drawing ----------------------------------------------------------------------------

def run(tok, args, cwd):
    r = subprocess.run([tok] + args, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if r.returncode != 0:
        raise RuntimeError(r.stdout.decode("utf-8", "replace"))


def geometry(bpmn_path):
    """Bounds of the shapes and waypoints of the edges of the diagram, by element id."""
    tree = ET.parse(bpmn_path)
    shapes, edges = {}, {}
    ids = {}
    parents = {child: p for p in tree.iter() for child in p}
    for el in tree.iter():
        i = el.get("id")
        if i:
            ids[i] = el
    for sh in tree.iter(BPMNDI + "BPMNShape"):
        b = sh.find(DC + "Bounds")
        if b is not None:
            shapes[sh.get("bpmnElement")] = tuple(float(b.get(k)) for k in
                                                  ("x", "y", "width", "height"))
    for ed in tree.iter(BPMNDI + "BPMNEdge"):
        el = ids.get(ed.get("bpmnElement"))
        pts = [(float(w.get("x")), float(w.get("y"))) for w in ed.iter(DI + "waypoint")]
        if el is None or not pts:
            continue
        src = el.get("sourceRef")
        tgt = el.get("targetRef")
        if src is None:  # data associations: source and target are child elements
            s = el.find(".//{*}sourceRef")
            t = el.find(".//{*}targetRef")
            src = s.text if s is not None else None
            tgt = t.text if t is not None else None
            if src not in shapes:  # output association: from the activity that holds it
                src = parents[el].get("id")
        edges[(src, tgt)] = pts
    return shapes, edges, ids


def participant_of(ids, element):
    """The participant (pool) id whose process is ``element``, or the element itself."""
    for i, el in ids.items():
        if el.tag.endswith("participant") and el.get("processRef") == element:
            return i
    return element


TEXT = re.compile(r"<text\b[^>]*>(.*?)</text>", re.S)


def crop(svg, box, labels=False):
    x, y, w, h = box
    svg = re.sub(r"<title>.*?</title>\n?", "", svg, flags=re.S)
    if not labels:
        svg = TEXT.sub(lambda m: m.group(0) if m.group(1) == "~" else "", svg)
    # the background rectangle of the whole picture
    svg = re.sub(r'<rect x="[-0-9.]+" y="[-0-9.]+" width="[0-9.]+" height="[0-9.]+" '
                 r'fill="#fff"/>\n', "", svg, count=1)
    head = re.match(r"<svg\b[^>]*>", svg).group(0)
    new_head = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="%g %g %g %g" '
                'width="%g" height="%g">' % (x, y, w, h, w, h))
    return svg.replace(head, new_head, 1)


def symbol_box(s, shapes, edges, ids):
    xs, ys, xe, ye = [], [], [], []
    pad = 3.0
    for f in s["focus"]:
        if f.startswith("edge:"):
            a, b = f[5:].split(">")
            pts = None
            for (src, tgt), p in edges.items():
                if src is None or tgt is None:
                    continue
                if {src, participant_of(ids, src)} & {a} and {tgt, participant_of(ids, tgt)} & {b}:
                    pts = p
            if pts is None:
                for (src, tgt), p in edges.items():
                    if a in (src, tgt) and (b in (src, tgt) or b == participant_of(ids, tgt)
                                            or b == participant_of(ids, src)):
                        pts = p
            if pts is None:
                raise RuntimeError("%s: no edge %s" % (s["id"], f))
            for px, py in pts:
                xs.append(px)
                ys.append(py)
                xe.append(px)
                ye.append(py)
            pad = 9.0
        else:
            if f not in shapes:
                raise RuntimeError("%s: no shape %s" % (s["id"], f))
            x, y, w, h = shapes[f]
            xs.append(x)
            ys.append(y)
            xe.append(x + w)
            ye.append(y + h)
    x0, y0 = min(xs) - pad, min(ys) - pad
    return (x0, y0, max(xe) + pad - x0, max(ye) + pad - y0)


def draw(tok, s, work):
    src = os.path.join(work, "m.tok")
    with open(src, "w", encoding="utf-8", newline="\n") as f:
        f.write(HEADER + s["model"])
    run(tok, ["bpmn", "m.tok", "-o", "m.bpmn", "--no-sidecar"], work)
    run(tok, ["svg", "m.tok", "-o", "m.svg", "--no-sidecar"], work)
    shapes, edges, ids = geometry(os.path.join(work, "m.bpmn"))
    with open(os.path.join(work, "m.svg"), encoding="utf-8") as f:
        svg = f.read()
    return crop(svg, symbol_box(s, shapes, edges, ids), s["labels"])


def page(symbols):
    """The reference page of the symbols (docs/symbols.md)."""
    out = ["# BPMN symbols", "",
           "Every BPMN 2.0.2 symbol that tok draws, with the statement that produces it and "
           "whether Camunda 8 executes it. The pictures are made by the tok engine itself "
           "(`tools/symbols.py` writes them from small models with `tok bpmn` and `tok svg`), "
           "so they are exactly what `tok svg` draws in a diagram. The catalogue with the short "
           "descriptions in English and Polish is `symbols/symbols.json`.", "",
           "\"Not executed\" means that Camunda 8 does not execute the element; `tok check` "
           "reports the code given in executable documents (`conformance-camunda.md`). "
           "\"Modelling only\" elements are exported and kept, and the engine ignores them. "
           "Groups (`group ID \"Label\" members=\"A B\"`) are exported as BPMN groups without a "
           "shape in the diagram, so they have no symbol here.", ""]
    family = None
    for s in symbols:
        if s["family"] != family:
            family = s["family"]
            out += ["## " + family, "", "| Symbol | Element | BPMN | tok | Camunda 8 |",
                    "|---|---|---|---|---|"]
        out.append('| <img src="symbols/%s.svg" alt="%s" height="36"> | %s | %s | `%s` | %s |'
                   % (s["id"], s["en"], s["en"], s["bpmn"], s["tok"], s["camunda"]))
        nxt = symbols.index(s) + 1
        if nxt == len(symbols) or symbols[nxt]["family"] != family:
            out.append("")
    return "\n".join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("-o", "--output", default=os.path.join("docs", "symbols"))
    ap.add_argument("--tok", default=os.environ.get("TOK") or shutil.which("tok"))
    ap.add_argument("--doc", help="also write the reference page of the symbols")
    args = ap.parse_args(argv)
    if not args.tok:
        sys.exit("symbols.py: the tok program was not found (--tok or TOK)")
    if os.path.exists(args.tok):
        args.tok = os.path.abspath(args.tok)
    os.makedirs(args.output, exist_ok=True)
    catalogue = []
    failed = 0
    with tempfile.TemporaryDirectory() as work:
        for s in S:
            try:
                svg = draw(args.tok, s, work)
            except RuntimeError as ex:
                failed += 1
                sys.stderr.write("%s: %s\n" % (s["id"], ex))
                continue
            with open(os.path.join(args.output, s["id"] + ".svg"), "w", encoding="utf-8",
                      newline="\n") as f:
                f.write(svg)
            catalogue.append({k: s[k] for k in ("id", "family", "bpmn", "tok", "camunda",
                                                "en", "pl")})
    with open(os.path.join(args.output, "symbols.json"), "w", encoding="utf-8",
              newline="\n") as f:
        json.dump({"symbols": catalogue}, f, ensure_ascii=False, indent=1)
        f.write("\n")
    if args.doc:
        with open(args.doc, "w", encoding="utf-8", newline="\n") as f:
            f.write(page(catalogue))
            f.write("\n")
    print("%d symbols in %s%s" % (len(catalogue), args.output,
                                  ", %d failed" % failed if failed else ""))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
