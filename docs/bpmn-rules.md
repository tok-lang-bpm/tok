# BPMN 2.0.2 rules enforced by `tok check`

`tok check` enforces rules of the BPMN 2.0.2 specification (OMG formal/13-12-09) on two layers: the tok checker reports violations in the text with `R` codes (they point to the tok line), and after the tok-level checks pass the exported BPMN XML goes through the BPMN validator shared with the course tool `bpmn.py` (`E`, `W` and `L` codes, see `diagnostics.md`). This page lists every rule with the clause of the specification it comes from and its code. Clause numbers are those of BPMN 2.0.2; "Table" and "Figure" refer to its numbered tables and figures.

Severity: an **error** breaks a rule the specification states with MUST or by its metamodel; a **warning** is a modelling practice the specification recommends (SHOULD, or the execution semantics of chapter 13 show the model cannot work as drawn); a **note** is a style convention of the course, not a BPMN rule.

## Document and identifiers

| Rule | Clause | Code |
|---|---|---|
| The file is well-formed XML of the BPMN 2.0 model namespace | 8.2.3, 15 (Exchange Formats) | E001 |
| The root element is `definitions` | 8.2.1 | E002 |
| Identifiers are unique in the model | 8.3.1 (Base Element, `id`) | E003; R001 in tok |
| References name existing elements | 8.3.1, 8.2.3 (IDREF and QName references) | E004; R002 in tok |
| A reference names a declaration of the right type (message, signal, error, escalation) | 8.4.3, 8.4.4, 8.4.11, 10.5.5 | E018; R003 in tok |
| Identifiers are XML names (ASCII in tok) | 8.3.1 (`id` is `xsd:ID`) | R004 |

## Sequence flows

| Rule | Clause | Code |
|---|---|---|
| A sequence flow has a source and a target flow node, and does not connect data, pools or artifacts | 8.4.13, 7.6.1 (Table 7.3) | E005 |
| A sequence flow does not cross the boundary of a pool or a sub-process | 7.6.1, 9.3, 10.3.5 | E006; R021 in tok |
| A start event has no incoming, an end event no outgoing sequence flow | 10.5.2, 10.5.3, 7.6.1 | E011 |
| A boundary event has no incoming sequence flow | 10.5.4 | E012 |
| A default flow leaves the element that names it and has no condition | 8.4.13 (`default`), 10.6.2, 10.6.3 | E015 |
| A parallel gateway has no conditions and no default flow on its outgoing flows | 10.6.4 | E017 |
| A compensation activity has no incoming or outgoing sequence flows | 10.7.1, 10.3 (`isForCompensation`) | E014; R051 in tok |
| An event sub-process has no incoming or outgoing sequence flows and starts with exactly one triggered start event | 10.3.5 (Event Sub-Process) | E013 |

## Message flows and collaboration

| Rule | Clause | Code |
|---|---|---|
| A message flow connects two different participants (pools or elements inside them), never two elements of one pool | 9.4, 7.6.2 (Table 7.4) | E007, E008; R022 in tok |
| A message flow ends at a message event, a task, a sub-process or a pool, not at a gateway, data or a non-message event | 7.6.2 (Table 7.4), 9.4 | E009 |
| In a model with several pools a message task or event should have a message flow | 9.4 | W054 |
| Correlation keys belong to a collaboration (`tCollaboration`; a process has none) | 8.4.2, 9.5.8 | R013 (`key` without pools) |
| Each correlation property retrieves its value from a message with a message path | 8.4.2 (Tables 8.31–8.33) | R013 (`prop` without `message=` or `path=`) |
| A participant multiplicity has whole numbers with `minimum` ≤ `maximum` and `maximum` ≥ 1 (default maximum 1) | 9.2 (Participant Multiplicity, Table 9.4) | R074 |

## Conversations

| Rule | Clause | Code |
|---|---|---|
| A conversation node links two or more participants of the collaboration | 9.4 (Conversation Node), 9.4.1 | R070 (R002 for an unknown pool) |
| The nodes of a sub-conversation link participants of the sub-conversation | 9.4.2 (Sub-Conversation) | R070 |
| The message flows of a conversation node are exchanged between its participants | 9.4.1 (Conversation, `messageFlowRefs`) | R071 |
| A participant association of a call relates a participant of the called collaboration (inner) with one of the call (outer) | 9.4.3 (Call Conversation), 9.5.6 (Participant Association) | R070 |
| A conversation link connects a participant and a conversation node | 9.4.4 (Conversation Link) | exporter (one link per pool of a top-level node) |

