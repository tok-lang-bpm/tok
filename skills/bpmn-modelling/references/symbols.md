# BPMN symbols

Every BPMN 2.0.2 symbol: its name (use it when talking with the user), the element of the OMG specification and whether Camunda 8 executes it in an executable model. The names and groups are the same as in the symbol catalogue of tok, where every symbol has a picture: [docs/symbols.md](https://github.com/tok-lang-bpm/tok/blob/main/docs/symbols.md).

## Start events

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| start event | StartEvent | executed |
| message start event | StartEvent with message trigger | executed |
| timer start event | StartEvent with timer trigger | executed |
| conditional start event | StartEvent with conditional trigger | executed in event sub-processes |
| signal start event | StartEvent with signal trigger | executed |
| multiple start event | StartEvent with multiple trigger | not executed (C104) |
| parallel multiple start event | StartEvent with parallel multiple trigger | not executed (C105) |

## Start events of event sub-processes

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| interrupting message start event of an event sub-process | StartEvent with message trigger in an event sub-process (interrupting) | executed |
| interrupting timer start event of an event sub-process | StartEvent with timer trigger in an event sub-process (interrupting) | executed |
| interrupting conditional start event of an event sub-process | StartEvent with conditional trigger in an event sub-process (interrupting) | executed |
| interrupting signal start event of an event sub-process | StartEvent with signal trigger in an event sub-process (interrupting) | executed |
| interrupting error start event of an event sub-process | StartEvent with error trigger in an event sub-process (interrupting) | executed |
| interrupting escalation start event of an event sub-process | StartEvent with escalation trigger in an event sub-process (interrupting) | executed |
| interrupting compensation start event of an event sub-process | StartEvent with compensation trigger in an event sub-process (interrupting) | executed |
| interrupting multiple start event of an event sub-process | StartEvent with multiple trigger in an event sub-process (interrupting) | not executed (C104) |
| interrupting parallel multiple start event of an event sub-process | StartEvent with parallel multiple trigger in an event sub-process (interrupting) | not executed (C105) |
| non-interrupting message start event of an event sub-process | StartEvent with message trigger in an event sub-process (non-interrupting) | executed |
| non-interrupting timer start event of an event sub-process | StartEvent with timer trigger in an event sub-process (non-interrupting) | executed |
| non-interrupting conditional start event of an event sub-process | StartEvent with conditional trigger in an event sub-process (non-interrupting) | executed |
| non-interrupting signal start event of an event sub-process | StartEvent with signal trigger in an event sub-process (non-interrupting) | executed |
| non-interrupting escalation start event of an event sub-process | StartEvent with escalation trigger in an event sub-process (non-interrupting) | executed |
| non-interrupting multiple start event of an event sub-process | StartEvent with multiple trigger in an event sub-process (non-interrupting) | not executed (C104) |
| non-interrupting parallel multiple start event of an event sub-process | StartEvent with parallel multiple trigger in an event sub-process (non-interrupting) | not executed (C105) |

## Intermediate events

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| catching message intermediate event | IntermediateCatchEvent with message trigger | executed |
| catching timer intermediate event | IntermediateCatchEvent with timer trigger | executed |
| catching conditional intermediate event | IntermediateCatchEvent with conditional trigger | executed |
| catching signal intermediate event | IntermediateCatchEvent with signal trigger | executed |
| catching multiple intermediate event | IntermediateCatchEvent with multiple trigger | not executed (C104) |
| catching parallel multiple intermediate event | IntermediateCatchEvent with parallel multiple trigger | not executed (C105) |
| catching link intermediate event | IntermediateCatchEvent with link trigger | executed |
| intermediate event (none) | IntermediateThrowEvent | executed |
| throwing message intermediate event | IntermediateThrowEvent with message trigger | executed |
| throwing signal intermediate event | IntermediateThrowEvent with signal trigger | executed |
| throwing escalation intermediate event | IntermediateThrowEvent with escalation trigger | executed |
| throwing compensation intermediate event | IntermediateThrowEvent with compensation trigger | executed |
| throwing multiple intermediate event | IntermediateThrowEvent with multiple trigger | not executed (C104) |
| throwing link intermediate event | IntermediateThrowEvent with link trigger | executed |

## Boundary events

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| interrupting message boundary event | BoundaryEvent with message trigger (interrupting) | executed |
| interrupting timer boundary event | BoundaryEvent with timer trigger (interrupting) | executed |
| interrupting conditional boundary event | BoundaryEvent with conditional trigger (interrupting) | executed |
| interrupting signal boundary event | BoundaryEvent with signal trigger (interrupting) | executed |
| interrupting error boundary event | BoundaryEvent with error trigger (interrupting) | executed |
| interrupting escalation boundary event | BoundaryEvent with escalation trigger (interrupting) | executed |
| interrupting multiple boundary event | BoundaryEvent with multiple trigger (interrupting) | not executed (C104) |
| interrupting parallel multiple boundary event | BoundaryEvent with parallel multiple trigger (interrupting) | not executed (C105) |
| cancel boundary event | BoundaryEvent with cancel trigger | not executed (C101) |
| compensation boundary event | BoundaryEvent with compensation trigger | executed |
| non-interrupting message boundary event | BoundaryEvent with message trigger (non-interrupting) | executed |
| non-interrupting timer boundary event | BoundaryEvent with timer trigger (non-interrupting) | executed |
| non-interrupting conditional boundary event | BoundaryEvent with conditional trigger (non-interrupting) | executed |
| non-interrupting signal boundary event | BoundaryEvent with signal trigger (non-interrupting) | executed |
| non-interrupting escalation boundary event | BoundaryEvent with escalation trigger (non-interrupting) | executed |
| non-interrupting multiple boundary event | BoundaryEvent with multiple trigger (non-interrupting) | not executed (C104) |
| non-interrupting parallel multiple boundary event | BoundaryEvent with parallel multiple trigger (non-interrupting) | not executed (C105) |

## End events

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| end event | EndEvent | executed |
| message end event | EndEvent with message trigger | executed |
| error end event | EndEvent with error trigger | executed |
| escalation end event | EndEvent with escalation trigger | executed |
| signal end event | EndEvent with signal trigger | executed |
| compensation end event | EndEvent with compensation trigger | executed |
| multiple end event | EndEvent with multiple trigger | not executed (C104) |
| cancel end event | EndEvent with cancel trigger | not executed (C101) |
| terminate end event | EndEvent with terminate trigger | executed |

## Activities

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| task | Task | executed |
| user task | UserTask | executed |
| manual task | ManualTask | executed |
| service task | ServiceTask | executed |
| script task | ScriptTask | executed |
| business rule task | BusinessRuleTask | executed |
| send task | SendTask | executed |
| receive task | ReceiveTask | executed |
| instantiating receive task | ReceiveTask (instantiate) | not executed (C106) |
| embedded sub-process | SubProcess | executed |
| event sub-process | SubProcess (triggeredByEvent) | executed |
| transaction | Transaction | not executed (C100) |
| ad-hoc sub-process | AdHocSubProcess | executed |
| call activity | CallActivity (process) | executed |
| call activity of a global task | CallActivity (global task) | not executed (C108) |

## Activity markers

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| task with the loop marker | standard loop | not executed (C103) |
| task with the parallel multi-instance marker | parallel multi-instance | executed |
| task with the sequential multi-instance marker | sequential multi-instance | executed |
| task with the compensation marker | compensation activity | executed |

## Gateways

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| exclusive gateway | ExclusiveGateway | executed |
| inclusive gateway | InclusiveGateway | executed |
| parallel gateway | ParallelGateway | executed |
| complex gateway | ComplexGateway | not executed (C102) |
| event-based gateway | EventBasedGateway | executed |
| instantiating exclusive event-based gateway | EventBasedGateway (instantiate) | not executed (C107) |
| instantiating parallel event-based gateway | EventBasedGateway (instantiate, parallel) | not executed (C107) |

## Data

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| data object | DataObjectReference | modelling only |
| collection data object | DataObjectReference (collection) | modelling only |
| data input | DataInput | modelling only |
| data output | DataOutput | modelling only |
| data store | DataStoreReference | modelling only |

## Connections

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| sequence flow | SequenceFlow | executed |
| conditional sequence flow | SequenceFlow with condition from an activity | executed |
| default sequence flow | SequenceFlow (default) | executed |
| message flow | MessageFlow | executed |
| association | Association | modelling only |
| data association | DataOutputAssociation | modelling only |

## Artifacts

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| text annotation | TextAnnotation | modelling only |

## Pools and lanes

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| pool | Participant (pool) with a process | executed |
| pool with two lanes | Lane | executed |
| black-box pool | Participant without a process (black-box pool) | modelling only |
| pool with participant multiplicity | Participant multiplicity | not executed (C123) |

## Conversations

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| conversation | Conversation | not executed (C120) |
| sub-conversation | SubConversation | not executed (C120) |
| call conversation | CallConversation | not executed (C121) |
| conversation link | ConversationLink | not executed (C120) |

## Choreographies

| Symbol (name) | BPMN element | Camunda 8 |
|---|---|---|
| choreography task | ChoreographyTask | not executed (C122) |
| sub-choreography | SubChoreography | not executed (C122) |
| call choreography | CallChoreography | not executed (C122) |
