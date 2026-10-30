# BPMN 2.0 elements: recognition and XML templates

## A. How to recognise the element type in the description of a step

Process descriptions rarely name the element directly. It is decided by the words that describe **who does the work and how**, and **what triggers the further course**.

### Tasks

| Cue in the description | Element | Tag |
|---|---|---|
| a person works in a system: fills in a form, marks a decision, "in SAP", "in the ERP", "in the task inbox" | user task | `userTask` |
| the system does it by itself: "automatically", "without human involvement", assigns a number, saves, blocks an amount | service task | `serviceTask` |
| a person without a system: a face-to-face conversation, a phone call, "is not recorded anywhere" | manual task | `manualTask` |
| "manually", but **in a system** ("enters manually into the ERP") — here "manually" means "not automatically" | user task | `userTask` |
| the result follows from a table of rules / a decision table, "nobody assesses it" | business rule task (DMN) | `businessRuleTask` |
| a calculation by a fixed formula, a transformation of data by a script | script task | `scriptTask` |
| sending a message to someone outside the pool — also when the system sends it automatically (the addressee in another pool decides) | send task | `sendTask` |
| waiting for a message as a separate piece of work | receive task | `receiveTask` |
| "several steps form a closed whole", "visible as one block", has its own beginning and end | sub-process (expanded) | `subProcess` |
| repeating the same activity until it succeeds | loop marker | `standardLoopCharacteristics` |
| the same activity for each element of a list | multi-instance marker | `multiInstanceLoopCharacteristics` |

### Events

| Cue | Element |
|---|---|
| the process starts because **a message / an e-mail / a request has arrived** | message start event (sender in another pool: add a message flow; sender inside the organisation that you do not model as a pool: no flow) |
| the process starts **at a given time / periodically** | timer start event |
| the process ends **at the moment a notification is sent** | message end event (addressee in the same pool, e.g. the applicant: no message flow) |
| the process simply ends, nobody is informed | plain end event |
| "everything ends immediately", including parallel paths | terminate end event |
| the process **waits** until someone sends something | message intermediate catch event |
| the process **waits** until time passes | timer intermediate catch event |
| "has N days for it; after that **the task is taken away from them**" | **interrupting** timer boundary event |
| "after the deadline the task **stays with them**, but in parallel…" | **non-interrupting** timer boundary event |
| "if the system reports an error, the activity is interrupted and…" | error boundary event (always interrupting) |
| the block "reports outwards that the case needs intervention higher up", and the work goes on | throwing an **escalation** in the sub-process and a non-interrupting escalation boundary event on the sub-process; if the description says that the path ends with the report — an escalation **end** event, if something follows — an **intermediate** throw event |
| someone announces something **to everyone at once** (to all instances/processes) | **signal** |
| "at any moment of the process…", this part "has no predecessor in the main flow" | **event sub-process** (interrupting when the process is interrupted immediately) |

Deadlines in working days ("3 working days") are usually written as a duration in calendar days (`P3D`). The calendar of working days belongs to the process engine, not to the notation. Record such a simplified interpretation in the mapping.

### Gateways

| Cue | Gateway |
|---|---|
| a question with yes/no answers, exactly one path | exclusive (XOR) `exclusiveGateway` |
| "both activities always, independently, in any order" | parallel (AND) `parallelGateway`, with the same kind of join |
| "one, several or none may happen", "waits for those that were actually started" | inclusive (OR) `inclusiveGateway`, with a default flow and an OR join |
| "nobody makes a decision – the further course depends on **what happens first**" | event-based `eventBasedGateway` |

### Pools, lanes, data

| Cue | Element |
|---|---|
| an external company or institution, "its internal way of working is not known" | a separate **black-box pool** (participant without `processRef`), communication by message flows |
| a role in the organisation that performs steps | a **lane** (`lane`) in the pool of the organisation |
| a system that performs automatic activities | a separate lane "System …" or automatic tasks in the lane of the person responsible (record the decision in the mapping) |
| a document passed between steps | data object `dataObjectReference` (state in brackets, e.g. `[submitted]` → `dataState`) |
| records, a database, a register that lasts longer than the process | data store `dataStoreReference` |

