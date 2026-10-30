# Diagnostics

Every diagnostic has a stable code, a severity and a line number. `tok check --lang pl` prints the Polish text; `--json` gives `code`, `severity`, `line`, `id` and `message` for programs, and the smallest BPMN conformance class of the document as `"bpmn_class"` (spec §13).

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
| P005 | error | the document must start with a header such as 'tok 1.0' | dokument musi zaczynać się nagłówkiem, na przykład „tok 1.0” |
| P006 | warning | unknown language version {version}; this tool reads versions 0.1, 0.3, 0.5, 0.6, 0.7 and 1.0 | nieznana wersja języka {version}; to narzędzie czyta wersje 0.1, 0.3, 0.5, 0.6, 0.7 i 1.0 |
| P007 | error | malformed attribute near '{near}' | niepoprawny atrybut w pobliżu „{near}” |
| P008 | error | malformed {keyword} line: {detail} | niepoprawny wiersz {keyword}: {detail} |
| P009 | error | malformed decision table row: {detail} | niepoprawny wiersz tabeli decyzyjnej: {detail} |
| P010 | error | a second header | drugi nagłówek dokumentu |

The implementation reads blocks nested at most 64 levels deep (a document of modelling tools nests a few); a deeper block is `P008` with the detail `its block is nested more than 64 levels deep`, and its content is skipped.

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

Language 0.6 (spec §5.5, §5.7, §6.4, §13). `R070`, `R072` and `R073` carry a detail text; the table gives its forms.

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| R070 | error | '{id}' must link at least two different pools (pools=) · pool '{p}' of '{id}' is not linked by the sub-conversation '{sub}' · map= of '{id}': '{outer}' is not one of its pools · map= of '{id}': '{inner}' is not a participant of the called element | „{id}” musi łączyć co najmniej dwa różne baseny (pools=) · basen „{p}” w „{id}” nie należy do podkonwersacji „{sub}” · map= w „{id}”: „{outer}” nie należy do jego basenów (pools=) · map= w „{id}”: „{inner}” nie jest uczestnikiem wywoływanego elementu |
| R071 | error | message flow '{flow}' named by '{id}' does not connect two of its pools | przepływ komunikatów „{flow}” wskazany w „{id}” nie łączy dwóch basenów tej konwersacji |
| R072 | error | choreography task '{id}' needs from= and to=, two different pools · '{id}' needs pools= with at least two pools and from= one of them · pool '{p}' of '{id}' is not a participant of the sub-choreography '{sub}' · call choreography '{id}' needs choreography=ID · map= of '{id}': '{outer}' is not one of its pools · map= of '{id}': '{inner}' is not a participant of the called element | zadanie choreografii „{id}” wymaga from= i to=, dwóch różnych basenów · „{id}” wymaga pools= z co najmniej dwoma basenami i from= wskazującego jeden z nich · basen „{p}” w „{id}” nie jest uczestnikiem podchoreografii „{sub}” · wywołanie choreografii „{id}” wymaga choreography=ID · map= w „{id}”: „{outer}” nie należy do jego basenów (pools=) · map= w „{id}”: „{inner}” nie jest uczestnikiem wywoływanego elementu |
| R073 | error | '{id}' ({kind}) cannot stand in a choreography; a choreography contains events, gateways and choreography activities · '{id}': the trigger {triggers} is not allowed on {kind} events in a choreography · '{id}': {kind} events in a choreography need a timer, conditional or signal trigger · choreography '{id}' has no flow nodes | „{id}” ({kind}) nie może wystąpić w choreografii; choreografia zawiera zdarzenia, bramki i czynności choreografii · „{id}”: wyzwalacz {triggers} nie może wystąpić w zdarzeniach {kind} w choreografii · „{id}”: zdarzenia {kind} w choreografii wymagają wyzwalacza czasowego, warunkowego albo sygnału · choreografia „{id}” nie zawiera elementów przepływu |
| R074 | error | participant multiplicity of pool '{id}' needs whole numbers with 0 <= min <= max and max >= 1 (min is 2 when only max is given, max is 1 when only min is given) | krotność uczestnika w basenie „{id}” wymaga liczb całkowitych, dla których 0 <= min <= max i max >= 1 (gdy podano tylko max, min wynosi 2; gdy podano tylko min, max wynosi 1) |
| R075 | error | '{id}' ({what}) is outside the BPMN conformance class {class}; it belongs to the class {needed} (only with `tok check --class`) | „{id}” ({what}) wykracza poza klasę zgodności BPMN {class}; należy do klasy {needed} |

