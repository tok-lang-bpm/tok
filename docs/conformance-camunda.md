# Conformance with Camunda 8

This table lists every element of BPMN, DMN and FEEL that Camunda 8.10 lists as supported in its published coverage (the BPMN coverage and DMN coverage pages of the Camunda 8 documentation), with the construct of tok that expresses it, the conformance case whose expected BPMN or DMN export fixes the translation (`conformance/cases`, compared byte for byte by every implementation, spec §15), and the case of the executable conformance run that deploys it to Camunda 8 Run and drives an instance to its expected end (`conformance/camunda/run.py`, below). Elements Camunda does not execute are listed at the end, with the codes `tok check` reports for them in executable documents.

Executable means: the header or a pool has the flag `executable`, and `target=` is `camunda` (the default). Without it, every element below is still modelled and exported, but the attributes of the executable layer are ignored (`R055`).

## Result

The run on Camunda 8 Run 8.10.7: the executable cases of `conformance/camunda` (documents of language 0.3, exported by the Python implementation of that language; the constructs are those of conformance cases 19–30, whose expected BPMN both the Python implementation and the engine produce) the process of case 63, a document of language 1.0 exported by the native `tok` program, and the decisions of cases 61 and 63 (exported by the native program, evaluated by Camunda and compared with `tok eval` and the expected values):

| Case | Result |
|---|---|
| `multi-instance`, `adhoc`, `call-activity`, `mappings`, `user-task`, `message-job`, `compensation`, `errors`, `conditional`, `listeners`, `connector`, `version-tag` | passed: deployed, started and run to completion, with the user tasks, jobs, messages, errors, variables and headers each case checks |
| `63-executable-10` (language 1.0: the process with its forms and decisions deployed together) | passed: a script in FEEL of conformance level 3 (`sum(for d in dni return d * 0.5)`), the condition of an exclusive gateway, a job with its retries, header and variables, a user task with its assignee and priority, the expected end event, the version tag `1.0` of the process definition |
| `61-executable-decisions` (9 evaluations of 6 decisions) | passed: deployed, evaluated by Camunda, equal to `tok eval` and to the expected values |
| `63-executable-10`, decisions (3 evaluations of 2 decisions, one input named with two words) | passed, as for case 61 |

All 15 runs passed (`PYTHONPATH=python/src python conformance/camunda/run.py`, with `TOK` set to the native program of release 1.0.0), on a fresh data folder of Camunda 8 Run; the process cases ran first and the decision cases last. The run evaluates the decisions last, for a reason that lies in Camunda 8 Run 8.10.7, not in the models: after decisions have been evaluated through the REST API, the exporter to its local secondary storage (H2) can stop on a primary key violation of a decision instance, and from then on searches no longer see new instances, so process cases run afterwards end with a timeout. When that happens, the cases pass again after Camunda 8 Run is stopped and its folders `camunda-data` and `camunda-zeebe-8.10.7/data` are emptied.

## BPMN