## B. XML templates

Namespaces: `xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"` and `xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"` (needed for expressions). The fragments below go into `<bpmn:process>` unless stated otherwise.

### Declarations (level of `<bpmn:definitions>`)

```xml
<bpmn:message id="Msg_Quotation" name="Quotation"/>
<bpmn:signal id="Sig_Freeze" name="Budget freeze"/>
<bpmn:error id="Err_Save" name="Save error" errorCode="SAVE"/>
<bpmn:escalation id="Esc_Deadline" name="Deadline exceeded" escalationCode="DEADLINE"/>
<bpmn:dataStore id="DataStore_Records" name="Records"/>
```

### Collaboration, pools and message flows

```xml
<bpmn:collaboration id="Collaboration_1">
  <bpmn:participant id="Participant_Org" name="Organisation" processRef="Process_1"/>
  <bpmn:participant id="Participant_Supplier" name="Supplier"/>  <!-- black-box pool -->
  <bpmn:messageFlow id="MF_Request" name="Request for quotation" sourceRef="Task_12" targetRef="Participant_Supplier" messageRef="Msg_Request"/>
  <bpmn:messageFlow id="MF_Quotation" name="Quotation" sourceRef="Participant_Supplier" targetRef="Catch_14" messageRef="Msg_Quotation"/>
</bpmn:collaboration>
```

### Lanes

```xml
<bpmn:laneSet id="LaneSet_1">
  <bpmn:lane id="Lane_Applicant" name="Applicant">
    <bpmn:flowNodeRef>Start_1</bpmn:flowNodeRef>
    <bpmn:flowNodeRef>Task_2</bpmn:flowNodeRef>
  </bpmn:lane>
</bpmn:laneSet>
```

### Tasks

```xml
<bpmn:userTask id="Task_2" name="2. Filling in the application"/>
<bpmn:serviceTask id="Task_3" name="3. Registering the application"/>
<bpmn:manualTask id="Task_5_2" name="5.2. Interview"/>
<bpmn:businessRuleTask id="Task_9" name="9. Formal check"/>
<bpmn:scriptTask id="Task_11" name="11. Calculating the cost" scriptFormat="text/plain">
  <bpmn:script>cost = fee + travel + 0.15 * fee</bpmn:script>
</bpmn:scriptTask>
<bpmn:sendTask id="Task_12" name="12. Request for quotation" messageRef="Msg_Request"/>
<bpmn:receiveTask id="Task_R" name="Receiving the confirmation" messageRef="Msg_Confirmation"/>
<bpmn:task id="Task_X" name="Task without a type">
  <bpmn:standardLoopCharacteristics/>                                  <!-- loop -->
</bpmn:task>
<bpmn:userTask id="Task_Y" name="Assessing each item">
  <bpmn:multiInstanceLoopCharacteristics isSequential="false"/>        <!-- multi-instance -->
</bpmn:userTask>
```

### Start and end events

```xml
<bpmn:startEvent id="Start_1" name="1. Announcement received">
  <bpmn:messageEventDefinition id="Start_1_def" messageRef="Msg_Announcement"/>
</bpmn:startEvent>
<bpmn:startEvent id="Start_T" name="Start of the quarter">
  <bpmn:timerEventDefinition id="Start_T_def">
    <bpmn:timeCycle xsi:type="bpmn:tFormalExpression">R/2026-01-01T08:00/P3M</bpmn:timeCycle>
  </bpmn:timerEventDefinition>
</bpmn:startEvent>
<bpmn:endEvent id="End_28" name="28. End – accepted">
  <bpmn:messageEventDefinition id="End_28_def" messageRef="Msg_Acceptance"/>
</bpmn:endEvent>
<bpmn:endEvent id="End_30" name="30. End – application expired"/>
<bpmn:endEvent id="End_T" name="Immediate end">
  <bpmn:terminateEventDefinition id="End_T_def"/>
</bpmn:endEvent>
```