Language 0.7 (spec §9). `R077`, `R078` and `R081` carry a detail text; the table gives its forms. References of `requires=` and `invoke=` to unknown elements or elements of a kind that cannot be required are `R002` and `R003`.

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| R076 | error | '{id}' requires itself through {path}; the requirements of a decision requirements graph form no cycle | „{id}” wymaga samego siebie przez {path}; wymagania w diagramie wymagań decyzyjnych nie mogą tworzyć cyklu |
| R077 | error | '{what}' has more than one form of logic (expr=, invoke=, a table, entries or elements) · with= on '{what}' requires invoke= · hit=, aggregation= and default= on '{what}' need a decision table · the block of '{what}' mixes element lines with entry and result lines · 'result' must be the last line of the context of '{what}', and only one | „{what}” ma więcej niż jedną postać logiki (expr=, invoke=, tabela, wpisy albo elementy) · with= w „{what}” wymaga invoke= · hit=, aggregation= i default= w „{what}” wymagają tabeli decyzyjnej · blok „{what}” łączy wiersze element z wierszami entry i result · „result” musi być ostatnim i jedynym takim wierszem kontekstu „{what}” |
| R078 | error | malformed {key}="…" item '{item}' on '{id}' · with= names '{param}', which is not a parameter of '{bkm}' | niepoprawny element listy {key}="…": „{item}” w „{id}” · with= wskazuje „{param}”, który nie jest parametrem „{bkm}” |
| R079 | warning | '{id}' uses '{name}' ({kind} '{other}') without requiring it; add it to requires= | „{id}” używa „{name}” ({kind_pl} „{other}”), choć nie wymaga tego elementu; dodaj go do requires= |
| R080 | error | the name '{name}' is used twice ('{a}' and '{b}'); the names of a decision requirements graph, of the entries of one context and of the parameters of one function are unique | nazwa „{name}” występuje dwa razy („{a}” i „{b}”); nazwy w diagramie wymagań decyzyjnych, wpisów jednego kontekstu i parametrów jednej funkcji muszą być różne |
| R081 | error | unknown type '{type}' (a FEEL type or an item of the document) · item '{id}' is both a BPMN item (structure=, kind=) and a DMN type (type=, allowed=, components) | nieznany typ „{type}” (typ FEEL albo item dokumentu) · item „{id}” jest jednocześnie typem BPMN (structure=, kind=) i typem DMN (type=, allowed=, składowe) |

Language 1.0 (spec §4.2). Deprecation warnings: forms that language 1.0 keeps, with the meaning they always had, but that a later major version may refuse. They are reported only in documents with the header `tok 1.0`, so documents of earlier versions keep every diagnostic they had.

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| R090 | warning | '{key}' is given more than once on this line; only the last value counts (deprecated in language 1.0) | „{key}” występuje w tym wierszu więcej niż raz; liczy się tylko ostatnia wartość (przestarzałe w języku 1.0) |

`R090` is reported for an attribute key or a flag written twice or more on one line (`doc="a" doc="b"`, `loop loop`): the language keeps the last value of a key and one copy of a flag, and `tok fmt` prints only those, so formatting the document removes the warning. The keys that may repeat are not reported: the trigger keys `message=`, `timer=`, `signal=`, `error=`, `escalation=`, `condition=` and `link=` on flow nodes, and `error=` on an `operation` (spec §3, §5.1.1). On an event and an operation every value of such a key counts; on another flow node the repetition is `R062`, as in language 0.5.

## Decision tables and relations (D)

`D006`–`D009` are reported in documents of language 0.7, where `D003` also accepts the hit policies `rule-order` and `output-order` and header cells are checked as `D009` instead of `D004`. `D007` carries a detail text; the table gives its forms.

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| D001 | error | decision '{id}' has no header row | decyzja „{id}” nie ma wiersza nagłówka |
| D002 | error | rule has {got} cells, the header has {expected} | reguła ma {got} komórek, a nagłówek {expected} |
| D003 | error | unknown hit policy '{hit}' | nieznana polityka trafień „{hit}” |
| D004 | error | header cell '{cell}' must be 'name: type' with type string, number, boolean or date | komórka nagłówka „{cell}” musi mieć postać „nazwa: typ” (string, number, boolean, date) |
| D005 | warning | decision '{id}' has no rules | decyzja „{id}” nie ma reguł |
| D006 | error | hit policy {hit} orders the matching rules by output values; give them for at least one output (name: type = …) | polityka trafień {hit} porządkuje pasujące reguły według wartości wyników; podaj je co najmniej dla jednego wyniku (nazwa: typ = …) |
| D007 | error | aggregation={a} requires hit=collect · unknown aggregation '{a}' (sum, count, min, max) · aggregation={a} needs a table with one output | aggregation={a} wymaga hit=collect · nieznana agregacja „{a}” (sum, count, min, max) · aggregation={a} wymaga tabeli z jednym wynikiem |
| D008 | error | row has {got} cells, the relation has {expected} columns | wiersz ma {got} komórek, a relacja ma {expected} kolumn |
| D009 | error | header cell '{cell}' must be 'name: type' or 'name: type = values' (language 0.7) | komórka nagłówka „{cell}” musi mieć postać „nazwa: typ” albo „nazwa: typ = wartości” (język 0.7) |
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

