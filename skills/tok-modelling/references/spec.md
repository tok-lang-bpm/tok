# tok — Language Specification, version 0.5

tok is a plain-text language for information-process models. A tok document describes a BPMN 2.0 process (pools, lanes, events, activities, gateways, flows, data), the DMN decision tables it calls, the forms people fill in, and the parameters needed to simulate it. Every tool built on tok reads the same text and produces the same projections: BPMN 2.0 XML with diagram layout, SVG, DMN XML, Camunda form JSON and simulation logs.

This document is normative. The reference implementation is the Rust engine in `rust/` (the `tok` command and the Python and R packages run it); the pure-Python implementation in `python/` is frozen at language 0.3, and the Julia package in `julia/` implements the core (parsing, formatting, core diagnostics, BPMN export) against the same conformance corpus.

Language 0.5 adds the elements of BPMN 2.0.2 process modelling that Camunda 8 does not execute: transactions and cancel events (§5.3), the complex gateway (§5.4), the attributes of the standard loop (§5.2.1), multiple and parallel multiple events (§5.1.1), instantiating receive tasks and event-based gateways (§5.2, §5.4), process data (§7.1), resources and performers (§7.2), global tasks, interfaces and operations (§5.6) and correlation keys (§6.3). Every BPMN 2.0.2 rule the checker enforces is listed with its clause in `bpmn-rules.md`.

## 1. Design goals

| # | Goal | Consequence |
|---|---|---|
| G1 | **Simple to read** for people who have never seen a grammar. | Every line starts with a keyword. Labels are quoted text in any language. |
| G2 | **Simple to implement** in any language with strings and lists. | A line-based tokenizer with six token types; no backtracking; one statement per line; blocks by indentation. |
| G3 | **Simple to check.** | Every element has an explicit identifier. Errors point to one line. Diagnostics have stable codes. |
| G4 | **One canonical form.** | `tok fmt` prints any valid document in exactly one way, so diffs and examples stay stable. |
| G5 | **The text is the model.** | XML, SVG, forms and simulation results are projections. No coordinates appear in the language. |
| G6 | **Layers stay apart.** | Structure (BPMN), decisions (DMN), forms, execution details and simulation parameters are separate statements or attributes; a tool may ignore any layer it does not need. |

## 2. Lexical structure

A document is UTF-8 text. It is read line by line; a line ends at `\n`, `\r\n` or `\r`. A byte order mark at the start is ignored.

- **Comments** start with `#` outside a string and run to the end of the line.
- **Blank lines** and comment-only lines are ignored.
- **Indentation** is made of spaces. Tabs in indentation are an error (`T002`). A line indented deeper than the previous statement line belongs to that statement's block. A block ends at the first line indented at or less than its parent.
- **Line continuation** does not exist. One statement is one line.

Tokens:

| Token | Form | Examples |
|---|---|---|
| `WORD` | a letter or `_`, then letters, digits, `_`, `-`, `.`, `:`, `/`, `+` | `user`, `check-policy`, `P14D`, `PT2M`, `bpmn:task` |
| `NUMBER` | optional `-`, digits, optional `.` and digits | `3`, `0.25`, `-1` |
| `STRING` | text in double quotes; `\"`, `\\` and `\n` (line break) are escapes | `"Sprawdzenie katalogów"` |
| `ARROW` | `->` | |
| `EQUALS` | `=` (only between a key and a value) | |
| `ROW` | a whole line whose first non-space character is `\|` | decision table rows (§9) |

A `WORD` immediately followed by `=` is an attribute key. Keywords are lowercase and case-sensitive.

**Identifiers** (`ID`) are `WORD`s that match `[A-Za-z_][A-Za-z0-9_-]*`. They must be unique in the document (`R001`). Identifiers become XML `id` attributes, so they are ASCII. Labels carry the human-readable text.

## 3. Attributes

Most statements end with attributes:

```
key=value      value is a WORD, NUMBER or STRING
flag           a bare WORD after the identifier and label: a boolean set to true
```

Attributes may appear in any order; the formatter prints them in the order of the attribute tables below. An unknown attribute key is a warning (`R010`); an unknown flag is an error (`R011`).

**Documentation** (language 0.5). `doc="Text"` gives the BPMN `documentation` of the element a statement exports: flow nodes, flows, message flows, pools (the participant), lanes, `data`, `store`, `input`, `output`, `property`, notes, groups, `global`, `interface` and `operation`; on the header it documents the process of a document without pools and the definitions of a document with pools. It is plain text (`\n` for a line break) and allowed at every level. The label of an `item` is already its documentation.

A key is normally given once; when it is repeated, the last value counts. In a document of language 0.5 the trigger keys of events (`message`, `timer`, `signal`, `error`, `escalation`, `condition`, `link`, §5.1.1) and `error` on an `operation` (§5.6) may repeat: every value counts, in written order, and the formatter prints the values of one key together.

## 4. Document structure

The first statement is the header:

```
tok 0.5 "Interlibrary loan" executable
```

| Part | Meaning |
|---|---|
| `tok 0.5` | language version (required). Accepted versions: `0.1`, `0.3` and `0.5`; a document written for an earlier version means the same under 0.5. The language version follows the minor version of the release that changed the language. Two things depend on the declared version, so that older documents keep their meaning: only a 0.5 document may give an event several triggers (in 0.1 and 0.3 two triggers are `R012` and a repeated key keeps its last value), and only in a 0.5 document do `resource` and `by=` also describe BPMN resources and performers (§7.2). Everything else of 0.5 may be used under any header; a document imported from BPMN that uses it is printed as `tok 0.5`. |
| label | document title; becomes `definitions/@name` and the default process name |
| `id=` | identifier of the definitions element (default: `definitions`) |
| `executable` | flag: every process in the document is executable (`isExecutable="true"`) |
| `target=camunda` | the execution platform for export; see §10 |
| `version-tag="v1"` | version tag of the process (executable layer, §10.3) |
| `process-name="Name"` | name of the process of a document without pools, when it differs from the title (language 0.5; `""` for a process without a name); a pool names its own process (§5.5) |
| `doc="Text"` | documentation (§3) |

