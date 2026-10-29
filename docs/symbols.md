# BPMN symbols

Every BPMN 2.0.2 symbol that tok draws, with the statement that produces it and whether Camunda 8 executes it. The pictures are made by the tok engine itself (`tools/symbols.py` writes them from small models with `tok bpmn` and `tok svg`), so they are exactly what `tok svg` draws in a diagram. The catalogue with the short descriptions in English and Polish is `symbols/symbols.json`.

"Not executed" means that Camunda 8 does not execute the element; `tok check` reports the code given in executable documents (`conformance-camunda.md`). "Modelling only" elements are exported and kept, and the engine ignores them. Groups (`group ID "Label" members="A B"`) are exported as BPMN groups without a shape in the diagram, so they have no symbol here.

## Start events

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/event-start-none.svg" alt="start event" height="36"> | start event | StartEvent | `start ID` | executed |
| <img src="symbols/event-start-message.svg" alt="message start event" height="36"> | message start event | StartEvent with message trigger | `start ID message="M"` | executed |
| <img src="symbols/event-start-timer.svg" alt="timer start event" height="36"> | timer start event | StartEvent with timer trigger | `start ID timer=P1D` | executed |
| <img src="symbols/event-start-conditional.svg" alt="conditional start event" height="36"> | conditional start event | StartEvent with conditional trigger | `start ID condition="x > 1"` | executed in event sub-processes |
| <img src="symbols/event-start-signal.svg" alt="signal start event" height="36"> | signal start event | StartEvent with signal trigger | `start ID signal="S"` | executed |
| <img src="symbols/event-start-multiple.svg" alt="multiple start event" height="36"> | multiple start event | StartEvent with multiple trigger | `start ID message="A" signal="B"` | not executed (C104) |
| <img src="symbols/event-start-parallel-multiple.svg" alt="parallel multiple start event" height="36"> | parallel multiple start event | StartEvent with parallel multiple trigger | `start ID message="A" message="B" parallel` | not executed (C105) |