## Choreographies

| Rule | Clause | Code |
|---|---|---|
| A choreography task has two participants, one of them initiating, and at least one message flow from the initiating participant | 11.5.1 (Choreography Task, Table 11.2) | R072; exporter (`TASK-message`) |
| A sub-choreography or call choreography has two or more participants, one of them initiating | 11.5.2, 11.5.3 (Tables 11.4, 11.5) | R072 |
| The participants of the activities inside a sub-choreography are participants of the sub-choreography | 11.5.2 | R072 |
| A call choreography calls a choreography; its participant associations relate participants of the called choreography with its own | 11.5.3 | R072 |
| A choreography contains choreography activities, events, gateways and sequence flows, no activities of processes and no data | 11.3, 11.4, 11.5 | R073 (P004 for data, notes and groups) |
| Start events: none, timer, conditional, signal (or several of these); intermediate events in normal flow: none, timer, conditional, link, catching signal; boundary events: timer, conditional, signal; end events: none, terminate | 11.6.1–11.6.3 (Tables 11.6–11.8) | R073 |
| A choreography has flow nodes | 11.1 | R073 |
| The message flows of a choreography are shown by the participant bands of its activities, not as edges | 12.2.3 (Choreography Activity shapes, participant bands) | D02 does not apply |

## Lanes

| Rule | Clause | Code |
|---|---|---|
| A flow node belongs to at most one lane of a lane set | 10.8 | E030 |
| If a process has lanes, every flow node is in a lane | 10.8 | W031; R030 in tok |
| A lane refers to flow nodes of its own process, not to elements inside a sub-process | 10.8 | W030 |
| In a document with pools, flow nodes are inside pools | 9.3 | R031 (tok structure) |

## Events

| Rule | Clause | Code |
|---|---|---|
| Each event position allows only the triggers of its table: start of a process, start of an event sub-process, start of an embedded sub-process (none only), end, intermediate catch, intermediate throw, boundary | 10.5.2 (Tables 10.84–10.86), 10.5.3 (Table 10.88), 10.5.4 (Tables 10.89, 10.90) | E010 |
| A cancel event appears only in a transaction (end) or on its boundary | 10.3.5 (Transaction), 10.5.3, 10.5.4, 10.5.5 | E010; R060 in tok |
| A boundary cancel event always interrupts the transaction | 10.5.4 (Table 10.90) | R060 |
| A boundary event is attached to an activity | 10.5.4 (`attachedToRef`) | E012; R032, R003 in tok |
| A boundary error event always interrupts its activity | 10.5.4 (Table 10.90), 10.5.6 | E012 |
| A boundary compensation event is connected with its handler by an association, not by a sequence flow | 10.7.1, 10.5.4 | E012 |
| A timer event definition gives a time date, a duration or a cycle | 10.5.5 (Timer Event Definition, Table 10.101) | W010 |
| An event with several event definitions is a multiple event; `parallelMultiple` needs a catching event with several definitions | 10.5.1, 10.5.2–10.5.4 (Multiple, Parallel Multiple), 10.5.5 | R062 |
| Terminate, cancel, compensation and link events have exactly one event definition | 10.5.3 (Table 10.88), 10.5.4 (Tables 10.89, 10.90) | R062 (0.5); R012 (0.1, 0.3) |
| A message event refers to a message | 10.5.5 (Message Event Definition, `messageRef`) | L010 (note: the reference is optional in the metamodel) |
| An escalation that is thrown is caught somewhere | 10.5.6 (Handling Events) | W053 |
| A boundary error or escalation event on a sub-process has something inside that throws it | 10.5.6 | W052 |
| Conditional start events of a process take no variable filter (Camunda extension; a conditional start of a process is evaluated without events) | 10.5.2, 10.5.5 (Conditional Event Definition) | R052 |

## Activities