After the header come top-level statements in any order:

- **flow nodes** (§5), **flows** (§6), **data** (§7) and **artifacts** (§8) — in a document without pools these form one process whose identifier is `process` (override with `process=ID` in the header);
- **pools** with their lanes and flow nodes (§5.5);
- **declarations** of language 0.5: item definitions (§7.1), global tasks and interfaces (§5.6), correlation keys (§6.3);
- **decisions** (§9), **forms** (§10.2), **simulation statements** (§11).

References may point forward: a flow may name a node declared later.

## 5. Flow nodes

Every flow node is declared on one line:

```
KIND ID ["Label"] [attributes]
```

### 5.1 Events

| Kind | BPMN element | Notes |
|---|---|---|
| `start` | `startEvent` | |
| `end` | `endEvent` | |
| `catch` | `intermediateCatchEvent` | needs a trigger |
| `throw` | `intermediateThrowEvent` | none or a trigger |
| `boundary` | `boundaryEvent` | needs `on=ID` (the activity) and a trigger |

Trigger attributes (one per event; several make a multiple event, §5.1.1):

| Attribute | Trigger | Allowed on | Combines with others |
|---|---|---|---|
| `message="Name"` | message | start, end, catch, throw, boundary | yes |
| `timer=VALUE` | timer: ISO 8601 duration (`P14D`, `PT2H`), date-time (`2027-01-15T09:00`), or cycle (`R3/PT1H`, quote it if it contains a space) | start, catch, boundary | yes |
| `signal="Name"` | signal | start, end, catch, throw, boundary | yes |
| `error="Name"` | error | end, boundary, start of an event sub-process | yes |
| `escalation="Name"` | escalation | end, throw, boundary, start of an event sub-process | yes |
| `condition="FEEL"` | conditional | start, catch, boundary | yes |
| `link="Name"` | link | catch, throw | no |
| `terminate` (flag) | terminate | end | no |
| `compensate` (flag) | compensation | throw, end, boundary | no |
| `cancel` (flag) | cancel (language 0.5) | end inside a transaction, boundary of a transaction (§5.3) | no |
| `error`, `escalation` (flags without a name) | any error, any escalation on boundary events and event sub-process starts; elsewhere (language 0.5) an error or escalation without a reference | as the attribute of the same name | no |
| `message`, `timer`, `signal`, `condition` (flags without a value, language 0.5) | a trigger of that kind whose message, time, signal or condition is not given (the event definition has no reference; a conditional one has an empty condition) | as the attribute of the same name | yes |

Other event attributes: `on=ID` (boundary only), `noninterrupting` (flag; boundary events and event sub-process starts), `parallel` (flag; a parallel multiple event, §5.1.1), `correlation="FEEL"` (message catch events in executable documents, §10).

Messages, signals, errors and escalations are declared implicitly by name. Two triggers with the same name refer to the same declaration. `error` and `escalation` accept an optional `code="…"`.

The trigger flags without a value describe a model whose details are not known yet, as BPMN tools draw them; the level `basic` rejects them like the other triggers (`R040`). In executable documents the BPMN rules report them (`C050`, `C080`); in simulation such a trigger waits for the event's `delay=`, like a message.

#### 5.1.1 Multiple and parallel multiple events (language 0.5)

```
start zgloszenie "Zgłoszenie e-mailem lub przez formularz" message="E-mail" message="Formularz"
catch potwierdzenie "Potwierdzenie i płatność" message="Potwierdzenie" signal="Wpłata" parallel
end koniec "Powiadomienia" message="Do klienta" signal="Zamknięto"
```

- An event with two or more triggers is a multiple event. A trigger attribute may repeat, and different trigger attributes may be combined, within the positions of the table above. Each trigger exports one event definition (identifiers `ID-def`, `ID-def-2`, …, in the order of the table and then in written order).
- On catching events (`start`, `catch`, `boundary`) BPMN reads several triggers as "any one of them". The flag `parallel` (`parallelMultiple="true"`) makes all of them necessary; it needs two or more triggers on a catching event (`R062`).
- On throwing events (`throw`, `end`) all triggers are thrown.
- The triggers marked "no" in the last column cannot be combined with other triggers (`R062`).

### 5.2 Activities

| Kind | BPMN element |
|---|---|
| `task` | `task` |
| `user` | `userTask` |
| `service` | `serviceTask` |
| `manual` | `manualTask` |
| `script` | `scriptTask` |
| `rule` | `businessRuleTask` |
| `send` | `sendTask` |
| `receive` | `receiveTask` |
| `call` | `callActivity` |

Activity attributes:

