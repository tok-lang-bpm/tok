# Importing BPMN 2.0 XML

`tok import model.bpmn` reads a BPMN 2.0 XML file, for example one drawn in Camunda Modeler or on demo.bpmn.io, and prints the same model as tok text. From Python:

```python
from tok.bpmn_import import import_bpmn
from tok.printer import fmt

doc, warnings = import_bpmn("model.bpmn")     # a path, XML text or bytes
print(fmt(doc))
for w in warnings:
    print(w)                                   # "element-id: explanation"
```

Import is the inverse of BPMN export. A document written by tok survives the round trip tok → XML → tok → XML unchanged, apart from diagram coordinates. A model from another tool keeps its structure, names and identifiers. Everything tok cannot express is reported as a warning.

Diagram interchange (`bpmndi`) is ignored, because tok has no coordinates. The exporter generates a new layout.

## Mapping

| BPMN 2.0 XML | tok |
|---|---|
| `definitions/@name` | document title |
| `definitions/@id`, `@targetNamespace` | header `id=`, `namespace=` (only when they differ from `definitions` and `http://bpmn.io/schema/bpmn`) |
| `collaboration/@id` | header `collaboration=` (only when it differs from `collaboration`) |
| a single `process` without a collaboration | top-level flow nodes; header `process=` when its id is not `process` |
| `participant` with `processRef` | `pool` with a block; `process=` when the process id is not `POOL-process` |
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
| `messageEventDefinition` | `message="NAME"` (the name of the referenced `message`) |
| `timerEventDefinition` | `timer=VALUE` (from `timeDuration`, `timeCycle` or `timeDate`) |
| `signal…`, `error…`, `escalationEventDefinition` | `signal=`, `error=`, `escalation=` with the declaration's name; `code=` from `errorCode` / `escalationCode` |
| `conditionalEventDefinition` | `condition="…"` (a leading `=` is removed) |
| `linkEventDefinition/@name` | `link=` |
| `terminateEventDefinition`, `compensateEventDefinition`, `cancelEventDefinition` | flags `terminate`, `compensate`, `cancel` |
| several event definitions on one event, `parallelMultiple="true"` | several trigger attributes (a multiple event), flag `parallel` |
| `cancelActivity="false"`, `isInterrupting="false"` | flag `noninterrupting` |
| `standardLoopCharacteristics` (`loopCondition`, `testBefore`, `loopMaximum`) | flag `loop` (`while=`, `test-before`, `loop-max=`) |
| `multiInstanceLoopCharacteristics` | flag `parallel` or `sequential` |
| `isForCompensation="true"` | flag `compensation` |
| `sendTask/@messageRef`, `receiveTask/@messageRef` | `message=` |
| `scriptTask/script` | `expr=` |
| `sequenceFlow` | `flow A -> B`; `name` → label; `conditionExpression` → `when=` (leading `=` removed); the source's `default` → flag `default` |
| `messageFlow` | `message A -> B "label"`; `message=` only when the referenced message's name differs from the label |
| `dataObjectReference` (+ `dataState`, `dataObject/@isCollection`) | `data ID "label" state=… collection` |
| `dataStoreReference` | `store ID "label"` |
| `itemSubjectRef` of a data object, data store reference, data input or output, property | `type=` |
| `itemDefinition` (`structureRef`, `itemKind`, `isCollection`, `documentation`) | `item ID "documentation" structure= kind= collection`; a `structureRef` with a namespace prefix other than XML Schema loses the prefix, with a warning |
| `ioSpecification/dataInput`, `dataOutput` of a process or sub-process | `input`, `output` in its block |
| `dataInput`, `dataOutput`, `property` of an activity (other than those its data associations use) | `input`, `output`, `property … on=ACTIVITY` |
| `property` of a process | `property` |
| `resource`, `performer` / `humanPerformer` / `potentialOwner` with `resourceRef` | `resource`, `by=` |
| `globalTask`, `globalUserTask`, `globalManualTask`, `globalScriptTask`, `globalBusinessRuleTask` | `global ID "name" kind=…` |
| `interface`, `operation` (`inMessageRef`, `outMessageRef`, `errorRef`), `operationRef` | `interface` with `operation … in= out= error=`, `operation=` |
| `correlationKey` of the collaboration with `correlationProperty` and its retrieval expressions | `key` with `prop NAME message= path=` |
| `dataInputAssociation/sourceRef` | `reads ACTIVITY DATA` |
| `dataOutputAssociation/targetRef` | `writes ACTIVITY DATA` |
| `textAnnotation` + `association` | `note ID "text" on=TARGET` |
| `group` + `categoryValue/@value` | `group ID "label"` (members must be added by hand, see below) |

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
| a compensation boundary event associated with a handler (`isForCompensation`) | `compensate=HANDLER` on the activity |
| `compensateEventDefinition/@activityRef` | `activity=` |
| `errorEventDefinition` / `escalationEventDefinition` without a reference on a boundary or start event | the flags `error` / `escalation` |
| `zeebe:conditionalFilter/@variableEvents` | `events=` |
| `zeebe:executionListeners`, `zeebe:taskListeners` | `listeners=` |
| `zeebe:versionTag` of a process | `version-tag=` on the header or the pool |

