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
| R012 | error | an event can have only one trigger ({triggers}) | zdarzenie może mieć tylko jeden wyzwalacz ({triggers}) |
| R013 | error | '{key}' requires {requirement} | „{key}” wymaga: {requirement} |
| R020 | error | a flow line with a label or attributes must contain exactly one arrow | wiersz flow z etykietą lub atrybutami musi mieć dokładnie jedną strzałkę |
| R021 | error | sequence flow {source} -> {target} crosses a process or sub-process boundary | przepływ sekwencji {source} -> {target} przekracza granicę procesu lub podprocesu |
| R022 | error | message flow {source} -> {target} must connect two different pools | przepływ komunikatów {source} -> {target} musi łączyć dwa różne baseny |
| R030 | error | '{id}' is in a pool with lanes but outside every lane | „{id}” leży w basenie z liniami, ale poza liniami |
| R031 | error | '{id}' is declared at top level, but the document has pools; move it into a pool | „{id}” zadeklarowano poza basenem, a dokument ma baseny; przenieś go do basenu |
| R032 | error | boundary event '{id}' needs on=ACTIVITY | zdarzenie graniczne „{id}” wymaga on=CZYNNOŚĆ |
| R040 | error | '{keyword}' is not available at level {level} | „{keyword}” nie jest dostępne na poziomie {level} |

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

## BPMN 2.0 rules (E, W, L)

After the tok-level checks pass, the document is exported to BPMN 2.0 and checked by the BPMN validator shared with the course tool `bpmn.py`: errors `E…` (the model breaks a rule of the BPMN specification, e.g. `E008` message flow inside one pool), warnings `W…` (probably a modelling mistake, e.g. `W051` a split joined by a gateway of another type) and style notes `L…` (e.g. `L020` a splitting gateway whose name is not a question). Their messages are in Polish; the list with fixes is in the `bpmn-modelowanie` skill reference (section C).