| Attribute | Meaning |
|---|---|
| `loop` (flag) | standard loop (§5.2.1) |
| `while="FEEL"`, `test-before` (flag), `loop-max=N` | condition, test position and maximum of the standard loop (language 0.5, §5.2.1) |
| `parallel` / `sequential` (flag) | multi-instance, parallel or sequential |
| `compensation` (flag) | the activity is a compensation handler |
| `message="Name"` | message of a `send` or `receive` task |
| `instantiate` (flag) | a `receive` task that starts the process (language 0.5, `instantiate="true"`): it has no incoming sequence flows and stands where a start event would (`R063`) |
| `operation=ID` | the operation (§5.6) a `service`, `send` or `receive` task performs (`operationRef`, language 0.5) |
| `process=ID` | called process of a `call` activity |
| `task=ID` | called global task (§5.6) of a `call` activity; `process=` and `task=` are exclusive (`R065`, language 0.5) |
| `form=ID` | form shown in a `user` task (§10.2) |
| `decision=ID` | decision called by a `rule` task (§9) |
| `expr="FEEL"` | expression evaluated by a `script` task |
| `result=NAME` | variable that receives the result of `decision=` or `expr=` |
| `type="job-type"` | job type of a `service` or `send` task in an executable model |
| `by=ID` | resource that performs the activity in simulation (§11) and, in a document of language 0.5, its BPMN performer (§7.2) |
| `duration="DIST"` | processing time in simulation (§11) |
| `assign="NAME=DIST; …"` | variables set when the activity completes in simulation (§11) |

#### 5.2.1 Standard loop

```
service proba "Próba połączenia" loop while="proby < 3" test-before loop-max=5
```

| tok | BPMN `standardLoopCharacteristics` |
|---|---|
| `loop` (flag) | the element itself |
| `while="FEEL"` | `loopCondition`: the activity repeats while it is true |
| `test-before` (flag) | `testBefore="true"`: the condition is evaluated before every iteration (default: after, so the activity runs at least once) |
| `loop-max=N` | `loopMaximum`, a whole number |

`while`, `test-before` and `loop-max` require `loop`, and `loop-max` is a whole number (`R061`).

### 5.3 Sub-processes

```
sub ID ["Label"] [attributes]
  ...flow nodes, flows and data of the sub-process...
eventsub ID ["Label"]
  start ID ["Label"] message="Cancel" noninterrupting
  ...
```

`sub` is an embedded sub-process; its block is a separate scope for sequence flows (§6).

```
adhoc ID ["Label"] [active="EXPR"] [until="EXPR"] [wait] [sequential]
  ...activities (no start or end events)...
```

`adhoc` is an ad-hoc sub-process: its activities are performed in any order and as often as needed, without sequence flows from a start event (flows between inner activities are allowed). It contains at least one activity and no start or end events (`R050`). `active`, `until` and `wait` belong to the executable layer (§10.3); `sequential` sets the BPMN ordering attribute. `eventsub` is an event sub-process: it has no incoming or outgoing sequence flows and its block starts with exactly one start event with a trigger.

**Transactions** (language 0.5):

```
transaction rezerwacja "Rezerwacja pokoju i biletu"
  start s
  service pokoj "Rezerwacja pokoju" compensate=zwolnij-pokoj
  service zwolnij-pokoj "Zwolnienie pokoju" compensation
  xor ok "Czy udało się?"
  end gotowe
  end anuluj "Anulowanie" cancel
  flow s -> pokoj -> ok -> gotowe "tak" when="ok"
  flow ok -> anuluj "nie" default
boundary anulowano "Rezerwacja anulowana" on=rezerwacja cancel
```

| tok | BPMN |
|---|---|
| `transaction ID ["Label"]` + block | `transaction`, a sub-process with the same block rules as `sub` and the attributes of activities |
| `method=compensate\|store\|image` | `method` (`##Compensate`, `##Store`, `##Image`); printed only when given (the BPMN default is compensate) |
| `end … cancel` | end event with `cancelEventDefinition`; only directly inside a transaction (`R060`) |
| `boundary … on=T cancel` | boundary event with `cancelEventDefinition`; `on=` names a transaction; always interrupting (`R060`) |

Compensation inside a transaction uses the syntax of §10.3 (`compensate=`, `compensation`, `throw … compensate`): a cancel end event rolls back the completed activities of the transaction through their compensation handlers, and the process continues from the cancel boundary event.

### 5.4 Gateways

| Kind | BPMN element |
|---|---|
| `xor` | `exclusiveGateway` |
| `and` | `parallelGateway` |
| `or` | `inclusiveGateway` |
| `eventgw` | `eventBasedGateway` |
| `complex` | `complexGateway` (language 0.5) |

A splitting gateway's label should be a question (`"Czy książka dostępna?"`); conditions and labels belong to its outgoing flows (§6).

```
complex wybor "Czy zebrano dość recenzji?" activation="_activationCount >= 2"
eventgw start-gw "Co przyszło najpierw?" instantiate parallel
```

- `complex` with `activation="FEEL"` → `activationCondition`: as a join it fires when the condition holds for the arriving tokens; the variable `_activationCount` is the number of tokens that have arrived. Its outgoing flows take `when=` and `default` like those of an inclusive gateway.
- `eventgw … instantiate` → `instantiate="true"`: the gateway starts the process; with `parallel` → `eventGatewayType="Parallel"`. Such a gateway has no incoming sequence flows, `parallel` requires `instantiate`, and the gateway stands in a process, not in a sub-process (`R063`).

### 5.5 Pools and lanes

```
pool ID ["Label"] [process=ID] [process-name="Name"] [executable]
  lane ID ["Label"]
    ...flow nodes...
  lane ID ["Label"]
    ...
pool ID ["Label"]            # no block: a black-box pool
```

- A pool with a block is a participant together with its process. The process identifier is `process=` or, by default, the pool identifier followed by `-process`. The process is named after the title when the document has one pool with a block, otherwise after the pool label; `process-name="Name"` (language 0.5) names it otherwise (`""`: no name), and is allowed only on a pool with a block (`R013`).
- A pool without a block is a black-box participant (no process).
- A lane's block declares the flow nodes that belong to that lane. Lanes may nest. If a pool has lanes, every top-level flow node of its process must be inside a lane (`R030`).
- If a document contains at least one pool, flow nodes may not appear at top level (`R031`).

