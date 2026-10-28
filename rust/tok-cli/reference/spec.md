# tok — Language Specification, version 1.0

tok is a plain-text language for information-process models. A tok document describes a BPMN 2.0 process (pools, lanes, events, activities, gateways, flows, data), the DMN decisions it calls, the forms people fill in, and the parameters needed to simulate it. Every tool built on tok reads the same text and produces the same projections: BPMN 2.0 XML with diagram layout, SVG, DMN XML, Camunda form JSON and simulation logs.

This document is normative. The reference implementation is the Rust engine in `rust/` (the `tok` command and the Python and R packages run it); the pure-Python implementation in `python/` is frozen at language 0.3, and the Julia package in `julia/` runs the Rust engine through its C interface and implements the core independently (parsing, formatting, core diagnostics, BPMN export) against the same conformance corpus.

Language 0.5 adds the elements of BPMN 2.0.2 process modelling that Camunda 8 does not execute: transactions and cancel events (§5.3), the complex gateway (§5.4), the attributes of the standard loop (§5.2.1), multiple and parallel multiple events (§5.1.1), instantiating receive tasks and event-based gateways (§5.2, §5.4), process data (§7.1), resources and performers (§7.2), global tasks, interfaces and operations (§5.6) and correlation keys (§6.3). Every BPMN 2.0.2 rule the checker enforces is listed with its clause in `bpmn-rules.md`.

Language 0.6 adds the interactions between participants: conversations (§6.4), choreographies (§5.7), and the multiplicity of participants with the participant associations of calls (§5.5, §5.7, §6.4). The tools check a document against the conformance classes of BPMN 2.0.2 (§13) and keep the diagram of an imported file in a layout sidecar (§16).

Language 0.7 adds decision models of DMN 1.5: decision requirements graphs with input data, business knowledge models and knowledge sources (§9.1), boxed expressions (§9.2), the hit policies, aggregations, output values and default outputs of decision tables (§9.3), DMN types (§9.4), and FEEL at conformance level 3 in every expression of a 0.7 document (§12). `tok eval` evaluates decisions with everything they require; the tools export DMN 1.5 with diagrams and import DMN (§9.5).

Language 1.0 is the stable language: the content of language 0.7, the limits of §2.1 as rules, the deprecations of §4.2 and the compatibility promise of §4.3. It adds no statement, attribute or flag. Wherever this specification says "a document of language 0.7", "a 0.7 document" or "language 0.7", it means a document whose header declares `tok 0.7` or `tok 1.0`; "0.5 or later" and "0.6 or later" include 1.0.

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

### 2.1 Limits

Everything that reads or walks a document, an expression or an XML file recurses into its nesting, so the nesting is bounded. The bounds hold for documents of every language version; a conforming implementation reads everything within them and reports what lies beyond them with these diagnostics, without failing:

| What | Bound | Beyond the bound |
|---|---|---|
| blocks of a document (a statement in the block of another statement is one level deeper: pools, lanes, sub-processes, choreographies, interfaces, keys, decisions, the lines of boxed expressions, DMN types, forms) | 64 levels | `P008` on the first line of a block nested deeper (detail `its block is nested more than 64 levels deep`); that block is skipped |
| a FEEL expression (each parenthesis, list, context, call, `if`, unary minus, type expression and filter is one level, and so is each operator of a chain such as `a + b + c`) | 160 levels | `F002` (detail `the expression is nested too deeply`); the expression is not evaluated |
| the elements of a BPMN or DMN file read by `tok import` | 64 levels | the file is refused with a message naming the bound |
| the elements of an XML file read by the BPMN tools (validation, layout, the layout sidecar) | 256 levels | the file is refused with a message naming the bound |

The bounds are far above what modelling tools write (the deepest model of the MIWG suite and of the DMN TCK nests 12 levels) and above what the tools write themselves for the deepest document they read (`fuzzing.md`).

## 3. Attributes

Most statements end with attributes:

```
key=value      value is a WORD, NUMBER or STRING
flag           a bare WORD after the identifier and label: a boolean set to true
```

Attributes may appear in any order; the formatter prints them in the order of the attribute tables below. An unknown attribute key is a warning (`R010`); an unknown flag is an error (`R011`).

**Documentation** (language 0.5). `doc="Text"` gives the BPMN `documentation` of the element a statement exports: flow nodes, flows, message flows, pools (the participant), lanes, `data`, `store`, `input`, `output`, `property`, notes, groups, `global`, `interface` and `operation`, and (language 0.6) choreographies, their activities and conversation nodes; on the header it documents the process of a document without pools and the definitions of a document with pools. It is plain text (`\n` for a line break) and allowed at every level. The label of an `item` is already its documentation.

A key is normally given once; when it is repeated, the last value counts. In a document of language 0.5 the trigger keys of events (`message`, `timer`, `signal`, `error`, `escalation`, `condition`, `link`, §5.1.1) and `error` on an `operation` (§5.6) may repeat: every value counts, in written order, and the formatter prints the values of one key together.

## 4. Document structure

