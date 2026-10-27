# tok — Language Specification, version 0.1

tok is a plain-text language for information-process models. A tok document describes a BPMN 2.0 process (pools, lanes, events, activities, gateways, flows, data), the DMN decision tables it calls, the forms people fill in, and the parameters needed to simulate it. Every tool built on tok reads the same text and produces the same projections: BPMN 2.0 XML with diagram layout, SVG, DMN XML, Camunda form JSON and simulation logs.

This document is normative. The reference implementation is the Python package in `python/`; the Julia package in `julia/` implements the core (parsing, checking, formatting, BPMN export) against the same conformance corpus.

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

A document is UTF-8 text. It is read line by line; a line ends at `
`, `

` or `
`. A byte order mark at the start is ignored.

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

## 4. Document structure

The first statement is the header:

```
tok 0.1 "Interlibrary loan" executable
```

| Part | Meaning |
|---|---|
| `tok 0.1` | language version (required) |
| label | document title; becomes `definitions/@name` and the default process name |
| `id=` | identifier of the definitions element (default: `definitions`) |
| `executable` | flag: every process in the document is executable (`isExecutable="true"`) |
| `target=camunda` | the execution platform for export; see §10 |

After the header come top-level statements in any order:

- **flow nodes** (§5), **flows** (§6), **data** (§7) and **artifacts** (§8) — in a document without pools these form one process whose identifier is `process` (override with `process=ID` in the header);
- **pools** with their lanes and flow nodes (§5.5);
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

Trigger attributes (at most one per event):

| Attribute | Trigger | Allowed on |
|---|---|---|
| `message="Name"` | message | start, end, catch, throw, boundary |
| `timer=VALUE` | timer: ISO 8601 duration (`P14D`, `PT2H`), date-time (`2027-01-15T09:00`), or cycle (`R3/PT1H`, quote it if it contains a space) | start, catch, boundary |
| `signal="Name"` | signal | start, end, catch, throw, boundary |
| `error="Name"` | error | end, boundary, start of an event sub-process |
| `escalation="Name"` | escalation | end, throw, boundary, start of an event sub-process |
| `condition="FEEL"` | conditional | start, catch, boundary |
| `link="Name"` | link | catch, throw |
| `terminate` (flag) | terminate | end |
| `compensate` (flag) | compensation | throw, end, boundary |

Other event attributes: `on=ID` (boundary only), `noninterrupting` (flag; boundary events and event sub-process starts), `correlation="FEEL"` (message catch events in executable documents, §10).

Messages, signals, errors and escalations are declared implicitly by name. Two triggers with the same name refer to the same declaration. `error` and `escalation` accept an optional `code="…"`.

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
| `loop` (flag) | standard loop |
| `parallel` / `sequential` (flag) | multi-instance, parallel or sequential |
| `compensation` (flag) | the activity is a compensation handler |
| `message="Name"` | message of a `send` or `receive` task |
| `process=ID` | called process of a `call` activity |
| `form=ID` | form shown in a `user` task (§10.2) |
| `decision=ID` | decision called by a `rule` task (§9) |
| `expr="FEEL"` | expression evaluated by a `script` task |
| `result=NAME` | variable that receives the result of `decision=` or `expr=` |
| `type="job-type"` | job type of a `service` or `send` task in an executable model |
| `by=ID` | resource that performs the activity in simulation (§11) |
| `duration="DIST"` | processing time in simulation (§11) |
| `assign="NAME=DIST; …"` | variables set when the activity completes in simulation (§11) |

### 5.3 Sub-processes

```
sub ID ["Label"] [attributes]
  ...flow nodes, flows and data of the sub-process...
eventsub ID ["Label"]
  start ID ["Label"] message="Cancel" noninterrupting
  ...
```

`sub` is an embedded sub-process; its block is a separate scope for sequence flows (§6). `eventsub` is an event sub-process: it has no incoming or outgoing sequence flows and its block starts with exactly one start event with a trigger.

### 5.4 Gateways

| Kind | BPMN element |
|---|---|
| `xor` | `exclusiveGateway` |
| `and` | `parallelGateway` |
| `or` | `inclusiveGateway` |
| `eventgw` | `eventBasedGateway` |

A splitting gateway's label should be a question (`"Czy książka dostępna?"`); conditions and labels belong to its outgoing flows (§6).

### 5.5 Pools and lanes

```
pool ID ["Label"] [process=ID] [executable]
  lane ID ["Label"]
    ...flow nodes...
  lane ID ["Label"]
    ...
pool ID ["Label"]            # no block: a black-box pool
```