### 5.6 Global tasks, interfaces and operations (language 0.5)

```
global sprawdzenie "Sprawdzenie tożsamości" kind=user
call sprawdz "Sprawdzenie tożsamości" task=sprawdzenie

interface katalog "Katalog biblioteczny"
  operation szukaj "Wyszukanie rekordu" in="Zapytanie" out="Rekord" error="Brak rekordu"
service wyszukaj "Wyszukanie w katalogu" operation=szukaj
```

| Statement | BPMN |
|---|---|
| `global ID ["Label"] [kind=task\|user\|manual\|script\|rule]` (top level) | `globalTask`, `globalUserTask`, `globalManualTask`, `globalScriptTask`, `globalBusinessRuleTask` (default `task`; another value is `R065`) |
| `call … task=ID` | `calledElement` pointing to the global task (`R066` when it is not declared) |
| `interface ID ["Label"]` (top level) with a block of `operation` lines | `interface` (its `name` is the label, or the identifier) |
| `operation ID ["Label"] in="Message" [out="Message"] [error="Error" …]` | `operation` with `inMessageRef`, `outMessageRef` and one `errorRef` per `error=`; `in=` is required (`R013`) |
| `operation=ID` on `service`, `send` and `receive` tasks | `operationRef`; the process that contains the task lists the interface in `supportedInterfaceRef` |

Message and error names are those of the document (§5.1). `global` and `interface` are top-level statements; `operation` lines are allowed only in the block of an `interface` (`P004`).

## 6. Flows

### 6.1 Sequence flows

```
flow A -> B -> C                        # a chain: two flows
flow G -> D "tak" when="zgoda = true"   # one flow with a label and a condition
flow G -> E "nie" default               # default flow of G
```

- A `flow` line contains two or more node identifiers separated by `->`.
- A line with a label or attributes must contain exactly one arrow (`R020`). Label and attributes describe that flow.
- Flow attributes: `when="FEEL"` (condition expression; in a document of language 0.5 `when=""` is a condition without an expression), `default` (flag: the default flow of a splitting `xor`/`or`/`complex` gateway or activity), `p=NUMBER` (branch probability in simulation, §11), `id=ID` (explicit identifier), `doc="Text"` (§3).
- Flow identifiers are derived as `f-SOURCE-TARGET`, with `-2`, `-3`… on repeated pairs, unless `id=` is given.
- Flow lines may appear anywhere: at top level, inside a pool, lane or sub-process block. The container of a flow is determined by its endpoints; both endpoints must be in the same process or sub-process (`R021`).

### 6.2 Message flows

```
message A -> B ["Label"] [message="Name"] [doc="Text"]
```

`A` and `B` are flow nodes or pools in different pools (`R022`). The label names the message flow; `message=` links it to a message declaration (default: the label).

### 6.3 Correlation keys (language 0.5)

```
key zamowienie "Numer zamówienia"
  prop numer message="Zamówienie" path="zamowienie.numer"
  prop numer message="Potwierdzenie" path="potwierdzenie.nr"
```

- `key ID ["Label"]` (top level) → `correlationKey` of the collaboration. BPMN 2.0.2 keeps correlation keys only in collaborations (`tCollaboration`), so a document with `key` needs pools (`R013`).
- Each `prop NAME message="M" path="…"` line belongs to the key above it; both attributes are required (`R013`). Every distinct `NAME` becomes one `correlationProperty` (identifier `CorrelationProperty_NAME`, shared by all keys that use the name), with one `correlationPropertyRetrievalExpression` per line: `messageRef` of the message `M` (§5.1) and `messagePath` with the `path` text.
- `correlation=` of executable documents (§10.1) is the Camunda form of correlation and is independent of `key`.

## 7. Data

```
data ID ["Label"] [state="złożony"] [type=ITEM] [object="Name"] [collection]
store ID ["Label"] [type=ITEM]
reads ACTIVITY DATA [input=ID]
writes ACTIVITY DATA [output=ID]
```

- `data` declares a data object reference (and its data object). Several `data` statements may share a label to show different states of one document; each needs its own identifier.
- `object="Name"` (language 0.5) names the data object of the reference when it differs from the label; the `data` statements with the same object name in one process or sub-process share one data object (`""`: a data object without a name). `object=` stands on `data` only (`R013`).
- `store` declares a data store reference; its data store is shared by all references with the same label.
- `reads` creates a data input association from the data element to the activity; `writes` creates a data output association. The data element may be a data object, a data store, or (language 0.5) an input, output or property (§7.1).
- In a document of language 0.5 `reads` may also name an end or intermediate throw event and `writes` a start, intermediate catch or boundary event: the event gets the data input or output, its association and its set, without an `ioSpecification` (`R003` for other events).
- `input=ID` on `reads` and `output=ID` on `writes` (language 0.5) connect the data element with the data input or output `ID` declared for the activity or event (`input ID … on=ACTIVITY`, or in the block of a sub-process) instead of a generated one (`R066` when it is not declared, `R013` when it belongs to another element).
- Data elements belong to the process (or sub-process) in whose block they are declared, or to the process of the first activity that reads or writes them.
- In a document of language 0.5 each association connects the data element with a data input (`ACTIVITY-in-N`) or data output (`ACTIVITY-out-N`) of the activity's `ioSpecification`, named after the data element, as BPMN 2.0.2 §10.4 describes; documents of 0.1 and 0.3 keep the target property that Camunda Modeler writes (`__targetRef_placeholder`).
- **State transitions.** Data objects carry `state=`; a transition is an activity that reads a data object in one state and writes it (another `data` statement with the same label) in another. The checker warns when an activity reads and writes a data object in the same state (`R064`).