### Intermediate events

```xml
<bpmn:intermediateCatchEvent id="Catch_14" name="14. Quotation received">
  <bpmn:messageEventDefinition id="Catch_14_def" messageRef="Msg_Quotation"/>
</bpmn:intermediateCatchEvent>
<bpmn:intermediateCatchEvent id="Catch_16" name="16. 5 days elapsed">
  <bpmn:timerEventDefinition id="Catch_16_def">
    <bpmn:timeDuration xsi:type="bpmn:tFormalExpression">P5D</bpmn:timeDuration>
  </bpmn:timerEventDefinition>
</bpmn:intermediateCatchEvent>
<bpmn:intermediateThrowEvent id="Throw_5_3" name="5.3. Deadline exceeded">
  <bpmn:escalationEventDefinition id="Throw_5_3_def" escalationRef="Esc_Deadline"/>
</bpmn:intermediateThrowEvent>
```

### Boundary events

A boundary event is an element of the process (a sibling of the task), not its child. The attribute `attachedToRef` points to the task. It has no incoming flow.

```xml
<!-- interrupting (the default): the task is taken away -->
<bpmn:boundaryEvent id="Boundary_8" name="8. 10 days elapsed" attachedToRef="Task_8">
  <bpmn:timerEventDefinition id="Boundary_8_def">
    <bpmn:timeDuration xsi:type="bpmn:tFormalExpression">P10D</bpmn:timeDuration>
  </bpmn:timerEventDefinition>
</bpmn:boundaryEvent>
<!-- non-interrupting: the task goes on, a parallel path starts -->
<bpmn:boundaryEvent id="Boundary_5_1" name="5.1. 3 days elapsed" attachedToRef="Task_5_1" cancelActivity="false">
  <bpmn:timerEventDefinition id="Boundary_5_1_def">
    <bpmn:timeDuration xsi:type="bpmn:tFormalExpression">P3D</bpmn:timeDuration>
  </bpmn:timerEventDefinition>
</bpmn:boundaryEvent>
<!-- error: always interrupts -->
<bpmn:boundaryEvent id="Boundary_3" name="3. Save error" attachedToRef="Task_3">
  <bpmn:errorEventDefinition id="Boundary_3_def" errorRef="Err_Save"/>
</bpmn:boundaryEvent>
<!-- escalation from a sub-process, non-interrupting -->
<bpmn:boundaryEvent id="Boundary_5a" name="5a. Intervention" attachedToRef="Sub_5" cancelActivity="false">
  <bpmn:escalationEventDefinition id="Boundary_5a_def" escalationRef="Esc_Deadline"/>
</bpmn:boundaryEvent>
```

### Gateways and sequence flows

```xml
<bpmn:exclusiveGateway id="Gw_6" name="6. Application approved?"/>
<bpmn:sequenceFlow id="F7" name="yes" sourceRef="Gw_6" targetRef="Task_9"/>
<bpmn:sequenceFlow id="F8" name="no" sourceRef="Gw_6" targetRef="Gw_7"/>

<bpmn:eventBasedGateway id="Gw_13" name="13. Waiting for the reply"/>   <!-- outgoing: catch events only -->

<bpmn:parallelGateway id="Gw_20" name="20. Start of the implementation"/>   <!-- outgoing flows without conditions -->
<bpmn:parallelGateway id="Gw_23" name="23. Synchronisation"/>

<bpmn:inclusiveGateway id="Gw_24" name="24. Additional activities needed?" default="F35"/>
<bpmn:sequenceFlow id="F33" name="travel" sourceRef="Gw_24" targetRef="Task_25">
  <bpmn:conditionExpression xsi:type="bpmn:tFormalExpression">training outside the place of work</bpmn:conditionExpression>
</bpmn:sequenceFlow>
<bpmn:sequenceFlow id="F35" name="none" sourceRef="Gw_24" targetRef="Gw_27"/>  <!-- default flow: no condition -->
<bpmn:inclusiveGateway id="Gw_27" name="27. Synchronisation"/>
```