## Start events of event sub-processes

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/event-subprocess-start-message.svg" alt="interrupting message start event of an event sub-process" height="36"> | interrupting message start event of an event sub-process | StartEvent with message trigger in an event sub-process (interrupting) | `eventsub … / start ID message="M"` | executed |
| <img src="symbols/event-subprocess-start-timer.svg" alt="interrupting timer start event of an event sub-process" height="36"> | interrupting timer start event of an event sub-process | StartEvent with timer trigger in an event sub-process (interrupting) | `eventsub … / start ID timer=P1D` | executed |
| <img src="symbols/event-subprocess-start-conditional.svg" alt="interrupting conditional start event of an event sub-process" height="36"> | interrupting conditional start event of an event sub-process | StartEvent with conditional trigger in an event sub-process (interrupting) | `eventsub … / start ID condition="x > 1"` | executed |
| <img src="symbols/event-subprocess-start-signal.svg" alt="interrupting signal start event of an event sub-process" height="36"> | interrupting signal start event of an event sub-process | StartEvent with signal trigger in an event sub-process (interrupting) | `eventsub … / start ID signal="S"` | executed |
| <img src="symbols/event-subprocess-start-error.svg" alt="interrupting error start event of an event sub-process" height="36"> | interrupting error start event of an event sub-process | StartEvent with error trigger in an event sub-process (interrupting) | `eventsub … / start ID error="E"` | executed |
| <img src="symbols/event-subprocess-start-escalation.svg" alt="interrupting escalation start event of an event sub-process" height="36"> | interrupting escalation start event of an event sub-process | StartEvent with escalation trigger in an event sub-process (interrupting) | `eventsub … / start ID escalation="X"` | executed |
| <img src="symbols/event-subprocess-start-compensation.svg" alt="interrupting compensation start event of an event sub-process" height="36"> | interrupting compensation start event of an event sub-process | StartEvent with compensation trigger in an event sub-process (interrupting) | `eventsub … / start ID compensate` | executed |
| <img src="symbols/event-subprocess-start-multiple.svg" alt="interrupting multiple start event of an event sub-process" height="36"> | interrupting multiple start event of an event sub-process | StartEvent with multiple trigger in an event sub-process (interrupting) | `eventsub … / start ID message="A" signal="B"` | not executed (C104) |
| <img src="symbols/event-subprocess-start-parallel-multiple.svg" alt="interrupting parallel multiple start event of an event sub-process" height="36"> | interrupting parallel multiple start event of an event sub-process | StartEvent with parallel multiple trigger in an event sub-process (interrupting) | `eventsub … / start ID message="A" message="B" parallel` | not executed (C105) |
| <img src="symbols/event-subprocess-start-message-noninterrupting.svg" alt="non-interrupting message start event of an event sub-process" height="36"> | non-interrupting message start event of an event sub-process | StartEvent with message trigger in an event sub-process (non-interrupting) | `eventsub … / start ID message="M" noninterrupting` | executed |
| <img src="symbols/event-subprocess-start-timer-noninterrupting.svg" alt="non-interrupting timer start event of an event sub-process" height="36"> | non-interrupting timer start event of an event sub-process | StartEvent with timer trigger in an event sub-process (non-interrupting) | `eventsub … / start ID timer=P1D noninterrupting` | executed |
| <img src="symbols/event-subprocess-start-conditional-noninterrupting.svg" alt="non-interrupting conditional start event of an event sub-process" height="36"> | non-interrupting conditional start event of an event sub-process | StartEvent with conditional trigger in an event sub-process (non-interrupting) | `eventsub … / start ID condition="x > 1" noninterrupting` | executed |
| <img src="symbols/event-subprocess-start-signal-noninterrupting.svg" alt="non-interrupting signal start event of an event sub-process" height="36"> | non-interrupting signal start event of an event sub-process | StartEvent with signal trigger in an event sub-process (non-interrupting) | `eventsub … / start ID signal="S" noninterrupting` | executed |
| <img src="symbols/event-subprocess-start-escalation-noninterrupting.svg" alt="non-interrupting escalation start event of an event sub-process" height="36"> | non-interrupting escalation start event of an event sub-process | StartEvent with escalation trigger in an event sub-process (non-interrupting) | `eventsub … / start ID escalation="X" noninterrupting` | executed |
| <img src="symbols/event-subprocess-start-multiple-noninterrupting.svg" alt="non-interrupting multiple start event of an event sub-process" height="36"> | non-interrupting multiple start event of an event sub-process | StartEvent with multiple trigger in an event sub-process (non-interrupting) | `eventsub … / start ID message="A" signal="B" noninterrupting` | not executed (C104) |
| <img src="symbols/event-subprocess-start-parallel-multiple-noninterrupting.svg" alt="non-interrupting parallel multiple start event of an event sub-process" height="36"> | non-interrupting parallel multiple start event of an event sub-process | StartEvent with parallel multiple trigger in an event sub-process (non-interrupting) | `eventsub … / start ID message="A" message="B" parallel noninterrupting` | not executed (C105) |