A file that uses any of these constructs is printed with the header `tok 0.3`; a file that uses an element of language 0.5 (transaction, cancel, complex gateway, loop attributes, multiple events, instantiation, process data, resources and performers, global tasks, interfaces, correlation keys) is printed with `tok 0.5`; other files keep `tok 0.1`.

## Identifiers

XML identifiers are kept when they are valid tok identifiers (ASCII letters, digits, `_` and `-`, starting with a letter or `_`). Others are converted: Polish letters lose their diacritics, other characters become `_`, and a leading digit gets a `_` prefix (`1end` → `_1end`). All references are rewritten consistently, and each conversion is reported.

Sequence and message flows keep their XML identifier through `id=` only when it differs from the identifier tok derives (`f-SOURCE-TARGET`, see spec §6.1). Models drawn in other tools usually have identifiers like `Flow_0x1y2z3`, so every flow gets its own `flow` line with `id=`. Python callers can pass `keep_ids=False` to drop these identifiers and get compact chains. The gateways' default flows are kept either way, because tok records them with the `default` flag rather than by identifier.

## Limits

These elements are reported and skipped, or simplified:

| Element | Behaviour |
|---|---|
| choreographies, conversations, participant associations | skipped; sequence flows to a skipped node are dropped |
| input and output sets of an `ioSpecification`, data input and output associations of events | the sets are not stored (tok writes one set of all inputs and one of all outputs) |
| `eventDefinitionRef` (reusable event definitions) | skipped |
| lanes inside a sub-process | ignored; the sub-process nodes stay in the sub-process |
| a node in a pool with lanes but in no lane | kept directly in the pool; `tok check` then reports `R030` |
| group membership | BPMN stores no member list; add `members="…"` by hand |
| correlation key of a message awaited by a receive task | not expressible; model the wait as a message catch event |
| process names | a single process is named after the document title; with several pools each process takes its pool's label |
| other extension elements (Camunda 7, Signavio, …) and `documentation` | skipped |
| line breaks in names | replaced by spaces |

## Round trip

The test suite (`python/tests/test_import.py`) checks:

1. for every example in `examples/` and for a document using pools, lanes, a sub-process, an event sub-process, escalation, an event-based gateway, data and notes: parse → export → import → print → parse → export gives the same semantic XML as the first export;
2. every BPMN model of the companion course imports, prints, parses again without diagnostics, formats stably and re-exports without BPMN errors.

For language 0.5 the Rust test `rust/tok-core/tests/import_05.rs` checks that every conformance case of 0.5 without errors survives export → import → print → parse → export with the same semantic XML, without warnings, and is printed as `tok 0.5`.