The first statement is the header:

```
tok 1.0 "Interlibrary loan" executable
```

| Part | Meaning |
|---|---|
| `tok 1.0` | language version (required): a number with its minor part. Accepted versions: `0.1`, `0.3`, `0.5`, `0.6`, `0.7` and `1.0`; a document written for an earlier version means the same under 1.0 (§4.1). Another number is `P006` (a warning; the document is read with every statement of 0.7 and without the deprecation warnings of 1.0), a version that is not a number `P005`, and a second header `P010`. The language version follows the minor version of the release that changed the language. Three things depend on the declared version, so that older documents keep their meaning: only a document of 0.5 or later may give an event several triggers (in 0.1 and 0.3 two triggers are `R012` and a repeated key keeps its last value), only there do `resource` and `by=` also describe BPMN resources and performers (§7.2), and only in a document of 0.6 or later do `min=` and `max=` give a pool a multiplicity (§5.5; in older documents they are unknown attributes, `R010`). Everything else of 0.5 and 0.6 may be used under any header; a document imported from BPMN that uses it is printed as `tok 0.5` or `tok 0.6`. The statements and attributes of language 0.7 (§9.1–§9.5) need the header `tok 0.7`: in earlier documents `inputdata`, `bkm`, `source`, `entry`, `result` and `element` are unknown keywords (`P001`), the new attributes unknown attributes (`R010`) and the new hit policies unknown (`D003`); a 0.7 document uses FEEL at conformance level 3 with numbers of 34 digits (§12), earlier documents the FEEL subset with numbers of 28 digits. |
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
- **choreographies** (§5.7) and **conversation nodes** (§6.4) of language 0.6;
- **decisions** and, in language 0.7, **input data**, **knowledge sources**, **business knowledge models** and DMN types (§9), **forms** (§10.2), **simulation statements** (§11).

References may point forward: a flow may name a node declared later.

### 4.1 Language 1.0

A document with the header `tok 1.0` is read, checked, printed, exported, imported, evaluated and simulated exactly as the same document with the header `tok 0.7` (§9, §12): every statement, attribute and flag of languages 0.1–0.7 is available, decisions are DMN 1.5 decision models, and FEEL is at conformance level 3 with numbers of 34 digits. The differences are only these:

- **Outputs.** The BPMN and DMN files carry the language version of the document as `exporterVersion` (`1.0`), as they carry `0.1`–`0.7` for older documents; everything else in a file is what the same document with the header `tok 0.7` gives. The decisions of a 1.0 document are exported as DMN 1.5 (namespace `https://www.omg.org/spec/DMN/20230324/MODEL/`); documents of 0.1–0.6 keep the DMN 1.3 export of their decision tables.
- **Deprecations** (§4.2) are reported only in documents with the header `tok 1.0`.

The header stays as written: `tok fmt` never changes the version of a document, and `tok import` writes the oldest version that expresses what it read (`0.1`, `0.3`, `0.5` or `0.6` for BPMN, `0.7` for DMN), each of which is a valid document with the same meaning in every 1.x release.

Documents of 0.1, 0.3, 0.5, 0.6 and 0.7 stay valid, mean what they meant and give every output they gave, byte for byte: their canonical text, diagnostics, BPMN, DMN, forms, SVG, `tok eval` results and simulations. What depends on the declared version (the table above) keeps depending on it; a 1.0 document has all of it.

### 4.2 Deprecations

A deprecated form keeps its meaning in every 1.x version; a document of language 1.0 that uses it gets a warning, and a later major version of the language may refuse it. Documents of earlier versions are not warned. Deprecated in language 1.0:

| Form | Warning | Instead |
|---|---|---|
| an attribute key given more than once on one line, except the keys that may repeat (§3: the trigger keys on flow nodes, `error` on an `operation`) | `R090` | the key once: only the last value has ever counted, and `tok fmt` prints only that one |
| a flag given more than once on one line | `R090` | the flag once; `tok fmt` prints it once |

### 4.3 Compatibility of 1.x

Language 1.0 is the language of every release 1.x of the tools. A release 1.x guarantees:

1. **Validity.** Every document that is valid under language 1.0 (without errors at the level `full`) stays valid, and a document with errors keeps its error codes. A new warning may be added to the documents of an existing version only as a documented bug fix (item 3); deprecations (§4.2) come only with a new language version in the header.
2. **Canonical form.** `tok fmt` prints every valid 1.0 document exactly as 1.0 prints it, and every document of an earlier version as before.
3. **Outputs.** The conformance corpus (§15) fixes the outputs of the tools. An output changes only for a bug fix that the changelog documents with the documents it affects; the expected files of the corpus change in the same release.
4. **New revisions of Camunda and DMN in minor releases.** A minor release may follow a new revision of Camunda 8 (attributes of the executable layer, codes of what it does not execute) or of DMN and FEEL. What changes the meaning or the outputs of a document comes with a new language version (`tok 1.1`, …) declared in the header; documents with the header `tok 1.0` keep the meaning and outputs of 1.0. A patch release never changes the language.
5. **Earlier versions.** Documents of 0.1, 0.3, 0.5, 0.6 and 0.7 are read in every release 1.x with the meaning and outputs of this specification.

