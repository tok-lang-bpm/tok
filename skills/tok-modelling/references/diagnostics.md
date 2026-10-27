# Diagnostics

Every diagnostic has a stable code, a severity and a line number. `tok check --lang pl` prints the Polish text; `--json` gives `code`, `severity`, `line`, `id` and `message` for programs and agents.

## Tokens and lines (T)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| T001 | error | invalid token: {detail} | niepoprawny znak lub tekst: {detail} |
| T002 | error | tab character in indentation; use spaces | tabulator we wcięciu; używaj spacji |
| T003 | error | indentation does not match any open block | wcięcie nie pasuje do żadnego otwartego bloku |

## Statements (P)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| P001 | error | unknown keyword '{word}' | nieznane słowo kluczowe „{word}” |
| P002 | error | '{keyword}' needs an identifier | „{keyword}” wymaga identyfikatora |
| P003 | error | '{keyword}' cannot have an indented block | „{keyword}” nie może mieć wciętego bloku |
| P004 | error | '{keyword}' is not allowed inside {context} | „{keyword}” nie może wystąpić w bloku {context} |
| P005 | error | the document must start with the header 'tok 0.1' | dokument musi zaczynać się nagłówkiem „tok 0.1” |
| P006 | warning | unknown language version {version}; this tool reads version 0.1 | nieznana wersja języka {version}; to narzędzie czyta wersję 0.1 |
| P007 | error | malformed attribute near '{near}' | niepoprawny atrybut w pobliżu „{near}” |
| P008 | error | malformed {keyword} line: {detail} | niepoprawny wiersz {keyword}: {detail} |
| P009 | error | malformed decision table row: {detail} | niepoprawny wiersz tabeli decyzyjnej: {detail} |
| P010 | error | a second header | drugi nagłówek dokumentu |

## References and structure (R)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| R001 | error | identifier '{id}' is already used on line {other} | identyfikator „{id}” jest już użyty w wierszu {other} |
| R002 | error | unknown {what} '{id}' | nieznany element ({what_pl}): „{id}” |
| R003 | error | '{id}' is a '{actual}' element; required here: {expected} | „{id}” to {actual}, a tu wymagany jest element: {expected_pl} |
| R004 | error | identifier '{id}' must use only ASCII letters, digits, '_' and '-' | identyfikator „{id}” może zawierać tylko litery ASCII, cyfry, „_” i „-” |
| R010 | warning | unknown attribute '{key}' on {kind} | nieznany atrybut „{key}” w elemencie {kind} |
| R011 | error | unknown flag '{flag}' on {kind} | nieznana flaga „{flag}” w elemencie {kind} |
| R012 | error | an event can have only one trigger ({triggers}) — documents of language 0.1 and 0.3; from 0.5 see R062 | zdarzenie może mieć tylko jeden wyzwalacz ({triggers}) |
| R013 | error | '{key}' requires {requirement} | „{key}” wymaga: {requirement} |
| R020 | error | a flow line with a label or attributes must contain exactly one arrow | wiersz flow z etykietą lub atrybutami musi mieć dokładnie jedną strzałkę |
| R021 | error | sequence flow {source} -> {target} crosses a process or sub-process boundary | przepływ sekwencji {source} -> {target} przekracza granicę procesu lub podprocesu |
| R022 | error | message flow {source} -> {target} must connect two different pools | przepływ komunikatów {source} -> {target} musi łączyć dwa różne baseny |
| R030 | error | '{id}' is in a pool with lanes but outside every lane | „{id}” leży w basenie z liniami, ale poza liniami |
| R031 | error | '{id}' is declared at top level, but the document has pools; move it into a pool | „{id}” zadeklarowano poza basenem, a dokument ma baseny; przenieś go do basenu |
| R032 | error | boundary event '{id}' needs on=ACTIVITY | zdarzenie graniczne „{id}” wymaga on=CZYNNOŚĆ |
| R040 | error | '{keyword}' is not available at level {level} | „{keyword}” nie jest dostępne na poziomie {level} |
| R050 | error | ad-hoc sub-process '{id}' must contain at least one activity and no start or end events | podproces ad hoc „{id}” musi zawierać co najmniej jedną czynność i żadnych zdarzeń początkowych ani końcowych |
| R051 | error | compensation handler without the flag `compensation`, or with sequence flows | procedura kompensacji bez flagi `compensation` albo z przepływami sekwencji |
| R052 | error | events= is not allowed on the root-level start event '{id}' | events= nie może wystąpić na zdarzeniu początkowym procesu „{id}” |
| R053 | error | malformed {key}="…" list: '{item}' (expected {expected}) | niepoprawna lista {key}="…": „{item}” (oczekiwano: {expected_pl}) |
| R054 | error | `collection=` without `parallel`/`sequential`; `output=` without `output-item=` or vice versa | `collection=` bez `parallel`/`sequential`; `output=` bez `output-item=` albo odwrotnie |
| R055 | warning | '{key}' belongs to the executable layer and is ignored because the document is not executable | „{key}” należy do warstwy wykonywalnej i jest pomijany, bo dokument nie jest wykonywalny |

