"""Diagnostic codes, severities and messages in English and Polish.

A diagnostic is created with a code and named arguments; the text is looked
up in ``MESSAGES`` when it is printed, so every tool shows the same wording.
Codes E, W, L and C come from the BPMN validator (``bpmnlib``) and carry
their own Polish text; ``translate_bpmn`` gives them an English prefix.
"""

ERROR, WARNING, NOTE = "error", "warning", "note"

MESSAGES = {
    # tokens
    "T001": (ERROR, "invalid token: {detail}", "niepoprawny znak lub tekst: {detail}"),
    "T002": (ERROR, "tab character in indentation; use spaces", "tabulator we wcięciu; używaj spacji"),
    "T003": (ERROR, "indentation does not match any open block", "wcięcie nie pasuje do żadnego otwartego bloku"),
    # statements
    "P001": (ERROR, "unknown keyword '{word}'", "nieznane słowo kluczowe „{word}”"),
    "P002": (ERROR, "'{keyword}' needs an identifier", "„{keyword}” wymaga identyfikatora"),
    "P003": (ERROR, "'{keyword}' cannot have an indented block", "„{keyword}” nie może mieć wciętego bloku"),
    "P004": (ERROR, "'{keyword}' is not allowed inside {context}", "„{keyword}” nie może wystąpić w bloku {context}"),
    "P005": (ERROR, "the document must start with the header 'tok 0.1'", "dokument musi zaczynać się nagłówkiem „tok 0.1”"),
    "P006": (WARNING, "unknown language version {version}; this tool reads version 0.1",
             "nieznana wersja języka {version}; to narzędzie czyta wersję 0.1"),
    "P007": (ERROR, "malformed attribute near '{near}'", "niepoprawny atrybut w pobliżu „{near}”"),
    "P008": (ERROR, "malformed {keyword} line: {detail}", "niepoprawny wiersz {keyword}: {detail}"),
    "P009": (ERROR, "malformed decision table row: {detail}", "niepoprawny wiersz tabeli decyzyjnej: {detail}"),
    "P010": (ERROR, "a second header", "drugi nagłówek dokumentu"),
    # references and structure
    "R001": (ERROR, "identifier '{id}' is already used on line {other}", "identyfikator „{id}” jest już użyty w wierszu {other}"),
    "R002": (ERROR, "unknown {what} '{id}'", "nieznany element ({what_pl}): „{id}”"),
    "R003": (ERROR, "'{id}' is a '{actual}' element; required here: {expected}",
             "„{id}” to {actual}, a tu wymagany jest element: {expected_pl}"),
    "R004": (ERROR, "identifier '{id}' must use only ASCII letters, digits, '_' and '-'",
             "identyfikator „{id}” może zawierać tylko litery ASCII, cyfry, „_” i „-”"),
    "R010": (WARNING, "unknown attribute '{key}' on {kind}", "nieznany atrybut „{key}” w elemencie {kind}"),
    "R011": (ERROR, "unknown flag '{flag}' on {kind}", "nieznana flaga „{flag}” w elemencie {kind}"),
    "R012": (ERROR, "an event can have only one trigger ({triggers})", "zdarzenie może mieć tylko jeden wyzwalacz ({triggers})"),
    "R013": (ERROR, "'{key}' requires {requirement}", "„{key}” wymaga: {requirement}"),
    "R020": (ERROR, "a flow line with a label or attributes must contain exactly one arrow",
             "wiersz flow z etykietą lub atrybutami musi mieć dokładnie jedną strzałkę"),
    "R021": (ERROR, "sequence flow {source} -> {target} crosses a process or sub-process boundary",
             "przepływ sekwencji {source} -> {target} przekracza granicę procesu lub podprocesu"),
    "R022": (ERROR, "message flow {source} -> {target} must connect two different pools",
             "przepływ komunikatów {source} -> {target} musi łączyć dwa różne baseny"),
    "R030": (ERROR, "'{id}' is in a pool with lanes but outside every lane", "„{id}” leży w basenie z liniami, ale poza liniami"),
    "R031": (ERROR, "'{id}' is declared at top level, but the document has pools; move it into a pool",
             "„{id}” zadeklarowano poza basenem, a dokument ma baseny; przenieś go do basenu"),
    "R032": (ERROR, "boundary event '{id}' needs on=ACTIVITY", "zdarzenie graniczne „{id}” wymaga on=CZYNNOŚĆ"),
    "R050": (ERROR, "ad-hoc sub-process '{id}' must contain at least one activity and no start or end events",
             "podproces ad hoc „{id}” musi zawierać co najmniej jedną czynność i żadnych zdarzeń początkowych ani końcowych"),
    "R051": (ERROR, "{detail}", "{detail_pl}"),
    "R052": (ERROR, "events= is not allowed on the root-level start event '{id}'",
             "events= nie może wystąpić na zdarzeniu początkowym procesu „{id}”"),
    "R053": (ERROR, "malformed {key}=\"…\" list: '{item}' (expected {expected})",
             "niepoprawna lista {key}=\"…\": „{item}” (oczekiwano: {expected_pl})"),
    "R054": (ERROR, "{detail}", "{detail_pl}"),
    "R055": (WARNING, "'{key}' belongs to the executable layer and is ignored because the document is not executable",
             "„{key}” należy do warstwy wykonywalnej i jest pomijany, bo dokument nie jest wykonywalny"),
    "C022": (ERROR, "message {kind} event '{id}' needs type=\"…\" (Camunda executes it as a job)",
             "zdarzenie {kind_pl} wiadomości „{id}” wymaga type=\"…\" (Camunda wykonuje je jako zadanie dla workera)"),
    "C090": (ERROR, "{key}=\"{value}\" must be an ISO 8601 date-time with a zone (e.g. 2027-01-15T12:00:00+01:00) or an expression starting with =",
             "{key}=\"{value}\" musi być datą i godziną ISO 8601 ze strefą (np. 2027-01-15T12:00:00+01:00) albo wyrażeniem zaczynającym się od ="),
    "C091": (ERROR, "priority must be an integer from 0 to 100 or an expression starting with =, not '{value}'",
             "priority musi być liczbą całkowitą od 0 do 100 albo wyrażeniem zaczynającym się od =, a nie „{value}”"),
    "R040": (ERROR, "'{keyword}' is not available at level {level}", "„{keyword}” nie jest dostępne na poziomie {level}"),
    # decisions
    "D001": (ERROR, "decision '{id}' has no header row", "decyzja „{id}” nie ma wiersza nagłówka"),
    "D002": (ERROR, "rule has {got} cells, the header has {expected}", "reguła ma {got} komórek, a nagłówek {expected}"),
    "D003": (ERROR, "unknown hit policy '{hit}'", "nieznana polityka trafień „{hit}”"),
    "D004": (ERROR, "header cell '{cell}' must be 'name: type' with type string, number, boolean or date",
             "komórka nagłówka „{cell}” musi mieć postać „nazwa: typ” (string, number, boolean, date)"),
    "D005": (WARNING, "decision '{id}' has no rules", "decyzja „{id}” nie ma reguł"),
    "D010": (WARNING, "rules {a} and {b} can both match {example} (hit policy unique)",
             "reguły {a} i {b} mogą pasować jednocześnie, np. {example} (polityka unique)"),
    # forms
    "M001": (ERROR, "unknown field type '{type}'", "nieznany typ pola „{type}”"),
    "M002": (ERROR, "field '{id}' of type {type} needs options=\"value:Label, …\"",
             "pole „{id}” typu {type} wymaga options=\"wartość:Etykieta, …\""),
    # FEEL
    "F001": (WARNING, "FEEL expression outside the supported subset: {detail}",
             "wyrażenie FEEL spoza obsługiwanego podzbioru: {detail_pl}"),
    "F002": (ERROR, "FEEL syntax error: {detail}", "błąd składni FEEL: {detail_pl}"),
    # simulation
    "S001": (ERROR, "invalid distribution '{text}': {detail}", "niepoprawny rozkład „{text}”: {detail}"),
    "S002": (WARNING, "probabilities of the flows leaving '{id}' sum to {total}, not 1",
             "prawdopodobieństwa przepływów z „{id}” sumują się do {total}, a nie do 1"),
    "S003": (WARNING, "gateway '{id}' has neither conditions nor probabilities; the simulator takes the first flow",
             "bramka „{id}” nie ma ani warunków, ani prawdopodobieństw; symulator wybierze pierwszy przepływ"),
}