## Intermediate events

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/event-catch-message.svg" alt="catching message intermediate event" height="36"> | catching message intermediate event | IntermediateCatchEvent with message trigger | `catch ID message="M"` | executed |
| <img src="symbols/event-catch-timer.svg" alt="catching timer intermediate event" height="36"> | catching timer intermediate event | IntermediateCatchEvent with timer trigger | `catch ID timer=P1D` | executed |
| <img src="symbols/event-catch-conditional.svg" alt="catching conditional intermediate event" height="36"> | catching conditional intermediate event | IntermediateCatchEvent with conditional trigger | `catch ID condition="x > 1"` | executed |
| <img src="symbols/event-catch-signal.svg" alt="catching signal intermediate event" height="36"> | catching signal intermediate event | IntermediateCatchEvent with signal trigger | `catch ID signal="S"` | executed |
| <img src="symbols/event-catch-multiple.svg" alt="catching multiple intermediate event" height="36"> | catching multiple intermediate event | IntermediateCatchEvent with multiple trigger | `catch ID message="A" signal="B"` | not executed (C104) |
| <img src="symbols/event-catch-parallel-multiple.svg" alt="catching parallel multiple intermediate event" height="36"> | catching parallel multiple intermediate event | IntermediateCatchEvent with parallel multiple trigger | `catch ID message="A" message="B" parallel` | not executed (C105) |
| <img src="symbols/event-catch-link.svg" alt="catching link intermediate event" height="36"> | catching link intermediate event | IntermediateCatchEvent with link trigger | `catch ID link="L"` | executed |
| <img src="symbols/event-throw-none.svg" alt="intermediate event (none)" height="36"> | intermediate event (none) | IntermediateThrowEvent | `throw ID` | executed |
| <img src="symbols/event-throw-message.svg" alt="throwing message intermediate event" height="36"> | throwing message intermediate event | IntermediateThrowEvent with message trigger | `throw ID message="M"` | executed |
| <img src="symbols/event-throw-signal.svg" alt="throwing signal intermediate event" height="36"> | throwing signal intermediate event | IntermediateThrowEvent with signal trigger | `throw ID signal="S"` | executed |
| <img src="symbols/event-throw-escalation.svg" alt="throwing escalation intermediate event" height="36"> | throwing escalation intermediate event | IntermediateThrowEvent with escalation trigger | `throw ID escalation="X"` | executed |
| <img src="symbols/event-throw-compensation.svg" alt="throwing compensation intermediate event" height="36"> | throwing compensation intermediate event | IntermediateThrowEvent with compensation trigger | `throw ID compensate` | executed |
| <img src="symbols/event-throw-multiple.svg" alt="throwing multiple intermediate event" height="36"> | throwing multiple intermediate event | IntermediateThrowEvent with multiple trigger | `throw ID message="A" signal="B"` | not executed (C104) |
| <img src="symbols/event-throw-link.svg" alt="throwing link intermediate event" height="36"> | throwing link intermediate event | IntermediateThrowEvent with link trigger | `throw ID link="L"` | executed |