| Camunda 8.10 | tok | Conformance case | Camunda run |
|---|---|---|---|
| pool (participant) and lane | `pool ID "Label" executable`, `lane` (spec §5.5) | 09, 13 | every case is a process; pools: 13 (export only) |
| embedded sub-process | `sub` (§5.3) | 08 | export only |
| call activity | `call … process=ID` with `binding=`, `version=`, `child-vars=`, `parent-vars=`, `business-id=` (§10.3) | 21 | `call-activity` |
| event sub-process (interrupting and not) | `eventsub` with a triggered start event, `noninterrupting` (§5.3) | 08, 27, 28 | export only |
| ad-hoc sub-process | `adhoc … active= until= wait` (§5.3, §10.3) | 20 | `adhoc` |
| service task | `service … type= retries= headers=` (§10.3) | 13, 24 | `mappings`, `compensation`, `errors` |
| user task (Camunda user task, form, assignment, schedule, priority) | `user … form= assignee= candidate-users= candidate-groups= due= follow-up= priority=` (§10.1, §10.3) | 12, 13, 23 | `user-task`, `multi-instance`, `listeners` |
| receive task | `receive … message= correlation=` | 09, 41 | export only |
| send task | `send … message= type=` | 24 | export only |
| business rule task (called decision) | `rule … decision= result= binding= version=` | 13, 57 | decisions: cases 61 and 63 (the decisions themselves) |
| script task (FEEL script) | `script … expr= result=` | 13, 63 | `adhoc`, `call-activity`, `multi-instance`, `listeners`, `63-executable-10` (FEEL of level 3) |
| manual task, undefined task | `manual`, `task` | 02 | export only |
| REST outbound connector | `service … connector=rest method= url= result=` (§10.3) | 25 | `connector` |
| exclusive gateway with conditions and default | `xor`, `flow … when= ` / `default` (§6.1, §10.1) | 03, 13 | `multi-instance`, `errors` |
| parallel gateway | `and` | 04 | export only |
| inclusive gateway | `or` | 05 | export only |
| event-based gateway | `eventgw` | 06 | export only |
| parallel and sequential multi-instance (collection, element, output, completion) | `parallel` / `sequential` with `collection= item= output= output-item= until=` (§10.3) | 19 | `multi-instance` |
| compensation (handler, boundary, throw and end events) | `compensate=HANDLER`, `compensation`, `throw … compensate [activity=]`, `end … compensate` (§10.3) | 26 | `compensation` |
| input and output mappings | `inputs= outputs=` (§10.3) | 22 | `mappings`, `errors` |
| execution and task listeners | `listeners=` (§10.3) | 29 | `listeners` |
| version tag | `version-tag=` on the header or a pool (§10.3) | 30 | `version-tag` |
| data objects, data stores (modelling) | `data`, `store`, `reads`, `writes` (§7) | 10 | export only |
| text annotation, group | `note`, `group` (§8) | 10, 44 | export only |
| none start, intermediate throw and end events | `start`, `throw`, `end` without a trigger | 02 | every case |
| message start, intermediate catch and throw, boundary, end events; correlation | `message="M"`, `correlation=` (§5.1, §10.1); a message throw or end event is a job (`type=`, `C022`) | 13, 24 | `message-job` (throw and end) |
| timer start, intermediate, boundary, event sub-process start (duration, date, cycle, expression) | `timer=` (§5.1, §10.3) | 07, 13 | export only |
| error end, boundary (with code, catch-all), event sub-process start; error variables | `error="E" code=`, the flag `error` (§5.1, §10.3) | 07, 27 | `errors` |
| signal start, intermediate catch and throw, boundary, end, event sub-process start | `signal="S"` | 07 | export only |
| conditional start (event sub-process), intermediate catch, boundary; variable filters | `condition=`, `events=` (§10.3) | 28 | `conditional` |
| escalation intermediate throw, end, boundary (with code, catch-all), event sub-process start | `escalation="X" code=`, the flag `escalation` | 07, 27 | export only |
| terminate end event | `end … terminate` | 06, 08 | export only |
| link intermediate catch and throw events | `link="L"` | 06 | export only |
| forms (Camunda Forms) | `form` with `text` and `field` (§10.2), exported as `ID.form` | 12, 63 | `63-executable-10` deploys its start form and the form of its user task with the process |

## DMN