# English text for the BPMN rules of bpmnlib (its own messages are Polish)
BPMN_EN = {
    "E001": "the file is not well-formed XML",
    "E002": "the root element is not bpmn:definitions",
    "E003": "an id is used more than once",
    "E004": "reference to a non-existent id",
    "E005": "sequence flow without both ends or connecting data, a pool or an annotation",
    "E006": "sequence flow between pools or into a sub-process",
    "E007": "message flow from an element outside the pools",
    "E008": "message flow inside one pool (use a sequence flow between lanes)",
    "E009": "message flow to a gateway, data or a non-message event",
    "E010": "trigger not allowed for this event position",
    "E011": "incoming flow to a start event or outgoing flow from an end event",
    "E012": "invalid boundary event (on a gateway, with incoming flow, non-interrupting error, …)",
    "E013": "event sub-process with sequence flows or without a triggered start event",
    "E014": "compensation activity with sequence flows",
    "E015": "default flow that does not leave the element or has a condition",
    "E016": "event-based gateway followed by something other than catch events or receive tasks",
    "E017": "condition or default flow on a parallel gateway",
    "E018": "reference to a declaration of the wrong type",
    "E030": "element in two lanes",
    "W010": "timer event without a time",
    "W016": "event after an event-based gateway has other incoming flows",
    "W020": "gateway with one incoming and one outgoing flow is unnecessary",
    "W021": "branch of an XOR/OR split without label, condition or default status",
    "W022": "branch without a condition at an element that splits conditionally (it has a default flow or other branches have conditions)",
    "W023": "sequence flow repeats another one (same source, target and condition)",
    "W030": "lane refers to an element inside a sub-process",
    "W031": "element outside every lane",
    "W040": "process without a start or end event",
    "W041": "element not reachable from a start event",
    "W042": "path without an end event (element without outgoing flow)",
    "W043": "sequence flow from an element back to itself",
    "W050": "OR split without a default flow",
    "W051": "join of a different type than the split (e.g. AND split joined by XOR)",
    "W052": "boundary escalation/error on a sub-process that nothing inside throws",
    "W053": "escalation thrown but never caught",
    "W054": "message task or event without a message flow in a model with several pools",
    "L010": "message event without messageRef",
    "L020": "name of a splitting gateway is not a question",
    "L021": "gateway both joins and splits",
    "L030": "task without a name",
}