## Boundary events

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/event-boundary-message.svg" alt="interrupting message boundary event" height="36"> | interrupting message boundary event | BoundaryEvent with message trigger (interrupting) | `boundary ID on=ACTIVITY message="M"` | executed |
| <img src="symbols/event-boundary-timer.svg" alt="interrupting timer boundary event" height="36"> | interrupting timer boundary event | BoundaryEvent with timer trigger (interrupting) | `boundary ID on=ACTIVITY timer=P1D` | executed |
| <img src="symbols/event-boundary-conditional.svg" alt="interrupting conditional boundary event" height="36"> | interrupting conditional boundary event | BoundaryEvent with conditional trigger (interrupting) | `boundary ID on=ACTIVITY condition="x > 1"` | executed |
| <img src="symbols/event-boundary-signal.svg" alt="interrupting signal boundary event" height="36"> | interrupting signal boundary event | BoundaryEvent with signal trigger (interrupting) | `boundary ID on=ACTIVITY signal="S"` | executed |
| <img src="symbols/event-boundary-error.svg" alt="interrupting error boundary event" height="36"> | interrupting error boundary event | BoundaryEvent with error trigger (interrupting) | `boundary ID on=ACTIVITY error="E"` | executed |
| <img src="symbols/event-boundary-escalation.svg" alt="interrupting escalation boundary event" height="36"> | interrupting escalation boundary event | BoundaryEvent with escalation trigger (interrupting) | `boundary ID on=ACTIVITY escalation="X"` | executed |
| <img src="symbols/event-boundary-multiple.svg" alt="interrupting multiple boundary event" height="36"> | interrupting multiple boundary event | BoundaryEvent with multiple trigger (interrupting) | `boundary ID on=ACTIVITY message="A" signal="B"` | not executed (C104) |
| <img src="symbols/event-boundary-parallel-multiple.svg" alt="interrupting parallel multiple boundary event" height="36"> | interrupting parallel multiple boundary event | BoundaryEvent with parallel multiple trigger (interrupting) | `boundary ID on=ACTIVITY message="A" message="B" parallel` | not executed (C105) |
| <img src="symbols/event-boundary-cancel.svg" alt="cancel boundary event" height="36"> | cancel boundary event | BoundaryEvent with cancel trigger | `boundary ID on=TRANSACTION cancel` | not executed (C101) |
| <img src="symbols/event-boundary-compensation.svg" alt="compensation boundary event" height="36"> | compensation boundary event | BoundaryEvent with compensation trigger | `service ID compensate=HANDLER` | executed |
| <img src="symbols/event-boundary-message-noninterrupting.svg" alt="non-interrupting message boundary event" height="36"> | non-interrupting message boundary event | BoundaryEvent with message trigger (non-interrupting) | `boundary ID on=ACTIVITY message="M" noninterrupting` | executed |
| <img src="symbols/event-boundary-timer-noninterrupting.svg" alt="non-interrupting timer boundary event" height="36"> | non-interrupting timer boundary event | BoundaryEvent with timer trigger (non-interrupting) | `boundary ID on=ACTIVITY timer=P1D noninterrupting` | executed |
| <img src="symbols/event-boundary-conditional-noninterrupting.svg" alt="non-interrupting conditional boundary event" height="36"> | non-interrupting conditional boundary event | BoundaryEvent with conditional trigger (non-interrupting) | `boundary ID on=ACTIVITY condition="x > 1" noninterrupting` | executed |
| <img src="symbols/event-boundary-signal-noninterrupting.svg" alt="non-interrupting signal boundary event" height="36"> | non-interrupting signal boundary event | BoundaryEvent with signal trigger (non-interrupting) | `boundary ID on=ACTIVITY signal="S" noninterrupting` | executed |
| <img src="symbols/event-boundary-escalation-noninterrupting.svg" alt="non-interrupting escalation boundary event" height="36"> | non-interrupting escalation boundary event | BoundaryEvent with escalation trigger (non-interrupting) | `boundary ID on=ACTIVITY escalation="X" noninterrupting` | executed |
| <img src="symbols/event-boundary-multiple-noninterrupting.svg" alt="non-interrupting multiple boundary event" height="36"> | non-interrupting multiple boundary event | BoundaryEvent with multiple trigger (non-interrupting) | `boundary ID on=ACTIVITY message="A" signal="B" noninterrupting` | not executed (C104) |
| <img src="symbols/event-boundary-parallel-multiple-noninterrupting.svg" alt="non-interrupting parallel multiple boundary event" height="36"> | non-interrupting parallel multiple boundary event | BoundaryEvent with parallel multiple trigger (non-interrupting) | `boundary ID on=ACTIVITY message="A" message="B" parallel noninterrupting` | not executed (C105) |

## End events

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/event-end-none.svg" alt="end event" height="36"> | end event | EndEvent | `end ID` | executed |
| <img src="symbols/event-end-message.svg" alt="message end event" height="36"> | message end event | EndEvent with message trigger | `end ID message="M"` | executed |
| <img src="symbols/event-end-error.svg" alt="error end event" height="36"> | error end event | EndEvent with error trigger | `end ID error="E"` | executed |
| <img src="symbols/event-end-escalation.svg" alt="escalation end event" height="36"> | escalation end event | EndEvent with escalation trigger | `end ID escalation="X"` | executed |
| <img src="symbols/event-end-signal.svg" alt="signal end event" height="36"> | signal end event | EndEvent with signal trigger | `end ID signal="S"` | executed |
| <img src="symbols/event-end-compensation.svg" alt="compensation end event" height="36"> | compensation end event | EndEvent with compensation trigger | `end ID compensate` | executed |
| <img src="symbols/event-end-multiple.svg" alt="multiple end event" height="36"> | multiple end event | EndEvent with multiple trigger | `end ID message="A" signal="B"` | not executed (C104) |
| <img src="symbols/event-end-cancel.svg" alt="cancel end event" height="36"> | cancel end event | EndEvent with cancel trigger | `end ID cancel` | not executed (C101) |
| <img src="symbols/event-end-terminate.svg" alt="terminate end event" height="36"> | terminate end event | EndEvent with terminate trigger | `end ID terminate` | executed |