### 7.1 Process data (language 0.5)

```
item wniosek-typ "Wniosek" structure="Wniosek"
item kwota-typ structure="xsd:decimal"
input wniosek "Wniosek o zakup" type=wniosek-typ
output decyzja "Decyzja"
property licznik "Licznik prób" on=proba
input kryteria "Kryteria oceny" on=ocena collection
```

| Statement | BPMN |
|---|---|
| `item ID ["Label"] [structure=NAME] [namespace="URI"] [kind=information\|physical] [collection]` (top level) | `itemDefinition` with `structureRef`, `itemKind`, `isCollection`; the label is its `documentation` (an item definition has no name) |
| `input ID ["Label"] [type=ITEM] [collection] [on=ACTIVITY]` | `dataInput` of the process (or of the sub-process in whose block it stands, or of the activity named by `on=`), in its `ioSpecification`; one `inputSet` lists every input |
| `output ID ["Label"] [type=ITEM] [collection] [on=ACTIVITY]` | `dataOutput`, likewise, with one `outputSet` |
| `property ID ["Label"] [type=ITEM] [on=ID]` | `property` of the process (or sub-process), or of the activity or event named by `on=` |
| `type=ITEM` on `data`, `store`, `input`, `output`, `property` | `itemSubjectRef` (of the data object for `data`, of the reference for `store`) |

- `input`, `output` and `property` stand wherever `data` may stand and are printed with the data of their block.
- `structure=` is a qualified XML name: a type name without a prefix (`Wniosek`), an XML Schema type (`xsd:string`; the exporter declares the prefix), or a name with another prefix whose namespace `namespace=` gives (`structure="feel:string" namespace="https://www.omg.org/spec/DMN/20191111/FEEL/"`; the exporter declares the prefix). A prefix has one namespace in a document and cannot be one that the exporter uses itself (`bpmn`, `bpmndi`, `dc`, `di`, `xsi`, `xsd`, `zeebe`, `modeler`, `xml`). Anything else, such as a file path, or `namespace=` without a prefixed name, is `R013`; `kind=` other than `information` or `physical` is `R013`.
- A reference of `type=` to an undeclared item is `R066`; `on=` must name an activity, for `input` also an end or intermediate throw event, for `output` a start, intermediate catch or boundary event, and for `property` any event.
- A process input or output is drawn as a data input or output shape. Inputs and outputs of activities are not drawn.

### 7.2 Resources and performers

```
resource bibliotekarz "Bibliotekarz" count=2
user wypozycz "Wypożyczenie" by=bibliotekarz
```

In a document of language 0.5, `resource` (§11) also exports a BPMN `resource` (its `name` is the label, or the identifier), and `by=` exports a `performer` of the activity (`humanPerformer` on `user` and `manual` tasks) with `resourceRef`. `count=` remains a simulation setting. In documents of 0.1 and 0.3 both stay simulation statements only.

## 8. Artifacts

```
note ID "Text" [on=ID] [doc="Text"]
group ID "Label" members="ID ID ID" [doc="Text"]
```

A note with `on=` is associated with that element.

## 9. Decisions

```
decision ID ["Label"] [hit=first]
  | rodzaj: string | cena: number || tryb: string |
  | "e-book"       | -             || "kierownik"  |
  | "drukowana"    | <= 150        || "automatyczny" |
  | -              | -             || "kierownik"  |
```

- The block of a `decision` is a table. The first row is the header: input columns, then `||`, then output columns. Each header cell is `name: type` where type is `string`, `number`, `boolean` or `date`. An input name is a FEEL expression over process variables (usually a variable name); an output name is the result variable.
- Each further row is a rule. Input cells are FEEL unary tests (`-` means any value); output cells are FEEL expressions.
- `hit=` is `unique` (default), `first`, `any`, `priority` or `collect`.
- An optional last cell after a second `||` holds a rule annotation: `| "e-book" | - || "kierownik" || licencje |`.
- A `rule` task calls the decision with `decision=ID`; its result is stored in `result=` (default: the single output name).

## 10. Execution

The execution statements and attributes form an **optional layer**. Descriptive and analytic models never need them, and the levels `basic` and `analytic` of `tok check` reject them (§13). Outside this layer, compatibility with Camunda means the set of elements available in Camunda Modeler and the rules for connecting and organizing them (pools, lanes, flows).

### 10.1 Executable profile

A document with the header flag `executable` (or a pool with `executable`) describes a process for a process engine. The reference implementation exports it for **Camunda 8** (`target=camunda`, the default):

| tok | Camunda 8 / Zeebe export |
|---|---|
| `user … form=F` | `zeebe:userTask`, `zeebe:formDefinition formId="F"` |
| `start … form=F` | start form, `zeebe:formDefinition formId="F"` |
| `rule … decision=D result=r` | `zeebe:calledDecision decisionId="D" resultVariable="r"` |
| `script … expr="E" result=r` | `zeebe:script expression="=E" resultVariable="r"` |
| `service … type="T"` | `zeebe:taskDefinition type="T"` |
| `flow … when="E"` | `conditionExpression` with body `=E` |
| `catch … message="M" correlation="E"` | message `M` with `zeebe:subscription correlationKey="=E"` |

FEEL expressions are written **without** the leading `=`; the exporter adds it.

### 10.2 Forms

```
form ID ["Title"]
  text "## Propozycja: {{tytul}}"
  field KEY ["Label"] type=textfield [required] [options="value:Label, value:Label"] [min=0] [max=10]
```

