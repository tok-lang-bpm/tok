# Conformance with BPMN 2.0.2 and DMN 1.5

This table lists the element classes of the metamodels of BPMN 2.0.2 (OMG formal/13-12-09, chapters 8–12 and the diagram interchange of chapter 12) and DMN 1.5, with the construct of tok that expresses each, what the export writes and the import reads, and the conformance gate that checks it: the conformance cases (`conformance/cases`, spec §15), the BPMN Model Interchange Working Group models (`miwg.md`) and the DMN Technology Compatibility Kit (`dmn-tck.md`). Rows marked **not in tok** are gaps of the language, reported by the import.

Headline numbers:

- **MIWG**: 22 reference models, 3533 of 3770 semantic facts kept through import, print and export (93.7 %); 7 models completely; every fact not kept is reported by the import (`miwg.md`).
- **DMN TCK**: compliance level 2, 126 of 126 results (100 %), 28 of 28 models; level 3, 3313 of 3331 results (99.5 %), 108 of 114 models (`dmn-tck.md`).
- **Conformance corpus**: 61 cases (`conformance/cases`) whose canonical text, diagnostics and semantic BPMN every implementation reproduces byte for byte, with the DMN files of every decision requirements graph of language 0.7.
- **BPMN rules**: every rule of BPMN 2.0.2 the checker enforces is listed with its clause in `bpmn-rules.md`; the conformance classes Descriptive, Analytic, Common Executable and Full are checked by `tok check --class` (spec §13).

Column *MIWG* names the fact category of `miwg.md` that compares the element (`structure`, `nodes`, `events`, `activities`, `gateways`, `flows`, `data`, `declarations`, `documentation`, `artifacts`, `camunda`) and a model that uses it.

## BPMN 2.0.2

### Infrastructure and foundation (chapter 8)

| Element | tok | Export / import | Cases | MIWG |
|---|---|---|---|---|
| `Definitions` (name, id, target namespace) | the header: title, `id=`, `namespace=` (§4) | both | 01 | structure, every model |
| `Import` | **not in tok** | import reports it | – | C.1.1, C.8.x, C.9.x, C.10.0 (not kept) |
| `BaseElement`: `id` | the identifier of a statement; identifiers that are not tok identifiers are converted (`import.md`) | both | every case | every model |
| `Documentation` | `doc="…"` (plain text, §3) | both; other text formats reported | 44–46 | documentation, C.3.0, C.7.0, C.9.0 |
| `ExtensionElements`, `Extension` | the Camunda extensions of the executable layer (§10); other vendors' extensions are not kept | both for Camunda; import reports the others | 13, 19–30 | camunda, C.8.1, C.9.x; vendor (not kept by design) |
| `ItemDefinition` (structure, kind, collection) | `item … structure= kind= collection` (§7.1) | both | 38 | declarations, C.1.1, C.8.1 |
| `Message` | declared by name: `message="M"` (§5.1) | both; the item of a message is not kept | 13, 24 | declarations, A.4.1, C.8.1 |
| `Signal`, `Error` (code), `Escalation` (code) | by name, `code=` | both | 07, 27 | declarations, C.2.0, C.6.0 |
| `Resource`, `ResourceRole`, `Performer`, `HumanPerformer` | `resource`, `by=` (language 0.5, §7.2) | both; `PotentialOwner` and resource assignment expressions are not kept | 39 | activities, C.1.0, C.7.0 |
| `Expression`, `FormalExpression` | FEEL text in the expression attributes (§12) | both; the expression language attribute is not kept | 13, 56 | flows, A.2.1 |
| `CorrelationKey`, `CorrelationProperty`, `CorrelationPropertyRetrievalExpression` | `key`, `prop` (§6.3) | both | 41 | declarations, C.9.1 |
| `CorrelationSubscription` | `correlation=` of executable documents (Camunda) | both | 13 | camunda |
| `Interface`, `Operation` | `interface`, `operation` (§5.6) | both | 40 | declarations, C.8.1 |
| `EndPoint` | **not in tok** | import reports it | – | – |
| `Category`, `CategoryValue` | the label of a `group` | both | 10 | artifacts, B.1.0 |

### Process (chapter 10)