A change that breaks one of these guarantees is reserved for language 2.0.

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

**Participant multiplicity** (language 0.6):

```
pool recenzent "Recenzent" min=2 max=5
pool autor "Autor" max=3
```

| tok | BPMN |
|---|---|
| `min=N`, `max=N` on a pool | `participantMultiplicity` with `minimum` and `maximum`: several participants of this kind take part in the collaboration |

- When only `max=` is given the minimum is 2; when only `min=` is given the maximum is that of BPMN, 1. Both are whole numbers with `0 <= min <= max` and `max >= 1` (`R074`).
- When the maximum is greater than 1 the pool is drawn with the multi-instance marker (three vertical bars) at the bottom, and so are the participant bands of the pool in choreographies.
- In documents of 0.1, 0.3 and 0.5 `min=` and `max=` on a pool are unknown attributes (`R010`) and change nothing.

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

### 5.7 Choreographies (language 0.6)

A choreography describes the order of the message exchanges between participants, without the processes of the participants. It is a top-level block; its participants are pools of the document, usually black-box pools.

```
pool klient "Klient"
pool sklep "Sklep"

choreography zakupy "Zakupy w sklepie"
  start s
  choreo zamow "Złożenie zamówienia" from=klient to=sklep message="Zamówienie" reply="Potwierdzenie"
  xor dostepne "Czy towar dostępny?"
  choreo wyslij "Wysyłka" from=sklep to=klient message="Paczka"
  choreo odmowa "Odmowa" from=sklep to=klient message="Brak towaru"
  end e1
  end e2
  flow s -> zamow -> dostepne
  flow dostepne -> wyslij "tak" when="dostepne"
  flow dostepne -> odmowa "nie" default
  flow wyslij -> e1
  flow odmowa -> e2
```

| Statement | BPMN |
|---|---|
| `choreography ID ["Label"]` + block | `choreography`, a root element of its own: its participants (the pools named by its activities, identifiers `CHOREOGRAPHY-POOL`), the message flows of its tasks, then its flow nodes and sequence flows |
| `choreo ID ["Label"] from=P to=P [message="M"] [reply="M"]` | `choreographyTask` with two `participantRef`s, `initiatingParticipantRef` = `from`, the message flow `ID-message` from `from` to `to` (with the message `M`) and, with `reply=`, the message flow `ID-reply` back (`reply=""`: a reply without a message name) |
| `subchoreo ID ["Label"] pools="P P …" from=P` + block | `subChoreography` with its participants and initiating participant; its block holds flow nodes and flows like the block of a choreography |
| `callchoreo ID ["Label"] pools="P P …" from=P choreography=ID [map="inner:outer; …"]` | `callChoreography` with `calledChoreographyRef`; `map=` gives `participantAssociation`s: a pool taking part in the called choreography (inner) and one of `pools=` (outer) |
| flags `loop`, `parallel`, `sequential` on `choreo`, `subchoreo`, `callchoreo` | `loopType` `Standard`, `MultiInstanceParallel`, `MultiInstanceSequential` |
| `doc="Text"` on all of them | `documentation` |

- `from=` and `to=` of a task name two different pools; `pools=` of a sub-choreography or call names at least two pools and `from=` one of them; the participants of an activity inside a sub-choreography are participants of the sub-choreography; `choreography=` names a choreography of the document (`R072`; unknown pools and choreographies are `R002`). A message name is declared like the message of an event (§5.1).
- The block of a choreography holds choreography activities, gateways (`xor`, `and`, `or`, `eventgw`, `complex`), events and sequence flows (§6.1); data, notes and groups do not belong to it (`P004`). The events follow BPMN 2.0.2 §11.6: a start event without a trigger or with timer, conditional or signal triggers; an intermediate catch event with a timer, conditional, signal or link trigger; an intermediate throw event without a trigger or with a link trigger; a boundary event on a choreography activity with a timer, conditional or signal trigger; an end event without a trigger or with `terminate`. Other flow nodes and triggers are `R073`, and so is a choreography without flow nodes.
- Each choreography is drawn in a diagram of its own, below the collaboration, laid out like a process: choreography activities are rounded rectangles with one band per participant (the initiating participant on top, white; the others at the bottom, shaded), an envelope above or below the band of the participant that sends a message (white for the initiating message, shaded for the reply) with the message name, the loop and multi-instance markers, a thick border for a call, and a `+` for a call and for a sub-choreography without content.

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

`A` and `B` are flow nodes or pools in different pools (`R022`). The label names the message flow; `message=` links it to a message declaration (default: the label). A message flow that a conversation names (§6.4) is printed with its identifier (`id=`), also when it is the derived one.

### 6.3 Correlation keys (language 0.5)

```
key zamowienie "Numer zamówienia"
  prop numer message="Zamówienie" path="zamowienie.numer"
  prop numer message="Potwierdzenie" path="potwierdzenie.nr"
```