Field types: `textfield`, `textarea`, `number`, `checkbox`, `select`, `radio`, `datetime`. The field key becomes a process variable. For `select` and `radio`, `options` lists `value:Label` pairs separated by commas; the values `true` and `false` become booleans, numbers become numbers. A form exports to a Camunda form file `ID.form`.

### 10.3 Executable layer (language 0.3)

The attributes below complete the coverage of what Camunda 8 executes. They are meaningful in executable documents; elsewhere `tok check` reports them as warnings (`R055`) and the exporter ignores them.

**Value kinds.** Two kinds of attribute values exist:

| Kind | Attributes | Written as |
|---|---|---|
| always an expression | `when`, `expr`, `condition`, `correlation`, `collection`, `output-item`, `until`, `active`, right-hand sides in `inputs`/`outputs` | FEEL without `=`: `until="gotowe = true"` |
| static value or expression | `assignee`, `candidate-users`, `candidate-groups`, `due`, `follow-up`, `priority`, `process`, `type`, `timer`, `business-id`, `version` | as is (`assignee="kierownik"`); an expression computed at run time starts with `=` (`assignee="=wniosek.przelozony"`) |

The second kind follows the convention of Camunda Modeler. A document that is not executable never contains `=`.

**Multi-instance.**

| tok | Camunda 8 / BPMN |
|---|---|
| `parallel` / `sequential` | `multiInstanceLoopCharacteristics isSequential` |
| `collection="expr"` | `zeebe:loopCharacteristics inputCollection` |
| `item=name` | `inputElement` |
| `output=name`, `output-item="expr"` | `outputCollection`, `outputElement` (both or neither, `R054`) |
| `until="expr"` | `completionCondition` |

`collection` requires `parallel` or `sequential` (`R054`).

**Ad-hoc sub-process.** `active="expr"` (list of inner element ids) → `zeebe:adHoc activeElementsCollection`; `until="expr"` → `completionCondition`; `wait` → `cancelRemainingInstances="false"` (by default the remaining elements are cancelled).

**Call activity.** `process=` (static or `=expr`) → `zeebe:calledElement processId`; `binding=latest|deployment|version-tag` and `version="tag"` → `bindingType`, `versionTag`; `child-vars=all|none`, `parent-vars=all|none` → `propagateAllChildVariables`, `propagateAllParentVariables` (default `all`); `business-id=` → `businessId`. `binding=` and `version=` are also accepted on `rule` tasks (decision binding) and on elements with `form=` (form binding).

**Variable mappings.** `inputs="target = expr; …"` and `outputs="target = expr; …"` on activities, sub-processes and call activities; `outputs` also on catch, boundary and start events. The target is a variable or a nested property (`nadawca.iban`). Exported as `zeebe:ioMapping`. A malformed pair list is `R053`.

**User tasks.** `assignee`, `candidate-users`, `candidate-groups` (comma-separated when static) → `zeebe:assignmentDefinition`; `due`, `follow-up` (ISO 8601 with a zone when static, `C090`) → `zeebe:taskSchedule`; `priority` (0–100, `C091`) → `zeebe:priorityDefinition`.

**Jobs.** `type`, `retries`, `headers="key: value; …"` → `zeebe:taskDefinition`, `zeebe:taskHeaders` on service and send tasks and on message throw and end events; in executable documents a message throw or end event needs `type=` (`C022`), because Camunda executes it as a job.

**Connectors.** `connector=rest` with `method=`, `url=`, `result=` (and optional `body=`, `query=`, `headers-in=`) expands to the Camunda REST outbound connector; other connectors are reachable through `type=`, `inputs=` and `headers=`.

**Compensation.** `compensate=HANDLER` on an activity attaches a compensation boundary event associated with the handler; the handler carries the flag `compensation` and has no sequence flows (`R051`). When the activity has a boundary event with the flag `compensate` (`boundary ID "Label" on=ACTIVITY compensate`), that event is the one associated with the handler. `throw … compensate [activity=ID]` and `end … compensate [activity=ID]` set `activityRef`.

**Errors and escalations.** The flags `error` and `escalation` without a name on boundary events and event sub-process starts catch every error or escalation. Camunda 8 passes to the catching event only the variables sent together with the error; they are merged into the process with `outputs=` on the catching event. The code and message of the error are not available as variables.

**Conditional events.** `events="create, update"` → `zeebe:conditionalFilter variableEvents` on intermediate, boundary and event sub-process start events; not on root-level start events (`R052`).

**Listeners.** `listeners="start: type; end: type"` on any element (execution listeners); on user tasks the events `creating`, `assigning`, `updating`, `completing`, `canceling` define task listeners; an entry may give retries as `start: type (5)`.

**Process version.** `version-tag="v1"` on the header (document without pools) or on a pool → `zeebe:versionTag`.

### 10.4 Elements Camunda 8 does not execute (language 0.5)

In executable documents for Camunda (`target=camunda`, the default for executable documents) every element of language 0.5 that Camunda 8.10 does not execute is reported with a code naming it; descriptive and analytic documents, and executable documents with another `target=`, are not affected.

| Code | Severity | Element |
|---|---|---|
| `C100` | error | transaction |
| `C101` | error | cancel end or boundary event |
| `C102` | error | complex gateway |
| `C103` | error | standard loop with `while=`, `test-before` or `loop-max=` |
| `C104` | error | multiple event (several triggers) |
| `C105` | error | parallel multiple event |
| `C106` | error | instantiating receive task |
| `C107` | error | instantiating event-based gateway |
| `C108` | error | call activity of a global task (`task=`) |
| `C109` | warning | `operation=` (ignored: workers take jobs by `type=`) |
| `C110` | warning | correlation key (ignored: messages are correlated by `correlation=`) |

## 11. Simulation

Simulation statements give a tok model the parameters of a discrete-event simulation. They have no effect on BPMN export.