| Rule | Clause | Code |
|---|---|---|
| A receive task that instantiates the process (`instantiate="true"`) has no incoming sequence flows and stands in a process | 10.3.3 (Receive Task, Figure 10.16), 13.2 | R063 |
| `instantiate` belongs to receive tasks (and event-based gateways) only | 10.3.3, 10.6.6 | R063 |
| A call activity calls one callable element: a process or a global task | 10.3.6, 10.3.7 | R065 |
| A global task is one of `globalTask`, `globalUserTask`, `globalManualTask`, `globalScriptTask`, `globalBusinessRuleTask` | 10.3.7 (Table 10.26) | R065 |
| A called global task, an operation and an item definition are declared | 10.3.7, 8.5.3, 8.4.10 | R066 |
| An operation has an input message | 8.5.3 (Table 8.66, `inMessageRef`) | R013 |
| The loop condition, `testBefore` and `loopMaximum` belong to a standard loop; `loopMaximum` is an integer | 10.3.8 (Table 10.28) | R061 |
| A transaction's `method` is `##Compensate`, `##Store` or `##Image` | 10.3.5 (Table 10.21) | R060 |
| An ad-hoc sub-process contains activities and no start or end events | 10.3.5 (Ad-Hoc Sub-Process) | R050 |
| A compensation handler is an activity marked `isForCompensation` | 10.7.1 | R051 |
| The `ioSpecification` of an activity or process has one input set and one output set listing its data inputs and outputs | 10.4.1 (Table 10.58) | exporter (always generated so) |
| A data input association of an activity targets one of the activity's data inputs; a data output association has one of its data outputs as source | 10.4.1 (Data Associations) | exporter (language 0.5) |

## Gateways

| Rule | Clause | Code |
|---|---|---|
| An event-based gateway is followed only by intermediate catch events (message, timer, signal, conditional) or receive tasks, at least two of them | 10.6.6 | E016 |
| An event after an event-based gateway has no other incoming sequence flows | 10.6.6 | W016 |
| An instantiating event-based gateway has no incoming sequence flows; the parallel type is used only for instantiation | 10.6.6 (Table 10.127) | R063 |
| A gateway with one incoming and one outgoing flow does nothing | 10.6 (a gateway controls divergence and convergence) | W020 |
| A branch of an exclusive or inclusive split has a condition, a label or is the default | 10.6.2, 10.6.3 | W021 |
| When an element splits conditionally (it has a default flow, or some outgoing flows have conditions), every other outgoing flow has a condition: the default is taken only when no condition is true | 10.6.2, 10.6.3, 13.3.4 | W022 |
| No two sequence flows have the same source, target and condition | 10.5.1 (a duplicate is the same path twice) | W023 |
| An inclusive split has a default flow, so that at least one branch is taken | 10.6.3 | W050 |
| A split and its join have matching types (an AND split joined by XOR runs the rest several times; an XOR split joined by AND deadlocks); a complex gateway may join any split | 13.4.1–13.4.5 | W051 |
| A complex gateway's activation condition decides when the join fires | 10.6.5 (Tables 10.125, 10.126), 13.4.5 | (simulated, no code) |

## Reachability

| Rule | Clause | Code |
|---|---|---|
| A process has a start event (or an instantiating receive task or event-based gateway) and an end event | 10.5.2, 10.5.3, 13.2 | W040 |
| Every flow node is reachable from a start | 13.2, 13.3.1 | W041 |
| Every path ends at an end event | 10.5.3, 13.2 | W042 |
| A sequence flow does not lead from an element back to itself; a repeated activity is a loop or a decision | 10.3.8 (loop characteristics), 13.3.1 | W043 |

## Data

| Rule | Clause | Code |
|---|---|---|
| A data association connects a data element and an activity | 10.4.1 (Data Associations) | R003 (`reads`/`writes` with a data object, store, input, output or property) |
| A data state transition reads a data object in one state and writes it in another | 10.4.1 (Data Object, `dataState`) | R064 |
| `structureRef` of an item definition is a qualified name; `itemKind` is Information or Physical | 8.4.10 (Table 8.47) | R013 |

## Style notes (course conventions, not BPMN rules)

| Convention | Code |
|---|---|
| The name of a splitting gateway is a question | L020 |
| A gateway either joins or splits, not both | L021 |
| Every task has a name | L030 |

## Diagram interchange

| Rule | Clause | Code |
|---|---|---|
| Every drawn element (flow nodes, process data inputs and outputs, data objects and stores, flows, pools, lanes, annotations) has a shape or an edge | 12.2 (BPMN DI) | D01 |
| Shapes and edges refer to existing elements | 12.2 | D02 |