## Activities

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/task-task.svg" alt="task" height="36"> | task | Task | `task ID` | executed |
| <img src="symbols/task-user.svg" alt="user task" height="36"> | user task | UserTask | `user ID` | executed |
| <img src="symbols/task-manual.svg" alt="manual task" height="36"> | manual task | ManualTask | `manual ID` | executed |
| <img src="symbols/task-service.svg" alt="service task" height="36"> | service task | ServiceTask | `service ID` | executed |
| <img src="symbols/task-script.svg" alt="script task" height="36"> | script task | ScriptTask | `script ID` | executed |
| <img src="symbols/task-rule.svg" alt="business rule task" height="36"> | business rule task | BusinessRuleTask | `rule ID` | executed |
| <img src="symbols/task-send.svg" alt="send task" height="36"> | send task | SendTask | `send ID` | executed |
| <img src="symbols/task-receive.svg" alt="receive task" height="36"> | receive task | ReceiveTask | `receive ID` | executed |
| <img src="symbols/task-receive-instantiating.svg" alt="instantiating receive task" height="36"> | instantiating receive task | ReceiveTask (instantiate) | `receive ID message="M" instantiate` | not executed (C106) |
| <img src="symbols/subprocess.svg" alt="embedded sub-process" height="36"> | embedded sub-process | SubProcess | `sub ID (block)` | executed |
| <img src="symbols/subprocess-event.svg" alt="event sub-process" height="36"> | event sub-process | SubProcess (triggeredByEvent) | `eventsub ID (block)` | executed |
| <img src="symbols/subprocess-transaction.svg" alt="transaction" height="36"> | transaction | Transaction | `transaction ID (block)` | not executed (C100) |
| <img src="symbols/subprocess-adhoc.svg" alt="ad-hoc sub-process" height="36"> | ad-hoc sub-process | AdHocSubProcess | `adhoc ID (block)` | executed |
| <img src="symbols/call-activity.svg" alt="call activity" height="36"> | call activity | CallActivity (process) | `call ID process=P` | executed |
| <img src="symbols/call-activity-global-task.svg" alt="call activity of a global user task" height="36"> | call activity of a global user task | CallActivity (global task) | `call ID task=GLOBAL` | not executed (C108) |

## Activity markers

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/marker-loop.svg" alt="task with the loop marker" height="36"> | task with the loop marker | standard loop | `task ID loop` | not executed (C103) |
| <img src="symbols/marker-multi-instance-parallel.svg" alt="task with the parallel multi-instance marker" height="36"> | task with the parallel multi-instance marker | parallel multi-instance | `task ID parallel` | executed |
| <img src="symbols/marker-multi-instance-sequential.svg" alt="task with the sequential multi-instance marker" height="36"> | task with the sequential multi-instance marker | sequential multi-instance | `task ID sequential` | executed |
| <img src="symbols/marker-compensation.svg" alt="task with the compensation marker" height="36"> | task with the compensation marker | compensation activity | `service ID compensation` | executed |

## Gateways

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/gateway-exclusive.svg" alt="exclusive gateway" height="36"> | exclusive gateway | ExclusiveGateway | `xor ID` | executed |
| <img src="symbols/gateway-inclusive.svg" alt="inclusive gateway" height="36"> | inclusive gateway | InclusiveGateway | `or ID` | executed |
| <img src="symbols/gateway-parallel.svg" alt="parallel gateway" height="36"> | parallel gateway | ParallelGateway | `and ID` | executed |
| <img src="symbols/gateway-complex.svg" alt="complex gateway" height="36"> | complex gateway | ComplexGateway | `complex ID` | not executed (C102) |
| <img src="symbols/gateway-event-based.svg" alt="event-based gateway" height="36"> | event-based gateway | EventBasedGateway | `eventgw ID` | executed |
| <img src="symbols/gateway-event-based-instantiating.svg" alt="instantiating exclusive event-based gateway" height="36"> | instantiating exclusive event-based gateway | EventBasedGateway (instantiate) | `eventgw ID instantiate` | not executed (C107) |
| <img src="symbols/gateway-event-based-instantiating-parallel.svg" alt="instantiating parallel event-based gateway" height="36"> | instantiating parallel event-based gateway | EventBasedGateway (instantiate, parallel) | `eventgw ID instantiate parallel` | not executed (C107) |