```
simulate cases=200 seed=1 arrival="exp(2h)" start="2027-01-04T08:00"
resource bibliotekarz "Bibliotekarz" count=2
variable cena "unif(20, 300)"
variable rodzaj "cat(drukowana=0.7, e-book=0.3)"
```

| Statement or attribute | Meaning |
|---|---|
| `simulate cases=N` | number of process instances (default 100) |
| `seed=N` | random seed; the same seed gives the same log |
| `arrival="DIST"` | time between consecutive case arrivals |
| `start="DATETIME"` | simulated time of the first arrival (default `2027-01-04T08:00`) |
| `resource ID ["Label"] count=N` | a pool of `N` interchangeable performers |
| `variable NAME "DIST"` | a case variable drawn once per case at its start |
| activity `by=ID` | the activity waits for a free performer of resource `ID` |
| activity `duration="DIST"` | processing time |
| activity `assign="NAME=DIST; …"` | variables set when the activity completes (e.g. a decision of a person) |
| catch event, receive task or message boundary event `delay="DIST"` | time until the awaited message or signal arrives |
| flow `p=NUMBER` | probability of the branch at an `xor` or `or` gateway without conditions |

**Distributions** (`DIST`): a duration (`30m`), a number, or a call:

| Call | Meaning |
|---|---|
| `fixed(x)` | always `x` |
| `unif(a, b)` | uniform between `a` and `b` |
| `tri(a, mode, b)` | triangular |
| `norm(mean, sd)` | normal; durations are truncated at 0 |
| `lnorm(mean, sd)` | log-normal given by the **mean and standard deviation** of the result |
| `exp(mean)` | exponential with the given mean |
| `bern(p)` | `true` with probability `p` |
| `cat(a=p, b=q, …)` | one of the labels with the given probabilities |

Durations are a number followed by a unit: `s`, `m` (minutes), `h`, `d`. In `unif`, `tri`, `norm`, `lnorm` and `exp` either all arguments are durations (the result is a duration) or all are numbers.

**Semantics.** Each case starts at its arrival time with its `variable`s drawn. Tokens move along sequence flows. An activity with `by=` queues (first come, first served) until a performer is free, then holds it for its duration. At an `xor` gateway the first outgoing flow (in document order) whose `when` holds is taken, else the default flow; if the flows have `p=` instead, one is drawn. An `or` gateway activates every flow whose condition holds (or which is drawn with its `p`), else the default. An `and` gateway activates all outgoing flows and a joining `and` waits for all incoming tokens; a joining `or` waits for the tokens that were actually activated. An `eventgw` races its following catch events: timers fire after their duration, messages after their `delay`. A boundary timer fires if the activity is still running when the timer elapses: an interrupting one cancels the activity, a non-interrupting one starts an extra token. `rule` tasks evaluate their decision, `script` tasks their expression. Time in the log is simulated calendar time without working-hours calendars (version 0.1).

**Language 0.5.** Transactions run like sub-processes; a cancel end event stops the transaction, runs the compensation handlers of its completed activities (latest completed first) and then continues from the cancel boundary event of the transaction (without one, the token ends there). A complex gateway splits like an `or` gateway; as a join it lets one token through as soon as `activation` holds, with `_activationCount` the number of tokens arrived so far, absorbs the later tokens and resets when all expected tokens have arrived (without `activation` it joins like `or`). A standard loop repeats while `while` holds, at most `loop-max` times, with `loopCounter` the number of finished iterations, testing before the first iteration with `test-before` and otherwise after each one. A catching multiple event fires on the first of its triggers (on the last with `parallel`); its timers use their durations, its other triggers the event's `delay=`. When a process has no start event, an instantiating receive task or event-based gateway starts the cases. `docs/simulation.md` gives the details.

The simulation produces an **event log** (one row per activity instance: case, activity, resource, enqueue, start, end) and **indicators**: number of cases per end event, cycle time (mean, median, 90th percentile), waiting and processing time per activity, utilization per resource.

## 12. FEEL subset

Conditions, script expressions, loop conditions (`while`), activation conditions (`activation`, with the variable `_activationCount`) and decision cells use a subset of FEEL (DMN 1.5, chapter 10). The subset is defined in `feel.md`. Expressions outside the subset are passed unchanged to the exported XML and reported by `tok check` as `F001` (a warning); the simulator cannot evaluate them.

## 13. Validation

`tok check` reports diagnostics with a code, a severity, a line number and a message in English or Polish (`--lang pl`). Codes are grouped:

| Prefix | Layer |
|---|---|
| `T` | tokens and lines (lexer) |
| `P` | statements (parser) |
| `R` | references and structure of the tok document |
| `E`, `W`, `L` | BPMN 2.0 rules: errors, warnings, style notes (shared with the `bpmn.py` validator) |
| `C` | executable profile for Camunda 8 |
| `D` | decision tables |
| `F` | FEEL |
| `S` | simulation parameters |

The full list is in `diagnostics.md`. The `--level` option restricts the accepted elements to a teaching level:

| Level | Accepts |
|---|---|
| `basic` | `start`, `end`, `task`, `user`, `service`, `manual`, `xor`, `and`, pools, lanes, message flows, data, notes; no attributes of the executable layer |
| `analytic` | everything except the executable profile and layer (§10.1, §10.3), forms and decisions |
| `full` (default) | everything, including all elements of language 0.5 |