LEVELS = ("basic", "analytic", "full")


class Diag:
    __slots__ = ("code", "line", "id", "args", "text_pl", "text_en", "severity_override")

    def __init__(self, code, line=0, eid=None, text_pl=None, severity=None, text_en=None, **args):
        self.code = code
        self.line = line
        self.id = eid
        self.args = args
        self.text_pl = text_pl            # ready text (BPMN validator messages)
        self.text_en = text_en
        self.severity_override = severity

    @property
    def severity(self):
        if self.severity_override:
            return self.severity_override
        entry = MESSAGES.get(self.code)
        return entry[0] if entry else ERROR

    def message(self, lang="en"):
        if self.text_pl is not None:
            if lang == "en":
                if self.text_en:
                    return self.text_en
                if self.code in BPMN_EN:
                    return "%s (%s)" % (BPMN_EN[self.code], self.id) if self.id else BPMN_EN[self.code]
            return self.text_pl
        entry = MESSAGES.get(self.code)
        if not entry:
            return self.code
        template = entry[1] if lang == "en" else entry[2]
        try:
            return template.format(**self.args)
        except (KeyError, IndexError):
            return template

    def as_dict(self, lang="en"):
        return {"code": self.code, "severity": self.severity, "line": self.line, "id": self.id,
                "message": self.message(lang)}

    def __repr__(self):
        return "Diag(%s, line %d)" % (self.code, self.line)


def format_diag(d, lang="en", path=None):
    sev = {"en": {ERROR: "error", WARNING: "warning", NOTE: "note"},
           "pl": {ERROR: "błąd", WARNING: "ostrzeżenie", NOTE: "uwaga"}}[lang][d.severity]
    where = "%s:%d" % (path, d.line) if path else ("line %d" if lang == "en" else "wiersz %d") % d.line
    return "%s: %s %s: %s" % (where, sev, d.code, d.message(lang))


def sort_key(d):
    return (d.line, d.code)