In documents of language 0.7 every expression is full FEEL (`feel.md` §9): `F002` reports syntax errors, and `F001` only externally defined functions, which tok does not evaluate.

An expression nested more than 160 levels deep (parentheses, lists, contexts, calls and the other nested parts, and each operator of a chain such as `a + b + c`, count one level) is `F002` with the detail `the expression is nested too deeply` (Polish: `wyrażenie jest zbyt głęboko zagnieżdżone`).

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

The executable layer (spec §10.3) adds:

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| C022 | error | message {kind} event '{id}' needs type="…" (Camunda executes it as a job) | zdarzenie {kind_pl} wiadomości „{id}” wymaga type="…" (Camunda wykonuje je jako zadanie dla workera) |
| C090 | error | {key}="{value}" must be an ISO 8601 date-time with a zone (e.g. 2027-01-15T12:00:00+01:00) or an expression starting with = | {key}="{value}" musi być datą i godziną ISO 8601 ze strefą (np. 2027-01-15T12:00:00+01:00) albo wyrażeniem zaczynającym się od = |
| C091 | error | priority must be an integer from 0 to 100 or an expression starting with =, not '{value}' | priority musi być liczbą całkowitą od 0 do 100 albo wyrażeniem zaczynającym się od =, a nie „{value}” |

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

Elements of language 0.6 in executable documents for Camunda (spec §10.4); reported only with `target=camunda` (the default).

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| C120 | warning | conversation '{id}' is ignored by Camunda 8.10 (conversations describe the collaboration, not its execution) | konwersacja „{id}” jest pomijana przez Camunda 8.10 (konwersacje opisują współpracę, a nie jej wykonanie) |
| C121 | warning | call conversation '{id}' is ignored by Camunda 8.10 | wywołanie konwersacji „{id}” jest pomijane przez Camunda 8.10 |
| C122 | error | choreography '{id}' is not executed by Camunda 8.10 | choreografia „{id}” nie jest wykonywana przez Camunda 8.10 |
| C123 | warning | participant multiplicity of pool '{id}' is ignored by Camunda 8.10 | krotność uczestnika w basenie „{id}” jest pomijana przez Camunda 8.10 |

Decisions of language 0.7 in executable documents for Camunda (spec §10.4); reported only with `target=camunda` (the default).

| Code | Severity | Message (English) | Message (Polish) |
|---|---|---|---|
| C130 | error | business knowledge model '{id}' whose logic is an invocation is not executed by Camunda 8.10 | model wiedzy biznesowej „{id}”, którego logiką jest wywołanie, nie jest wykonywany przez Camunda 8.10 |
| C131 | error | a function definition in '{id}' whose body is not a literal expression is not executed by Camunda 8.10 | definicja funkcji w „{id}”, której treścią nie jest wyrażenie literalne, nie jest wykonywana przez Camunda 8.10 |
| C132 | error | decision '{id}' has no decision logic; Camunda 8.10 does not deploy it | decyzja „{id}” nie ma logiki decyzyjnej; Camunda 8.10 jej nie wdroży |

## BPMN 2.0 rules (E, W, L)

After the tok-level checks pass, the document is exported to BPMN 2.0 and checked by the BPMN validator shared with the course tool `bpmn.py`: errors `E…` (the model breaks a rule of the BPMN specification, e.g. `E008` message flow inside one pool), warnings `W…` (probably a modelling mistake, e.g. `W051` a split joined by a gateway of another type) and style notes `L…` (e.g. `L020` a splitting gateway whose name is not a question). The validator writes its messages in Polish, with the identifiers of the elements (`--lang pl`); in English `tok check` gives the text below followed by the identifier. `bpmn-rules.md` gives the clause of BPMN 2.0.2 behind every code.