- `key ID ["Label"]` (top level) → `correlationKey` of the collaboration. BPMN 2.0.2 keeps correlation keys only in collaborations (`tCollaboration`), so a document with `key` needs pools (`R013`).
- Each `prop NAME message="M" path="…"` line belongs to the key above it; both attributes are required (`R013`). Every distinct `NAME` becomes one `correlationProperty` (identifier `CorrelationProperty_NAME`, shared by all keys that use the name), with one `correlationPropertyRetrievalExpression` per line: `messageRef` of the message `M` (§5.1) and `messagePath` with the `path` text.
- `correlation=` of executable documents (§10.1) is the Camunda form of correlation and is independent of `key`.

### 6.4 Conversations (language 0.6)

A conversation groups the message flows that belong to one logical exchange between pools; a conversation diagram shows the pools and the conversations between them.

```
pool czytelnik "Czytelnik"
pool biblioteka "Biblioteka"
pool hurtownia "Hurtownia"

message czytelnik -> biblioteka "Zamówienie" id=m-zamowienie
message biblioteka -> czytelnik "Odbiór" id=m-odbior

conversation wypozyczenie "Wypożyczenie" pools="czytelnik biblioteka" messages="m-zamowienie m-odbior"
subconversation zakup "Zakup książek" pools="biblioteka hurtownia"
  conversation oferta "Oferta" pools="biblioteka hurtownia"
  conversation zamowienie "Zamówienie" pools="biblioteka hurtownia"
callconversation reklamacja "Reklamacja" pools="biblioteka hurtownia" collaboration=reklamacje
```

| Statement | BPMN |
|---|---|
| `conversation ID ["Label"] pools="P P …" [messages="F F …"]` | `conversation` with one `participantRef` per pool and one `messageFlowRef` per message flow |
| `subconversation ID ["Label"] pools="…"` + block of conversation statements | `subConversation` holding the conversation nodes of its block |
| `callconversation ID ["Label"] pools="…" collaboration=ID [map="inner:outer; …"]` | `callConversation` with `calledCollaborationRef` and one `participantAssociation` per `map=` pair (outer: one of `pools=`) |
| `doc="Text"` | `documentation` |
| a top-level conversation node | one `conversationLink` (identifier `NODE-link-POOL`) from each of its pools to the node |

- `pools=` names at least two different pools of the document (`R070`; an unknown pool is `R002`); a node inside a sub-conversation links only pools of the sub-conversation (`R070`). `messages=` names message flows (§6.2) between two of its pools (`R071`). `messages=` is allowed on all three statements.
- `collaboration=` names a choreography of the document (a choreography is a collaboration in BPMN; the inner participants of `map=` are then its pools) or a collaboration of another file, whose identifier is written as it is. `map=` belongs to calls only (elsewhere it is an unknown attribute, `R010`); a malformed pair is `R053`.
- Conversation nodes stand only at top level and in the block of a `subconversation` (`P004`). The canonical form prints them after the message flows.
- A conversation node is drawn as a hexagon (a thick border for a call, a `+` for a sub-conversation) in the gap below the first pool it links, where no message flow crosses the gap, with its name on the right; its links are double lines to the pools. The nodes inside a sub-conversation are not drawn.

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

In documents of languages 0.1–0.6 a decision is exactly this table; it is exported as one DMN 1.3 file per decision, and `tok eval` gives the object of its outputs. Language 0.7 extends decisions to complete DMN 1.5 decision models (§9.1–§9.5); the table above means the same there.

### 9.1 Decision requirements graphs (language 0.7)

```
inputdata wiek "Wiek czytelnika" type=number
inputdata konto "Rodzaj konta" type=string
source regulamin "Regulamin wypożyczalni" kind="regulamin" uri="https://biblioteka.example/regulamin"
bkm kara "Kara za zwłokę" params="dni: number, stawka: number" type=number expr="dni * stawka"
decision limit "Limit wypożyczeń" requires="wiek konto regulamin" hit=first
  | wiek: number | konto: string          || limit: number |
  | < 18         | -                      || 5             |
  | -            | -                      || 10            |
decision blokada "Czy zablokować konto?" type=boolean requires="limit" expr="limit < 1"
```

| Statement | Attributes (in canonical order) | DMN 1.5 |
|---|---|---|
| `inputdata ID ["Label"]` | `name=`, `type=`, `doc=` | `inputData` with its `variable` |
| `source ID ["Label"]` | `name=`, `requires=`, `kind=`, `uri=`, `doc=` | `knowledgeSource` (`type`, `locationURI`) |
| `bkm ID ["Label"]` + optional block | `name=`, `params=`, `type=`, `requires=`, `hit=`, `aggregation=`, `default=`, `expr=`, `invoke=`, `with=`, `doc=` | `businessKnowledgeModel` with `encapsulatedLogic` |
| `decision ID ["Label"]` + optional block | `name=`, `type=`, `requires=`, `question=`, `answers=`, `hit=`, `aggregation=`, `default=`, `expr=`, `invoke=`, `with=`, `doc=` | `decision` with `variable`, `question`, `allowedAnswers` and its expression |