| Element | tok | Export / import | Cases | MIWG |
|---|---|---|---|---|
| `Process` (executable, name) | a document without pools, or a pool with a block; `executable`, `process=`, `process-name=` (§4, §5.5) | both; `processType` and `isClosed` are not kept | 01, 09, 44 | structure, every model |
| `Task`, `UserTask`, `ServiceTask`, `ManualTask`, `ScriptTask`, `BusinessRuleTask`, `SendTask`, `ReceiveTask` | `task`, `user`, `service`, `manual`, `script`, `rule`, `send`, `receive` (§5.2) | both; `implementation`, `scriptFormat`, `startQuantity` and `completionQuantity` other than 1 are not kept | 02, 13 | nodes, activities, B.2.0, C.3.0 |
| `CallActivity` | `call … process=` or `task=` (§5.2, §5.6) | both | 21, 40 | activities, C.9.0 |
| `GlobalTask`, `GlobalUserTask`, `GlobalManualTask`, `GlobalScriptTask`, `GlobalBusinessRuleTask` | `global … kind=` (§5.6) | both | 40 | declarations |
| `SubProcess`, event sub-process (`triggeredByEvent`) | `sub`, `eventsub` (§5.3) | both | 08, 27 | nodes, B.2.0, C.6.0 |
| `AdHocSubProcess` (ordering, completion condition, cancel remaining instances) | `adhoc … sequential until= wait` (§5.3, §10.3) | both | 20 | activities |
| `Transaction` (method) | `transaction … method=` (§5.3) | both | 33 | activities, C.6.0 |
| `StandardLoopCharacteristics` | `loop while= test-before loop-max=` (§5.2.1) | both | 35 | activities, B.2.0 |
| `MultiInstanceLoopCharacteristics` | `parallel`/`sequential` with `collection=`, `item=`, `output=`, `until=` (§10.3) | both; the BPMN loop cardinality, loop data references, data items, behaviour other than `All` and complex behaviour definitions are not kept (the Camunda extension carries the collection and its items), and a completion condition only in executable processes | 19 | activities, B.2.0 |
| compensation (`isForCompensation`, `CompensateEventDefinition` with `activityRef`) | `compensation`, `compensate=`, `throw … compensate activity=` (§10.3) | both | 26 | activities, events, C.6.0 |
| `ExclusiveGateway`, `ParallelGateway`, `InclusiveGateway`, `EventBasedGateway`, `ComplexGateway` (instantiate, type, activation condition) | `xor`, `and`, `or`, `eventgw … instantiate parallel`, `complex … activation=` (§5.4) | both; the gateway direction is derived from the flows | 03–06, 34, 37 | gateways, A.2.0, B.2.0 |
| `StartEvent`, `EndEvent`, `IntermediateCatchEvent`, `IntermediateThrowEvent`, `BoundaryEvent` (interrupting, attached to) | `start`, `end`, `catch`, `throw`, `boundary … on= noninterrupting` (§5.1) | both | 02, 07 | nodes, events, A.3.0, C.2.0 |
| event definitions: message, timer (duration, date, cycle), signal, error, escalation, conditional, link, terminate, compensation, cancel | the trigger attributes and flags of §5.1 | both | 06, 07, 26, 28, 33 | events, B.2.0, C.2.0 |
| events with several definitions, `parallelMultiple` | several triggers, `parallel` (§5.1.1) | both | 36 | events, B.2.0 |
| event definitions without a reference | trigger flags without a value (language 0.5, §5.1) | both | 45 | events, A.3.0, C.10.0 |
| instantiating `ReceiveTask` | `receive … instantiate` | both | 37 | activities |
| `SequenceFlow` (condition, default) | `flow … when= default` (§6.1) | both; `isImmediate` is not kept | 02, 03 | flows, every model |
| `LaneSet`, `Lane` (nested, flow node references) | `lane` blocks (§5.5) | both | 09 | structure, A.4.0, B.2.0 |
| `DataObject`, `DataObjectReference` (state, collection, item) | `data … state= object= type= collection` (§7) | both | 10, 44 | data, C.5.0 |
| `DataStore`, `DataStoreReference` | `store` | both; capacity, `isUnlimited` and the item of a data store are not kept | 10 | data, C.4.0 |
| `DataInput`, `DataOutput`, `InputSet`, `OutputSet`, `InputOutputSpecification`, `Property` | `input`, `output`, `property` (§7.1) | both; further input and output sets are not kept | 38 | data, C.1.1, C.8.1 |
| `DataInputAssociation`, `DataOutputAssociation` | `reads`, `writes` (with `input=`, `output=`) (§7) | both; transformations and assignments are not kept | 10, 46 | data, C.4.0 |
| `TextAnnotation`, `Association`, `Group` | `note … on=`, `group … members=` (§8) | both; group membership is not stored in BPMN and is reported | 10, 44 | artifacts, B.1.0 |

### Collaboration, conversation and choreography (chapters 9, 11)

