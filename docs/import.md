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
| `callActivity/@calledElement` | `call … process=` |
| `subProcess`, `subProcess[@triggeredByEvent="true"]` | `sub`, `eventsub` |
| `exclusiveGateway`, `parallelGateway`, `inclusiveGateway`, `eventBasedGateway` | `xor`, `and`, `or`, `eventgw` |
| `messageEventDefinition` | `message="NAME"` (the name of the referenced `message`) |
| `timerEventDefinition` | `timer=VALUE` (from `timeDuration`, `timeCycle` or `timeDate`) |
| `signal…`, `error…`, `escalationEventDefinition` | `signal=`, `error=`, `escalation=` with the declaration's name; `code=` from `errorCode` / `escalationCode` |
| `conditionalEventDefinition` | `condition="…"` (a leading `=` is removed) |
| `linkEventDefinition/@name` | `link=` |
| `terminateEventDefinition`, `compensateEventDefinition` | flags `terminate`, `compensate` |
| `cancelActivity="false"`, `isInterrupting="false"` | flag `noninterrupting` |
| `standardLoopCharacteristics` | flag `loop` |
| `multiInstanceLoopCharacteristics` | flag `parallel` or `sequential` |
| `isForCompensation="true"` | flag `compensation` |
| `sendTask/@messageRef`, `receiveTask/@messageRef` | `message=` |
| `scriptTask/script` | `expr=` |
| `sequenceFlow` | `flow A -> B`; `name` → label; `conditionExpression` → `when=` (leading `=` removed); the source's `default` → flag `default` |
| `messageFlow` | `message A -> B "label"`; `message=` only when the referenced message's name differs from the label |
| `dataObjectReference` (+ `dataState`, `dataObject/@isCollection`) | `data ID "label" state=… collection` |
| `dataStoreReference` | `store ID "label"` |
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

## Identifiers

XML identifiers are kept when they are valid tok identifiers (ASCII letters, digits, `_` and `-`, starting with a letter or `_`). Others are converted: Polish letters lose their diacritics, other characters become `_`, and a leading digit gets a `_` prefix (`1end` → `_1end`). All references are rewritten consistently, and each conversion is reported.

Sequence and message flows keep their XML identifier through `id=` only when it differs from the identifier tok derives (`f-SOURCE-TARGET`, see spec §6.1). Models drawn in other tools usually have identifiers like `Flow_0x1y2z3`, so every flow gets its own `flow` line with `id=`. Python callers can pass `keep_ids=False` to drop these identifiers and get compact chains. The gateways' default flows are kept either way, because tok records them with the `default` flag rather than by identifier.

## Limits

These elements are reported and skipped, or simplified:

| Element | Behaviour |
|---|---|
| `complexGateway`, choreographies, conversations, global tasks, participant associations, correlation keys | skipped; sequence flows to a skipped node are dropped |
| `transaction`, `adHocSubProcess` | imported as an embedded `sub` |
| `ioSpecification` (inputs, outputs, input and output sets) | skipped; `reads` and `writes` are kept |
| several event definitions on one event (multiple, parallel multiple) | only the first one is imported |
| `eventDefinitionRef` (reusable event definitions) | skipped |
| `activityRef` of a compensation event | skipped |
| lanes inside a sub-process | ignored; the sub-process nodes stay in the sub-process |
| a node in a pool with lanes but in no lane | kept directly in the pool; `tok check` then reports `R030` |
| group membership | BPMN stores no member list; add `members="…"` by hand |
| correlation key of a message awaited by a receive task | not expressible; model the wait as a message catch event |
| process names | a single process is named after the document title; with several pools each process takes its pool's label |
| other extension elements (Camunda 7, Signavio, `zeebe:ioMapping`, task headers, …) and `documentation` | skipped |
| line breaks in names | replaced by spaces |

## Round trip

The test suite (`python/tests/test_import.py`) checks:

1. for every example in `examples/` and for a document using pools, lanes, a sub-process, an event sub-process, escalation, an event-based gateway, data and notes: parse → export → import → print → parse → export gives the same semantic XML as the first export;
2. every BPMN model of the companion course imports, prints, parses again without diagnostics, formats stably and re-exports without BPMN errors.