### Sub-process and event sub-process

```xml
<bpmn:subProcess id="Sub_5" name="5. Assessment by the supervisor">
  <bpmn:startEvent id="Sub_5_start"/>
  <bpmn:userTask id="Task_5_1" name="5.1. Review of merit"/>
  <bpmn:endEvent id="Sub_5_end"/>
  <bpmn:sequenceFlow id="F5_1" sourceRef="Sub_5_start" targetRef="Task_5_1"/>
  <bpmn:sequenceFlow id="F5_2" sourceRef="Task_5_1" targetRef="Sub_5_end"/>
  <!-- the boundary events of tasks inside the sub-process are here as well -->
</bpmn:subProcess>

<bpmn:subProcess id="EventSub_31" name="31. Budget freeze" triggeredByEvent="true">
  <bpmn:startEvent id="Start_31" name="31. Budget freeze">   <!-- isInterrupting="false" = non-interrupting -->
    <bpmn:signalEventDefinition id="Start_31_def" signalRef="Sig_Freeze"/>
  </bpmn:startEvent>
  <bpmn:serviceTask id="Task_32" name="32. Cancelling the application"/>
  <bpmn:endEvent id="End_33" name="33. End – cancelled"/>
  <bpmn:sequenceFlow id="F31_1" sourceRef="Start_31" targetRef="Task_32"/>
  <bpmn:sequenceFlow id="F31_2" sourceRef="Task_32" targetRef="End_33"/>
</bpmn:subProcess>
```

### Data

```xml
<bpmn:dataObject id="DataObject_Application" name="Application"/>
<bpmn:dataObjectReference id="DOR_Application_submitted" name="Application" dataObjectRef="DataObject_Application">
  <bpmn:dataState id="DOR_Application_submitted_state" name="submitted"/>
</bpmn:dataObjectReference>
<bpmn:dataStoreReference id="DSR_Records" name="Records" dataStoreRef="DataStore_Records"/>

<!-- the task writes data: inside the task -->
<bpmn:serviceTask id="Task_3" name="3. Registering the application">
  <bpmn:dataOutputAssociation id="DOA_3">
    <bpmn:targetRef>DSR_Records</bpmn:targetRef>
  </bpmn:dataOutputAssociation>
</bpmn:serviceTask>

<!-- the task reads data: inside the task -->
<bpmn:userTask id="Task_18" name="18. Reviewing the application">
  <bpmn:property id="Task_18_prop" name="__targetRef_placeholder"/>
  <bpmn:dataInputAssociation id="DIA_18">
    <bpmn:sourceRef>DOR_Quotation</bpmn:sourceRef>
    <bpmn:targetRef>Task_18_prop</bpmn:targetRef>
  </bpmn:dataInputAssociation>
</bpmn:userTask>
```

In an element that contains `dataOutputAssociation` or `dataInputAssociation` the order is: first `property` and the data associations, and only then the event definitions. For catch events: `dataOutputAssociation` before `messageEventDefinition`.

## C. Validator codes

`tok check` gives each finding as its code, the message, the id of the element and the line of the element in the file. The messages below are the English ones; `--lang pl` gives them in Polish.

**Errors** (`E…`): the model breaks a rule of BPMN; `tok check` exits with code 1. Fix them before laying out the model with `tok bpmn`.