Language 0.5 (spec §5–§7). `R060`–`R063` and `R065` carry a detail text; the table gives its forms.

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| R060 | error | cancel end event '{id}' must be inside a transaction · cancel boundary event '{id}' must be attached to a transaction · cancel boundary event '{id}' is always interrupting · '{id}': cancel is allowed only on end events inside a transaction and on boundary events of a transaction · method="{m}" of transaction '{id}' must be compensate, store or image | zdarzenie końcowe anulowania „{id}” musi leżeć wewnątrz transakcji · zdarzenie graniczne anulowania „{id}” musi być przypięte do transakcji · zdarzenie graniczne anulowania „{id}” zawsze przerywa czynność · „{id}”: cancel może wystąpić tylko w zdarzeniu końcowym wewnątrz transakcji i w zdarzeniu granicznym transakcji · method="{m}" transakcji „{id}” musi mieć wartość compensate, store albo image |
| R061 | error | {keys} on '{id}' requires the flag loop · loop-max of '{id}' must be a whole number, not '{value}' | {keys} w „{id}” wymaga flagi loop · loop-max w „{id}” musi być liczbą całkowitą, a nie „{value}” |
| R062 | error | '{id}': {triggers} cannot be combined with other triggers · parallel on '{id}' requires a catching event (start, catch, boundary) · parallel on '{id}' requires two or more triggers · '{key}' is given more than once on '{id}'; only events have several triggers | „{id}”: {triggers} nie może łączyć się z innymi wyzwalaczami · parallel w „{id}” wymaga zdarzenia przechwytującego (start, catch, boundary) · parallel w „{id}” wymaga co najmniej dwóch wyzwalaczy · „{key}” podano kilka razy w „{id}”; kilka wyzwalaczy może mieć tylko zdarzenie |
| R063 | error | instantiating '{id}' cannot have incoming sequence flows · instantiating '{id}' must be in a process, not in a sub-process · instantiate on '{id}' is allowed only on receive tasks and event-based gateways · parallel event-based gateway '{id}' requires the flag instantiate | „{id}” rozpoczynające proces nie może mieć przychodzących przepływów sekwencji · „{id}” rozpoczynające proces musi leżeć w procesie, a nie w podprocesie · instantiate w „{id}” może wystąpić tylko w zadaniu odbioru i w bramce opartej na zdarzeniach · równoległa bramka oparta na zdarzeniach „{id}” wymaga flagi instantiate |
| R064 | warning | activity '{activity}' reads and writes data object '{data}' in the same state '{state}'; a state transition writes another state | czynność „{activity}” odczytuje i zapisuje obiekt danych „{data}” w tym samym stanie „{state}”; przejście stanu zapisuje inny stan |
| R065 | error | call activity '{id}' has both process= and task=; it calls one of them · kind="{kind}" of global task '{id}' must be task, user, manual, script or rule | czynność wywołania „{id}” ma jednocześnie process= i task=; wywołuje jedno z nich · kind="{kind}" zadania globalnego „{id}” musi mieć wartość task, user, manual, script albo rule |
| R066 | error | unknown {what} '{id}'; declare it with '{keyword}' (type= → item, task= → global, operation= → operation) | nieznany element ({what_pl}): „{id}”; zadeklaruj go słowem „{keyword}” |

## Decision tables (D)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| D001 | error | decision '{id}' has no header row | decyzja „{id}” nie ma wiersza nagłówka |
| D002 | error | rule has {got} cells, the header has {expected} | reguła ma {got} komórek, a nagłówek {expected} |
| D003 | error | unknown hit policy '{hit}' | nieznana polityka trafień „{hit}” |
| D004 | error | header cell '{cell}' must be 'name: type' with type string, number, boolean or date | komórka nagłówka „{cell}” musi mieć postać „nazwa: typ” (string, number, boolean, date) |
| D005 | warning | decision '{id}' has no rules | decyzja „{id}” nie ma reguł |
| D010 | warning | rules {a} and {b} can both match {example} (hit policy unique) | reguły {a} i {b} mogą pasować jednocześnie, np. {example} (polityka unique) |

## Forms (M)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| M001 | error | unknown field type '{type}' | nieznany typ pola „{type}” |
| M002 | error | field '{id}' of type {type} needs options="value:Label, …" | pole „{id}” typu {type} wymaga options="wartość:Etykieta, …" |