- A pool with a block is a participant together with its process. The process identifier is `process=` or, by default, the pool identifier followed by `-process`.
- A pool without a block is a black-box participant (no process).
- A lane's block declares the flow nodes that belong to that lane. Lanes may nest. If a pool has lanes, every top-level flow node of its process must be inside a lane (`R030`).
- If a document contains at least one pool, flow nodes may not appear at top level (`R031`).

## 6. Flows

### 6.1 Sequence flows

```
flow A -> B -> C                        # a chain: two flows
flow G -> D "tak" when="zgoda = true"   # one flow with a label and a condition
flow G -> E "nie" default               # default flow of G
```

- A `flow` line contains two or more node identifiers separated by `->`.
- A line with a label or attributes must contain exactly one arrow (`R020`). Label and attributes describe that flow.
- Flow attributes: `when="FEEL"` (condition expression), `default` (flag: the default flow of a splitting `xor`/`or` gateway or activity), `p=NUMBER` (branch probability in simulation, §11), `id=ID` (explicit identifier).
- Flow identifiers are derived as `f-SOURCE-TARGET`, with `-2`, `-3`… on repeated pairs, unless `id=` is given.
- Flow lines may appear anywhere: at top level, inside a pool, lane or sub-process block. The container of a flow is determined by its endpoints; both endpoints must be in the same process or sub-process (`R021`).

### 6.2 Message flows

```
message A -> B ["Label"] [message="Name"]
```

`A` and `B` are flow nodes or pools in different pools (`R022`). The label names the message flow; `message=` links it to a message declaration (default: the label).

## 7. Data

```
data ID ["Label"] [state="złożony"] [collection]
store ID ["Label"]
reads ACTIVITY DATA
writes ACTIVITY DATA
```

- `data` declares a data object reference (and its data object). Several `data` statements may share a label to show different states of one document; each needs its own identifier.
- `store` declares a data store reference; its data store is shared by all references with the same label.
- `reads` creates a data input association from the data element to the activity; `writes` creates a data output association.
- Data elements belong to the process (or sub-process) in whose block they are declared, or to the process of the first activity that reads or writes them.

## 8. Artifacts

```
note ID "Text" [on=ID]
group ID "Label" members="ID ID ID"
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

The simulation produces an **event log** (one row per activity instance: case, activity, resource, enqueue, start, end) and **indicators**: number of cases per end event, cycle time (mean, median, 90th percentile), waiting and processing time per activity, utilization per resource.

## 12. FEEL subset

Conditions, script expressions and decision cells use a subset of FEEL (DMN 1.5, chapter 10). The subset is defined in `feel.md`. Expressions outside the subset are passed unchanged to the exported XML and reported by `tok check` as `F001` (a warning); the simulator cannot evaluate them.

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
| `basic` | `start`, `end`, `task`, `user`, `service`, `manual`, `xor`, `and`, pools, lanes, message flows, data, notes |
| `analytic` | everything except the executable profile, forms and decisions |
| `full` (default) | everything |

## 14. Canonical form

`tok fmt` prints a document in this order and style:

1. the header;
2. top-level flow nodes in declaration order, then pools (each with its lanes and nodes in declaration order);
   `data` and `store` statements are printed at the end of the block they were declared in (pool, sub-process or top level);
3. all sequence flows, in declaration order, as chains: a chain continues while the next flow has no label or attributes, starts at the last node of the previous flow, and that node has exactly one outgoing flow;
4. message flows; 5. `reads` and `writes`; 6. notes and groups; 7. decisions; 8. forms; 9. simulation statements (`simulate`, `resource`, `variable`);
10. two-space indentation, one space between tokens, labels always quoted, attributes in the order of the tables in this document, blank line between sections.

Comments are kept. A comment line is attached to the statement that follows it and printed above it; a comment at the end of a line stays at the end of that statement's line. Comments at the end of the document are printed last.

`fmt` is idempotent: formatting a formatted document changes nothing.

## 15. Conformance

An implementation conforms at level **core** if, for every document in `conformance/`:

1. it reports the same set of diagnostic codes (`expected/*.diag`);
2. its `fmt` output equals `expected/*.tok`;
3. its BPMN export, ignoring diagram interchange (`bpmndi`) and attribute order, equals `expected/*.bpmn`.

Level **full** adds DMN and form export, FEEL evaluation (`expected/*.eval.json`) and simulation with a fixed seed (`expected/*.sim.json`, compared within tolerance, since random number generators differ between languages).

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
