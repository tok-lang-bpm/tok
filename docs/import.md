# Importing BPMN 2.0 XML and DMN

`tok import model.bpmn` reads a BPMN 2.0 XML file, for example one drawn in Camunda Modeler or on demo.bpmn.io, and prints the same model as tok text. From Python:

```python
from tok.bpmn_import import import_bpmn
from tok.printer import fmt

doc, warnings = import_bpmn("model.bpmn")     # a path, XML text or bytes
print(fmt(doc))
for w in warnings:
    print(w)                                   # "element-id: explanation"
```

Import is the inverse of BPMN export. A document written by tok survives the round trip tok → XML → tok → XML unchanged, apart from diagram coordinates. A model from another tool keeps its structure, names, identifiers and documentation. Everything tok cannot express is reported as a warning: every element, attribute, extension and documentation the import does not keep is named (an extension element or attribute of another tool once per document). The BPMN MIWG reference models are the gate of this promise (`miwg.md`).

Diagram interchange (`bpmndi`) is not part of the text, because tok has no coordinates. `tok import model.bpmn -o model.tok` writes the diagram of the file into the layout sidecar `model.tok.layout` next to the document (`--no-layout`: not), and `tok bpmn`, `tok svg` and `tok export` draw the document with it: unchanged elements keep their geometry, new ones are placed around them (see [Layout sidecar](#layout-sidecar)). Without a sidecar the exporter generates a new layout.

## Mapping

| BPMN 2.0 XML | tok |
|---|---|
| `definitions/@name` | document title |
| `documentation` (the first one) of any element tok exports | `doc="…"` of its statement; of the process of a document without pools on the header, of the definitions of a document with pools on the header |
| `definitions/@id`, `@targetNamespace` | header `id=`, `namespace=` (only when they differ from `definitions` and `http://bpmn.io/schema/bpmn`) |
| `collaboration/@id` | header `collaboration=` (only when it differs from `collaboration`) |
| a single `process` without a collaboration | top-level flow nodes; header `process=` when its id is not `process`; `process-name=` when its name differs from the title |
| `participant` with `processRef` | `pool` with a block; `process=` when the process id is not `POOL-process`; `process-name=` when the process name differs from the one tok derives (the title for one pool with a process, otherwise the pool label) |
| several collaborations | their participants and message flows are merged into the first one (reported) |
| `participant` without `processRef` | black-box `pool` |
| `process` not referenced by any participant (next to a collaboration) | a new pool `PROCESS-pool`, with a warning |
| `process/@isExecutable="true"` | header flag `executable` if all processes are executable, else the pool flag `executable` |
| `laneSet/lane`, `childLaneSet` | nested `lane` blocks; a node goes into the deepest lane that lists it in `flowNodeRef` |
| `startEvent` … `boundaryEvent` | `start`, `end`, `catch`, `throw`, `boundary` (`on=` from `attachedToRef`) |
| `task`, `userTask`, `serviceTask`, `manualTask`, `scriptTask`, `businessRuleTask`, `sendTask`, `receiveTask` | `task`, `user`, `service`, `manual`, `script`, `rule`, `send`, `receive` |
| `callActivity/@calledElement` | `call … process=`; `call … task=` when it names a global task |
| `subProcess`, `subProcess[@triggeredByEvent="true"]` | `sub`, `eventsub` |
| `transaction` (`method`) | `transaction` (`method=`) |
| `exclusiveGateway`, `parallelGateway`, `inclusiveGateway`, `eventBasedGateway`, `complexGateway` | `xor`, `and`, `or`, `eventgw`, `complex` (`activationCondition` → `activation=`) |
| `instantiate="true"` of a receive task or event-based gateway, `eventGatewayType="Parallel"` | flags `instantiate`, `parallel` |
| `messageEventDefinition` | `message="NAME"` (the name of the referenced `message`); without `messageRef` the flag `message` |
| `timerEventDefinition` | `timer=VALUE` (from `timeDuration`, `timeCycle` or `timeDate`); without a time the flag `timer` |
| `signal…`, `error…`, `escalationEventDefinition` | `signal=`, `error=`, `escalation=` with the declaration's name; `code=` from `errorCode` / `escalationCode`; without a reference the flags `signal`, `error`, `escalation` |
| `conditionalEventDefinition` | `condition="…"` (a leading `=` is removed); with an empty condition the flag `condition` |
| `linkEventDefinition/@name` | `link=` |
| `terminateEventDefinition`, `compensateEventDefinition`, `cancelEventDefinition` | flags `terminate`, `compensate`, `cancel` |
| several event definitions on one event, `parallelMultiple="true"` | several trigger attributes (a multiple event), flag `parallel` |
| `cancelActivity="false"`, `isInterrupting="false"` | flag `noninterrupting` |
| `standardLoopCharacteristics` (`loopCondition`, `testBefore`, `loopMaximum`) | flag `loop` (`while=`, `test-before`, `loop-max=`) |
| `multiInstanceLoopCharacteristics` | flag `parallel` or `sequential` |
| `isForCompensation="true"` | flag `compensation` |
| `sendTask/@messageRef`, `receiveTask/@messageRef` | `message=` |
| `scriptTask/script` | `expr=` |
| `sequenceFlow` | `flow A -> B`; `name` → label; `conditionExpression` → `when=` (leading `=` removed; an empty condition gives `when=""`); the source's `default` → flag `default` |
| `messageFlow` | `message A -> B "label"`; `message=` only when the referenced message's name differs from the label |
| `dataObjectReference` (+ `dataState`, `dataObject/@isCollection`) | `data ID "label" state=… collection`; `object="NAME"` when its data object is shared by several references or named differently |
| `dataObject` without a reference, or named directly by a data association | `data` with the identifier of the data object (reported) |
| `dataStoreReference` | `store ID "label"` |
| `itemSubjectRef` of a data object, data store reference, data input or output, property | `type=` |
| `itemDefinition` (`structureRef`, `itemKind`, `isCollection`, `documentation`) | `item ID "documentation" structure= kind= collection`; a `structureRef` with a namespace prefix other than XML Schema keeps it, with `namespace="URI"` |
| `ioSpecification/dataInput`, `dataOutput` of a process or sub-process | `input`, `output` in its block |
| `dataInput`, `dataOutput`, `property` of an activity or an event | `input`, `output`, `property … on=ID`, except a data input or output that only serves one data association and carries nothing tok would not generate (no type, collection flag or documentation, named after the data element, and with identifiers kept the identifier `ID-in-N` or `ID-out-N`) |
| `property` of a process | `property` |
| `resource`, `performer` / `humanPerformer` / `potentialOwner` with `resourceRef` | `resource`, `by=` |
| `globalTask`, `globalUserTask`, `globalManualTask`, `globalScriptTask`, `globalBusinessRuleTask` | `global ID "name" kind=…` |
| `interface`, `operation` (`inMessageRef`, `outMessageRef`, `errorRef`), `operationRef` | `interface` with `operation … in= out= error=`, `operation=` |
| `correlationKey` of the collaboration with `correlationProperty` and its retrieval expressions | `key` with `prop NAME message= path=` |
| `dataInputAssociation/sourceRef` | `reads ACTIVITY DATA`; `input=ID` when the association ends in a declared `input` |
| `dataOutputAssociation/targetRef` | `writes ACTIVITY DATA`; `output=ID` when the association starts at a declared `output` |
| data associations of events | `reads EVENT DATA` (throwing events), `writes EVENT DATA` (catching events) |
| `textAnnotation` + `association` | `note ID "text" on=TARGET` |
| `group` + `categoryValue/@value` | `group ID "label"` (members must be added by hand, see below) |
| `participantMultiplicity` (`minimum`, `maximum`) | `min=`, `max=` on the pool (`min=` is left out when it is 2 and a maximum is given) |
| `conversation`, `subConversation`, `callConversation` of a collaboration (`participantRef`, `messageFlowRef`, `calledCollaborationRef`, `participantAssociation`) | `conversation`, `subconversation` with its block, `callconversation` with `pools=`, `messages=`, `collaboration=`, `map=` |
| `conversationLink` between a participant and a conversation node | the pool joins `pools=` of the node (tok writes one link per pool) |
| `choreography` | `choreography ID "name"` with its block |
| participants of a choreography | the pools they stand for: the pool whose identifier follows `CHOREOGRAPHY-` (as tok exports them), else the only pool with the participant's name, else a new black-box pool |
| `choreographyTask` (`initiatingParticipantRef`, `participantRef`, `messageFlowRef`) | `choreo ID "name" from= to=`; the message of the flow from the initiating participant → `message=`, of the flow back → `reply=` |
| `subChoreography`, `callChoreography` (`calledChoreographyRef`, `participantAssociation`) | `subchoreo … pools= from=` with its block, `callchoreo … pools= from= choreography= map=` |
| `loopType` of a choreography activity | flags `loop`, `parallel`, `sequential` |

Camunda 8 extensions (namespace `http://camunda.org/schema/zeebe/1.0`):

| Extension | tok |
|---|---|
| `zeebe:formDefinition/@formId` on a user task or start event | `form=` |
| `zeebe:calledDecision` | `decision=`, `result=` |
| `zeebe:script` | `expr=` (leading `=` removed), `result=` |
| `zeebe:taskDefinition/@type` on a service or send task | `type=` |
| `zeebe:subscription/@correlationKey` of a message | `correlation=` on every catch, start or boundary event that waits for that message |
| `zeebe:userTask` | implied by `user` in an executable document |
| `adHocSubProcess` (`ordering`, `cancelRemainingInstances`, `completionCondition`, `zeebe:adHoc`) | `adhoc` with `sequential`, `wait`, `until=`, `active=` |
| `zeebe:loopCharacteristics`, `completionCondition` of a multi-instance activity | `collection=`, `item=`, `output=`, `output-item=`, `until=` |
| `zeebe:calledElement` | `process=`, `binding=`, `version=`, `child-vars=none`, `parent-vars=none`, `business-id=` |
| `bindingType`, `versionTag` of `zeebe:calledDecision` and `zeebe:formDefinition` | `binding=`, `version=` |
| `zeebe:ioMapping` | `inputs=`, `outputs=` (pairs `target = expression`) |
| `zeebe:assignmentDefinition`, `zeebe:taskSchedule`, `zeebe:priorityDefinition` | `assignee=`, `candidate-users=`, `candidate-groups=`, `due=`, `follow-up=`, `priority=` |
| `zeebe:taskDefinition/@retries`, `zeebe:taskHeaders`; `zeebe:taskDefinition` on message throw and end events | `retries=`, `headers=`; `type=` |
| a service task with job type `io.camunda:http-json:1` | `connector=rest` with `method=`, `url=`, `body=`, `query=`, `headers-in=`, `result=` |
| a compensation boundary event associated with a handler (`isForCompensation`) | `compensate=HANDLER` on the activity; the boundary event itself is kept (`boundary … compensate`) when it has a name, documentation or, with identifiers kept, an identifier other than `ACTIVITY-compensate` |
| `compensateEventDefinition/@activityRef` | `activity=` |
| `errorEventDefinition` / `escalationEventDefinition` without a reference on a boundary or start event | the flags `error` / `escalation` |
| `zeebe:conditionalFilter/@variableEvents` | `events=` |
| `zeebe:executionListeners`, `zeebe:taskListeners` | `listeners=` |
| `zeebe:versionTag` of a process | `version-tag=` on the header or the pool |

A file that uses any of these constructs is printed with the header `tok 0.3`; a file that uses an element of language 0.5 (transaction, cancel, complex gateway, loop attributes, multiple events, instantiation, process data, resources and performers, global tasks, interfaces, correlation keys, documentation, triggers without a value, process names, namespaces of structures, named data inputs and outputs of associations, data of events, shared data objects, empty conditions) is printed with `tok 0.5`; a file with conversations, choreographies or participant multiplicity is printed with `tok 0.6`; other files keep `tok 0.1`.

## Identifiers

XML identifiers are kept when they are valid tok identifiers (ASCII letters, digits, `_` and `-`, starting with a letter or `_`). Others are converted: Polish letters lose their diacritics, other characters become `_`, and a leading digit gets a `_` prefix (`1end` → `_1end`). All references are rewritten consistently, and each conversion is reported.

An element the import declares that has no identifier, or whose identifier an element before it already has, gets an identifier of its own, named after its element (`startEvent`, `task-2`); references to a repeated identifier lead to the first element that has it. Both are reported. A message flow without both ends is dropped and reported, like a sequence flow to an element that is not imported.

Files whose elements are nested more than 64 levels deep are refused (the BPMN and DMN files of modelling tools nest about a dozen levels); the deepest blocks of a tok document are 64 levels deep as well (`P008`).

Sequence and message flows keep their XML identifier through `id=` only when it differs from the identifier tok derives (`f-SOURCE-TARGET`, see spec §6.1). Models drawn in other tools usually have identifiers like `Flow_0x1y2z3`, so every flow gets its own `flow` line with `id=`. `tok import --no-flow-ids` (Python: `keep_ids=False`) drops these identifiers and gives compact chains; message flows that a conversation names keep theirs. The gateways' default flows are kept either way, because tok records them with the `default` flag rather than by identifier.

## Limits

These elements are reported and skipped, or simplified:

| Element | Behaviour |
|---|---|
| participant associations, conversation associations and message flow associations of a collaboration, correlation keys of conversation nodes and choreographies, global choreography tasks and global conversations | reported and skipped |
| identifiers of conversation links and of the message flows of choreography tasks | generated by tok (`NODE-link-POOL`, `TASK-message`, `TASK-reply`); other identifiers are reported |
| tasks and other activities of processes inside a choreography | reported and skipped; sequence flows to a skipped node are dropped |
| input and output sets of an `ioSpecification` other than one set of all inputs and one of all outputs | reported; tok writes one set of all inputs and one of all outputs |
| `eventDefinitionRef` (reusable event definitions) | skipped |
| lanes inside a sub-process | ignored; the sub-process nodes stay in the sub-process |
| a node in a pool with lanes but in no lane | kept directly in the pool; `tok check` then reports `R030` |
| group membership | BPMN stores no member list; add `members="…"` by hand |
| correlation key of a message awaited by a receive task | not expressible; model the wait as a message catch event |
| process names | a single process is named after the document title; with several pools each process takes its pool's label |
| other extension elements and attributes (Camunda 7, Signavio, Trisotech, …) | reported once per element or attribute name and skipped |
| documentation of an element without `doc=` (the process of a pool, messages, data objects behind references, …), further documentation elements, text formats other than plain text | reported and skipped |
| attributes without a tok equivalent: `isImmediate`, `implementation`, `startQuantity`, `completionQuantity`, `processType`, `isClosed`, expression and type languages, data store capacity, the item of a message | reported and skipped (`isImmediate` once per document) |
| messages, signals, errors, escalations and data stores no element uses, imports, transformations of data associations, BPMN elements tok does not know | reported and skipped |
| `potentialOwner`, `performer` of a user task | `by=` (reported: the export writes `humanPerformer` on user and manual tasks, `performer` elsewhere) |
| line breaks in names and annotation texts | replaced by spaces |

## Round trip

The test suite (`python/tests/test_import.py`) checks:

1. for every example in `examples/` and for a document using pools, lanes, a sub-process, an event sub-process, escalation, an event-based gateway, data and notes: parse → export → import → print → parse → export gives the same semantic XML as the first export;
2. every BPMN model of the companion course imports, prints, parses again without diagnostics, formats stably and re-exports without BPMN errors.

For languages 0.5 and 0.6 the Rust test `rust/tok-core/tests/import_05.rs` checks that every conformance case of these languages without errors survives export → import → print → parse → export with the same semantic XML, without warnings, and is printed with the header of the case (`tok 0.5`, `tok 0.6`).

The Rust test `rust/tok-core/tests/miwg.rs` compares the semantic content of the BPMN MIWG reference models and of the models in `conformance/import` with that of their export after import and printing; `miwg.md` describes it and lists what is kept.

## Layout sidecar

`tok import model.bpmn -o model.tok` also writes `model.tok.layout` (spec §16): one line per shape (`ID x y w h`), per edge (`ID x1 y1 x2 y2 …`) and per label (`ID/label x y w h`) of the diagram, by tok identifier, in whole units of the file, after the header line `tok-layout 1`.

- Elements whose identifiers the import keeps are written under them. Elements tok identifies otherwise (data associations, associations of notes, flows with derived identifiers, generated compensation boundary events, renamed notes) are matched by what they connect. Participant bands of choreography activities are not written; they follow from the bounds of the activity.
- A file with several diagrams (several collaborations, a sub-process drawn in a diagram of its own, choreographies): the first diagram keeps its coordinates and every further one moves below the one before it.
- `tok bpmn`, `tok svg` and `tok export` read the sidecar next to the document (`--no-sidecar`: not). A shape whose identifier is still in the model keeps its bounds and label; an edge keeps its waypoints when both of its ends kept their bounds and it still touches them (within 25 units). New shapes are placed at the distance the automatic layout puts them from a neighbour that kept its place, moved aside from other shapes; pools and lanes grow to the right and sub-processes in both directions to hold them; other edges are routed again. Groups, which the automatic layout does not draw, are drawn with the bounds of the sidecar.
- The sidecar never changes the meaning of the model; deleting it gives the automatic layout again.

The Rust test `rust/tok-bpmn/tests/sidecar.rs` imports every file of `conformance/import` and every MIWG reference model, draws the document with its sidecar and checks that every shape, edge and label of the file that the import keeps is drawn with the geometry of the file (in whole units; the further diagrams of a file moved by one offset each), and that a task added to an imported model is placed next to its predecessor without moving the other shapes.

## Importing DMN

`tok import model.dmn [-o model.tok]` reads a DMN 1.1, 1.2, 1.3, 1.4 or 1.5 file (any file whose root element is a DMN `definitions` is recognised) and prints a document of language 0.7 (spec §9), which is also a valid document of every release 1.x (spec §4.1). The decision model keeps its elements, identifiers, names, requirements, decision logic and types; everything tok does not express is reported on standard error, as for BPMN.

| DMN | tok |
|---|---|
| `definitions/@name`, `@namespace` | document title; header `namespace=` (only when it differs from `http://camunda.org/schema/1.0/dmn`) |
| `inputData` with `variable/@typeRef` | `inputdata ID … type=` |
| `decision` (`question`, `allowedAnswers`, `variable/@typeRef`) | `decision ID … question= answers= type=`; `type=` is left out when it repeats the type of a table's only output |
| `businessKnowledgeModel` with `encapsulatedLogic` (`formalParameter`s, `@typeRef`) | `bkm ID … params="name: type, …" type=` |
| `knowledgeSource` (`type`, `@locationURI`) | `source ID … kind= uri=` |
| `informationRequirement`, `knowledgeRequirement`, `authorityRequirement` (`href="#id"`, also with the namespace of the file before `#`) | `requires="ID …"` |
| `@name`, `@label`, `description` | `name=` (when it is not the tok identifier), label, `doc=` |
| `literalExpression/text` | `expr="…"` |
| `decisionTable` (`hitPolicy`, `aggregation`, `input`/`inputExpression`, `inputValues`, `output`, `outputValues`, `defaultOutputEntry`, `rule` with entries, `description` and `annotationEntry`) | table rows; `hit=`, `aggregation=`, header cells `name: type = values`, `default=`; rule annotations in the annotation column; an input or output without `typeRef` has the type `any`, a single output without a name the name of its decision |
| `context` with `contextEntry`s | `entry NAME [type=]` lines; the entry without a variable is `result` |
| `list` | `element` lines |
| `relation` (`column`s, `row`s) | rows without `\|\|` |
| `invocation` with `binding`s | `invoke=BKM with="param = FEEL; …"` |
| `functionDefinition` (`kind="FEEL"`) in a context, a list or a function | `params=` on its line; as the logic of a decision or of a business knowledge model, a `result` line with `params=` |
| `conditional`, `filter`, `for`, `some`, `every` (DMN 1.5) whose parts are literal expressions | the equivalent literal expression (`if (c) then (a) else if (c) = false then (b) else null`, `(l)[m]`, `for x in l return (e)`, `some x in l satisfies (c)`) |
| `@typeRef` of an expression in a box without a type | `type=` of the line |
| `itemDefinition` (`typeRef`, `allowedValues` or `typeConstraint`, `isCollection`, `itemComponent`s) | `item ID … type= allowed= collection` with component lines; an item without a base type and components gets `type=Any` |

FEEL text is kept as it is written; in table cells, line breaks outside strings become spaces and inside strings the escape `\n`, because a row is one line. DMN identifiers that are not tok identifiers are converted as for BPMN (`Identifiers`).

Not kept, and reported: decision services, imports of other models and requirements of elements of other models, text annotations, associations and groups, organisation units and the owners of knowledge sources, performance indicators, objectives, decision makers and owners, using processes and tasks, function items of item definitions, function definitions of kind Java or PMML, bindings and relation cells that are not literal expressions, decision tables and relations with a header or row cell tok cannot write in a row (an input expression or output name with `=` outside a string, a cell with `|` or `#` outside a string, a type with a prefix other than `feel:`), and the diagram (DMNDI): tok lays out the decision requirements diagram again on export. The DMN files tok exports import back to the same document, apart from comments and the order of `requires=` (grouped by the kind of requirement).

The test cases of the DMN Technology Compatibility Kit (`dmn-tck.md`) are imported, printed, parsed again and evaluated; the import must keep everything their evaluation needs.