| Camunda 8.10 | tok (language 0.7) | Conformance case | Camunda run |
|---|---|---|---|
| decision requirements graph (decisions, input data, knowledge sources, requirements) | `decision`, `inputdata`, `source`, `requires=` (§9.1) | 53, 61 | case 61 |
| decision table with every hit policy (unique, first, any, priority, collect with sum, count, min, max, rule order, output order), input and output values, default output | table rows, `hit=`, `aggregation=`, `name: type = values`, `default=` (§9.3) | 11, 55, 61 | case 61 (`limit`: first) |
| literal expression | `expr=` (§9.2) | 53, 61 | case 61 (`blokada`) |
| business knowledge model with a literal expression; invocation from FEEL | `bkm … params= expr=`, a call in FEEL (§9.1) | 53, 61 | case 61 (`naleznosc`) |
| boxed context | `entry`, `result` lines (§9.2) | 54, 61 | case 61 (`podsumowanie`) |
| boxed list | `element` lines | 54, 61 | case 61 (`kanaly`) |
| relation | table rows without `\|\|` | 54 | export only |
| boxed invocation (of a business knowledge model) | `invoke= with=` | 54 | export only |
| function definition with a literal body | `params=` on a line | 54 | export only |
| item definitions (types, allowed values, components, collections) | `item … type= allowed= collection` with components (§9.4) | 54, 61 | case 61 (`do-naprawy`: a list of contexts) |
| names with spaces and symbols | `name="…"`; in an executable document the FEEL names are written in backticks (§9.5) | 61 | case 61 (`Zaległe opłaty`, `opłata dzienna`) |
| DMN 1.5 files with DMNDI | `tok export`: one `NAME.dmn` per graph (§9.5) | 53–61 (`expected/*.dmn`) | case 61 deploys the files |

## FEEL

Camunda 8 evaluates FEEL (the FEEL engine of Camunda, DMN 1.4 FEEL with extensions) in BPMN expressions and in decisions.

| Camunda 8.10 | tok | Conformance case | Camunda run |
|---|---|---|---|
| expressions in conditions, mappings, scripts, multi-instance, ad-hoc, timers, assignments, listeners | FEEL without the leading `=` in the expression attributes; static-or-expression attributes take `=` (§10.3) | 13, 19–29 | `conditional`, `mappings`, `multi-instance`, `adhoc`, `user-task` |
| unary tests in decision tables | table cells (§9, §9.3) | 11, 55 | case 61 |
| FEEL of DMN 1.5 conformance level 3 (temporal values, contexts, lists, filters, iteration, quantifiers, ranges, functions, the standard library) | every expression of a 0.7 document (§12, `feel.md` §9) | 56 | case 61 evaluates contexts, filters and lists; the rest is checked by the DMN TCK (`dmn-tck.md`) against the expected values of the TCK, not on Camunda |

## Not executed by Camunda 8.10

| BPMN 2.0.2 element | tok | Code in executable documents | Case |
|---|---|---|---|
| transaction, cancel events | `transaction`, `cancel` | `C100`, `C101` | 33, 42 |
| complex gateway | `complex` | `C102` | 34, 42 |
| standard loop with condition, test or maximum | `loop while= test-before loop-max=` | `C103` | 35, 42 |
| multiple and parallel multiple events | several triggers, `parallel` | `C104`, `C105` | 36, 42 |
| instantiating receive task, instantiating event-based gateway | `instantiate` | `C106`, `C107` | 37, 42 |
| call activity of a global task | `call … task=` | `C108` | 40, 42 |
| operations, correlation keys (ignored) | `operation=`, `key` | `C109`, `C110` (warnings) | 40, 41, 42 |
| conversations, call conversations (ignored), choreographies, participant multiplicity (ignored) | `conversation`, `callconversation`, `choreography`, `min=`/`max=` | `C120`–`C123` | 47–51 |
| business knowledge model whose logic is an invocation; function definition with a body that is not a literal expression; decision without logic | `bkm … invoke=`, `params=` on a boxed line, a `decision` without logic | `C130`–`C132` | 57 |

## Gaps

- The Camunda run executes 13 BPMN cases and 12 decision evaluations. Timers, signals, escalations, link and terminate events, event sub-processes, inclusive and event-based gateways, receive and send tasks, data, and forms as they are shown to a user are covered by the conformance cases (their exported XML is fixed and checked by the BPMN rules) but are not driven on Camunda.
- Decision services, imports of other DMN models and externally defined functions are not in the language (`dmn-tck.md`, Selection).
- The coverage pages of Camunda list what the engine executes; elements Camunda Modeler draws but the engine rejects are the rows of the last table.