## Data

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/data-object.svg" alt="data object" height="36"> | data object | DataObjectReference | `data ID` | modelling only |
| <img src="symbols/data-object-collection.svg" alt="collection data object" height="36"> | collection data object | DataObjectReference (collection) | `data ID collection` | modelling only |
| <img src="symbols/data-input.svg" alt="data input" height="36"> | data input | DataInput | `input ID` | modelling only |
| <img src="symbols/data-output.svg" alt="data output" height="36"> | data output | DataOutput | `output ID` | modelling only |
| <img src="symbols/data-store.svg" alt="data store" height="36"> | data store | DataStoreReference | `store ID` | modelling only |

## Connections

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/flow-sequence.svg" alt="sequence flow" height="36"> | sequence flow | SequenceFlow | `flow A -> B` | executed |
| <img src="symbols/flow-conditional.svg" alt="conditional sequence flow" height="36"> | conditional sequence flow | SequenceFlow with condition from an activity | `flow A -> B when="feel"` | executed |
| <img src="symbols/flow-default.svg" alt="default sequence flow" height="36"> | default sequence flow | SequenceFlow (default) | `flow G -> B default` | executed |
| <img src="symbols/flow-message.svg" alt="message flow" height="36"> | message flow | MessageFlow | `message A -> B` | executed |
| <img src="symbols/association.svg" alt="association" height="36"> | association | Association | `note ID "Text" on=ID` | modelling only |
| <img src="symbols/association-data.svg" alt="data association" height="36"> | data association | DataOutputAssociation | `writes ACTIVITY DATA` | modelling only |

## Artifacts

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/annotation.svg" alt="text annotation" height="36"> | text annotation | TextAnnotation | `note ID "Text"` | modelling only |

## Pools and lanes

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/pool.svg" alt="pool" height="36"> | pool | Participant (pool) with a process | `pool ID "Label" (block)` | executed |
| <img src="symbols/lane.svg" alt="pool with two lanes" height="36"> | pool with two lanes | Lane | `lane ID "Label"` | executed |
| <img src="symbols/pool-black-box.svg" alt="black-box pool" height="36"> | black-box pool | Participant without a process (black-box pool) | `pool ID "Label" (no block)` | modelling only |
| <img src="symbols/pool-multiplicity.svg" alt="pool with participant multiplicity" height="36"> | pool with participant multiplicity | Participant multiplicity | `pool ID "Label" min=2 max=5` | not executed (C123) |

## Conversations

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/conversation.svg" alt="conversation" height="36"> | conversation | Conversation | `conversation ID "Label" pools="P Q"` | not executed (C120) |
| <img src="symbols/conversation-sub.svg" alt="sub-conversation" height="36"> | sub-conversation | SubConversation | `subconversation ID "Label" pools="P Q" (block)` | not executed (C120) |
| <img src="symbols/conversation-call.svg" alt="call conversation" height="36"> | call conversation | CallConversation | `callconversation ID "Label" pools="P Q" collaboration=ID` | not executed (C121) |
| <img src="symbols/conversation-link.svg" alt="conversation link" height="36"> | conversation link | ConversationLink | `pools= of a conversation` | not executed (C120) |

## Choreographies

| Symbol | Element | BPMN | tok | Camunda 8 |
|---|---|---|---|---|
| <img src="symbols/choreography-task.svg" alt="choreography task" height="36"> | choreography task | ChoreographyTask | `choreo ID "Label" from=P to=Q message="M"` | not executed (C122) |
| <img src="symbols/choreography-sub.svg" alt="sub-choreography" height="36"> | sub-choreography | SubChoreography | `subchoreo ID "Label" pools="P Q" from=P (block)` | not executed (C122) |
| <img src="symbols/choreography-call.svg" alt="call choreography" height="36"> | call choreography | CallChoreography | `callchoreo ID "Label" pools="P Q" from=P choreography=ID` | not executed (C122) |