| Element | tok | Export / import | Cases | MIWG |
|---|---|---|---|---|
| `Collaboration`, `Participant`, black-box participant | `pool` with or without a block (§5.5) | both; several collaborations are merged into one | 09 | structure, A.4.0, C.4.0 |
| `ParticipantMultiplicity` | `pool … min= max=` (§5.5) | both | 50 | structure |
| `MessageFlow` | `message A -> B` (§6.2) | both | 09 | flows, A.4.0 |
| `Conversation`, `SubConversation`, `CallConversation`, `ConversationLink` | `conversation`, `subconversation`, `callconversation` (§6.4) | both | 47 | structure |
| `ParticipantAssociation` | `map="inner:outer"` on calls (§5.7, §6.4) | both | 47, 48 | structure |
| `Choreography`, `ChoreographyTask`, `SubChoreography`, `CallChoreography` (initiating participant, loop type) | `choreography`, `choreo`, `subchoreo`, `callchoreo` (§5.7) | both | 48, 49 | nodes |
| `PartnerEntity`, `PartnerRole`, `GlobalChoreographyTask`, `GlobalConversation`, `MessageFlowAssociation`, `ConversationAssociation` | **not in tok** | import reports them | – | – |

### Diagram interchange (chapter 12)

| Element | tok | Export / import | Cases | MIWG |
|---|---|---|---|---|
| `BPMNDiagram`, `BPMNPlane`, `BPMNShape`, `BPMNEdge`, `BPMNLabel` | no coordinates in the language (G5): the export lays every diagram out; the import keeps the drawing in the layout sidecar `FILE.tok.layout` (§16), which `tok bpmn`, `tok svg` and `tok export` reuse | export always; import to the sidecar | sidecar tests (`rust/tok-bpmn/tests/sidecar.rs`) | not compared by the gate (diagrams are laid out again) |
| `BPMNLabelStyle`, colours | **not in tok** | – | – | – |

## DMN 1.5

| Element | tok (language 0.7) | Export / import | Cases | DMN TCK |
|---|---|---|---|---|
| `Definitions` | the header (§4) | both | 53 | every model |
| `Import` | **not in tok** | import reports it | – | `0086`, `0089`, `1160` excluded |
| `Decision` (variable, question, allowed answers) | `decision … name= type= question= answers=` (§9.1) | both | 53, 61 | every model |
| `InputData` | `inputdata … name= type=` | both | 53 | every model |
| `BusinessKnowledgeModel` | `bkm … params= type=` | both | 53, 54 | levels 2 and 3 |
| `KnowledgeSource` (type, location) | `source … kind= uri=` | both; the owner is not kept | 53 | – |
| `InformationRequirement`, `KnowledgeRequirement`, `AuthorityRequirement` | `requires=` (the kind of the element gives the kind of requirement) | both | 53, 58 | every model |
| `DecisionService` | **not in tok** | import reports it | – | `0085` excluded, test cases of type `decisionService` not evaluated |
| `ItemDefinition` (type, allowed values, components, collection) | `item … type= allowed= collection` with components (§9.4) | both; function items are not kept | 54 | levels 2 and 3 |
| `LiteralExpression` | `expr=` (§9.2) | both | 53 | every level |
| `DecisionTable` (hit policies, aggregation, input and output values, default output, annotations) | table rows, `hit=`, `aggregation=`, `default=`, `name: type = values`, a last cell after `\|\|` (§9, §9.3) | both; a table whose cells tok cannot write in a row is not kept and is reported | 11, 55 | levels 2 and 3 |
| `Context`, `List`, `Relation`, `Invocation`, `FunctionDefinition` (FEEL) | `entry`/`result`, `element`, rows without `\|\|`, `invoke= with=`, `params=` (§9.2) | both; functions of kind Java or PMML are not kept | 54 | level 3 |
| `Conditional`, `For`, `Some`, `Every`, `Filter` (DMN 1.4 boxed expressions) | imported as the equivalent literal expression (`import.md`) | import | – | level 3 |
| FEEL (DMN 1.5 chapter 10) | conformance level 3 in a 0.7 document (§12, `feel.md` §9) | – | 56 | 3313 of 3331 results of level 3 |
| `TextAnnotation`, `Association`, `Group`, `OrganizationUnit`, `PerformanceIndicator` | **not in tok** | import reports them | – | – |
| DMNDI (`DMNDiagram`, `DMNShape`, `DMNEdge`) | the export lays the decision requirements diagram out; the import does not keep the drawing | export | 53–61 (`expected/*.dmn`) | – |

## Gaps

What tok does not express, for the 1.0 release:

- BPMN: imports, end points, partner entities and roles, global choreography tasks and conversations, message flow and conversation associations, `isImmediate`, expression and type languages, `implementation`, quantities other than 1, `processType` and `isClosed`, potential owners and resource assignment expressions, the capacity of data stores, transformations of data associations, further input and output sets, label styles. All of them are reported by the import (`miwg.md`, *Not kept, and why*).
- DMN: imports, decision services, artifacts, organisation units and performance indicators, Java and PMML functions, and the drawing of an imported decision requirements diagram (`import.md`, *Importing DMN*).
- FEEL: the zone identifiers whose offset depends on the date (no IANA time zone database) and the other results of the TCK baseline (`dmn-tck.md`).