- **Names.** The FEEL name of an element is its identifier, or `name="…"` when it is not a tok identifier (`name="Zaległe opłaty"`). Other elements refer to it by that name. Names of the graph are unique (`R080`). The label is the `label` of the DMN element.
- **Requirements.** `requires="ID …"` lists the elements an element depends on; the kind of each named element gives the kind of requirement: decisions and input data are information requirements, business knowledge models knowledge requirements, knowledge sources authority requirements. A decision may require decisions, input data, business knowledge models and knowledge sources; a business knowledge model business knowledge models and knowledge sources; a knowledge source decisions, input data and knowledge sources (`R002`, `R003`). Requirements form no cycle (`R076`). A decision sees exactly what it requires; a name of the graph used without the requirement is `R079`.
- **Business knowledge models** are functions: `params="name: type, …"` declares the parameters (types optional, `R078` for a malformed list), `type=` the type of the result, and the logic (§9.2) is the body. A decision calls a required model by its name in FEEL (`kara(10, 0.5)`) or with an invocation (`invoke=`).
- **Knowledge sources** describe the authority behind a decision (a regulation, an expert); they have no logic.
- A decision without logic is allowed: it describes the decision in the diagram, and its value is `null`.

### 9.2 Boxed expressions (language 0.7)

The logic of a decision, of a business knowledge model and of each line of its block is one of:

| tok | DMN 1.5 |
|---|---|
| `expr="FEEL"` | literal expression |
| block of table rows with `\|\|` | decision table (§9.3) |
| block of table rows without `\|\|`: a header of columns `name: type`, then rows of FEEL expressions | relation (a list of contexts) |
| block of `entry NAME …` lines, optionally ending with one `result …` line | context; the result line is the entry without a variable, whose value is the value of the context; without it the value is the context of all entries |
| block of `element …` lines | list |
| `invoke=BKM with="param = FEEL; …"` | invocation with one binding per parameter (parameters not bound are `null`) |
| `params="name: type, …"` on `entry`, `result` or `element` | function definition whose body is the line's logic |

```
decision oplata "Opłata za przetrzymanie" type=number requires="rodzaj dni naliczenie"
  entry stawki
    | typ: string  | stawka: number |
    | "książka"    | 0.5            |
    | "czasopismo" | 0.2            |
  entry stawka type=number expr="stawki[typ = rodzaj].stawka[1]"
  entry zaokraglenie params="x: number" expr="round half up(x, 2)"
  entry naliczona invoke=naliczenie with="dni = dni; stawka = stawka"
  result expr="zaokraglenie(naliczona)"

decision kategorie "Kategorie kont"
  element expr="\"student\""
  element expr="\"doktorant\""
```

- `entry NAME [type=T] …`: the name is a FEEL name, a word or a quoted string (`entry "do naprawy" expr="…"`); entries of one context have different names (`R080`); each entry sees the entries before it. `type=` converts the value of the line to the type (§9.4).
- An entry, result or element whose logic is a table, a relation, a context or a list carries it in its own block, indented below the line.
- A statement has exactly one form of logic, `with=` needs `invoke=`, the result is the last line of its context, and element lines are not mixed with entry and result lines (`R077`).

### 9.3 Decision tables in language 0.7

| tok | DMN 1.5 |
|---|---|
| `hit=unique\|first\|any\|priority\|collect\|rule-order\|output-order` | hit policy (`RULE ORDER`, `OUTPUT ORDER`) |
| `aggregation=sum\|count\|min\|max` (with `hit=collect` and one output, `D007`) | aggregation of `COLLECT` |
| header cell `name: type = "a", "b"` | input values of an input column, output values of an output column (unary tests) |
| `default="FEEL"` (one output) or `default="out = FEEL; …"` | default output entries, the result when no rule matches |
| header types: `string`, `number`, `boolean`, `date`, `time`, `date and time`, `days and time duration`, `years and months duration`, `list`, `context`, `any`, or a DMN type of the document (§9.4) | `typeRef` |