| Code | Problem | Fix |
|---|---|---|
| E001 | the file is not well-formed XML | check that the tags are closed and the characters `&`, `<` in texts (`&amp;`, `&lt;`) |
| E002 | the root element is not bpmn:definitions | use the template `template.bpmn` |
| E003 | an id is used more than once | give unique identifiers |
| E004 | reference to a non-existent id | check `sourceRef`, `targetRef`, `attachedToRef`, `messageRef`, `flowNodeRef` |
| E005 | sequence flow without both ends or connecting data, a pool or an annotation | a sequence flow connects only tasks, events and gateways; data is connected with a data association |
| E006 | sequence flow between pools or into a sub-process | between pools: `messageFlow`; sub-process: its own start and end |
| E007 | message flow from an element outside the pools | both ends must belong to participants of the collaboration |
| E008 | message flow inside one pool (use a sequence flow between lanes) | replace it with a `sequenceFlow` |
| E009 | message flow to a gateway, data or a non-message event | the target/source can be a task, a message event or a pool |
| E010 | trigger not allowed for this event position (e.g. a timer in an end event, a trigger in the start of a plain sub-process) | match the event type to its position |
| E011 | incoming flow to a start event or outgoing flow from an end event | reverse or move the flow |
| E012 | invalid boundary event (on a gateway, with incoming flow, non-interrupting error, compensation with a flow) | a boundary event: on a task/sub-process, without incoming flows; an error always interrupts |
| E013 | event sub-process with sequence flows or without a triggered start event | remove the flows; one start event with a trigger |
| E014 | compensation activity with sequence flows | it is connected by an association with the compensation event |
| E015 | default flow that does not leave the element or has a condition | `default` points to an outgoing flow without a condition |
| E016 | event-based gateway followed by something other than catch events or receive tasks (a task/gateway, fewer than 2 outgoing flows or conditions) | only catch events or receive tasks, without conditions |
| E017 | condition or default flow on a parallel gateway | remove it or change the gateway to XOR/OR |
| E018 | reference to a declaration of the wrong type (`messageRef`/`signalRef`/`errorRef`/`escalationRef`) | point to a declaration of the right type |
| E030 | element in two lanes | keep it in one |

**Warnings** (`W…`, `K…`, `D…`): the model is read, but it probably contains a modelling mistake.

| Code | Problem |
|---|---|
| W010 | timer event without a time |
| W016 | event after an event-based gateway has other incoming flows |
| W020 | gateway with one incoming and one outgoing flow is unnecessary |
| W021 | branch of an XOR/OR split without label, condition or default status |
| W022 | branch without a condition at an element that splits conditionally (it has a default flow or other branches have conditions) |
| W023 | sequence flow repeats another one (same source, target and condition) |
| W030 | lane refers to an element inside a sub-process |
| W031 | element outside every lane |
| W040 | process without a start or end event |
| W041 | element not reachable from a start event |
| W042 | path without an end event (element without outgoing flow) |
| W043 | sequence flow from an element back to itself |
| W044 | activity or event with several outgoing flows without conditions (implicit split; all of them are taken) |
| W045 | sub-process without a start event |
| W046 | sub-process without an end event |
| W047 | event with several triggers (multiple event), reported as an error by bpmn.io linters |
| W050 | OR split without a default flow |
| W051 | join of a different type than the split (e.g. AND split joined by XOR) |
| W052 | boundary escalation/error on a sub-process that nothing inside throws |
| W053 | escalation thrown but never caught |
| W054 | message task or event without a message flow in a model with several pools |
| K01 | name does not start with the step number of the process description (e.g. '12. Request for quotation') |
| K02 | a step of the process description has no element in the model (`--steps-from`) |
| K03 | an element carries a step number that is not in the process description (`--steps-from`) |
| D01 | diagram coordinates of an element that is no longer in the model; run tok bpmn on the file to lay it out again |
| D02 | elements without diagram coordinates (BPMNDI); run tok bpmn on the file to lay it out |

**Notes** (`L…`): modelling style.

| Code | Note |
|---|---|
| L010 | message event without messageRef |
| L020 | name of a splitting gateway is not a question |
| L021 | gateway both joins and splits |
| L030 | task without a name |

