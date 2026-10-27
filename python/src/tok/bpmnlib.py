#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
bpmn.py - narzedzie kursowe do pracy z modelami BPMN 2.0 (pliki .bpmn).

Polecenia:
  check   walidacja struktury i regul BPMN 2.0 (komunikaty po polsku)
  layout  automatyczny uklad diagramu: generuje BPMNDI (wspolrzedne)
  svg     render diagramu do pliku SVG
  all     check -> layout -> check -> svg (zalecany przebieg)

Opcja --camunda (check, all) dodaje kontrole modelu wykonywalnego dla Camunda 8
(rozszerzenia zeebe:, warunki FEEL, formularze i decyzje w tym samym folderze).

Przyklady:
  python3 bpmn.py all model.bpmn --kroki
  python3 bpmn.py check model.bpmn --json
  python3 bpmn.py all proces.bpmn --camunda

Wymaga wylacznie biblioteki standardowej Pythona (3.8+).
"""
import argparse
import json
import os
import re
import sys
import xml.etree.ElementTree as ET

VERSION = "1.2"

NS = {
    "bpmn": "http://www.omg.org/spec/BPMN/20100524/MODEL",
    "bpmndi": "http://www.omg.org/spec/BPMN/20100524/DI",
    "dc": "http://www.omg.org/spec/DD/20100524/DC",
    "di": "http://www.omg.org/spec/DD/20100524/DI",
    "xsi": "http://www.w3.org/2001/XMLSchema-instance",
}
M = "{%s}" % NS["bpmn"]
BDI = "{%s}" % NS["bpmndi"]
DC = "{%s}" % NS["dc"]
DI = "{%s}" % NS["di"]
XSI = "{%s}" % NS["xsi"]

ACT = {"task", "userTask", "serviceTask", "sendTask", "receiveTask", "manualTask",
       "scriptTask", "businessRuleTask", "subProcess", "adHocSubProcess", "transaction",
       "callActivity"}
SUBP = {"subProcess", "adHocSubProcess", "transaction"}
EVT = {"startEvent", "endEvent", "intermediateCatchEvent", "intermediateThrowEvent", "boundaryEvent"}
GWS = {"exclusiveGateway", "parallelGateway", "inclusiveGateway", "eventBasedGateway", "complexGateway"}
FLOWNODE = ACT | EVT | GWS
DATA = {"dataObjectReference", "dataStoreReference"}
CONTAINER = {"process"} | SUBP

DEF_PL = {
    "": "zwykłe (bez wyzwalacza)",
    "messageEventDefinition": "wiadomość",
    "timerEventDefinition": "czasowe",
    "signalEventDefinition": "sygnał",
    "conditionalEventDefinition": "warunkowe",
    "errorEventDefinition": "błąd",
    "escalationEventDefinition": "eskalacja",
    "compensateEventDefinition": "kompensacja",
    "cancelEventDefinition": "anulowanie",
    "terminateEventDefinition": "zakończenie (terminate)",
    "linkEventDefinition": "łącznik (link)",
}
TAG_PL = {
    "startEvent": "zdarzenie początkowe", "endEvent": "zdarzenie końcowe",
    "intermediateCatchEvent": "pośrednie zdarzenie przechwytujące",
    "intermediateThrowEvent": "pośrednie zdarzenie rzucające",
    "boundaryEvent": "zdarzenie graniczne",
    "exclusiveGateway": "bramka wykluczająca (XOR)", "parallelGateway": "bramka równoległa (AND)",
    "inclusiveGateway": "bramka inkluzywna (OR)", "eventBasedGateway": "bramka oparta na zdarzeniach",
    "complexGateway": "bramka złożona",
    "task": "zadanie", "userTask": "zadanie użytkownika", "serviceTask": "zadanie usługowe",
    "sendTask": "zadanie wysyłania", "receiveTask": "zadanie odbioru", "manualTask": "zadanie ręczne",
    "scriptTask": "zadanie skryptowe", "businessRuleTask": "zadanie reguły biznesowej",
    "subProcess": "podproces", "adHocSubProcess": "podproces ad hoc", "transaction": "transakcja",
    "callActivity": "czynność wywołania",
}
ALLOWED_DEFS = {
    "start_top": {"", "messageEventDefinition", "timerEventDefinition", "signalEventDefinition",
                  "conditionalEventDefinition"},
    "start_eventsub": {"", "messageEventDefinition", "timerEventDefinition", "signalEventDefinition",
                       "conditionalEventDefinition", "errorEventDefinition", "escalationEventDefinition",
                       "compensateEventDefinition"},
    "start_sub": {""},
    "endEvent": {"", "messageEventDefinition", "errorEventDefinition", "escalationEventDefinition",
                 "signalEventDefinition", "terminateEventDefinition", "compensateEventDefinition",
                 "cancelEventDefinition"},
    "intermediateCatchEvent": {"messageEventDefinition", "timerEventDefinition", "signalEventDefinition",
                               "conditionalEventDefinition", "linkEventDefinition"},
    "intermediateThrowEvent": {"", "messageEventDefinition", "escalationEventDefinition",
                               "signalEventDefinition", "compensateEventDefinition", "linkEventDefinition"},
    "boundaryEvent": {"messageEventDefinition", "timerEventDefinition", "signalEventDefinition",
                      "conditionalEventDefinition", "errorEventDefinition", "escalationEventDefinition",
                      "compensateEventDefinition", "cancelEventDefinition"},
}
STEP_RE = re.compile(r"^\s*(\d+(?:\.\d+)?[a-z]?)(?:\s*[\u2013-]\s*\d+[a-z]?)?(?:[.):]|\s)")


def _step_numbers(name):
    """Numery kroków z początku nazwy; zakres „31–33” daje {31, 32, 33}."""
    mt = re.match(r"^\s*(\d+(?:\.\d+)?[a-z]?)(?:\s*[\u2013-]\s*(\d+[a-z]?))?(?:[.):]|\s)", name or "")
    if not mt:
        return set()
    a, b = mt.group(1), mt.group(2)
    if b and a.isdigit() and b.isdigit() and int(a) < int(b) <= int(a) + 50:
        return {str(k) for k in range(int(a), int(b) + 1)}
    return {a}


def _step_key(k):
    parts = re.findall(r"\d+|[a-z]+", k)
    return [int(p) if p.isdigit() else p for p in parts]


def read_opis(path):
    """Numery kroków z pierwszej kolumny tabel Markdown w opisie procesu."""
    steps = set()
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line.startswith("|"):
                continue
            cells = [c.strip() for c in line.strip("|").split("|")]
            if cells and re.match(r"^\d+(?:\.\d+)?[a-z]?$", cells[0]):
                steps.add(cells[0].lower())
    return steps


_KNOWN_NS = set(NS.values())


def local(tag):
    """Nazwa lokalna elementu BPMN/DI; elementy rozszerzeń (np. zeebe:) dostają prefiks „ext:”,
    żeby nie pomylić np. zeebe:userTask z bpmn:userTask."""
    if "}" not in tag:
        return tag
    uri, name = tag[1:].split("}", 1)
    return name if uri in _KNOWN_NS else "ext:" + name


def ns_uri(tag):
    return tag[1:].split("}", 1)[0] if tag.startswith("{") else ""


# --------------------------------------------------------------------------- I/O

class LoadError(Exception):
    pass


def load(path):
    """Wczytuje plik .bpmn; zwraca (tree, mapa prefiks->URI z pliku wejściowego)."""
    in_map = {}
    try:
        for _ev, item in ET.iterparse(path, events=("start-ns",)):
            prefix, uri = item
            in_map.setdefault(prefix, uri)
        tree = ET.parse(path)
    except ET.ParseError as ex:
        raise LoadError("Plik nie jest poprawnym XML: %s" % ex)
    except OSError as ex:
        raise LoadError("Nie można otworzyć pliku: %s" % ex)
    known = set(NS.values())
    for prefix, uri in in_map.items():
        if uri in known or not prefix or re.match(r"ns\d+$", prefix):
            continue
        try:
            ET.register_namespace(prefix, uri)
        except ValueError:
            pass
    for p, u in NS.items():
        ET.register_namespace(p, u)
    return tree, in_map


def _indent(elem, level=0):
    pad = "\n" + "  " * level
    if len(elem):
        if not elem.text or not elem.text.strip():
            elem.text = pad + "  "
        for child in elem:
            _indent(child, level + 1)
        last = elem[-1]
        if not last.tail or not last.tail.strip():
            last.tail = pad
    if level and (not elem.tail or not elem.tail.strip()):
        elem.tail = pad


def _fix_qnames(root, in_map):
    """Przepisuje prefiksy w wartościach xsi:type na prefiksy używane przy zapisie."""
    out_prefix = {v: k for k, v in NS.items()}
    for e in root.iter():
        v = e.get(XSI + "type")
        if not v:
            continue
        if ":" in v:
            p, loc = v.split(":", 1)
        else:
            p, loc = "", v
        uri = in_map.get(p)
        if uri in out_prefix:
            e.set(XSI + "type", "%s:%s" % (out_prefix[uri], loc))


def save(tree, path, in_map):
    root = tree.getroot()
    _fix_qnames(root, in_map)
    for e in root.iter():
        if len(e):
            if e.text is not None and not e.text.strip():
                e.text = None
            for c in e:
                if c.tail is not None and not c.tail.strip():
                    c.tail = None
    _indent(root)
    text = ET.tostring(root, encoding="unicode")
    # zachowaj deklaracje przestrzeni nazw użyte tylko w wartościach atrybutów (np. xsd:string)
    m = re.match(r"<[^>]+>", text)
    head = m.group(0) if m else ""
    extra = []
    for prefix, uri in in_map.items():
        if not prefix or re.match(r"ns\d+$", prefix):
            continue
        if ("xmlns:%s=" % prefix) not in head and uri not in NS.values():
            if re.search(r'="%s:' % re.escape(prefix), text):
                extra.append(' xmlns:%s="%s"' % (prefix, uri))
    if extra and head:
        text = head[:-1] + "".join(extra) + head[-1] + text[len(head):]
    with open(path, "w", encoding="utf-8") as f:
        f.write('<?xml version="1.0" encoding="UTF-8"?>\n')
        f.write(text)
        f.write("\n")


# ------------------------------------------------------------------------- model

class Model:
    def __init__(self, tree):
        self.tree = tree
        self.root = tree.getroot()
        self.reindex()

    def reindex(self):
        r = self.root
        self.parent = {}
        for p in r.iter():
            for c in p:
                self.parent[c] = p
        self.ids = {}
        self.dups = []
        for e in r.iter():
            i = e.get("id")
            if i is None:
                continue
            if i in self.ids:
                self.dups.append(i)
            else:
                self.ids[i] = e
        self.seq = list(r.iter(M + "sequenceFlow"))
        self.msg = list(r.iter(M + "messageFlow"))
        self.assoc = list(r.iter(M + "association"))
        self.processes = r.findall(M + "process")
        self.collabs = r.findall(M + "collaboration")
        self.participants = [p for c in self.collabs for p in c.findall(M + "participant")]
        self.out, self.inc = {}, {}
        for f in self.seq:
            self.out.setdefault(f.get("sourceRef"), []).append(f)
            self.inc.setdefault(f.get("targetRef"), []).append(f)
        self.boundaries = {}
        for b in r.iter(M + "boundaryEvent"):
            self.boundaries.setdefault(b.get("attachedToRef"), []).append(b)

    def get(self, i):
        return self.ids.get(i) if i else None

    @staticmethod
    def tag(e):
        return local(e.tag)

    def container(self, e):
        p = self.parent.get(e)
        while p is not None and local(p.tag) not in CONTAINER:
            p = self.parent.get(p)
        return p

    def process_of(self, e):
        if local(e.tag) == "process":
            return e
        p = self.parent.get(e)
        while p is not None and local(p.tag) != "process":
            p = self.parent.get(p)
        return p

    def participant_of(self, e):
        if e is None:
            return None
        if local(e.tag) == "participant":
            return e
        proc = self.process_of(e)
        if proc is None:
            return None
        for p in self.participants:
            if p.get("processRef") == proc.get("id"):
                return p
        return None

    @staticmethod
    def eventdefs(e):
        d = [local(c.tag) for c in e if local(c.tag).endswith("EventDefinition")]
        for c in e.findall(M + "eventDefinitionRef"):
            d.append("ref")
        return d

    @staticmethod
    def is_event_sub(e):
        return local(e.tag) == "subProcess" and e.get("triggeredByEvent") == "true"

    @staticmethod
    def name(e):
        if e is None:
            return "?"
        return (e.get("name") or "").strip()

    def label(self, e):
        if e is None:
            return "?"
        n = self.name(e)
        return "„%s” (%s)" % (n, e.get("id")) if n else e.get("id") or local(e.tag)

    def lanes_of(self, proc):
        res = []

        def walk(ls, depth, parent):
            for lane in ls.findall(M + "lane"):
                res.append((lane, depth, parent))
                cls = lane.find(M + "childLaneSet")
                if cls is not None:
                    walk(cls, depth + 1, lane)

        for ls in proc.findall(M + "laneSet"):
            walk(ls, 0, None)
        return res

    @staticmethod
    def is_leaf_lane(lane):
        cls = lane.find(M + "childLaneSet")
        return cls is None or not cls.findall(M + "lane")


# -------------------------------------------------------------------- validation

REF_ATTRS = ("sourceRef", "targetRef", "attachedToRef", "default", "processRef", "messageRef",
             "signalRef", "errorRef", "escalationRef", "dataObjectRef", "dataStoreRef",
             "categoryValueRef", "activityRef")
TEXT_REFS = ("flowNodeRef", "incoming", "outgoing", "sourceRef", "targetRef")


def validate(m, kroki=False, opis_kroki=None):
    issues = []

    def add(level, code, eid, msg):
        issues.append({"poziom": level, "kod": code, "id": eid, "komunikat": msg})

    r = m.root
    if r.tag != M + "definitions":
        add("błąd", "E002", None, "Korzeń dokumentu musi być <bpmn:definitions> w przestrzeni nazw "
            "http://www.omg.org/spec/BPMN/20100524/MODEL.")
        return issues

    for d in sorted(set(m.dups)):
        add("błąd", "E003", d, "Identyfikator „%s” występuje więcej niż raz." % d)

    # --- referencje
    for e in r.iter():
        t = local(e.tag)
        if ns_uri(e.tag) != NS["bpmn"]:
            continue
        for a in REF_ATTRS:
            v = e.get(a)
            if v and v not in m.ids:
                add("błąd", "E004", e.get("id"),
                    "%s %s: atrybut %s=\"%s\" wskazuje na nieistniejący element."
                    % (t, e.get("id") or "", a, v))
        if t in TEXT_REFS and e.text and e.text.strip():
            v = e.text.strip()
            if v not in m.ids:
                owner = m.parent.get(e)
                add("błąd", "E004", owner.get("id") if owner is not None else None,
                    "<%s>%s</%s> w elemencie %s wskazuje na nieistniejący element."
                    % (t, v, t, owner.get("id") if owner is not None else "?"))

    # --- typy elementów wskazywanych przez referencje
    ref_types = {"messageRef": ("message", "wiadomość (<bpmn:message>)"),
                 "signalRef": ("signal", "sygnał (<bpmn:signal>)"),
                 "errorRef": ("error", "błąd (<bpmn:error>)"),
                 "escalationRef": ("escalation", "eskalację (<bpmn:escalation>)"),
                 "processRef": ("process", "proces (<bpmn:process>)"),
                 "dataObjectRef": ("dataObject", "obiekt danych (<bpmn:dataObject>)"),
                 "dataStoreRef": ("dataStore", "magazyn danych (<bpmn:dataStore>)")}
    for e in r.iter():
        if ns_uri(e.tag) != NS["bpmn"]:
            continue
        for a, (want, opis) in ref_types.items():
            v = e.get(a)
            tgt = m.get(v)
            if tgt is not None and local(tgt.tag) != want:
                owner = e if e.get("id") and not local(e.tag).endswith("EventDefinition") else m.parent.get(e)
                add("błąd", "E018", owner.get("id") if owner is not None else None,
                    "%s: atrybut %s=\"%s\" powinien wskazywać %s, a wskazuje %s."
                    % (m.label(owner), a, v, opis, local(tgt.tag)))

    # --- przepływy sekwencji
    for f in m.seq:
        fid = f.get("id")
        if not f.get("sourceRef") or not f.get("targetRef"):
            add("błąd", "E005", fid, "Przepływ sekwencji %s musi mieć atrybuty sourceRef i targetRef." % fid)
            continue
        s, t = m.get(f.get("sourceRef")), m.get(f.get("targetRef"))
        if s is None or t is None:
            continue
        if local(s.tag) not in FLOWNODE or local(t.tag) not in FLOWNODE:
            add("błąd", "E005", fid, "Przepływ sekwencji %s łączy %s z %s. Przepływ sekwencji może łączyć "
                "tylko zadania, zdarzenia i bramki (nie obiekty danych, baseny ani adnotacje)."
                % (fid, local(s.tag), local(t.tag)))
            continue
        c = m.parent.get(f)
        if m.container(s) is not c or m.container(t) is not c:
            add("błąd", "E006", fid, "Przepływ sekwencji %s (%s → %s) przekracza granicę procesu lub "
                "podprocesu. Między basenami używa się przepływu komunikatów; wnętrze podprocesu ma własne "
                "zdarzenie początkowe." % (fid, m.label(s), m.label(t)))

    # --- przepływy komunikatów
    for f in m.msg:
        fid = f.get("id")
        s, t = m.get(f.get("sourceRef")), m.get(f.get("targetRef"))
        if s is None or t is None:
            continue
        ps, pt = m.participant_of(s), m.participant_of(t)
        if ps is None or pt is None:
            add("błąd", "E007", fid, "Przepływ komunikatów %s: jeden z końców nie należy do żadnej basenu "
                "(uczestnika). Komunikaty płyną między basenami." % fid)
        elif ps is pt:
            add("błąd", "E008", fid, "Przepływ komunikatów %s łączy elementy tego samego basenu %s "
                "(%s → %s). Wewnątrz jednego basenu, także między liniami, używa się przepływu sekwencji; "
                "komunikaty płyną tylko między basenami." % (fid, m.label(ps), m.label(s), m.label(t)))
        for x, role in ((s, "źródłem"), (t, "celem")):
            tx = local(x.tag)
            if tx in GWS or tx in DATA or tx == "textAnnotation":
                add("błąd", "E009", fid, "Przepływ komunikatów %s: %s nie może być %s (%s)."
                    % (fid, TAG_PL.get(tx, tx), role, m.label(x)))
            elif tx in EVT:
                defs = m.eventdefs(x)
                if role == "źródłem":
                    ok = tx in ("intermediateThrowEvent", "endEvent") and "messageEventDefinition" in defs
                    if not ok:
                        add("błąd", "E009", fid, "Przepływ komunikatów %s wychodzi ze zdarzenia %s, które nie "
                            "jest zdarzeniem rzucającym typu wiadomość." % (fid, m.label(x)))
                else:
                    ok = tx in ("startEvent", "intermediateCatchEvent", "boundaryEvent") and \
                        "messageEventDefinition" in defs
                    if not ok:
                        add("błąd", "E009", fid, "Przepływ komunikatów %s wchodzi do zdarzenia %s, które nie "
                            "jest zdarzeniem przechwytującym typu wiadomość." % (fid, m.label(x)))

    # --- zdarzenia
    for e in r.iter():
        t = local(e.tag)
        if t not in EVT or ns_uri(e.tag) != NS["bpmn"]:
            continue
        eid = e.get("id")
        defs = m.eventdefs(e)
        cont = m.container(e)
        kind = t
        if t == "startEvent":
            if cont is not None and m.is_event_sub(cont):
                kind = "start_eventsub"
            elif cont is not None and local(cont.tag) in SUBP:
                kind = "start_sub"
            else:
                kind = "start_top"
        allowed = ALLOWED_DEFS[kind]
        dset = [d for d in defs if d != "ref"] or ([""] if "ref" not in defs else [])
        for d in dset:
            if d not in allowed:
                where = {"start_top": "zdarzenie początkowe procesu",
                         "start_eventsub": "zdarzenie początkowe podprocesu zdarzeniowego",
                         "start_sub": "zdarzenie początkowe zwykłego podprocesu (musi być bez wyzwalacza)",
                         }.get(kind, TAG_PL.get(kind, kind))
                add("błąd", "E010", eid, "%s: wyzwalacz „%s” jest niedozwolony dla: %s."
                    % (m.label(e), DEF_PL.get(d, d), where))
        if "cancelEventDefinition" in defs:
            host = m.get(e.get("attachedToRef")) if t == "boundaryEvent" else cont
            if host is None or local(host.tag) != "transaction":
                add("błąd", "E010", eid, "%s: zdarzenie anulowania dopuszczalne jest tylko w transakcji "
                    "lub na jej granicy." % m.label(e))
        if t == "startEvent" and m.inc.get(eid):
            add("błąd", "E011", eid, "%s: zdarzenie początkowe nie może mieć przepływu wchodzącego." % m.label(e))
        if t == "endEvent" and m.out.get(eid):
            add("błąd", "E011", eid, "%s: zdarzenie końcowe nie może mieć przepływu wychodzącego." % m.label(e))
        if t == "boundaryEvent":
            host = m.get(e.get("attachedToRef"))
            if host is not None and local(host.tag) not in ACT:
                add("błąd", "E012", eid, "%s: zdarzenie graniczne musi być przyczepione do zadania lub "
                    "podprocesu, a jest do %s." % (m.label(e), m.label(host)))
            if m.inc.get(eid):
                add("błąd", "E012", eid, "%s: zdarzenie graniczne nie może mieć przepływu wchodzącego."
                    % m.label(e))
            if "errorEventDefinition" in defs and e.get("cancelActivity") == "false":
                add("błąd", "E012", eid, "%s: graniczne zdarzenie błędu zawsze przerywa czynność "
                    "(cancelActivity nie może być false)." % m.label(e))
            if "compensateEventDefinition" in defs and m.out.get(eid):
                add("błąd", "E012", eid, "%s: graniczne zdarzenie kompensacji łączy się z obsługą "
                    "asocjacją, nie przepływem sekwencji." % m.label(e))
        for td in e.findall(M + "timerEventDefinition"):
            if not any((c.text or "").strip() for c in td):
                add("ostrzeżenie", "W010", eid, "%s: zdarzenie czasowe bez określenia czasu "
                    "(timeDuration, timeDate lub timeCycle)." % m.label(e))
        for md in e.findall(M + "messageEventDefinition"):
            if not md.get("messageRef"):
                add("uwaga", "L010", eid, "%s: zdarzenie wiadomości bez messageRef (warto zadeklarować "
                    "<bpmn:message> i wskazać ją)." % m.label(e))

    # --- podprocesy zdarzeniowe i obsługa kompensacji
    for e in r.iter(M + "subProcess"):
        if not m.is_event_sub(e):
            continue
        eid = e.get("id")
        if m.inc.get(eid) or m.out.get(eid):
            add("błąd", "E013", eid, "%s: podproces zdarzeniowy nie ma przepływów wchodzących ani "
                "wychodzących — uruchamia go jego zdarzenie początkowe." % m.label(e))
        starts = e.findall(M + "startEvent")
        if len(starts) != 1:
            add("błąd", "E013", eid, "%s: podproces zdarzeniowy musi mieć dokładnie jedno zdarzenie "
                "początkowe (jest %d)." % (m.label(e), len(starts)))
        for s in starts:
            if not m.eventdefs(s):
                add("błąd", "E013", s.get("id"), "%s: zdarzenie początkowe podprocesu zdarzeniowego musi "
                    "mieć wyzwalacz (np. wiadomość, sygnał, błąd, eskalację)." % m.label(s))
    for e in r.iter():
        if local(e.tag) in ACT and e.get("isForCompensation") == "true":
            if m.inc.get(e.get("id")) or m.out.get(e.get("id")):
                add("błąd", "E014", e.get("id"), "%s: czynność kompensacyjna nie ma przepływów sekwencji."
                    % m.label(e))

    # --- bramki
    for e in r.iter():
        t = local(e.tag)
        if t not in GWS and t not in ACT:
            continue
        eid = e.get("id")
        outs = m.out.get(eid, [])
        ins = m.inc.get(eid, [])
        dflt = e.get("default")
        if dflt:
            df = m.get(dflt)
            if df is None or df.get("sourceRef") != eid:
                add("błąd", "E015", eid, "%s: przepływ domyślny %s nie wychodzi z tego elementu."
                    % (m.label(e), dflt))
            elif df.find(M + "conditionExpression") is not None:
                add("błąd", "E015", eid, "%s: przepływ domyślny %s nie może mieć warunku." % (m.label(e), dflt))
        if t == "eventBasedGateway":
            if len(outs) < 2:
                add("błąd", "E016", eid, "%s: bramka oparta na zdarzeniach potrzebuje co najmniej dwóch "
                    "wyjść." % m.label(e))
            for f in outs:
                tg = m.get(f.get("targetRef"))
                if tg is None:
                    continue
                tt = local(tg.tag)
                ok = tt == "receiveTask" or (tt == "intermediateCatchEvent" and set(m.eventdefs(tg)) & {
                    "messageEventDefinition", "timerEventDefinition", "signalEventDefinition",
                    "conditionalEventDefinition"})
                if not ok:
                    add("błąd", "E016", eid, "%s: po bramce opartej na zdarzeniach musi wystąpić pośrednie "
                        "zdarzenie przechwytujące (wiadomość, czas, sygnał, warunek) lub zadanie odbioru, "
                        "a jest %s." % (m.label(e), m.label(tg)))
                elif len(m.inc.get(tg.get("id"), [])) > 1:
                    add("ostrzeżenie", "W016", tg.get("id"), "%s: zdarzenie po bramce opartej na "
                        "zdarzeniach nie powinno mieć innych przepływów wchodzących." % m.label(tg))
                if f.find(M + "conditionExpression") is not None:
                    add("błąd", "E016", f.get("id"), "Przepływy z bramki opartej na zdarzeniach nie mają "
                        "warunków (%s)." % f.get("id"))
        if t == "parallelGateway":
            for f in outs:
                if f.find(M + "conditionExpression") is not None:
                    add("błąd", "E017", f.get("id"), "Przepływ %s z bramki równoległej %s nie może mieć "
                        "warunku." % (f.get("id"), m.label(e)))
            if dflt:
                add("błąd", "E017", eid, "%s: bramka równoległa nie ma przepływu domyślnego." % m.label(e))
        if t in GWS:
            if len(ins) == 1 and len(outs) == 1:
                add("ostrzeżenie", "W020", eid, "%s: bramka z jednym wejściem i jednym wyjściem jest zbędna."
                    % m.label(e))
            if t in ("exclusiveGateway", "inclusiveGateway", "complexGateway") and len(outs) > 1:
                for f in outs:
                    if f.get("id") == dflt or f.find(M + "conditionExpression") is not None:
                        continue
                    if not (f.get("name") or "").strip():
                        add("ostrzeżenie", "W021", f.get("id"), "Gałąź %s bramki %s nie ma ani etykiety, "
                            "ani warunku, ani statusu przepływu domyślnego." % (f.get("id"), m.label(e)))
                nm = m.name(e)
                if nm and not nm.endswith("?"):
                    add("uwaga", "L020", eid, "%s: nazwa rozgałęziającej bramki powinna być pytaniem "
                        "(kończyć się „?”)." % m.label(e))
            if t == "inclusiveGateway" and len(outs) > 1 and not dflt:
                add("ostrzeżenie", "W050", eid, "%s: rozgałęziająca bramka inkluzywna (OR) powinna mieć przepływ "
                    "domyślny (atrybut default), używany gdy żaden warunek nie jest spełniony." % m.label(e))
            if len(ins) > 1 and len(outs) > 1:
                add("uwaga", "L021", eid, "%s: bramka jednocześnie scala i rozgałęzia; czytelniej "
                    "rozdzielić to na dwie bramki." % m.label(e))
        if t in ACT and not m.name(e) and not m.is_event_sub(e):
            add("uwaga", "L030", eid, "%s bez nazwy (%s)." % (TAG_PL.get(t, t).capitalize(), eid))

    # --- linie
    for proc in m.processes:
        lanes = m.lanes_of(proc)
        if not lanes:
            continue
        direct = {c.get("id") for c in proc if local(c.tag) in FLOWNODE}
        seen = {}
        for lane, _d, _p in lanes:
            if not m.is_leaf_lane(lane):
                continue
            for ref in lane.findall(M + "flowNodeRef"):
                v = (ref.text or "").strip()
                if not v:
                    continue
                if v in seen:
                    add("błąd", "E030", v, "Element %s jest przypisany do dwóch linii (%s i %s)."
                        % (m.label(m.get(v)), seen[v], lane.get("name") or lane.get("id")))
                seen[v] = lane.get("name") or lane.get("id")
                if v in m.ids and v not in direct:
                    add("ostrzeżenie", "W030", v, "Linia %s wskazuje element %s, który nie jest bezpośrednim "
                        "elementem procesu %s (elementy wnętrza podprocesu nie należą do linii)."
                        % (lane.get("name") or lane.get("id"), v, proc.get("id")))
        for c in proc:
            ct = local(c.tag)
            if ct in FLOWNODE and ct != "boundaryEvent" and c.get("id") not in seen:
                add("ostrzeżenie", "W031", c.get("id"), "%s nie jest przypisany do żadnej linii "
                    "(dodaj <bpmn:flowNodeRef> w odpowiednim <bpmn:lane>)." % m.label(c))

    # --- osiągalność i ślepe zaułki
    def containers():
        for e in r.iter():
            if local(e.tag) in CONTAINER and ns_uri(e.tag) == NS["bpmn"]:
                yield e

    for cont in containers():
        nodes = [c for c in cont if local(c.tag) in FLOWNODE]
        if not nodes:
            continue
        ids = {n.get("id") for n in nodes}
        starts = [n.get("id") for n in nodes if local(n.tag) == "startEvent"]
        is_top = local(cont.tag) == "process"
        if not starts:
            if is_top:
                add("ostrzeżenie", "W040", cont.get("id"), "Proces %s nie ma zdarzenia początkowego."
                    % m.label(cont))
            continue
        if is_top and not any(local(n.tag) == "endEvent" for n in nodes):
            add("ostrzeżenie", "W040", cont.get("id"), "Proces %s nie ma zdarzenia końcowego." % m.label(cont))
        links = {}
        for n in nodes:
            if local(n.tag) == "intermediateCatchEvent":
                for ld in n.findall(M + "linkEventDefinition"):
                    links.setdefault(ld.get("name"), []).append(n.get("id"))
        seen, stack = set(), list(starts)
        while stack:
            u = stack.pop()
            if u in seen:
                continue
            seen.add(u)
            for f in m.out.get(u, []):
                stack.append(f.get("targetRef"))
            for b in m.boundaries.get(u, []):
                stack.append(b.get("id"))
            ue = m.get(u)
            if ue is not None and local(ue.tag) == "intermediateThrowEvent":
                for ld in ue.findall(M + "linkEventDefinition"):
                    stack.extend(links.get(ld.get("name"), []))
        for n in nodes:
            nid = n.get("id")
            t = local(n.tag)
            if m.is_event_sub(n) or n.get("isForCompensation") == "true":
                continue
            if t == "boundaryEvent" and "compensateEventDefinition" in m.eventdefs(n):
                continue
            if nid not in seen:
                add("ostrzeżenie", "W041", nid, "%s jest nieosiągalny ze zdarzenia początkowego." % m.label(n))
            if t != "endEvent" and not m.out.get(nid):
                if t == "boundaryEvent" and "compensateEventDefinition" in m.eventdefs(n):
                    continue
                if t == "intermediateThrowEvent" and "linkEventDefinition" in m.eventdefs(n):
                    continue
                add("ostrzeżenie", "W042", nid, "%s nie ma przepływu wychodzącego — ścieżka powinna kończyć "
                    "się zdarzeniem końcowym." % m.label(n))

    # --- parowanie rozwidleń i złączeń
    def trace_back(f):
        src = m.get(f.get("sourceRef"))
        guard = 0
        while src is not None and local(src.tag) not in GWS and guard < 500:
            ins_ = m.inc.get(src.get("id"), [])
            if len(ins_) != 1 or len(m.out.get(src.get("id"), [])) != 1:
                return None
            src = m.get(ins_[0].get("sourceRef"))
            guard += 1
        return src

    for j in r.iter():
        jt = local(j.tag)
        if jt not in GWS or len(m.inc.get(j.get("id"), [])) < 2:
            continue
        origins = [trace_back(f) for f in m.inc.get(j.get("id"), [])]
        if any(o is None for o in origins) or len({o.get("id") for o in origins}) != 1:
            continue
        g = origins[0]
        gt = local(g.tag)
        if len(m.out.get(g.get("id"), [])) < 2 or gt == jt:
            continue
        if gt == "eventBasedGateway" and jt == "exclusiveGateway":
            continue
        skutek = {"parallelGateway": "wszystkie gałęzie dotrą do złączenia, więc dalsza część procesu wykona się "
                                     "kilka razy",
                  "exclusiveGateway": "tylko jedna gałąź dotrze do złączenia, więc złączenie może czekać w "
                                      "nieskończoność (zakleszczenie)",
                  "inclusiveGateway": "liczba aktywnych gałęzi jest zmienna; złącz je bramką inkluzywną (OR)",
                  "eventBasedGateway": "aktywna jest tylko jedna gałąź; złącz je bramką wykluczającą (XOR)",
                  "complexGateway": "złączenie powinno mieć ten sam typ co rozwidlenie"}.get(gt, "")
        add("ostrzeżenie", "W051", j.get("id"), "%s (%s) łączy gałęzie rozwidlone przez %s (%s): %s."
            % (m.label(j), TAG_PL.get(jt, jt), m.label(g), TAG_PL.get(gt, gt), skutek))

    # --- eskalacje i błędy łapane na podprocesach
    for b in r.iter(M + "boundaryEvent"):
        host = m.get(b.get("attachedToRef"))
        if host is None or local(host.tag) not in SUBP:
            continue
        for dname, ra, rodzaj, zaimek in (("escalationEventDefinition", "escalationRef", "eskalację", "jej"),
                                          ("errorEventDefinition", "errorRef", "błąd", "go")):
            for d in b.findall(M + dname):
                ref = d.get(ra)
                throwers = [x for x in host.iter() if x is not host and local(x.tag) in
                            ("intermediateThrowEvent", "endEvent") and
                            any(ref is None or dd.get(ra) == ref for dd in x.findall(M + dname))]
                if not throwers:
                    add("ostrzeżenie", "W052", b.get("id"), "%s łapie %s%s na podprocesie %s, ale nic wewnątrz "
                        "podprocesu %s nie rzuca — zdarzenie nigdy nie wystąpi."
                        % (m.label(b), rodzaj, (" „%s”" % ref) if ref else "", m.label(host), zaimek))
    for x in r.iter():
        if local(x.tag) not in ("intermediateThrowEvent", "endEvent"):
            continue
        for d in x.findall(M + "escalationEventDefinition"):
            ref = d.get("escalationRef")
            caught = False
            anc = m.container(x)
            while anc is not None and not caught:
                for b in m.boundaries.get(anc.get("id"), []):
                    for dd in b.findall(M + "escalationEventDefinition"):
                        if dd.get("escalationRef") in (None, ref):
                            caught = True
                for es in anc.iter(M + "subProcess"):
                    if m.is_event_sub(es):
                        for st in es.findall(M + "startEvent"):
                            for dd in st.findall(M + "escalationEventDefinition"):
                                if dd.get("escalationRef") in (None, ref):
                                    caught = True
                anc = m.container(anc)
            if not caught:
                add("ostrzeżenie", "W053", x.get("id"), "%s rzuca eskalację, której nic nie łapie (brak "
                    "granicznego zdarzenia eskalacji z tym samym escalationRef na otaczającym podprocesie)."
                    % m.label(x))

    # --- elementy komunikacyjne bez przepływu komunikatów
    if len(m.participants) >= 2:
        ends = set()
        for f in m.msg:
            ends.add(f.get("sourceRef"))
            ends.add(f.get("targetRef"))
        for e in r.iter():
            t = local(e.tag)
            is_msg = t in ("sendTask", "receiveTask") or (
                t in ("intermediateThrowEvent", "intermediateCatchEvent", "boundaryEvent")
                and "messageEventDefinition" in m.eventdefs(e))
            if is_msg and e.get("id") not in ends:
                add("ostrzeżenie", "W054", e.get("id"), "%s wysyła lub odbiera wiadomość, ale nie łączy go żaden "
                    "przepływ komunikatów. Jeśli partner jest osobnym basenem, dodaj messageFlow; jeśli jest wewnątrz "
                    "tego samego basenu, rozważ inny typ elementu." % m.label(e))

    # --- numery kroków (konwencja kursowa)
    if kroki:
        for e in r.iter():
            t = local(e.tag)
            if t not in FLOWNODE or ns_uri(e.tag) != NS["bpmn"] or t == "boundaryEvent":
                continue
            cont = m.container(e)
            defs = m.eventdefs(e)
            if t == "endEvent" and not defs:
                continue
            if t == "startEvent" and not defs and cont is not None and local(cont.tag) in SUBP:
                continue
            if t in GWS and len(m.out.get(e.get("id"), [])) <= 1 and not m.name(e):
                continue
            if not STEP_RE.match(m.name(e)):
                add("ostrzeżenie", "K01", e.get("id"), "%s: nazwa powinna zaczynać się numerem kroku z opisu "
                    "procesu, np. „12. Zapytanie ofertowe”." % m.label(e))

    if kroki and opis_kroki:
        w_modelu = set()
        for e in r.iter():
            if local(e.tag) in FLOWNODE:
                w_modelu |= _step_numbers(m.name(e))
        for k in sorted(opis_kroki - w_modelu, key=_step_key):
            add("ostrzeżenie", "K02", None, "Krok %s z opisu procesu nie ma w modelu żadnego elementu "
                "(nazwa zaczynająca się od „%s.”)." % (k, k))
        for k in sorted(w_modelu - opis_kroki, key=_step_key):
            add("ostrzeżenie", "K03", None, "Model zawiera element z numerem kroku %s, którego nie ma w opisie "
                "procesu." % k)

    # --- DI
    diagrams = r.findall(BDI + "BPMNDiagram")
    if diagrams:
        have = set()
        for s in r.iter():
            if local(s.tag) in ("BPMNShape", "BPMNEdge"):
                be = s.get("bpmnElement")
                have.add(be)
                if be not in m.ids:
                    add("ostrzeżenie", "D01", s.get("id"), "Współrzędne %s dotyczą elementu %s, którego już nie ma "
                        "w modelu. Po zmianach w modelu uruchom: bpmn.py all (odświeży układ)." % (s.get("id"), be))
        missing = []
        for e in r.iter():
            t = local(e.tag)
            if (t in FLOWNODE or t in DATA or t in ("sequenceFlow", "messageFlow", "participant",
                                                      "lane", "textAnnotation")) \
                    and ns_uri(e.tag) == NS["bpmn"] and e.get("id") not in have:
                missing.append(e.get("id"))
        if missing:
            add("ostrzeżenie", "D02", None, "%d elementów nie ma współrzędnych (BPMNDI), np. %s. Uruchom: "
                "bpmn.py layout." % (len(missing), ", ".join(missing[:5])))
    return issues


# ------------------------------------------------------------- profil Camunda 8

ZEEBE = "http://camunda.org/schema/zeebe/1.0"
MODELER = "http://camunda.org/schema/modeler/1.0"
Z = "{%s}" % ZEEBE


def _zeebe(e, name):
    ext = e.find(M + "extensionElements")
    return None if ext is None else ext.find(Z + name)


def _resource_ids(folder):
    """Identyfikatory formularzy (.form) i decyzji (.dmn) w folderze modelu."""
    forms, decisions = set(), set()
    try:
        names = os.listdir(folder or ".")
    except OSError:
        return forms, decisions
    for n in names:
        path = os.path.join(folder or ".", n)
        if n.endswith(".form"):
            try:
                with open(path, encoding="utf-8") as f:
                    fid = json.load(f).get("id")
                if fid:
                    forms.add(fid)
            except (OSError, ValueError):
                pass
        elif n.endswith(".dmn"):
            try:
                for d in ET.parse(path).getroot().iter():
                    if d.tag.endswith("}decision") and d.get("id"):
                        decisions.add(d.get("id"))
            except (OSError, ET.ParseError):
                pass
    return forms, decisions


def _message_of(m, e):
    ref = e.get("messageRef")
    if ref is None:
        d = e.find(M + "messageEventDefinition")
        ref = d.get("messageRef") if d is not None else None
    return m.get(ref)


def validate_camunda(m, folder):
    """Reguły modelu wykonywalnego dla Camunda 8 (kody C…)."""
    issues = []

    def add(level, code, eid, msg):
        issues.append({"poziom": level, "kod": code, "id": eid, "komunikat": msg})

    r = m.root
    if r.get("{%s}executionPlatform" % MODELER) != "Camunda Cloud":
        add("ostrzeżenie", "C001", r.get("id"), "Brak atrybutu modeler:executionPlatform=\"Camunda Cloud\" "
            "w <bpmn:definitions>. Camunda Modeler otworzy plik jako model Camunda 7.")
    executable = [p for p in m.processes if p.get("isExecutable") == "true"]
    if not executable:
        add("błąd", "C002", None, "Żaden proces nie ma isExecutable=\"true\". Silnik nie wdroży modelu.")
    forms, decisions = _resource_ids(folder)

    for proc in executable:
        for e in proc.iter():
            t = local(e.tag)
            lab = m.label(e)
            if t == "userTask":
                if _zeebe(e, "userTask") is None:
                    add("ostrzeżenie", "C010", e.get("id"), "Zadanie użytkownika %s nie ma <zeebe:userTask/>. "
                        "Bez niego nie trafi do Tasklist jako zadanie Camunda." % lab)
                fd = _zeebe(e, "formDefinition")
                if fd is None:
                    add("uwaga", "C011", e.get("id"), "Zadanie użytkownika %s nie ma formularza "
                        "(zeebe:formDefinition). W Tasklist można je tylko zakończyć bez danych." % lab)
                elif fd.get("formId") and fd.get("formId") not in forms:
                    add("ostrzeżenie", "C012", e.get("id"), "Formularza „%s” (zadanie %s) nie ma w żadnym "
                        "pliku .form w folderze modelu." % (fd.get("formId"), lab))
            elif t == "startEvent":
                fd = _zeebe(e, "formDefinition")
                if fd is not None and fd.get("formId") and fd.get("formId") not in forms:
                    add("ostrzeżenie", "C012", e.get("id"), "Formularza startowego „%s” nie ma w żadnym "
                        "pliku .form w folderze modelu." % fd.get("formId"))
            elif t in ("serviceTask", "sendTask"):
                td = _zeebe(e, "taskDefinition")
                if td is None or not td.get("type"):
                    add("błąd", "C020", e.get("id"), "%s %s wymaga <zeebe:taskDefinition type=\"…\"/>. "
                        "Bez niego wdrożenie się nie powiedzie." % (TAG_PL[t].capitalize(), lab))
                else:
                    add("uwaga", "C021", e.get("id"), "%s %s czeka na program (worker) obsługujący typ „%s”. "
                        "Bez niego instancja zatrzyma się w tym miejscu." % (TAG_PL[t].capitalize(), lab,
                                                                          td.get("type")))
            elif t == "businessRuleTask":
                cd = _zeebe(e, "calledDecision")
                if cd is None and _zeebe(e, "taskDefinition") is None:
                    add("błąd", "C030", e.get("id"), "Zadanie reguły biznesowej %s wymaga "
                        "<zeebe:calledDecision decisionId=\"…\" resultVariable=\"…\"/>." % lab)
                elif cd is not None:
                    if not cd.get("decisionId") or not cd.get("resultVariable"):
                        add("błąd", "C031", e.get("id"), "zeebe:calledDecision w %s wymaga atrybutów "
                            "decisionId i resultVariable." % lab)
                    elif cd.get("decisionId") not in decisions:
                        add("ostrzeżenie", "C032", e.get("id"), "Decyzji „%s” (zadanie %s) nie ma w żadnym "
                            "pliku .dmn w folderze modelu." % (cd.get("decisionId"), lab))
            elif t == "scriptTask":
                sc = _zeebe(e, "script")
                if sc is None and _zeebe(e, "taskDefinition") is None:
                    add("błąd", "C040", e.get("id"), "Zadanie skryptowe %s wymaga "
                        "<zeebe:script expression=\"=…\" resultVariable=\"…\"/>." % lab)
                elif sc is not None:
                    if not (sc.get("expression") or "").strip().startswith("=") or not sc.get("resultVariable"):
                        add("błąd", "C041", e.get("id"), "zeebe:script w %s: wyrażenie FEEL musi zaczynać się "
                            "od „=”, a resultVariable jest wymagany." % lab)
            elif t == "receiveTask" or (t in ("intermediateCatchEvent", "boundaryEvent")
                                       and "messageEventDefinition" in m.eventdefs(e)):
                msg = _message_of(m, e)
                if msg is None:
                    add("błąd", "C050", e.get("id"), "%s nie wskazuje wiadomości (messageRef)." % lab)
                else:
                    sub = _zeebe(msg, "subscription")
                    if not msg.get("name"):
                        add("błąd", "C051", msg.get("id"), "Wiadomość %s musi mieć nazwę (name): po niej silnik "
                            "dopasowuje wiadomość." % m.label(msg))
                    if sub is None or not (sub.get("correlationKey") or "").strip().startswith("="):
                        add("błąd", "C052", msg.get("id"), "Wiadomość %s, na którą czeka %s, wymaga "
                            "<zeebe:subscription correlationKey=\"=…\"/>." % (m.label(msg), lab))
            elif t in ("complexGateway", "transaction"):
                add("ostrzeżenie", "C060", e.get("id"), "%s %s nie jest wykonywany przez Camunda 8."
                    % (TAG_PL[t].capitalize(), lab))

            if t in ("exclusiveGateway", "inclusiveGateway"):
                outs = m.out.get(e.get("id"), [])
                if len(outs) > 1:
                    for f in outs:
                        if f.get("id") == e.get("default"):
                            continue
                        ce = f.find(M + "conditionExpression")
                        body = (ce.text or "").strip() if ce is not None else ""
                        if not body:
                            add("błąd", "C070", f.get("id"), "Przepływ %s z bramki %s nie ma warunku. Silnik "
                                "wymaga warunku na każdym wyjściu oprócz domyślnego." % (m.label(f), lab))
                        elif not body.startswith("="):
                            add("błąd", "C071", f.get("id"), "Warunek przepływu %s musi być wyrażeniem FEEL "
                                "zaczynającym się od „=”, np. =zgoda = true." % m.label(f))
            if t == "timerEventDefinition":
                if not any((c.text or "").strip() for c in e):
                    ev = m.parent[e]
                    add("błąd", "C080", ev.get("id"), "Zdarzenie czasowe %s nie ma wartości "
                        "(timeDuration, timeDate albo timeCycle)." % m.label(ev))
    return issues


def summarize(m):
    r = m.root
    n_nodes = sum(1 for e in r.iter() if local(e.tag) in FLOWNODE)
    n_lanes = sum(1 for e in r.iter(M + "lane"))
    return ("węzły: %d, przepływy sekwencji: %d, przepływy komunikatów: %d, baseny: %d, linie: %d"
            % (n_nodes, len(m.seq), len(m.msg), len(m.participants), n_lanes))


def print_report(path, m, issues, stream=sys.stdout):
    lv = {"błąd": 0, "ostrzeżenie": 0, "uwaga": 0}
    for i in issues:
        lv[i["poziom"]] = lv.get(i["poziom"], 0) + 1
    stream.write("Plik: %s\n" % path)
    stream.write("Model: %s\n" % summarize(m))
    stream.write("Błędy: %d · Ostrzeżenia: %d · Uwagi: %d\n" % (lv["błąd"], lv["ostrzeżenie"], lv["uwaga"]))
    order = {"błąd": 0, "ostrzeżenie": 1, "uwaga": 2}
    for i in sorted(issues, key=lambda x: order.get(x["poziom"], 3)):
        tag = {"błąd": "BŁĄD", "ostrzeżenie": "OSTRZEŻENIE", "uwaga": "UWAGA"}[i["poziom"]]
        stream.write("  [%s %s] %s\n" % (tag, i["kod"], i["komunikat"]))
    if not issues:
        stream.write("  Brak zastrzeżeń.\n")


# ------------------------------------------------------------------------ layout

COL_GAP = 50
ROW_GAP = 50
PAD_L = 30
PAD_R = 30
PAD_T = 30
PAD_B = 30
SUB_PAD_T = 40
MIN_LANE_H = 130
POOL_GAP = 40
BB_H = 70  # wysokość basenu zamkniętego (black box)


class Box:
    def __init__(self):
        self.pos = {}
        self.children = {}
        self.w = 0
        self.h = 0
        self.lanes = []
        self.back = set()


def _has_flow_children(e):
    return any(local(c.tag) in FLOWNODE for c in e)


def _even10(v):
    return int((v + 9) // 10 * 10)


def node_size(e):
    t = local(e.tag)
    if t in ACT:
        return (100, 80)
    if t in EVT:
        return (36, 36)
    if t in GWS:
        return (50, 50)
    if t == "dataObjectReference":
        return (36, 50)
    if t == "dataStoreReference":
        return (50, 50)
    if t == "textAnnotation":
        txt = (e.findtext(M + "text") or "").strip()
        lines = max(1, len(_wrap(txt, 120, 11)))
        return (130, max(30, _even10(lines * 14 + 10)))
    return (100, 80)


def layout_container(m, cont, lane_list=None, top_pad=PAD_T):
    box = Box()
    kids = list(cont)
    nodes = [c for c in kids if local(c.tag) in FLOWNODE and local(c.tag) != "boundaryEvent"
             and not m.is_event_sub(c)]
    evsubs = [c for c in kids if m.is_event_sub(c)]
    extras = [c for c in kids if local(c.tag) in DATA or local(c.tag) == "textAnnotation"]
    sizes = {}
    for n in nodes + evsubs:
        nid = n.get("id")
        if local(n.tag) in SUBP and _has_flow_children(n):
            cb = layout_container(m, n, None, top_pad=SUB_PAD_T)
            box.children[nid] = cb
            box.back |= cb.back
            sizes[nid] = (cb.w, cb.h)
        else:
            sizes[nid] = node_size(n)
    for x in extras:
        sizes[x.get("id")] = node_size(x)

    ids = [n.get("id") for n in nodes]
    idset = set(ids)
    host_of = {}
    for n in nodes:
        for b in m.boundaries.get(n.get("id"), []):
            host_of[b.get("id")] = n.get("id")
    succ = {i: [] for i in ids}
    pred = {i: [] for i in ids}
    flows_of = {}
    for f in m.seq:
        if m.parent.get(f) is not cont:
            continue
        s, t = f.get("sourceRef"), f.get("targetRef")
        via_b = s in host_of
        s2 = host_of.get(s, s)
        if s2 in idset and t in idset:
            succ[s2].append((t, via_b))
            pred[t].append((s2, via_b))
            flows_of.setdefault((s2, t), []).append(f.get("id"))

    # odwrócone krawędzie (pętle) - DFS
    starts = [i for i in ids if local(m.get(i).tag) == "startEvent"]
    roots = starts + [i for i in ids if not pred[i] and i not in starts] + ids
    color = {i: 0 for i in ids}
    disc = {}
    back = set()
    cnt = 0
    for rt in roots:
        if color[rt]:
            continue
        color[rt] = 1
        disc[rt] = cnt
        cnt += 1
        stack = [(rt, iter(succ[rt]))]
        while stack:
            u, it = stack[-1]
            nxt = next(it, None)
            if nxt is None:
                color[u] = 2
                stack.pop()
                continue
            v = nxt[0]
            if color[v] == 1:
                back.add((u, v))
            elif color[v] == 0:
                color[v] = 1
                disc[v] = cnt
                cnt += 1
                stack.append((v, iter(succ[v])))
    for (u, v) in back:
        box.back.update(flows_of.get((u, v), []))

    # warstwy
    layer = {i: 0 for i in ids}
    indeg = {i: 0 for i in ids}
    fwd = {i: [] for i in ids}
    for u in ids:
        for v, _vb in succ[u]:
            if (u, v) in back:
                continue
            fwd[u].append(v)
            indeg[v] += 1
    queue = sorted([i for i in ids if indeg[i] == 0], key=lambda i: disc.get(i, 0))
    topo = []
    while queue:
        u = queue.pop(0)
        topo.append(u)
        for v in fwd[u]:
            layer[v] = max(layer[v], layer[u] + 1)
            indeg[v] -= 1
            if indeg[v] == 0:
                queue.append(v)
    for i in ids:
        if i not in topo:
            topo.append(i)

    # linie
    nl = len(lane_list) if lane_list else 1
    lane_idx = {}
    if lane_list:
        for k, ln in enumerate(lane_list):
            for ref in ln.findall(M + "flowNodeRef"):
                v = (ref.text or "").strip()
                lane_idx.setdefault(v, k)
    for i in topo:
        if i not in lane_idx:
            ps = [p for p, _ in pred[i] if p in lane_idx]
            lane_idx[i] = lane_idx[ps[0]] if ps else 0

    # wiersze
    occ = set()
    row = {}

    def place(i, lay, pref):
        rr = max(0, pref)
        while (lane_idx[i], lay, rr) in occ:
            rr += 1
        occ.add((lane_idx[i], lay, rr))
        row[i] = rr

    for i in sorted(ids, key=lambda x: (layer[x], disc.get(x, 10 ** 6))):
        pref = 0
        for p, vb in pred[i]:
            if (p, i) in back:
                continue
            if lane_idx.get(p) == lane_idx[i] and p in row:
                pref = row[p] + (1 if vb else 0)
                break
        place(i, layer[i], pref)

    # obiekty danych i adnotacje
    assoc_target = {}

    def _link(xid, nid):
        lst = assoc_target.setdefault(xid, [])
        if nid not in lst:
            lst.append(nid)

    evsub_ids = {ev.get("id") for ev in evsubs}

    def _lift(e):
        """Węzeł tego kontenera, w którym leży e (np. podproces zawierający zadanie)."""
        while e is not None and e.get("id") not in idset and e.get("id") not in evsub_ids:
            if e.get("id") in host_of:
                return host_of[e.get("id")]
            p_ = m.parent.get(e)
            if p_ is None or p_ is cont:
                return e.get("id")
            e = p_
        return e.get("id") if e is not None else None

    for n in cont.iter():
        if local(n.tag) not in FLOWNODE or n is cont:
            continue
        nid = _lift(n)
        for da in n.findall(M + "dataInputAssociation"):
            for s in da.findall(M + "sourceRef"):
                _link((s.text or "").strip(), nid)
        for da in n.findall(M + "dataOutputAssociation"):
            t = da.find(M + "targetRef")
            if t is not None:
                _link((t.text or "").strip(), nid)
    for a in m.assoc:
        s, t = a.get("sourceRef"), a.get("targetRef")
        if t in idset:
            _link(s, t)
        if s in idset:
            _link(t, s)
    for x in extras:
        xid = x.get("id")
        hosts = [h for h in assoc_target.get(xid, []) if h in idset]
        if not hosts:
            evh = [h for h in assoc_target.get(xid, []) if h in evsub_ids]
            if evh and lane_list:
                for kk, ln in enumerate(lane_list):
                    if any((r_.text or "").strip() == evh[0] for r_ in ln.findall(M + "flowNodeRef")):
                        lane_idx[xid] = kk
                layer[xid] = 0
                lane_idx.setdefault(xid, nl - 1)
                place(xid, 0, 0)
                continue
        host = None
        for h in hosts:
            if (lane_idx[h], layer[h], row[h] + 1) not in occ:
                host = h
                break
        if host is None and hosts:
            host = hosts[0]
        if host in idset:
            layer[xid] = layer[host]
            lane_idx[xid] = lane_idx[host]
            pref = row[host] + 1
        else:
            layer[xid] = 0
            lane_idx[xid] = 0
            pref = 0
        place(xid, layer[xid], pref)

    # podprocesy zdarzeniowe - osobny wiersz na dole linii
    ev_rows = {}
    for ev in evsubs:
        eid = ev.get("id")
        k = nl - 1
        if lane_list:
            for kk, ln in enumerate(lane_list):
                if any((r.text or "").strip() == eid for r in ln.findall(M + "flowNodeRef")):
                    k = kk
        lane_idx[eid] = k
        used = [rr for (ll, _lay, rr) in occ if ll == k] + [rr for (e2, rr) in ev_rows.items()
                                                            if lane_idx[e2] == k]
        ev_rows[eid] = (max(used) + 1) if used else 0

    # kolumny
    all_ids = ids + [x.get("id") for x in extras]
    nlay = (max(layer[i] for i in all_ids) + 1) if all_ids else 1
    colw = [36] * nlay
    for i in all_ids:
        colw[layer[i]] = max(colw[layer[i]], sizes[i][0])
    colx = []
    x = PAD_L
    for lay in range(nlay):
        colx.append(x)
        x += colw[lay] + COL_GAP
    content_w = x - COL_GAP + PAD_R
    for ev in evsubs:
        content_w = max(content_w, PAD_L + sizes[ev.get("id")][0] + PAD_R)
    content_w = _even10(content_w)

    # wysokości wierszy i linii
    rowh = {}
    for i in all_ids:
        key = (lane_idx[i], row[i])
        rowh[key] = max(rowh.get(key, 0), sizes[i][1])
    for eid, rr in ev_rows.items():
        key = (lane_idx[eid], rr)
        rowh[key] = max(rowh.get(key, 0), sizes[eid][1])
    lane_geo = []
    y = 0
    for k in range(nl):
        rows = sorted(rr for (ll, rr) in rowh if ll == k)
        top = y + top_pad
        rtop = {}
        yy = top
        for rr in rows:
            rtop[rr] = yy
            yy += rowh[(k, rr)] + ROW_GAP
        bottom = (yy - ROW_GAP) if rows else top
        lane_h = _even10(max(bottom - y + PAD_B, MIN_LANE_H if lane_list else 0))
        lane_geo.append((y, lane_h, rtop))
        y += lane_h
    box.h = y
    box.w = content_w
    for i in all_ids:
        k = lane_idx[i]
        rr = row[i]
        _ly, _lh, rtop = lane_geo[k]
        w, h = sizes[i]
        cx = colx[layer[i]] + colw[layer[i]] // 2
        cy = rtop[rr] + rowh[(k, rr)] // 2
        box.pos[i] = [int(cx - w // 2), int(cy - h // 2), w, h]
    for eid, rr in ev_rows.items():
        k = lane_idx[eid]
        _ly, _lh, rtop = lane_geo[k]
        w, h = sizes[eid]
        box.pos[eid] = [PAD_L, rtop[rr], w, h]
    if lane_list:
        box.lanes = [(lane_list[k].get("id"), lane_geo[k][0], lane_geo[k][1]) for k in range(nl)]
    else:
        box.lanes = [(None, 0, box.h)]
    # podprocesy - rozmiar zaokrąglony
    return box


def _place_box(box, X, Y, out):
    for i, (x, y, w, h) in box.pos.items():
        out[i] = [X + x, Y + y, w, h]
        if i in box.children:
            _place_box(box.children[i], X + x, Y + y, out)


def _clip(b, px, py):
    x, y, w, h = b
    cx, cy = x + w / 2.0, y + h / 2.0
    dx, dy = px - cx, py - cy
    if dx == 0 and dy == 0:
        return (cx, cy)
    sx = (w / 2.0) / abs(dx) if dx else 1e9
    sy = (h / 2.0) / abs(dy) if dy else 1e9
    s = min(sx, sy)
    return (cx + dx * s, cy + dy * s)


def _seg_hits(p1, p2, box):
    x, y, w, h = box
    x0, y0, x1, y1 = x + 3, y + 3, x + w - 3, y + h - 3
    (ax, ay), (bx, by) = p1, p2
    if abs(ax - bx) < 0.5:
        return x0 < ax < x1 and max(min(ay, by), y0) < min(max(ay, by), y1)
    if abs(ay - by) < 0.5:
        return y0 < ay < y1 and max(min(ax, bx), x0) < min(max(ax, bx), x1)
    return False


def _score(pts, obstacles):
    hits = 0
    for k in range(len(pts) - 1):
        for ob in obstacles:
            if _seg_hits(pts[k], pts[k + 1], ob):
                hits += 1
    return hits * 100 + len(pts)


def _obstacles_for(m, out, skip_ids, container):
    """Kształty, przez które krawędź nie powinna przechodzić."""
    anc = set()
    c = container
    while c is not None:
        anc.add(c.get("id"))
        c = m.container(c)
    obs = []
    for i, bx in out.items():
        if i in skip_ids or i in anc:
            continue
        e = m.get(i)
        if e is None:
            continue
        t = local(e.tag)
        if t in FLOWNODE or t in DATA or t == "textAnnotation":
            if t == "boundaryEvent" and e.get("attachedToRef") in skip_ids:
                continue
            obs.append(bx)
    return obs


def _route_seq(m, f, out, back_ids, counter):
    s_id, t_id = f.get("sourceRef"), f.get("targetRef")
    s, t = out[s_id], out[t_id]
    se, te = m.get(s_id), m.get(t_id)
    sx, sy, sw, sh = s
    tx, ty, tw, th = t
    scx, scy = sx + sw / 2.0, sy + sh / 2.0
    tcx, tcy = tx + tw / 2.0, ty + th / 2.0
    sb, tb = sy + sh, ty + th
    s_split = local(se.tag) in GWS and len(m.out.get(s_id, [])) > 1
    t_join = local(te.tag) in GWS and len(m.inc.get(t_id, [])) > 1
    skip = {s_id, t_id}
    if local(se.tag) == "boundaryEvent":
        skip.add(se.get("attachedToRef"))
    obs = _obstacles_for(m, out, skip, m.parent.get(f))
    cands = []

    if local(se.tag) == "boundaryEvent":
        if tx >= sx + sw:
            if tcy >= sb + 5:
                cands.append([(scx, sb), (scx, tcy), (tx, tcy)])
            yb = sb + 20
            cands.append([(scx, sb), (scx, yb), (tx - 20, yb), (tx - 20, tcy), (tx, tcy)])
        if ty > sb + 30:
            cands.append([(scx, sb), (scx, ty - 20), (tcx, ty - 20), (tcx, ty)])
        counter[0] += 1
        yb = max(sb, tb) + 25 + 8 * (counter[0] % 4)
        cands.append([(scx, sb), (scx, yb), (tcx, yb), (tcx, tb)])
        return min(cands, key=lambda c: _score(c, obs))

    backward = f.get("id") in back_ids or tx < sx + sw - 1
    if not backward:
        if abs(scy - tcy) < 1.5:
            cands.append([(sx + sw, scy), (tx, tcy)])
        if s_split:
            ey = sb if tcy > scy else sy
            cands.append([(scx, ey), (scx, tcy), (tx, tcy)])
        if t_join:
            ey = ty if scy < tcy else tb
            cands.append([(sx + sw, scy), (tcx, scy), (tcx, ey)])
        midx = tx - COL_GAP // 2
        cands.append([(sx + sw, scy), (midx, scy), (midx, tcy), (tx, tcy)])
        midx2 = sx + sw + COL_GAP // 2
        if midx2 < tx - 5:
            cands.append([(sx + sw, scy), (midx2, scy), (midx2, tcy), (tx, tcy)])
        if s_split:
            ey = sb if tcy > scy else sy
            jy = ey + (15 if tcy > scy else -15)
            cands.append([(scx, ey), (scx, jy), (midx, jy), (midx, tcy), (tx, tcy)])
        return min(cands, key=lambda c: _score(c, obs))
    # przepływ wsteczny (pętla)
    if ty - 20 > sb + 5:
        cands.append([(scx, sb), (scx, ty - 20), (tcx, ty - 20), (tcx, ty)])
    if tb + 20 < sy - 5:
        cands.append([(scx, sy), (scx, tb + 20), (tcx, tb + 20), (tcx, tb)])
    counter[0] += 1
    k = 8 * (counter[0] % 4)
    lo = [b for b in obs if b[0] < max(scx, tcx) and b[0] + b[2] > min(scx, tcx)]
    yb = max(sb, tb) + 25 + k
    cands.append([(scx, sb), (scx, yb), (tcx, yb), (tcx, tb)])
    ya = min(sy, ty) - 25 - k
    cands.append([(scx, sy), (scx, ya), (tcx, ya), (tcx, ty)])
    if lo:
        yb2 = max([sb, tb] + [b[1] + b[3] for b in lo if b[1] >= min(sy, ty) - 5]) + 25 + k
        cands.append([(scx, sb), (scx, yb2), (tcx, yb2), (tcx, tb)])
    return min(cands, key=lambda c: _score(c, obs))


def _event_side_point(box, side, dy=8):
    x, y, w, h = box
    cx, cy = x + w / 2.0, y + h / 2.0
    if w == h and w <= 40:
        r = w / 2.0
        dx = (r * r - dy * dy) ** 0.5
        return (cx + dx, cy + dy) if side == "right" else (cx - dx, cy + dy)
    return (x + w, cy + dy) if side == "right" else (x, cy + dy)


def _route_msg_node_pool(m, node_id, pool_box, out, from_node, k):
    """Przepływ komunikatów między węzłem a basenem zamkniętym (black box)."""
    n = out[node_id]
    nx, ny, nw, nh = n
    ncx = nx + nw / 2.0
    px, py, pw, ph = pool_box
    below = py >= ny + nh
    edge_y = py if below else py + ph
    obs = _obstacles_for(m, out, {node_id}, None)
    cands = []
    if below:
        cands.append([(ncx, ny + nh), (ncx, edge_y)])
    else:
        cands.append([(ncx, ny), (ncx, edge_y)])
    for side in ("right", "left"):
        p0 = _event_side_point(n, side)
        chx = (nx + nw + 14 + 6 * k) if side == "right" else (nx - 14 - 6 * k)
        cands.append([p0, (chx, p0[1]), (chx, edge_y)])
    best = min(cands, key=lambda c: _score(c, obs))
    return best if from_node else list(reversed(best))


def _spread_ports(m, wp, out):
    """Rozsuwa końce kilku krawędzi trafiających w ten sam punkt boku zadania."""
    sides = {}
    for fid, pts in wp.items():
        e = m.get(fid)
        if e is None or local(e.tag) not in ("sequenceFlow", "messageFlow") or len(pts) < 2:
            continue
        for idx, nid in ((0, e.get("sourceRef")), (len(pts) - 1, e.get("targetRef"))):
            ne = m.get(nid)
            if nid not in out or ne is None or local(ne.tag) not in ACT:
                continue
            x, y, w, h = out[nid]
            px, py = pts[idx]
            side = None
            if abs(px - (x + w / 2.0)) < 0.6 and abs(py - y) < 0.6:
                side = "top"
            elif abs(px - (x + w / 2.0)) < 0.6 and abs(py - (y + h)) < 0.6:
                side = "bottom"
            elif abs(px - x) < 0.6 and abs(py - (y + h / 2.0)) < 0.6:
                side = "left"
            elif abs(px - (x + w)) < 0.6 and abs(py - (y + h / 2.0)) < 0.6:
                side = "right"
            if side:
                sides.setdefault((nid, side), []).append((fid, idx))
    for (nid, side), lst in sides.items():
        if len(lst) < 2:
            continue
        x, y, w, h = out[nid]
        horiz = side in ("top", "bottom")

        def far(item):
            fid, idx = item
            pts = wp[fid]
            q = pts[-1] if idx == 0 else pts[0]
            return q[0] if horiz else q[1]

        lst.sort(key=far)
        n = len(lst)
        step = min(16.0, ((w if horiz else h) - 20) / max(n - 1, 1))
        for k, (fid, idx) in enumerate(lst):
            d = (k - (n - 1) / 2.0) * step
            pts = wp[fid]
            nb = 1 if idx == 0 else len(pts) - 2
            if horiz:
                old = pts[idx][0]
                pts[idx] = (old + d, pts[idx][1])
                if len(pts) == 2 or abs(pts[nb][0] - old) < 0.6:
                    pts[nb] = (pts[nb][0] + d, pts[nb][1])
            else:
                old = pts[idx][1]
                pts[idx] = (pts[idx][0], old + d)
                if len(pts) == 2 or abs(pts[nb][1] - old) < 0.6:
                    pts[nb] = (pts[nb][0], pts[nb][1] + d)


def layout(m):
    """Generuje kompletne BPMNDI; zwraca liczbę kształtów."""
    out = {}
    back = set()
    lanes_out = {}
    pools = {}
    if m.collabs:
        collab = m.collabs[0]
        plane_ref = collab.get("id")
        entries = []
        for p in collab.findall(M + "participant"):
            proc = m.get(p.get("processRef"))
            if proc is not None and local(proc.tag) == "process" and _has_flow_children(proc):
                lanes_all = m.lanes_of(proc)
                leaves = [ln for ln, _d, _p in lanes_all if m.is_leaf_lane(ln)]
                maxd = (max(d for _l, d, _p in lanes_all) + 1) if lanes_all else 0
                box = layout_container(m, proc, leaves or None)
                back |= box.back
                entries.append((p, proc, box, lanes_all, maxd))
            else:
                entries.append((p, None, None, [], 0))
        W = 600
        for (_p, _proc, box, _la, maxd) in entries:
            if box is not None:
                W = max(W, 30 + maxd * 30 + box.w)
        y = 0
        for (p, proc, box, lanes_all, maxd) in entries:
            pid = p.get("id")
            if box is None:
                out[pid] = [0, y, W, BB_H]
                pools[pid] = out[pid]
                y += BB_H + POOL_GAP
                continue
            out[pid] = [0, y, W, box.h]
            pools[pid] = out[pid]
            X0 = 30 + maxd * 30
            _place_box(box, X0, y, out)
            band = {lid: (ly, lh) for lid, ly, lh in box.lanes if lid}
            if lanes_all:
                def lane_range(lane):
                    if m.is_leaf_lane(lane):
                        return band.get(lane.get("id"), (0, box.h))
                    rs = [lane_range(c) for c in lane.find(M + "childLaneSet").findall(M + "lane")]
                    y0 = min(a for a, _b in rs)
                    y1 = max(a + b for a, b in rs)
                    return (y0, y1 - y0)
                for lane, depth, _par in lanes_all:
                    ly, lh = lane_range(lane)
                    lanes_out[lane.get("id")] = [30 + depth * 30, y + ly, W - 30 - depth * 30, lh]
            y += box.h + POOL_GAP
    else:
        plane_ref = m.processes[0].get("id") if m.processes else None
        y = 0
        for proc in m.processes:
            box = layout_container(m, proc, None)
            back |= box.back
            _place_box(box, 0, y, out)
            y += box.h + POOL_GAP

    # zdarzenia graniczne
    for host, bl in m.boundaries.items():
        if host not in out:
            continue
        hx, hy, hw, hh = out[host]
        for k, b in enumerate(bl):
            cx = hx + hw - 30 - 44 * k
            cx = max(cx, hx + 18)
            out[b.get("id")] = [int(cx - 18), int(hy + hh - 18), 36, 36]

    # krawędzie
    wp = {}
    counter = [0]
    for f in m.seq:
        if f.get("sourceRef") in out and f.get("targetRef") in out:
            wp[f.get("id")] = [tuple(p) for p in _route_seq(m, f, out, back, counter)]
    allbox = dict(out)
    allbox.update(lanes_out)
    mcount = {}
    for f in m.msg:
        s_id, t_id = f.get("sourceRef"), f.get("targetRef")
        if s_id not in allbox or t_id not in allbox:
            continue
        se, te = m.get(s_id), m.get(t_id)
        s, t = allbox[s_id], allbox[t_id]
        ps, pt = m.participant_of(se), m.participant_of(te)
        s_is_pool = local(se.tag) == "participant"
        t_is_pool = local(te.tag) == "participant"
        key = (ps.get("id") if ps is not None else s_id, pt.get("id") if pt is not None else t_id)
        k = mcount.get(key, 0)
        mcount[key] = k + 1
        if t_is_pool and not s_is_pool and s_id in out:
            wp[f.get("id")] = _route_msg_node_pool(m, s_id, t, out, True, k)
            continue
        if s_is_pool and not t_is_pool and t_id in out:
            wp[f.get("id")] = _route_msg_node_pool(m, t_id, s, out, False, k)
            continue
        s_pool = pools.get(ps.get("id")) if ps is not None else s
        sxc, txc = s[0] + s[2] / 2.0, t[0] + t[2] / 2.0
        if s_is_pool:
            sxc = min(max(txc, s[0] + 40), s[0] + s[2] - 10)
        if t_is_pool:
            txc = min(max(sxc, t[0] + 40), t[0] + t[2] - 10)
        if s[1] + s[3] / 2.0 <= t[1] + t[3] / 2.0:
            y1, y2 = s[1] + s[3], t[1]
            gy = (s_pool[1] + s_pool[3] + POOL_GAP / 2.0 - 6 + 6 * k) if s_pool else (y1 + y2) / 2
        else:
            y1, y2 = s[1], t[1] + t[3]
            gy = (s_pool[1] - POOL_GAP / 2.0 + 6 - 6 * k) if s_pool else (y1 + y2) / 2
        if abs(sxc - txc) < 1:
            wp[f.get("id")] = [(sxc, y1), (txc, y2)]
        else:
            wp[f.get("id")] = [(sxc, y1), (sxc, gy), (txc, gy), (txc, y2)]
    _spread_ports(m, wp, out)
    for a in m.assoc:
        s_id, t_id = a.get("sourceRef"), a.get("targetRef")
        if s_id in out and t_id in out:
            s, t = out[s_id], out[t_id]
            p1 = _clip(s, t[0] + t[2] / 2.0, t[1] + t[3] / 2.0)
            p2 = _clip(t, s[0] + s[2] / 2.0, s[1] + s[3] / 2.0)
            wp[a.get("id")] = [p1, p2]
    n_da = 0
    for e in list(m.root.iter()):
        t = local(e.tag)
        if t not in ("dataInputAssociation", "dataOutputAssociation"):
            continue
        owner = m.parent.get(e)
        if owner is None or owner.get("id") not in out:
            continue
        if t == "dataInputAssociation":
            srcs = [(s.text or "").strip() for s in e.findall(M + "sourceRef")]
            pair = [(sid, owner.get("id")) for sid in srcs if sid in out]
        else:
            tg = e.find(M + "targetRef")
            tid = (tg.text or "").strip() if tg is not None else ""
            pair = [(owner.get("id"), tid)] if tid in out else []
        if not pair:
            continue
        if not e.get("id"):
            n_da += 1
            nid = "DataAssociation_%d" % n_da
            while nid in m.ids:
                n_da += 1
                nid = "DataAssociation_%d" % n_da
            e.set("id", nid)
            m.ids[nid] = e
        a, b = pair[0]
        s, tb = out[a], out[b]
        p1 = _clip(s, tb[0] + tb[2] / 2.0, tb[1] + tb[3] / 2.0)
        p2 = _clip(tb, s[0] + s[2] / 2.0, s[1] + s[3] / 2.0)
        wp[e.get("id")] = [p1, p2]

    # zapis DI
    for d in m.root.findall(BDI + "BPMNDiagram"):
        m.root.remove(d)
    diag = ET.SubElement(m.root, BDI + "BPMNDiagram", {"id": "BPMNDiagram_1"})
    plane = ET.SubElement(diag, BDI + "BPMNPlane", {"id": "BPMNPlane_1", "bpmnElement": plane_ref or ""})
    shapes = {}
    shapes.update(out)
    shapes.update(lanes_out)
    order = []
    for p in m.participants:
        if p.get("id") in shapes:
            order.append(p.get("id"))
    for proc in m.processes:
        for ln, _d, _p in m.lanes_of(proc):
            if ln.get("id") in shapes:
                order.append(ln.get("id"))
    for e in m.root.iter():
        i = e.get("id")
        if i in shapes and i not in order and local(e.tag) != "boundaryEvent":
            order.append(i)
    for e in m.root.iter(M + "boundaryEvent"):
        if e.get("id") in shapes:
            order.append(e.get("id"))
    for i in order:
        e = m.get(i)
        t = local(e.tag)
        attrs = {"id": i + "_di", "bpmnElement": i}
        if t in ("participant", "lane"):
            attrs["isHorizontal"] = "true"
        if t in SUBP:
            attrs["isExpanded"] = "true" if _has_flow_children(e) else "false"
        if t == "exclusiveGateway":
            attrs["isMarkerVisible"] = "true"
        sh = ET.SubElement(plane, BDI + "BPMNShape", attrs)
        x, y, w, h = shapes[i]
        ET.SubElement(sh, DC + "Bounds", {"x": str(int(round(x))), "y": str(int(round(y))),
                                          "width": str(int(round(w))), "height": str(int(round(h)))})
    for e in m.root.iter():
        i = e.get("id")
        if i in wp:
            ed = ET.SubElement(plane, BDI + "BPMNEdge", {"id": i + "_di", "bpmnElement": i})
            for (px, py) in wp[i]:
                ET.SubElement(ed, DI + "waypoint", {"x": str(int(round(px))), "y": str(int(round(py)))})
    m.reindex()
    return len(order)


# --------------------------------------------------------------------------- SVG

def _esc(s):
    return (s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;"))


def _wrap(text, maxw, fs):
    cw = fs * 0.56
    maxc = max(4, int(maxw / cw))
    lines, cur = [], ""
    for w in text.split():
        while len(w) > maxc:
            if cur:
                lines.append(cur)
                cur = ""
            lines.append(w[:maxc - 1] + "-")
            w = w[maxc - 1:]
        if not cur:
            cur = w
        elif len(cur) + 1 + len(w) <= maxc:
            cur += " " + w
        else:
            lines.append(cur)
            cur = w
    if cur:
        lines.append(cur)
    return lines


def _text(lines, x, y, fs, anchor="middle", extra=""):
    if not lines:
        return ""
    lh = fs * 1.2
    out = ['<text x="%.1f" y="%.1f" font-size="%d" text-anchor="%s" %s>' % (x, y, fs, anchor, extra)]
    for k, ln in enumerate(lines):
        out.append('<tspan x="%.1f" dy="%.1f">%s</tspan>' % (x, 0 if k == 0 else lh, _esc(ln)))
    out.append("</text>")
    return "".join(out)


STROKE = "#1f2328"


def _event_icon(defs, cx, cy, filled):
    fill = STROKE if filled else "#fff"
    ln = "#fff" if filled else STROKE
    s = []
    for d in defs[:1] if len(defs) == 1 else []:
        if d == "messageEventDefinition":
            s.append('<rect x="%.1f" y="%.1f" width="16" height="11" fill="%s" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 8, cy - 5.5, fill, STROKE))
            s.append('<polyline points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="none" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 8, cy - 5.5, cx, cy + 1, cx + 8, cy - 5.5, ln))
        elif d == "timerEventDefinition":
            s.append('<circle cx="%.1f" cy="%.1f" r="11" fill="#fff" stroke="%s" stroke-width="1.5"/>' % (cx, cy, STROKE))
            for k in range(12):
                import math
                a = k * math.pi / 6
                r1 = 8.5 if k % 3 else 7.5
                s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1"/>'
                         % (cx + r1 * math.cos(a), cy + r1 * math.sin(a), cx + 10.5 * math.cos(a),
                            cy + 10.5 * math.sin(a), STROKE))
            s.append('<polyline points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="none" stroke="%s" stroke-width="1.5"/>'
                     % (cx, cy - 7, cx, cy, cx + 5, cy + 2, STROKE))
        elif d == "signalEventDefinition":
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="%s" stroke="%s" stroke-width="1.3"/>'
                     % (cx, cy - 9, cx + 8.5, cy + 6, cx - 8.5, cy + 6, fill, STROKE))
        elif d == "errorEventDefinition":
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="%s" '
                     'stroke="%s" stroke-width="1.3" stroke-linejoin="round"/>'
                     % (cx - 8, cy + 8, cx - 3, cy - 6, cx + 2, cy + 2, cx + 8, cy - 8, cx + 3, cy + 6,
                        cx - 2, cy - 2, fill, STROKE))
        elif d == "escalationEventDefinition":
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="%s" stroke="%s" '
                     'stroke-width="1.3" stroke-linejoin="round"/>'
                     % (cx, cy - 9, cx + 6.5, cy + 8, cx, cy + 2, cx - 6.5, cy + 8, fill, STROKE))
        elif d == "conditionalEventDefinition":
            s.append('<rect x="%.1f" y="%.1f" width="13" height="16" fill="#fff" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 6.5, cy - 8, STROKE))
            for k in range(4):
                s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1"/>'
                         % (cx - 4, cy - 5 + 3.5 * k, cx + 4, cy - 5 + 3.5 * k, STROKE))
        elif d == "linkEventDefinition":
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f" '
                     'fill="%s" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 8, cy - 3, cx + 1, cy - 3, cx + 1, cy - 7, cx + 8, cy, cx + 1, cy + 7, cx + 1,
                        cy + 3, cx - 8, cy + 3, fill, STROKE))
        elif d == "terminateEventDefinition":
            s.append('<circle cx="%.1f" cy="%.1f" r="10" fill="%s"/>' % (cx, cy, STROKE))
        elif d == "compensateEventDefinition":
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="%s" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 9, cy, cx - 1, cy - 6, cx - 1, cy + 6, fill, STROKE))
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="%s" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 1, cy, cx + 7, cy - 6, cx + 7, cy + 6, fill, STROKE))
        elif d == "cancelEventDefinition":
            s.append('<path d="M%.1f,%.1f L%.1f,%.1f M%.1f,%.1f L%.1f,%.1f" stroke="%s" stroke-width="4"/>'
                     % (cx - 7, cy - 7, cx + 7, cy + 7, cx + 7, cy - 7, cx - 7, cy + 7, STROKE))
    if len(defs) > 1:
        import math
        pts = " ".join("%.1f,%.1f" % (cx + 9 * math.cos(-math.pi / 2 + k * 2 * math.pi / 5),
                                      cy + 9 * math.sin(-math.pi / 2 + k * 2 * math.pi / 5)) for k in range(5))
        s.append('<polygon points="%s" fill="%s" stroke="%s" stroke-width="1.2"/>' % (pts, fill, STROKE))
    return "".join(s)


def _task_icon(t, x, y):
    s = []
    if t == "userTask":
        s.append('<circle cx="%.1f" cy="%.1f" r="4" fill="#fff" stroke="%s" stroke-width="1.2"/>' % (x + 14, y + 12, STROKE))
        s.append('<path d="M%.1f,%.1f Q%.1f,%.1f %.1f,%.1f Q%.1f,%.1f %.1f,%.1f Z" fill="#fff" stroke="%s" '
                 'stroke-width="1.2"/>' % (x + 6, y + 26, x + 6, y + 17, x + 14, y + 17, x + 22, y + 17,
                                           x + 22, y + 26, STROKE))
    elif t == "serviceTask":
        s.append('<circle cx="%.1f" cy="%.1f" r="6.5" fill="none" stroke="%s" stroke-width="3.4" '
                 'stroke-dasharray="2.6 2.5"/>' % (x + 14, y + 15, STROKE))
        s.append('<circle cx="%.1f" cy="%.1f" r="4.6" fill="#fff" stroke="%s" stroke-width="1.2"/>' % (x + 14, y + 15, STROKE))
        s.append('<circle cx="%.1f" cy="%.1f" r="1.8" fill="none" stroke="%s" stroke-width="1"/>' % (x + 14, y + 15, STROKE))
    elif t == "manualTask":
        s.append('<path d="M%.1f,%.1f h12 a3,3 0 0 1 0,6 h-1 M%.1f,%.1f h-12 v-8 l5,-5 h2 v5 h8 a2,2 0 0 1 0,4 h-3 '
                 'M%.1f,%.1f h3 a2,2 0 0 1 0,4 h-2" fill="#fff" stroke="%s" stroke-width="1.2" '
                 'stroke-linejoin="round"/>' % (x + 9, y + 17, x + 18, y + 23, x + 16, y + 17, STROKE))
    elif t == "scriptTask":
        s.append('<path d="M%.1f,%.1f h11 c-4,4 4,9 0,13 h-11 c4,-4 -4,-9 0,-13 z" fill="#fff" stroke="%s" '
                 'stroke-width="1.2"/>' % (x + 9, y + 7, STROKE))
        for k in range(3):
            s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1"/>'
                     % (x + 9.5, y + 10.5 + 3 * k, x + 16.5, y + 10.5 + 3 * k, STROKE))
    elif t == "businessRuleTask":
        s.append('<rect x="%.1f" y="%.1f" width="18" height="13" fill="#fff" stroke="%s" stroke-width="1.2"/>'
                 % (x + 6, y + 8, STROKE))
        s.append('<rect x="%.1f" y="%.1f" width="18" height="4" fill="%s"/>' % (x + 6, y + 8, "#9aa1a9"))
        s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1"/>'
                 % (x + 6, y + 16.5, x + 24, y + 16.5, STROKE))
        s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1"/>'
                 % (x + 11, y + 12, x + 11, y + 21, STROKE))
    elif t in ("sendTask", "receiveTask"):
        filled = t == "sendTask"
        fill = STROKE if filled else "#fff"
        ln = "#fff" if filled else STROKE
        s.append('<rect x="%.1f" y="%.1f" width="17" height="12" fill="%s" stroke="%s" stroke-width="1.2"/>'
                 % (x + 6, y + 8, fill, STROKE))
        s.append('<polyline points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="none" stroke="%s" stroke-width="1.2"/>'
                 % (x + 6, y + 8, x + 14.5, y + 15, x + 23, y + 8, ln))
    return "".join(s)


def _markers(e, x, y, w, h):
    t = local(e.tag)
    mk = []
    if e.find(M + "standardLoopCharacteristics") is not None:
        mk.append("loop")
    mi = e.find(M + "multiInstanceLoopCharacteristics")
    if mi is not None:
        mk.append("mi_seq" if mi.get("isSequential") == "true" else "mi_par")
    if t in SUBP and not _has_flow_children(e):
        mk.append("plus")
    if t == "callActivity":
        called = e.get("calledElement")
        if called:
            mk.append("plus")
    if t == "adHocSubProcess":
        mk.append("adhoc")
    if e.get("isForCompensation") == "true":
        mk.append("comp")
    if not mk:
        return ""
    s = []
    total = len(mk) * 16
    bx = x + w / 2.0 - total / 2.0
    by = y + h - 18
    for k, m_ in enumerate(mk):
        cx = bx + 16 * k + 8
        cy = by + 7
        if m_ == "loop":
            s.append('<path d="M%.1f,%.1f A5.5,5.5 0 1 1 %.1f,%.1f" fill="none" stroke="%s" stroke-width="1.3"/>'
                     % (cx - 4, cy + 4, cx + 4, cy + 4, STROKE))
            s.append('<path d="M%.1f,%.1f l-1,4 l4,-1" fill="none" stroke="%s" stroke-width="1.3"/>' % (cx - 4, cy + 0, STROKE))
        elif m_ == "mi_par":
            for d in (-4, 0, 4):
                s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="2"/>'
                         % (cx + d, cy - 6, cx + d, cy + 6, STROKE))
        elif m_ == "mi_seq":
            for d in (-4, 0, 4):
                s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="2"/>'
                         % (cx - 6, cy + d, cx + 6, cy + d, STROKE))
        elif m_ == "plus":
            s.append('<rect x="%.1f" y="%.1f" width="12" height="12" fill="#fff" stroke="%s" stroke-width="1.2"/>'
                     % (cx - 6, cy - 6, STROKE))
            s.append('<path d="M%.1f,%.1f h8 M%.1f,%.1f v8" stroke="%s" stroke-width="1.4"/>'
                     % (cx - 4, cy, cx, cy - 4, STROKE))
        elif m_ == "adhoc":
            s.append('<text x="%.1f" y="%.1f" font-size="16" text-anchor="middle">~</text>' % (cx, cy + 5))
        elif m_ == "comp":
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="none" stroke="%s"/>'
                     % (cx - 7, cy, cx, cy - 5, cx, cy + 5, STROKE))
            s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="none" stroke="%s"/>'
                     % (cx, cy, cx + 7, cy - 5, cx + 7, cy + 5, STROKE))
    return "".join(s)


def _flow_label(nm, pts):
    """Etykieta przepływu sekwencji: przy pierwszym wyraźnym odcinku trasy."""
    lines = _wrap(nm, 90, 10)
    (x1, y1), (x2, y2) = pts[0], pts[1]
    first_vertical = abs(x2 - x1) < abs(y2 - y1)
    if first_vertical and len(pts) >= 3 and abs(y2 - y1) > 40:
        return _text(lines, x2 + 8, y2 - 7, 10, "start")
    if not first_vertical and len(pts) >= 3 and abs(pts[2][0] - x2) < 0.6 and abs(pts[2][1] - y2) > 30:
        # pierwszy odcinek poziomy, potem zakręt: etykieta przy zakręcie, by nie nachodziła na sąsiednie gałęzie
        if pts[2][1] > y2:
            return _text(lines, x2 + 6, y2 + 18, 10, "start")
        return _text(lines, x2 + 6, y2 - 8, 10, "start")
    k = 0
    while k < len(pts) - 2 and ((pts[k + 1][0] - pts[k][0]) ** 2 + (pts[k + 1][1] - pts[k][1]) ** 2) ** 0.5 < 30:
        k += 1
    (x1, y1), (x2, y2) = pts[k], pts[k + 1]
    if abs(x2 - x1) >= abs(y2 - y1):
        return _text(lines, x1 + 8, y1 - 7, 10, "start")
    if y2 > y1:
        return _text(lines, x1 + 6, y1 + 16, 10, "start")
    return _text(lines, x1 + 6, y1 - 8, 10, "start")


def render_svg(m, title=None):
    shapes, edges = {}, {}
    for s in m.root.iter(BDI + "BPMNShape"):
        b = s.find(DC + "Bounds")
        if b is None:
            continue
        shapes[s.get("bpmnElement")] = (float(b.get("x")), float(b.get("y")), float(b.get("width")),
                                        float(b.get("height")))
    for e in m.root.iter(BDI + "BPMNEdge"):
        edges[e.get("bpmnElement")] = [(float(p.get("x")), float(p.get("y"))) for p in e.findall(DI + "waypoint")]
    if not shapes:
        raise LoadError("Model nie ma współrzędnych (BPMNDI). Najpierw uruchom: bpmn.py layout")

    xs, ys = [], []
    for (x, y, w, h) in shapes.values():
        xs += [x, x + w, x + w / 2.0 - 62, x + w / 2.0 + 62]
        ys += [y, y + h + 36]
    for pts in edges.values():
        for (x, y) in pts:
            xs.append(x)
            ys.append(y)
    minx, miny = min(xs) - 20, min(ys) - 20
    maxx, maxy = max(xs) + 60, max(ys) + 20
    W, H = maxx - minx, maxy - miny
    P = []
    P.append('<svg xmlns="http://www.w3.org/2000/svg" viewBox="%.0f %.0f %.0f %.0f" width="%.0f" height="%.0f" '
             'font-family="Arial, Helvetica, sans-serif">' % (minx, miny, W, H, W, H))
    if title:
        P.append("<title>%s</title>" % _esc(title))
    P.append('<defs>'
             '<marker id="seqEnd" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" '
             'markerWidth="11" markerHeight="11" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="%s"/></marker>'
             '<marker id="msgEnd" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" '
             'markerWidth="11" markerHeight="11" orient="auto"><path d="M0.5,0.5 L9.5,5 L0.5,9.5 z" fill="#fff" '
             'stroke="%s"/></marker>'
             '<marker id="msgStart" viewBox="0 0 10 10" refX="5" refY="5" markerUnits="userSpaceOnUse" '
             'markerWidth="9" markerHeight="9"><circle cx="5" cy="5" r="4" fill="#fff" stroke="%s"/></marker>'
             '<marker id="assocEnd" viewBox="0 0 10 10" refX="10" refY="5" markerUnits="userSpaceOnUse" '
             'markerWidth="10" markerHeight="10" orient="auto"><path d="M0,0 L10,5 L0,10" fill="none" '
             'stroke="%s"/></marker>'
             '</defs>' % (STROKE, STROKE, STROKE, STROKE))
    P.append('<rect x="%.0f" y="%.0f" width="%.0f" height="%.0f" fill="#fff"/>' % (minx, miny, W, H))

    labels = []

    def el(i):
        return m.get(i)

    # baseny i linie
    for i, (x, y, w, h) in shapes.items():
        e = el(i)
        if e is None or local(e.tag) not in ("participant", "lane"):
            continue
        hw = 30
        if local(e.tag) == "participant" and not e.get("processRef"):
            P.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="#f2f4f7" stroke="%s" '
                     'stroke-width="1.3"/>' % (x, y, w, h, STROKE))
            P.append(_text([Model.name(e)], x + w / 2.0, y + h / 2.0 + 5, 13, "middle", 'font-weight="bold"'))
            continue
        P.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="#fff" stroke="%s" stroke-width="1.3"/>'
                 % (x, y, w, h, STROKE))
        P.append('<rect x="%.1f" y="%.1f" width="%d" height="%.1f" fill="%s" stroke="%s" stroke-width="1.3"/>'
                 % (x, y, hw, h, "#f2f4f7" if local(e.tag) == "participant" else "#f8f9fb", STROKE))
        nm = Model.name(e)
        if nm:
            lines = _wrap(nm, max(h - 10, 40), 12)[:3]
            cx, cy = x + hw / 2.0, y + h / 2.0
            lh = 14.4
            y0 = cy - (len(lines) - 1) * lh / 2.0
            tx = '<g transform="rotate(-90 %.1f %.1f)">' % (cx, cy)
            for k, ln in enumerate(lines):
                tx += '<text x="%.1f" y="%.1f" font-size="12" text-anchor="middle" dominant-baseline="middle" ' \
                      'transform="translate(0 %.1f)">%s</text>' % (cx, cy, (k - (len(lines) - 1) / 2.0) * lh, _esc(ln))
            tx += "</g>"
            _ = y0
            P.append(tx)

    # podprocesy (tła)
    for i, (x, y, w, h) in shapes.items():
        e = el(i)
        if e is None or local(e.tag) not in SUBP or not _has_flow_children(e):
            continue
        dash = ' stroke-dasharray="2 3"' if Model.is_event_sub(e) else ""
        P.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" rx="10" fill="#fff" stroke="%s" '
                 'stroke-width="1.5"%s/>' % (x, y, w, h, STROKE, dash))
        if local(e.tag) == "transaction":
            P.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" rx="8" fill="none" stroke="%s"/>'
                     % (x + 3, y + 3, w - 6, h - 6, STROKE))
        nm = Model.name(e)
        if nm:
            P.append(_text(_wrap(nm, w - 20, 11)[:2], x + 10, y + 18, 11, "start", 'font-weight="bold"'))
        P.append(_markers(e, x, y, w, h))

    # krawędzie
    for i, pts in edges.items():
        e = el(i)
        if e is None or len(pts) < 2:
            continue
        t = local(e.tag)
        pstr = " ".join("%.1f,%.1f" % p for p in pts)
        if t == "sequenceFlow":
            P.append('<polyline points="%s" fill="none" stroke="%s" stroke-width="1.5" stroke-linejoin="round" '
                     'marker-end="url(#seqEnd)"/>' % (pstr, STROKE))
            src = el(e.get("sourceRef"))
            if src is not None and src.get("default") == i:
                (x1, y1), (x2, y2) = pts[0], pts[1]
                dx, dy = x2 - x1, y2 - y1
                L = max((dx * dx + dy * dy) ** 0.5, 1)
                ux, uy = dx / L, dy / L
                mx, my = x1 + ux * 10, y1 + uy * 10
                P.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="1.5"/>'
                         % (mx - 5 * ux - 5 * uy, my - 5 * uy + 5 * ux, mx + 5 * ux + 5 * uy, my + 5 * uy - 5 * ux, STROKE))
            if src is not None and local(src.tag) in ACT and e.find(M + "conditionExpression") is not None:
                (x1, y1), (x2, y2) = pts[0], pts[1]
                dx, dy = x2 - x1, y2 - y1
                L = max((dx * dx + dy * dy) ** 0.5, 1)
                ux, uy = dx / L, dy / L
                pts_d = [(x1, y1), (x1 + ux * 7 - uy * 5, y1 + uy * 7 + ux * 5), (x1 + ux * 14, y1 + uy * 14),
                         (x1 + ux * 7 + uy * 5, y1 + uy * 7 - ux * 5)]
                P.append('<polygon points="%s" fill="#fff" stroke="%s" stroke-width="1.2"/>'
                         % (" ".join("%.1f,%.1f" % p for p in pts_d), STROKE))
            nm = Model.name(e)
            if nm:
                labels.append(_flow_label(nm, pts))
        elif t == "messageFlow":
            P.append('<polyline points="%s" fill="none" stroke="%s" stroke-width="1.3" stroke-dasharray="7 4" '
                     'marker-start="url(#msgStart)" marker-end="url(#msgEnd)"/>' % (pstr, STROKE))
            nm = Model.name(e)
            if nm:
                k = len(pts) // 2
                (x1, y1), (x2, y2) = pts[k - 1], pts[k]
                labels.append(_text(_wrap(nm, 110, 10), (x1 + x2) / 2.0 + 6, (y1 + y2) / 2.0 - 4, 10, "start",
                                    'font-style="italic"'))
        elif t == "association":
            me = ' marker-end="url(#assocEnd)"' if e.get("associationDirection") in ("One", "Both") else ""
            P.append('<polyline points="%s" fill="none" stroke="%s" stroke-width="1.2" stroke-dasharray="2 3"%s/>'
                     % (pstr, STROKE, me))
        elif t in ("dataInputAssociation", "dataOutputAssociation"):
            P.append('<polyline points="%s" fill="none" stroke="%s" stroke-width="1.2" stroke-dasharray="2 3" '
                     'marker-end="url(#assocEnd)"/>' % (pstr, STROKE))

    # węzły
    boundary_draw = []
    for i, (x, y, w, h) in shapes.items():
        e = el(i)
        if e is None:
            continue
        t = local(e.tag)
        if t in ("participant", "lane"):
            continue
        if t == "boundaryEvent":
            boundary_draw.append((i, x, y, w, h))
            continue
        if t in SUBP and _has_flow_children(e):
            continue
        P.append(_draw_node(m, e, t, x, y, w, h, labels))
    for (i, x, y, w, h) in boundary_draw:
        e = el(i)
        P.append(_draw_node(m, e, "boundaryEvent", x, y, w, h, labels))
    P.extend(labels)
    P.append("</svg>")
    return "\n".join(P)


def _draw_node(m, e, t, x, y, w, h, labels):
    s = []
    nm = Model.name(e)
    cx, cy = x + w / 2.0, y + h / 2.0
    if t in ACT:
        sw = "4" if t == "callActivity" else "1.5"
        s.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" rx="10" fill="#fff" stroke="%s" '
                 'stroke-width="%s"/>' % (x, y, w, h, STROKE, sw))
        s.append(_task_icon(t, x, y))
        has_icon = t in ("userTask", "serviceTask", "manualTask", "scriptTask", "businessRuleTask", "sendTask",
                         "receiveTask")
        for fs in (11, 10, 9):
            lines = _wrap(nm, w - 12, fs)
            if len(lines) * fs * 1.2 <= h - (30 if has_icon else 14):
                break
        lh = fs * 1.2
        top = cy - (len(lines) * lh) / 2.0 + fs * 0.85 + (6 if has_icon else 0)
        s.append(_text(lines, cx, top, fs))
        s.append(_markers(e, x, y, w, h))
    elif t in EVT:
        defs = Model.eventdefs(e)
        defs = [d for d in defs if d != "ref"]
        nonint = (t == "boundaryEvent" and e.get("cancelActivity") == "false") or \
                 (t == "startEvent" and e.get("isInterrupting") == "false")
        dash = ' stroke-dasharray="5 3"' if nonint else ""
        r = 18
        if t == "startEvent":
            s.append('<circle cx="%.1f" cy="%.1f" r="%d" fill="#fff" stroke="%s" stroke-width="1.6"%s/>'
                     % (cx, cy, r, STROKE, dash))
        elif t == "endEvent":
            s.append('<circle cx="%.1f" cy="%.1f" r="%d" fill="#fff" stroke="%s" stroke-width="3.5"/>' % (cx, cy, r - 1, STROKE))
        else:
            s.append('<circle cx="%.1f" cy="%.1f" r="%d" fill="#fff" stroke="%s" stroke-width="1.3"%s/>'
                     % (cx, cy, r, STROKE, dash))
            s.append('<circle cx="%.1f" cy="%.1f" r="%d" fill="none" stroke="%s" stroke-width="1.3"%s/>'
                     % (cx, cy, r - 3, STROKE, dash))
        filled = t in ("endEvent", "intermediateThrowEvent")
        s.append(_event_icon(defs, cx, cy, filled))
        if nm:
            labels.append(_text(_wrap(nm, 110, 10), cx, y + h + 13, 10))
    elif t in GWS:
        s.append('<polygon points="%.1f,%.1f %.1f,%.1f %.1f,%.1f %.1f,%.1f" fill="#fff" stroke="%s" '
                 'stroke-width="1.5"/>' % (cx, y, x + w, cy, cx, y + h, x, cy, STROKE))
        if t == "exclusiveGateway":
            s.append('<path d="M%.1f,%.1f L%.1f,%.1f M%.1f,%.1f L%.1f,%.1f" stroke="%s" stroke-width="4"/>'
                     % (cx - 8, cy - 8, cx + 8, cy + 8, cx + 8, cy - 8, cx - 8, cy + 8, STROKE))
        elif t == "parallelGateway":
            s.append('<path d="M%.1f,%.1f L%.1f,%.1f M%.1f,%.1f L%.1f,%.1f" stroke="%s" stroke-width="4"/>'
                     % (cx - 11, cy, cx + 11, cy, cx, cy - 11, cx, cy + 11, STROKE))
        elif t == "inclusiveGateway":
            s.append('<circle cx="%.1f" cy="%.1f" r="10" fill="none" stroke="%s" stroke-width="2.5"/>' % (cx, cy, STROKE))
        elif t == "complexGateway":
            s.append('<path d="M%.1f,%.1f L%.1f,%.1f M%.1f,%.1f L%.1f,%.1f M%.1f,%.1f L%.1f,%.1f M%.1f,%.1f L%.1f,%.1f" '
                     'stroke="%s" stroke-width="3"/>' % (cx - 11, cy, cx + 11, cy, cx, cy - 11, cx, cy + 11, cx - 8,
                                                        cy - 8, cx + 8, cy + 8, cx + 8, cy - 8, cx - 8, cy + 8, STROKE))
        elif t == "eventBasedGateway":
            import math
            s.append('<circle cx="%.1f" cy="%.1f" r="12" fill="none" stroke="%s" stroke-width="1.2"/>' % (cx, cy, STROKE))
            s.append('<circle cx="%.1f" cy="%.1f" r="9.5" fill="none" stroke="%s" stroke-width="1.2"/>' % (cx, cy, STROKE))
            if e.get("eventGatewayType") == "Parallel":
                s.append('<path d="M%.1f,%.1f h12 M%.1f,%.1f v12" stroke="%s" stroke-width="2"/>'
                         % (cx - 6, cy, cx, cy - 6, STROKE))
            else:
                pts = " ".join("%.1f,%.1f" % (cx + 6 * math.cos(-math.pi / 2 + k * 2 * math.pi / 5),
                                              cy + 6 * math.sin(-math.pi / 2 + k * 2 * math.pi / 5)) for k in range(5))
                s.append('<polygon points="%s" fill="none" stroke="%s" stroke-width="1.2"/>' % (pts, STROKE))
        if nm:
            labels.append(_text(_wrap(nm, 120, 10), cx, y + h + 13, 10))
    elif t == "dataObjectReference":
        s.append('<path d="M%.1f,%.1f h%.1f l10,10 v%.1f h-%.1f z M%.1f,%.1f v10 h10" fill="#fff" stroke="%s" '
                 'stroke-width="1.3" stroke-linejoin="round"/>'
                 % (x, y, w - 10, h - 10, w, x + w - 10, y, STROKE))
        dobj = m.get(e.get("dataObjectRef"))
        if dobj is not None and dobj.get("isCollection") == "true":
            for d in (-4, 0, 4):
                s.append('<line x1="%.1f" y1="%.1f" x2="%.1f" y2="%.1f" stroke="%s" stroke-width="2"/>'
                         % (cx + d, y + h - 13, cx + d, y + h - 4, STROKE))
        st = e.find(M + "dataState")
        lab = nm or (Model.name(dobj) if dobj is not None else "")
        if st is not None and st.get("name"):
            lab += " [%s]" % st.get("name")
        if lab:
            labels.append(_text(_wrap(lab, 110, 10), cx, y + h + 13, 10))
    elif t == "dataStoreReference":
        rx = w / 2.0
        s.append('<path d="M%.1f,%.1f a%.1f,6 0 0 0 %.1f,0 a%.1f,6 0 0 0 -%.1f,0 v%.1f a%.1f,6 0 0 0 %.1f,0 v-%.1f" '
                 'fill="#fff" stroke="%s" stroke-width="1.3"/>'
                 % (x, y + 6, rx, w, rx, w, h - 12, rx, w, h - 12, STROKE))
        s.append('<path d="M%.1f,%.1f a%.1f,6 0 0 0 %.1f,0 M%.1f,%.1f a%.1f,6 0 0 0 %.1f,0" fill="none" '
                 'stroke="%s" stroke-width="1.1"/>' % (x, y + 11, rx, w, x, y + 16, rx, w, STROKE))
        ds = m.get(e.get("dataStoreRef"))
        lab = nm or (Model.name(ds) if ds is not None else "")
        if lab:
            labels.append(_text(_wrap(lab, 110, 10), cx, y + h + 13, 10))
    elif t == "textAnnotation":
        s.append('<path d="M%.1f,%.1f h-10 v%.1f h10" fill="none" stroke="%s" stroke-width="1.3"/>'
                 % (x + 10, y, h, STROKE))
        txt = (e.findtext(M + "text") or "").strip()
        s.append(_text(_wrap(txt, w - 10, 11), x + 6, y + 15, 11, "start"))
    return "".join(s)


# --------------------------------------------------------------------------- CLI

def cmd_check(args):
    try:
        tree, _ = load(args.plik)
    except LoadError as ex:
        _fatal(args, ex)
        return 2
    m = Model(tree)
    issues = validate(m, kroki=args.kroki, opis_kroki=_opis(args))
    if args.camunda:
        issues += validate_camunda(m, os.path.dirname(args.plik))
    if args.json:
        json.dump({"plik": args.plik, "model": summarize(m), "zastrzezenia": issues}, sys.stdout,
                  ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    else:
        print_report(args.plik, m, issues)
    return 1 if any(i["poziom"] == "błąd" for i in issues) else 0


def cmd_layout(args):
    try:
        tree, in_map = load(args.plik)
    except LoadError as ex:
        _fatal(args, ex)
        return 2
    m = Model(tree)
    errs = [i for i in validate(m) if i["poziom"] == "błąd" and i["kod"] in ("E002", "E003", "E004")]
    if errs:
        print_report(args.plik, m, errs)
        sys.stdout.write("Układ przerwany: najpierw popraw powyższe błędy.\n")
        return 1
    n = layout(m)
    dest = args.out or args.plik
    save(tree, dest, in_map)
    sys.stdout.write("Układ zapisany: %s (%d kształtów).\n" % (dest, n))
    return 0


def cmd_svg(args):
    try:
        tree, _ = load(args.plik)
    except LoadError as ex:
        _fatal(args, ex)
        return 2
    m = Model(tree)
    if not list(m.root.iter(BDI + "BPMNShape")):
        layout(m)
    svg = render_svg(m, title=os.path.basename(args.plik))
    dest = args.out or os.path.splitext(args.plik)[0] + ".svg"
    with open(dest, "w", encoding="utf-8") as f:
        f.write(svg)
    sys.stdout.write("SVG zapisany: %s\n" % dest)
    return 0


def cmd_all(args):
    try:
        tree, in_map = load(args.plik)
    except LoadError as ex:
        _fatal(args, ex)
        return 2
    m = Model(tree)
    opis = _opis(args)
    issues = [i for i in validate(m, kroki=args.kroki, opis_kroki=opis) if not i["kod"].startswith("D")]
    if args.camunda:
        issues += validate_camunda(m, os.path.dirname(args.plik))
    if any(i["poziom"] == "błąd" for i in issues):
        if args.json:
            json.dump({"plik": args.plik, "etap": "walidacja", "zastrzezenia": issues}, sys.stdout,
                      ensure_ascii=False, indent=2)
            sys.stdout.write("\n")
        else:
            print_report(args.plik, m, issues)
            sys.stdout.write("\nZatrzymano przed układem: popraw błędy i uruchom ponownie.\n")
        return 1
    n = layout(m)
    save(tree, args.plik, in_map)
    tree2, _ = load(args.plik)
    m2 = Model(tree2)
    issues2 = validate(m2, kroki=args.kroki, opis_kroki=opis)
    if args.camunda:
        issues2 += validate_camunda(m2, os.path.dirname(args.plik))
    svg_path = args.svg or os.path.splitext(args.plik)[0] + ".svg"
    with open(svg_path, "w", encoding="utf-8") as f:
        f.write(render_svg(m2, title=os.path.basename(args.plik)))
    if args.json:
        json.dump({"plik": args.plik, "svg": svg_path, "ksztalty": n, "model": summarize(m2),
                   "zastrzezenia": issues2}, sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    else:
        print_report(args.plik, m2, issues2)
        sys.stdout.write("\nUkład: %d kształtów zapisanych w %s\nDiagram: %s\n" % (n, args.plik, svg_path))
    return 1 if any(i["poziom"] == "błąd" for i in issues2) else 0


def _opis(args):
    path = getattr(args, "opis", None)
    if not path:
        return None
    try:
        steps = read_opis(path)
    except OSError as ex:
        sys.stdout.write("[UWAGA] Nie można odczytać opisu %s: %s\n" % (path, ex))
        return None
    if steps:
        args.kroki = True
    return steps or None


def _fatal(args, ex):
    if getattr(args, "json", False):
        json.dump({"plik": args.plik, "zastrzezenia": [{"poziom": "błąd", "kod": "E001", "id": None,
                                                         "komunikat": str(ex)}]},
                  sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    else:
        sys.stdout.write("[BŁĄD E001] %s\n" % ex)


def main(argv=None):
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8")
        except Exception:
            pass
    ap = argparse.ArgumentParser(prog="bpmn.py", description="Walidacja, układ i render modeli BPMN 2.0.")
    ap.add_argument("--version", action="version", version="bpmn.py " + VERSION)
    sub = ap.add_subparsers(dest="cmd")
    p = sub.add_parser("check", help="walidacja modelu")
    p.add_argument("plik")
    p.add_argument("--kroki", action="store_true", help="wymagaj numerów kroków w nazwach elementów")
    p.add_argument("--opis", help="plik z tabelą kroków (np. proces.md): sprawdź, czy model ma wszystkie kroki")
    p.add_argument("--json", action="store_true", help="wynik w formacie JSON")
    p.add_argument("--camunda", action="store_true", help="kontrola modelu wykonywalnego dla Camunda 8")
    p.set_defaults(fn=cmd_check)
    p = sub.add_parser("layout", help="automatyczny układ (generuje BPMNDI)")
    p.add_argument("plik")
    p.add_argument("-o", "--out", help="plik wynikowy (domyślnie nadpisuje wejściowy)")
    p.set_defaults(fn=cmd_layout)
    p = sub.add_parser("svg", help="render do SVG")
    p.add_argument("plik")
    p.add_argument("-o", "--out", help="plik SVG (domyślnie obok modelu)")
    p.set_defaults(fn=cmd_svg)
    p = sub.add_parser("all", help="walidacja, układ, walidacja i SVG")
    p.add_argument("plik")
    p.add_argument("--kroki", action="store_true", help="wymagaj numerów kroków w nazwach elementów")
    p.add_argument("--opis", help="plik z tabelą kroków (np. proces.md): sprawdź, czy model ma wszystkie kroki")
    p.add_argument("--svg", help="plik SVG (domyślnie obok modelu)")
    p.add_argument("--json", action="store_true", help="wynik w formacie JSON")
    p.add_argument("--camunda", action="store_true", help="kontrola modelu wykonywalnego dla Camunda 8")
    p.set_defaults(fn=cmd_all)
    args = ap.parse_args(argv)
    if not getattr(args, "fn", None):
        ap.print_help()
        return 2
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