- The value of a table with one output is the value of that output; with several outputs a context of the outputs; with `collect`, `rule-order` and `output-order` a list of them in the order of the rules (`output-order`: by the priority of the output values); with an aggregation the aggregated value. When no rule matches, the value is the default outputs when `default=` gives them, otherwise `null` (an empty list for `collect`, `rule-order` and `output-order`, and `0` for `aggregation=count`).
- `priority` and `output-order` order the matching rules by the position of their outputs in the output values, the first output first (`D006` when no output has output values; the allowed values of the output's type count as its output values). `priority` gives the first of them.
- `unique` with several matching rules and `any` with different outputs are violated hit policies: the value is `null` and `tok eval` reports the violation.
- In a 0.7 document a malformed header cell is `D009`; a relation row with another number of cells than its header `D008`.

### 9.4 Types (language 0.7)

```
item egzemplarz-t "Egzemplarz"
  item sygnatura type=string
  item rok type=number
  item "stan zachowania" type=string allowed="\"dobry\", \"zły\""
item egzemplarze-t "Lista egzemplarzy" type=egzemplarz-t collection
inputdata egzemplarze "Egzemplarze" type=egzemplarze-t
```

- In a 0.7 document an `item` with `type=` or `allowed=`, or with a block of component lines, is a DMN type (`itemDefinition`): `type=` the base type, `allowed="unary tests"` the allowed values, `collection` a list of such values, the components (`item NAME [type=T] [allowed=…] [collection]`, nested to any depth) the fields of a context. Its FEEL name is `name=` or the identifier. It is exported to the DMN files, not to BPMN. An `item` that is both a BPMN item (`structure=`, `kind=`) and a DMN type is `R081`.
- `type=` of input data, decisions, business knowledge models, lines and parameters, and the types of table columns, name a FEEL type (above, and `function`, `range`, `Any`, type expressions such as `list<number>`) or a DMN type of the document by identifier or name (`R081` otherwise).
- A value of input data, of a decision, of a business knowledge model or of a typed line that does not conform to its type is `null` (DMN 1.5 §10.3.2.9.4). Before that, a list of one value conforms through its element, a value through a list of one, and a date through the date and time at midnight UTC.

### 9.5 Evaluation and export (language 0.7)

- `tok eval FILE DECISION name=value …` evaluates the decision with what it requires: required decisions once each, input data from the arguments by FEEL name, business knowledge models as functions. Text given for input data of a temporal type is read as that type; a JSON object or list is a context or list (see `cli.md`). The result is the value of the decision as JSON. A violated hit policy in the decision or in a decision it requires is reported with exit code 1.
- A `rule` task of a 0.7 document evaluates its decision the same way in the simulator; the case variables are the input data, and `result=` defaults to the only output of a decision table, or else to the FEEL name of the decision.
- `tok export` writes one DMN 1.5 file per connected decision requirements graph, `NAME.dmn` after its first decision, with the DMN types of the document and a DMNDI diagram laid out automatically (decisions above what they require), and its picture `NAME.drd.svg`. A 0.7 document without a process writes no BPMN file.
- A document is exported **for Camunda 8** when its header or a pool has the flag `executable` and `target=` is `camunda` (the default), as for BPMN; this holds also for a document that has only decisions (`tok 0.7 "…" executable`). Its DMN files write every FEEL name that is not a plain name (a letter or `_`, then letters, digits and `_`) in backticks, and such a key of a context literal as a string, because Camunda's FEEL reads names with spaces and symbols only in that form (`feel.md` §9.5). Other documents are exported in standard FEEL.
- `tok import FILE.dmn` reads DMN 1.1–1.5 (`import.md`).

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

### 10.4 Elements Camunda 8 does not execute (languages 0.5, 0.6 and 0.7)

In executable documents for Camunda (`target=camunda`, the default for executable documents) every element of languages 0.5, 0.6 and 0.7 that Camunda 8.10 does not execute is reported with a code naming it; descriptive and analytic documents, and executable documents with another `target=`, are not affected. Camunda 8.10 executes decision requirements graphs, decision tables, literal expressions, contexts, relations, lists, invocations and business knowledge models through its DMN engine.

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
| `C120` | warning | conversation or sub-conversation (ignored: it describes the collaboration) |
| `C121` | warning | call conversation (ignored) |
| `C122` | error | choreography |
| `C123` | warning | participant multiplicity of a pool (ignored: every process instance is one participant) |
| `C130` | error | business knowledge model whose logic is an invocation |
| `C131` | error | function definition (`params=` on a line) whose body is not a literal expression |
| `C132` | error | decision without decision logic (Camunda rejects the DMN file) |

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

**Language 0.7.** A `rule` task evaluates its decision with the decisions, input data and business knowledge models it requires (§9.5); the case variables are the input data. Conditions and expressions of 0.7 documents that the simulator cannot evaluate are warnings, as before.

**Language 0.6.** Conversations and choreographies describe interactions, not the work of one participant: the simulator ignores them and simulates the process of the document as before. A pool with `min=`/`max=` is simulated once per case.

**Language 0.5.** Transactions run like sub-processes; a cancel end event stops the transaction, runs the compensation handlers of its completed activities (latest completed first) and then continues from the cancel boundary event of the transaction (without one, the token ends there). A complex gateway splits like an `or` gateway; as a join it lets one token through as soon as `activation` holds, with `_activationCount` the number of tokens arrived so far, absorbs the later tokens and resets when all expected tokens have arrived (without `activation` it joins like `or`). A standard loop repeats while `while` holds, at most `loop-max` times, with `loopCounter` the number of finished iterations, testing before the first iteration with `test-before` and otherwise after each one. A catching multiple event fires on the first of its triggers (on the last with `parallel`); its timers use their durations, its other triggers the event's `delay=`. When a process has no start event, an instantiating receive task or event-based gateway starts the cases. `docs/simulation.md` gives the details.

The simulation produces an **event log** (one row per activity instance: case, activity, resource, enqueue, start, end) and **indicators**: number of cases per end event, cycle time (mean, median, 90th percentile), waiting and processing time per activity, utilization per resource.

## 12. FEEL

Conditions, script expressions, loop conditions (`while`), activation conditions (`activation`, with the variable `_activationCount`), the values of the executable layer and decision logic are FEEL (DMN 1.5, chapter 10).

- In documents of languages 0.1–0.6 they use a subset of FEEL, defined in `feel.md` §1–§8, with numbers of 28 significant digits. Expressions outside the subset are passed unchanged to the exported XML and reported by `tok check` as `F001` (a warning); the simulator cannot evaluate them.
- In documents of language 0.7 they use FEEL at DMN conformance level 3 (`feel.md` §9): every expression, temporal values and arithmetic, the standard function library, contexts, ranges, filters, iterations, quantifiers and functions, names with spaces resolved against the names of the document, numbers of 34 significant digits. `F001` reports only what tok cannot evaluate (externally defined functions); `F002` syntax errors.

## 13. Validation

`tok check` reports diagnostics with a code, a severity, a line number and a message in English or Polish (`--lang pl`). Codes are grouped:

| Prefix | Layer |
|---|---|
| `T` | tokens and lines (lexer) |
| `P` | statements (parser) |
| `R` | references and structure of the tok document |
| `E`, `W`, `L` | BPMN 2.0 rules: errors, warnings, style notes (shared with the `bpmn.py` validator) |
| `C` | executable profile for Camunda 8 |
| `D` | decision tables and relations |
| `F` | FEEL |
| `S` | simulation parameters |

The full list is in `diagnostics.md`. The `--level` option restricts the accepted elements to a teaching level:

| Level | Accepts |
|---|---|
| `basic` | `start`, `end`, `task`, `user`, `service`, `manual`, `xor`, `and`, pools, lanes, message flows, data, notes; no attributes of the executable layer |
| `analytic` | everything except the executable profile and layer (§10.1, §10.3), forms and decisions |
| `full` (default) | everything, including all elements of languages 0.5 and 0.6 |

The levels `basic` and `analytic` reject every element of language 0.5 (`R040`): `transaction`, `complex`, the flags `cancel`, `test-before`, `instantiate` and `parallel` on events and event-based gateways, the attributes `while`, `loop-max`, `activation`, `task` and `operation`, events with several triggers, `type=` on data, and the statements `input`, `output`, `property`, `item`, `global`, `interface` and `key`. `resource` and `by=` keep their earlier level rules. The descriptive additions `doc=`, `process-name=`, `object=`, `when=""` and the data of events are allowed at every level; the trigger flags without a value (§5.1) are rejected at the level `basic` like the other triggers. They also reject the statements of language 0.6 (`R040`): `conversation`, `subconversation`, `callconversation`, `choreography` (once for the statement; its content is not reported again) and `min=`/`max=` on pools. The elements of decision requirements graphs of language 0.7 (`inputdata`, `source`, `bkm`, `decision`) are rejected together with decisions, once for the first of them, and DMN types like every `item`.

**BPMN conformance classes** (language 0.6). BPMN 2.0.2 (clause 2) defines three sub-classes of process modelling conformance: Descriptive (the visible elements of high-level modelling), Analytic (Descriptive and about half of the full language) and Common Executable (what an executable process needs: data inputs and outputs, item definitions, operations, but no pools, lanes or message flows). `tok check --class descriptive|analytic|common-executable|full` reports every element or attribute of the document outside the chosen class as `R075`, naming the BPMN element and the first class that contains it (in the order descriptive, analytic, common-executable, full). `full` is the whole modelling language, with conversations and choreographies, and contains everything. `tok check --json` always reports the smallest class the document fits, in the same order, as `"bpmn_class"`. The class of each tok construct is listed in `bpmn-rules.md`; the levels above are teaching levels and do not coincide with the classes.

## 14. Canonical form

`tok fmt` prints a document in this order and style:

1. the header;
2. `item` and `global` statements (a DMN type of language 0.7 with its components), then each `interface` with its operations;
3. top-level flow nodes in declaration order, then pools (each with its lanes and nodes in declaration order);
   `data`, `store`, `input`, `output` and `property` statements are printed at the end of the block they were declared in (pool, sub-process or top level); then each `choreography` with its nodes and, at the end of its block, the sequence flows of its nodes (as chains, as in item 4);
4. all other sequence flows, in declaration order, as chains: a chain continues while the next flow has no label or attributes, starts at the last node of the previous flow, and that node has exactly one outgoing flow;
5. message flows, then the conversation nodes (a sub-conversation with its block); 6. each `key` with its `prop` lines; 7. `reads` and `writes`; 8. notes and groups; 9. decisions — in a 0.7 document first `inputdata` and `source` statements, then each `bkm` and each `decision` with its block (table rows, then `entry`, `result` and `element` lines, each with its own block); 10. forms; 11. simulation statements (`simulate`, `resource`, `variable`);
12. two-space indentation, one space between tokens, labels always quoted, attributes in the order of the tables in this document (the values of a repeated key together, in written order; in 0.7 the values of `requires`, `params`, `with`, `question`, `answers`, `uri`, `allowed`, `expr` and `default` always quoted), blank line between sections.

Comments are kept. A comment line is attached to the statement that follows it and printed above it; a comment at the end of a line stays at the end of that statement's line. Comments at the end of the document are printed last.

`fmt` is idempotent: formatting a formatted document changes nothing.

## 15. Conformance

An implementation conforms at level **core** if, for every document in `conformance/`:

1. it reports the same set of diagnostic codes (`expected/*.diag`);
2. its `fmt` output equals `expected/*.tok`;
3. its BPMN export, ignoring diagram interchange (`bpmndi`) and attribute order, equals `expected/*.bpmn`.

Level **full** adds the diagnostic codes of `tok check` at the three levels (`expected/*.levels`), DMN and form export (for documents of language 0.7 the DMN files of every decision requirements graph, `expected/NAME.GRAPH.dmn`), FEEL evaluation and simulation with a fixed seed (compared within tolerance, since random number generators differ between languages).

Cases 01–32 are written for languages 0.1 and 0.3; their expected outputs are produced by the pure-Python implementation. Cases 33–46 cover language 0.5, one or more per section (transactions, complex gateway, standard loop, multiple events, instantiation, process data, resources, global tasks and interfaces, correlation, the Camunda codes `C100`–`C110`, the errors `R060`–`R066`, and in cases 44–46 documentation, trigger flags without a value, process names, namespaces of structures, named data inputs and outputs of associations, data of events and shared data objects). Cases 47 and following cover language 0.6: conversations (47), choreographies with sub-choreographies, calls and loop markers (48), the events and gateways of choreographies (49), participant multiplicity (50), the Camunda codes `C120`–`C123` (51) and the errors `R070`–`R074` (52). Cases 53 and following cover language 0.7: decision requirements graphs (53), boxed expressions and DMN types (54), the decision tables of 0.7 (55), FEEL of level 3 in a process with a rule task (56), the Camunda codes `C130`–`C132` (57), the errors `R076`–`R081`, `D003`, `D006`, `D007` and `F002` (58), the statements of 0.7 under an older header (59), the rows of relations and headers (`D008`, `D009`, 60) and an executable decision model for Camunda 8 with names of several words (61), which `conformance/camunda/run.py` also evaluates on Camunda. Cases 62 and following cover language 1.0: a document with the elements of every layer of the language (62), an executable document for Camunda 8 with a process, forms and a decision model (63), which `run.py` runs and evaluates on Camunda, the errors of the header — a version that does not exist and a second header (64), a version without its minor part (65), a version that is not a number (66) — and the deprecations of 1.0 (`R090`, 67). The expected outputs of cases 33 and following are produced by the Rust engine (`conformance/README.md`). The Julia core passes the core level for all cases.

The reference models of the BPMN Model Interchange Working Group in `conformance/miwg` are the conformance gate of BPMN import and export: their semantic content must survive import, printing and export; `miwg.md` lists, per model, what is kept and what is not.

The test cases of the DMN Technology Compatibility Kit in `conformance/dmn-tck` (compliance levels 2 and 3) are the conformance gate of DMN import, FEEL and decision evaluation: every model is imported, printed and evaluated for every test case, and the results must equal the expected values except for the cases listed with a reason in the baseline; `dmn-tck.md` gives the pass counts.

## 16. Layout sidecar

The language has no coordinates (G5). To keep the drawing of a model that moves between a modelling tool and tok, the tools keep it next to the document, in a plain text file `FILE.tok.layout` that never changes the meaning of the model:

```
tok-layout 1
Task_Zlozenie 176 30 100 80
F1 126 70 176 70
Start_Potrzeba/label 82 95 72 14
```

- The first line is `tok-layout 1`. Each further line names an element by its tok identifier: `ID x y w h` is the bounds of a shape, `ID x1 y1 x2 y2 …` the waypoints of an edge, `ID/label x y w h` the bounds of the label of a shape or an edge. Numbers are whole units of the BPMN file. A line with four numbers is a shape or an edge with two waypoints, as the element it names is a node or a flow. Empty lines and lines starting with `#` are ignored.
- `tok import FILE.bpmn -o FILE.tok` writes `FILE.tok.layout` from the diagram of the file (`--no-layout`: not). A file with several diagrams keeps the first one where it is; each further diagram moves below the one before it.
- `tok bpmn`, `tok svg` and `tok export` read `FILE.tok.layout` when it exists next to the document (`--no-sidecar`: not). Shapes whose identifiers are still in the model keep their geometry; new shapes are placed next to a neighbour that kept its place (as far from it as the automatic layout puts them, moved aside from other shapes), and pools, lanes and sub-processes grow to hold new content. An edge keeps its waypoints when both of its ends kept their geometry and it still touches them; other edges are routed again. Elements the sidecar names that are no longer in the model are ignored.

## 17. Example

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