| Code | Severity | Message (English) |
|---|---|---|
| E001 | error | the file is not well-formed XML |
| E002 | error | the root element is not bpmn:definitions |
| E003 | error | an id is used more than once |
| E004 | error | reference to a non-existent id |
| E005 | error | sequence flow without both ends or connecting data, a pool or an annotation |
| E006 | error | sequence flow between pools or into a sub-process |
| E007 | error | message flow from an element outside the pools |
| E008 | error | message flow inside one pool (use a sequence flow between lanes) |
| E009 | error | message flow to a gateway, data or a non-message event |
| E010 | error | trigger not allowed for this event position |
| E011 | error | incoming flow to a start event or outgoing flow from an end event |
| E012 | error | invalid boundary event (on a gateway, with incoming flow, non-interrupting error, …) |
| E013 | error | event sub-process with sequence flows or without a triggered start event |
| E014 | error | compensation activity with sequence flows |
| E015 | error | default flow that does not leave the element or has a condition |
| E016 | error | event-based gateway followed by something other than catch events or receive tasks |
| E017 | error | condition or default flow on a parallel gateway |
| E018 | error | reference to a declaration of the wrong type |
| E030 | error | element in two lanes |
| W010 | warning | timer event without a time |
| W016 | warning | event after an event-based gateway has other incoming flows |
| W020 | warning | gateway with one incoming and one outgoing flow is unnecessary |
| W021 | warning | branch of an XOR/OR split without label, condition or default status |
| W022 | warning | branch without a condition at an element that splits conditionally (it has a default flow or other branches have conditions) |
| W023 | warning | sequence flow repeats another one (same source, target and condition) |
| W030 | warning | lane refers to an element inside a sub-process |
| W031 | warning | element outside every lane |
| W040 | warning | process without a start or end event |
| W041 | warning | element not reachable from a start event |
| W042 | warning | path without an end event (element without outgoing flow) |
| W043 | warning | sequence flow from an element back to itself |
| W044 | warning | activity or event with several outgoing flows without conditions (implicit split; all of them are taken) |
| W045 | warning | sub-process without a start event |
| W046 | warning | sub-process without an end event |
| W047 | warning | event with several triggers (multiple event), reported as an error by bpmn.io linters |
| W050 | warning | OR split without a default flow |
| W051 | warning | join of a different type than the split (e.g. AND split joined by XOR) |
| W052 | warning | boundary escalation/error on a sub-process that nothing inside throws |
| W053 | warning | escalation thrown but never caught |
| W054 | warning | message task or event without a message flow in a model with several pools |
| L010 | note | message event without messageRef |
| L020 | note | name of a splitting gateway is not a question |
| L021 | note | gateway both joins and splits |
| L030 | note | task without a name |

## BPMN files (C, D, K)

`tok check FILE.bpmn` reports the BPMN 2.0 rules of the previous section, the diagram of the file (D) and, with its options, the step numbers (K: `--steps`, `--steps-from`) and the Camunda 8 profile of executable models (C: `--camunda`, the same codes as the executable profile of tok documents above). Their messages name the element; the English texts are:

| Code | Severity | Meaning |
|---|---|---|
| C001 | warning | no modeler:executionPlatform="Camunda Cloud" on <bpmn:definitions>; Camunda Modeler opens the file as a Camunda 7 model |
| C002 | error | no process has isExecutable="true"; the engine will not deploy the model |
| C010 | warning | user task without <zeebe:userTask/>; it does not reach Tasklist as a Camunda user task |
| C011 | note | user task without a form (zeebe:formDefinition); in Tasklist it can only be completed without data |
| C012 | warning | the form of the user task is in no .form file of the model's folder |
| C020 | error | service or send task without <zeebe:taskDefinition type="…"/>; the deployment fails |
| C021 | note | service or send task waits for a worker of its job type; without one the instance stops there |
| C030 | error | business rule task without <zeebe:calledDecision decisionId="…" resultVariable="…"/> |
| C031 | error | zeebe:calledDecision without decisionId or resultVariable |
| C032 | warning | the decision of the business rule task is in no .dmn file of the model's folder |
| C040 | error | script task without <zeebe:script expression="=…" resultVariable="…"/> |
| C041 | error | zeebe:script: the FEEL expression must start with '=' and resultVariable is required |
| C050 | error | message event or task without messageRef |
| C051 | error | message without a name; the engine correlates messages by name |
| C052 | error | message caught by an event or receive task without <zeebe:subscription correlationKey="=…"/> |
| C060 | warning | element not executed by Camunda 8 |
| C070 | error | flow leaving a gateway without a condition; the engine needs one on every outgoing flow except the default |
| C071 | error | condition of a flow is not a FEEL expression starting with '=' (e.g. =approved = true) |
| C080 | error | timer event without a value (timeDuration, timeDate or timeCycle) |
| D01 | warning | diagram coordinates of an element that is no longer in the model; run tok bpmn on the file to lay it out again |
| D02 | warning | elements without diagram coordinates (BPMNDI); run tok bpmn on the file to lay it out |
| K01 | warning | name does not start with the step number of the process description (e.g. '12. Request for quotation') |
| K02 | warning | a step of the process description has no element in the model |
| K03 | warning | an element carries a step number that is not in the process description |