## D. Executable model for Camunda 8 (option `--camunda`)

The option `--camunda` of `tok check` checks whether the model can be deployed to the Camunda 8 engine (e.g. Camunda 8 Run). Forms (`.form`) and decision tables (`.dmn`) must be in the same folder as the model.

### XML templates

In `<bpmn:definitions>` add the namespaces and the platform:

```xml
xmlns:zeebe="http://camunda.org/schema/zeebe/1.0"
xmlns:modeler="http://camunda.org/schema/modeler/1.0"
modeler:executionPlatform="Camunda Cloud" modeler:executionPlatformVersion="8.10.0"
```

```xml
<bpmn:process id="book-purchase-proposal" name="Book purchase proposal" isExecutable="true">

<!-- start form (starting the process from Tasklist) -->
<bpmn:startEvent id="Start_1" name="Proposal submitted">
  <bpmn:extensionElements>
    <zeebe:formDefinition formId="form-proposal"/>
  </bpmn:extensionElements>
</bpmn:startEvent>

<!-- user task with a form -->
<bpmn:userTask id="Task_Approval" name="Approving the proposal">
  <bpmn:extensionElements>
    <zeebe:userTask/>
    <zeebe:formDefinition formId="form-approval"/>
  </bpmn:extensionElements>
</bpmn:userTask>

<!-- business rule task calling a DMN table -->
<bpmn:businessRuleTask id="Task_Mode" name="Determining the approval mode">
  <bpmn:extensionElements>
    <zeebe:calledDecision decisionId="approval-mode" resultVariable="mode"/>
  </bpmn:extensionElements>
</bpmn:businessRuleTask>

<!-- script task: a FEEL expression -->
<bpmn:scriptTask id="Task_Cost" name="Calculating the purchase cost">
  <bpmn:extensionElements>
    <zeebe:script expression="=price + 12.5" resultVariable="cost"/>
  </bpmn:extensionElements>
</bpmn:scriptTask>

<!-- service task: needs a program (worker) that handles the type -->
<bpmn:serviceTask id="Task_Notification" name="Sending the notification">
  <bpmn:extensionElements>
    <zeebe:taskDefinition type="notification"/>
  </bpmn:extensionElements>
</bpmn:serviceTask>

<!-- condition on a flow: a FEEL expression starts with "=" -->
<bpmn:sequenceFlow id="Flow_Yes" name="yes" sourceRef="Gw_Approved" targetRef="Task_Cost">
  <bpmn:conditionExpression xsi:type="bpmn:tFormalExpression">=approved = true</bpmn:conditionExpression>
</bpmn:sequenceFlow>

<!-- a message the process waits for: name and correlation key -->
<bpmn:message id="Msg_Reply" name="supplier-reply">
  <bpmn:extensionElements>
    <zeebe:subscription correlationKey="=order_number"/>
  </bpmn:extensionElements>
</bpmn:message>
```

### Form (`.form` file, JSON)

The `id` attribute of the form must be equal to `formId` in the model. The key of a field (`key`) becomes the name of a process variable. Field types: `textfield`, `textarea`, `number`, `select`, `radio`, `checkbox`, `datetime`, `text` (text only; `{{variable}}` inserts a value).

```json
{
  "id": "form-approval",
  "type": "default",
  "executionPlatform": "Camunda Cloud",
  "executionPlatformVersion": "8.10.0",
  "components": [
    { "type": "text", "id": "Text_1", "text": "## Proposal: {{title}}" },
    { "type": "radio", "id": "Field_Approved", "key": "approved", "label": "Decision",
      "values": [ { "label": "approve", "value": true }, { "label": "reject", "value": false } ],
      "validate": { "required": true } },
    { "type": "number", "id": "Field_Amount", "key": "amount", "label": "Amount (EUR)", "decimalDigits": 2 }
  ],
  "schemaVersion": 18
}
```