The levels `basic` and `analytic` reject every element of language 0.5 (`R040`): `transaction`, `complex`, the flags `cancel`, `test-before`, `instantiate` and `parallel` on events and event-based gateways, the attributes `while`, `loop-max`, `activation`, `task` and `operation`, events with several triggers, `type=` on data, and the statements `input`, `output`, `property`, `item`, `global`, `interface` and `key`. `resource` and `by=` keep their earlier level rules. The descriptive additions `doc=`, `process-name=`, `object=`, `when=""` and the data of events are allowed at every level; the trigger flags without a value (§5.1) are rejected at the level `basic` like the other triggers.

## 14. Canonical form

`tok fmt` prints a document in this order and style:

1. the header;
2. `item` and `global` statements, then each `interface` with its operations;
3. top-level flow nodes in declaration order, then pools (each with its lanes and nodes in declaration order);
   `data`, `store`, `input`, `output` and `property` statements are printed at the end of the block they were declared in (pool, sub-process or top level);
4. all sequence flows, in declaration order, as chains: a chain continues while the next flow has no label or attributes, starts at the last node of the previous flow, and that node has exactly one outgoing flow;
5. message flows; 6. each `key` with its `prop` lines; 7. `reads` and `writes`; 8. notes and groups; 9. decisions; 10. forms; 11. simulation statements (`simulate`, `resource`, `variable`);
12. two-space indentation, one space between tokens, labels always quoted, attributes in the order of the tables in this document (the values of a repeated key together, in written order), blank line between sections.

Comments are kept. A comment line is attached to the statement that follows it and printed above it; a comment at the end of a line stays at the end of that statement's line. Comments at the end of the document are printed last.

`fmt` is idempotent: formatting a formatted document changes nothing.

## 15. Conformance

An implementation conforms at level **core** if, for every document in `conformance/`:

1. it reports the same set of diagnostic codes (`expected/*.diag`);
2. its `fmt` output equals `expected/*.tok`;
3. its BPMN export, ignoring diagram interchange (`bpmndi`) and attribute order, equals `expected/*.bpmn`.

Level **full** adds the diagnostic codes of `tok check` at the three levels (`expected/*.levels`), DMN and form export, FEEL evaluation and simulation with a fixed seed (compared within tolerance, since random number generators differ between languages).

Cases 01–32 are written for languages 0.1 and 0.3; their expected outputs are produced by the pure-Python implementation. Cases 33 and following cover language 0.5, one or more per section (transactions, complex gateway, standard loop, multiple events, instantiation, process data, resources, global tasks and interfaces, correlation, the Camunda codes `C100`–`C110`, the errors `R060`–`R066`, and in cases 44–46 documentation, trigger flags without a value, process names, namespaces of structures, named data inputs and outputs of associations, data of events and shared data objects); their expected outputs are produced by the Rust engine (`conformance/README.md`). The Julia core passes the core level for all cases.

The reference models of the BPMN Model Interchange Working Group in `conformance/miwg` are the conformance gate of BPMN import and export: their semantic content must survive import, printing and export; `miwg.md` lists, per model, what is kept and what is not.

## 16. Example

```
tok 0.1 "Propozycja zakupu książki" executable

pool biblioteka "Biblioteka uczelniana" process=propozycja-zakupu
  lane czytelnik "Czytelnik"
    start zlozona "Propozycja złożona" form=f-propozycja
  lane system "System biblioteczny"
    rule tryb "Ustalenie trybu akceptacji" decision=tryb-akceptacji result=tryb
    xor potrzebna "Czy potrzebna akceptacja kierownika?"
    xor scalenie
    script koszt "Wyliczenie kosztu zakupu" expr="cena + 12.5" result=koszt
  lane kierownik "Kierownik Oddziału Gromadzenia"
    user akceptacja "Akceptacja propozycji" form=f-akceptacja
    boundary termin "Minęło 7 dni" on=akceptacja timer=P7D
    xor zgoda "Czy kierownik się zgodził?"
    end odrzucona "Propozycja odrzucona"
    end wygasla "Propozycja wygasła"
  lane gromadzenie "Bibliotekarz gromadzenia"
    user zamowienie "Złożenie zamówienia u dostawcy" form=f-zamowienie
    end zamowiona "Książka zamówiona"

flow zlozona -> tryb -> potrzebna
flow potrzebna -> akceptacja "tak" when="tryb = \"kierownik\""
flow potrzebna -> scalenie "nie" default
flow akceptacja -> zgoda
flow zgoda -> scalenie "tak" when="zgoda = true"
flow zgoda -> odrzucona "nie" default
flow termin -> wygasla
flow scalenie -> koszt -> zamowienie -> zamowiona

decision tryb-akceptacji "Tryb akceptacji" hit=first
  | rodzaj: string | cena: number || tryb: string   ||               |
  | "e-book"       | -            || "kierownik"    || licencje      |
  | "drukowana"    | <= 150       || "automatyczny" || tania książka |
  | -              | -            || "kierownik"    || pozostałe     |

form f-propozycja "Propozycja zakupu"
  text "## Propozycja zakupu książki"
  field tytul "Tytuł" required type=textfield
  field autor "Autor" type=textfield
  field rodzaj "Rodzaj publikacji" required type=select options="drukowana:książka drukowana, e-book:e-book" default=drukowana
  field cena "Przybliżona cena (zł)" required type=number min=0
  field uzasadnienie "Uzasadnienie" type=textarea

form f-akceptacja "Akceptacja"
  text "## Propozycja: {{tytul}}"
  field zgoda "Decyzja" required type=radio options="true:zgoda na zakup, false:odmowa"
  field komentarz "Komentarz dla czytelnika" type=textarea

form f-zamowienie "Zamówienie"
  text "## Zamówienie: {{tytul}} (koszt z wysyłką: {{koszt}} zł)"
  field dostawca "Dostawca" required type=textfield
  field numer_zamowienia "Numer zamówienia" required type=textfield
```