## FEEL (F)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| F001 | warning | FEEL expression outside the supported subset: {detail} | wyrażenie FEEL spoza obsługiwanego podzbioru: {detail_pl} |
| F002 | error | FEEL syntax error: {detail} | błąd składni FEEL: {detail_pl} |

## Simulation (S)

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| S001 | error | invalid distribution '{text}': {detail} | niepoprawny rozkład „{text}”: {detail} |
| S002 | warning | probabilities of the flows leaving '{id}' sum to {total}, not 1 | prawdopodobieństwa przepływów z „{id}” sumują się do {total}, a nie do 1 |
| S003 | warning | gateway '{id}' has neither conditions nor probabilities; the simulator takes the first flow | bramka „{id}” nie ma ani warunków, ani prawdopodobieństw; symulator wybierze pierwszy przepływ |

## Executable profile (C)

Checked when the header or a pool has the flag `executable`. The codes are the same as in the `--camunda` mode of the `bpmn.py` validator.

| Code | Severity | Meaning |
|---|---|---|
| C011 | note | user task without `form=` |
| C020 | error | service or send task without `type=` (job type for a worker) |
| C030 | error | rule task without `decision=` |
| C040 | error | script task without `expr=` |
| C052 | error | message catch event, message boundary event or receive task without `correlation=` |
| C070 | error | flow leaving an `xor`/`or` gateway with several outgoing flows has neither `when=` nor `default` |
| C022 | error | message throw or end event without `type=` (Camunda executes it as a job) |
| C090 | error | static `due=`/`follow-up=` that is not an ISO 8601 date-time with a zone |
| C091 | error | static `priority=` outside the integers 0–100 |

Elements of language 0.5 that Camunda 8.10 does not execute (spec §10.4); reported only with `target=camunda` (the default).

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| C100 | error | transaction '{id}' is not executed by Camunda 8.10 | transakcja „{id}” nie jest wykonywana przez Camunda 8.10 |
| C101 | error | cancel event '{id}' is not executed by Camunda 8.10 | zdarzenie anulowania „{id}” nie jest wykonywane przez Camunda 8.10 |
| C102 | error | complex gateway '{id}' is not executed by Camunda 8.10 | bramka złożona „{id}” nie jest wykonywana przez Camunda 8.10 |
| C103 | error | standard loop of '{id}' (while=, test-before, loop-max) is not executed by Camunda 8.10; use multi-instance | pętla standardowa „{id}” (while=, test-before, loop-max) nie jest wykonywana przez Camunda 8.10; użyj wielu instancji |
| C104 | error | multiple event '{id}' (several triggers) is not executed by Camunda 8.10 | zdarzenie wielokrotne „{id}” (kilka wyzwalaczy) nie jest wykonywane przez Camunda 8.10 |
| C105 | error | parallel multiple event '{id}' is not executed by Camunda 8.10 | równoległe zdarzenie wielokrotne „{id}” nie jest wykonywane przez Camunda 8.10 |
| C106 | error | instantiating receive task '{id}' is not executed by Camunda 8.10; use a message start event | zadanie odbioru „{id}” rozpoczynające proces nie jest wykonywane przez Camunda 8.10; użyj zdarzenia początkowego wiadomości |
| C107 | error | instantiating event-based gateway '{id}' is not executed by Camunda 8.10 | bramka oparta na zdarzeniach „{id}” rozpoczynająca proces nie jest wykonywana przez Camunda 8.10 |
| C108 | error | call activity '{id}' calls the global task '{task}', which Camunda 8.10 does not execute | czynność wywołania „{id}” wywołuje zadanie globalne „{task}”, którego Camunda 8.10 nie wykonuje |
| C109 | warning | operation '{operation}' of '{id}' is ignored by Camunda 8.10 (workers take jobs by type=) | operacja „{operation}” w „{id}” jest pomijana przez Camunda 8.10 (workery pobierają zadania według type=) |
| C110 | warning | correlation key '{id}' is ignored by Camunda 8.10 (messages are correlated by correlation=) | klucz korelacji „{id}” jest pomijany przez Camunda 8.10 (wiadomości koreluje correlation=) |

## BPMN 2.0 rules (E, W, L)

After the tok-level checks pass, the document is exported to BPMN 2.0 and checked by the BPMN validator shared with the course tool `bpmn.py`: errors `E…` (the model breaks a rule of the BPMN specification, e.g. `E008` message flow inside one pool), warnings `W…` (probably a modelling mistake, e.g. `W051` a split joined by a gateway of another type) and style notes `L…` (e.g. `L020` a splitting gateway whose name is not a question). Their messages are in Polish; the list with fixes is in the `bpmn-modelowanie` skill reference (section C). `bpmn-rules.md` lists every BPMN 2.0.2 rule the checker enforces with its clause of the specification and its code.