### Decision table (`.dmn` file)

The `id` attribute of the `decision` element must be equal to `decisionId` in the model, and the `name` of the output is the name of the variable with the result of the table. An empty input entry is `-`.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/" xmlns:modeler="http://camunda.org/schema/modeler/1.0"
             id="Definitions_Mode" name="Approval mode" namespace="http://example.org/process"
             modeler:executionPlatform="Camunda Cloud" modeler:executionPlatformVersion="8.10.0">
  <decision id="approval-mode" name="Approval mode">
    <decisionTable id="Table_Mode" hitPolicy="FIRST">
      <input id="In_Price" label="Price">
        <inputExpression id="InExpr_Price" typeRef="number"><text>price</text></inputExpression>
      </input>
      <output id="Out_Mode" label="Mode" name="mode" typeRef="string"/>
      <rule id="Rule_1">
        <inputEntry id="Rule_1_In"><text>&lt;= 150</text></inputEntry>
        <outputEntry id="Rule_1_Out"><text>"automatic"</text></outputEntry>
      </rule>
      <rule id="Rule_2">
        <inputEntry id="Rule_2_In"><text>-</text></inputEntry>
        <outputEntry id="Rule_2_Out"><text>"manager"</text></outputEntry>
      </rule>
    </decisionTable>
  </decision>
</definitions>
```

### Codes

| Code | Severity | Problem | Fix |
|---|---|---|---|
| C001 | warning | no modeler:executionPlatform="Camunda Cloud" on <bpmn:definitions>; Camunda Modeler opens the file as a Camunda 7 model | add the platform attributes to `definitions` |
| C002 | error | no process has isExecutable="true"; the engine will not deploy the model | set `isExecutable="true"` on the process that is to run in the engine |
| C010 | warning | user task without <zeebe:userTask/>; it does not reach Tasklist as a Camunda user task | add it in `extensionElements` |
| C011 | note | user task without a form (zeebe:formDefinition); in Tasklist it can only be completed without data | add `zeebe:formDefinition` and a `.form` file |
| C012 | warning | the form of the user task is in no .form file of the model's folder | correct `formId` or add the form |
| C020 | error | service or send task without <zeebe:taskDefinition type="…"/>; the deployment fails | add the task type |
| C021 | note | service or send task waits for a worker of its job type; without one the instance stops there | replace it with a script task or a user task |
| C030 | error | business rule task without <zeebe:calledDecision decisionId="…" resultVariable="…"/> | add the decision call |
| C031 | error | zeebe:calledDecision without decisionId or resultVariable | add `decisionId` and `resultVariable` |
| C032 | warning | the decision of the business rule task is in no .dmn file of the model's folder | correct the decision identifier (the `id` attribute of the `decision` element) |
| C040 | error | script task without <zeebe:script expression="=…" resultVariable="…"/> | add `expression="=…"` and `resultVariable` |
| C041 | error | zeebe:script: the FEEL expression must start with '=' and resultVariable is required | write the expression as `=…` and add `resultVariable` |
| C050 | error | message event or task without messageRef | add `messageRef` to a declared message |
| C051 | error | message without a name; the engine correlates messages by name | give the message a `name` |
| C052 | error | message caught by an event or receive task without <zeebe:subscription correlationKey="=…"/> | declare the message with a name and a correlation key |
| C060 | warning | element not executed by Camunda 8 (a complex gateway or a transaction) | Camunda 8 does not execute them; rebuild the fragment |
| C070 | error | flow leaving a gateway without a condition; the engine needs one on every outgoing flow except the default | add a FEEL condition or make the flow the default one |
| C071 | error | condition of a flow is not a FEEL expression starting with '=' (e.g. =approved = true) | write the condition as `=expression` |
| C080 | error | timer event without a value (timeDuration, timeDate or timeCycle) | add `timeDuration` (e.g. `P7D`), `timeDate` or `timeCycle` |