The `D` codes are not reported by `tok check` (tok generates the diagram); `bpmn.py check` reports them for files drawn elsewhere. The Camunda codes (`C…`) belong to the execution profile of Camunda 8, not to BPMN (`diagnostics.md`).

## Conformance classes

BPMN 2.0.2 (clause 2, Tables 2.1–2.4) defines three sub-classes of process modelling conformance. `tok check --class CLASS` reports every element outside `CLASS` as `R075` with the first class (in the order descriptive, analytic, common-executable, full) that contains it; `tok check --json` reports the smallest class of the document as `"bpmn_class"`. Elements and attributes the tables do not list are only in `full`, which is the whole modelling language including conversations and choreographies.

| Class | BPMN 2.0.2 | Contains |
|---|---|---|
| `descriptive` | Table 2.1 | visible elements of high-level modelling |
| `analytic` | Tables 2.1 and 2.2 | Descriptive and about half of the language: conditions, default flows, loops, more events and gateways, messages |
| `common-executable` | Tables 2.3 and 2.4 | what an executable process needs (not a superset of the other two: no pools, lanes or message flows) |
| `full` | the whole of clause 2 | everything |

The levels of `tok check --level` are teaching levels of the course and do not coincide with the classes:

| tok level | Accepts | Smallest BPMN class of a document at this level |
|---|---|---|
| `basic` | start, end, task, user, service, manual task, exclusive and parallel gateways, pools, lanes, message flows, data, notes | `descriptive` when it has no manual task, no labelled message flow and no data state; else `analytic` (labelled message flows declare messages) or `full` (manual tasks, `state=`) |
| `analytic` | everything except the executable layer, forms and decisions (and the elements of languages 0.5 and 0.6) | `analytic` for the constructs of Table 2.2; `full` for script, business rule and manual tasks, event sub-processes, ad-hoc sub-processes, compensation, data states, intermediate events without a trigger and the other elements outside Tables 2.1–2.2 |
| `full` | everything | any class |

Class of each construct of tok (`D` descriptive, `A` analytic, `E` common executable; a construct in no sub-class belongs to `full` only):

| tok | BPMN | Classes |
|---|---|---|
| `pool`, `lane`, `group`, `store`, `note … on=` (association), `doc=` | participant, lane, group, data store reference, association, documentation | D A |
| `user`, `service`, `call … process=`, `sub`, `xor`, `and`, `start` (no trigger or `message`), `end` (no trigger, `message`, `terminate`), `data`, `reads`, `writes`, `note`, `flow` | userTask, serviceTask, callActivity, subProcess, exclusive and parallel gateways, none and message start and end events, terminate end event, data object, data associations, text annotation, sequence flow | D A E |
| `task`, `start … timer=`, `message` without a label | task, timer start event, message flow | D A |
| `send`, `receive`, `or`, `message … "Label"` or `message=`, `start … signal=`/`condition=`, `end … signal=`/`error=`/`escalation=`, `catch … signal=`/`condition=`/`link=`, `throw … signal=`/`escalation=`/`link=`, boundary message, timer, signal and conditional events (interrupting or not), non-interrupting boundary escalation event, `default` flow of an `or` gateway | Table 2.2 | A |
| `catch … message=`/`timer=`, `throw … message=`, boundary error event, `eventgw`, `loop`, `parallel`/`sequential` (multi-instance), `when=` and `default` on flows leaving `xor`, `when=` on flows leaving `or` | Tables 2.2 and 2.3 | A E |
| `while=` (loop condition), `collection`/`type=` on `data`, `input`, `output`, `item`, `interface` and `operation`, `operation=` on a service task, `resource` and `by=` on a user task (language 0.5), data of events and named data inputs and outputs of associations | Tables 2.3 and 2.4 | E |
| `manual`, `script`, `rule`, `eventsub`, `adhoc`, `transaction`, `complex`, `call … task=`, `global`, `throw` without a trigger, interrupting boundary escalation, compensation, cancel, error and escalation starts, multiple and parallel multiple events, `instantiate`, `test-before`, `loop-max=`, `state=`, `property`, `key`, `type=` on `store`, `operation=` on send and receive tasks, `by=` on other activities, conditions and default flags on flows leaving activities or `complex`, `min=`/`max=`, conversations, choreographies | outside Tables 2.1–2.4 | full |

Execution attributes of Camunda, decisions, forms and simulation statements are not BPMN elements and belong to no class.
