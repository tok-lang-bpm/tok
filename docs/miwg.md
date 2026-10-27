# BPMN MIWG conformance gate

The reference models of the BPMN Model Interchange Working Group (MIWG) test suite are the conformance gate of the BPMN import and export of language 0.5 (`design-0.5.md` §12). The 22 models in `conformance/miwg` are unmodified copies of the suite's reference files; they keep their licence, CC BY 3.0, and their attribution is given in [`conformance/miwg/README.md`](../conformance/miwg/README.md).

## What the gate checks

For every reference model the test `rust/tok-core/tests/miwg.rs`:

1. imports it with its identifiers (`tok import`, keeping identifiers), prints the tok text, parses the text again and exports semantic BPMN (`tok bpmn` without the diagram);
2. reduces the reference model and the export to their semantic content, a set of *facts* (`rust/tok-core/tests/semantic/mod.rs`), and compares the two sets;
3. fails when
   - the model does not import, its tok text does not parse or is not in canonical form;
   - a fact is lost or added that the baseline `rust/tok-core/tests/fixtures/miwg/baseline.tsv` does not list: what survived before must survive now;
   - a lost or added fact is not accounted for by a warning of the import (every dropped thing is reported);
   - a second round trip (export, import, export) changes the semantic BPMN.

The same round trip runs over the BPMN files of `conformance/import` (models drawn in Camunda Modeler for the course), which must survive without losing or adding anything.

### Facts

| Category | Compared |
|---|---|
| structure | definitions (identifier, name, target namespace, expression and type language), collaborations, participants with their process and multiplicity, processes (identifier, name, executable, process type, closed), lanes with their parent lane and members |
| nodes | every flow node by identifier and element, its name and its process or sub-process |
| events | event definitions (kinds, and the message, signal, error or escalation by name; time, condition, link name, compensated activity), interrupting, parallel multiple, attachment of boundary events |
| activities | standard loop (test before, maximum, condition) and multi-instance (sequential, cardinality, completion condition, data items, behaviour), compensation handlers, called element, script, message, instantiation, operation, performers and their resources, `implementation`, `startQuantity`, `completionQuantity`, triggered by event, transaction method, ad-hoc ordering |
| gateways | direction, event-based type and instantiation, activation condition |
| flows | sequence flows (source, target, name, condition and its language, default, `isImmediate`), message flows (source, target, name, message by name) |
| data | data objects (name, state, collection, item) and their references, data stores (name, capacity, item) and their references, data inputs and outputs, properties, input and output sets, data associations with their transformations and assignments |
| declarations | item definitions (structure as a qualified name, kind, collection), messages, signals, errors and escalations with their items and codes, resources, interfaces and operations, global tasks, correlation keys and properties, imports |
| documentation | the text and text format of the documentation of any element |
| artifacts | text annotations, associations (unordered when they have no direction), groups and category values |
| camunda | Camunda 8 extension elements (`zeebe:`) with their attributes |
| vendor | extension elements and attributes of other tools |

Comparison rules:

- Identifiers are compared where tok keeps them: flow nodes, flows, lanes, participants, processes, data object and store references, data inputs, outputs and properties of processes and sub-processes, item definitions, resources, interfaces, operations, global tasks, correlation keys, annotations and groups. An identifier that is not a tok identifier is converted by the import (`docs/import.md`), and the conversion is applied to the reference model before comparing. Elements whose identifiers tok generates are compared by their content: data objects (by their references), event definitions, `ioSpecification`, input and output sets, data associations, associations, categories, and messages, signals, errors, escalations and data stores (by name, as tok declares them).
- Names are compared with white space collapsed (tok keeps names on one line); documentation text line by line; expressions without the leading `=` of Camunda; an attribute that states its default value (of BPMN or of Camunda: `startQuantity="1"`, `implementation="##WebService"` on a service task, `propagateAllChildVariables="true"`) is not a fact.
- The direction of a gateway is the stated `gatewayDirection` or, when it is `Unspecified`, the direction implied by its sequence flows; tok writes no direction, so a stated direction is kept when it agrees with the flows.
- A data input or output of an activity or event that carries nothing but its one data association (no item, no collection flag, no documentation, named after the data element or unnamed) is part of the association; tok writes such inputs and outputs itself for `reads` and `writes`.
- Diagram interchange (`bpmndi`: shapes, edges, coordinates, colours) is not compared: tok has no coordinates and lays every diagram out itself.

## Running

```text
cd rust
cargo test -p tok-core --test miwg                  # the gate
cargo run -p tok-core --example miwg                # every lost or added fact, per model
cargo run -p tok-core --example miwg -- C.3.0       # one model, with its import warnings and tok text
cargo run -p tok-core --example miwg -- --table     # the coverage table below
TOK_BLESS=1 cargo test -p tok-core --test miwg      # store the baseline after an improvement
```

`tools/check.sh` runs the gate with the other Rust tests.

## Coverage

Facts kept per category (`kept/total`, or the total when everything is kept; `–` where the model has none). The vendor extensions are not kept by design and are not counted in the last column.

| Model | structure | nodes | events | activities | gateways | flows | data | declarations | documentation | artifacts | camunda | vendor | Kept |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A.1.0 | 4 | 15 | – | – | – | 4 | – | – | – | – | – | – | 23/23 |
| A.2.0 | 4 | 24 | – | – | 2 | 9 | – | – | – | – | – | – | 39/39 |
| A.2.1 | 5/6 | 24 | – | – | 2 | 22/29 | – | – | – | – | – | 22 not kept | 53/61 |
| A.3.0 | 4 | 30 | 5 | – | – | 8 | – | – | – | – | – | – | 47/47 |
| A.4.0 | 20 | 51 | – | – | – | 19 | – | 2 | – | – | – | – | 92/92 |
| A.4.1 | 31 | 51 | – | – | – | 17/30 | – | – | – | – | – | 27 not kept | 99/112 |
| B.1.0 | 33 | 87 | 6 | 3/5 | 5 | 32 | 10/12 | 4 | – | 4 | – | – | 184/188 |
| B.2.0 | 60 | 279 | 57 | 8/13 | 8 | 96 | 9/11 | 4/5 | – | 4 | – | – | 525/533 |
| C.1.0 | 43/45 | 60 | 6 | 0/9 | 3 | 33/53 | – | 7 | – | – | – | 126 not kept | 152/183 |
| C.1.1 | 6/7 | 30 | – | 0/5 | 2 | 18/28 | 27/35 | 10/12 | – | – | – | 70 not kept | 93/119 |
| C.2.0 | 30 | 78 | 13 | – | 3 | 42 | – | 5 | – | – | – | – | 171/171 |
| C.3.0 | 6/7 | 41 | 8 | 0/7 | 3 | 23/25 | – | 3 | 5 | – | – | 67 not kept | 89/99 |
| C.4.0 | 51/54 | 120 | 14 | 1 | 6 | 45 | 57/61 | 3 | – | 2 | – | 8 not kept | 299/306 |
| C.5.0 | 48 | 104 | 2 | 1 | 12 | 52 | 98 | – | – | – | – | 2 not kept | 317/317 |
| C.6.0 | 5 | 111 | 18 | 3 | 5 | 32 | 6 | – | – | 2 | – | 1 not kept | 182/182 |
| C.7.0 | 24/26 | 31 | – | 3/6 | 3 | 14 | 52 | 8/9 | 5/10 | – | – | 20 not kept | 140/151 |
| C.8.0 | 5 | 51 | 2 | – | 2 | 26/42 | 20 | 2/3 | 1 | – | 0/33 | 167 not kept | 109/159 |
| C.8.1 | 10/12 | 51 | 3 | 7/17 | 2 | 26/45 | 143/146 | 70/75 | 0/1 | – | 32/35 | 300 not kept | 344/387 |
| C.9.0 | 9 | 72 | 11 | 3 | 3 | 31 | – | 5/7 | 19 | – | 34/36 | 8 not kept | 187/191 |
| C.9.1 | 9 | 30 | 7 | 1 | – | 7 | – | 1 | – | – | 2/3 | 13 not kept | 57/58 |
| C.9.2 | 9 | 57 | 16 | 5 | 1 | 16 | – | 4/5 | – | – | 12/14 | 4 not kept | 120/123 |
| C.10.0 | 33/34 | 58 | 4 | 1 | 2 | 27/43 | 76 | 10/11 | – | – | – | 184 not kept | 211/229 |

**Summary: 22 models, 3533 of 3770 facts kept (93.7 %); A.1.0, A.2.0, A.3.0, A.4.0, C.2.0, C.5.0 and C.6.0 completely; 237 facts not kept, each listed below and reported by the import; 52 facts added by design; 1019 facts of vendor extensions (in 15 models) not kept by design.**

### Before and after the gate

The same comparison with the import of the first release of language 0.5:

| Model | Kept before | Kept now | Lost before → now | Added before → now | Unreported before → now |
|---|---|---|---|---|---|
| A.1.0 | 23/23 | 23/23 | 0 → 0 | 1 → 0 | 1 → 0 |
| A.2.0 | 39/39 | 39/39 | 0 → 0 | 1 → 0 | 1 → 0 |
| A.2.1 | 49/61 | 53/61 | 12 → 8 | 0 → 0 | 14 → 0 |
| A.3.0 | 47/47 | 47/47 | 0 → 0 | 3 → 0 | 2 → 0 |
| A.4.0 | 92/92 | 92/92 | 0 → 0 | 5 → 3 | 1 → 0 |
| A.4.1 | 99/112 | 99/112 | 13 → 13 | 4 → 4 | 18 → 0 |
| B.1.0 | 184/188 | 184/188 | 4 → 4 | 11 → 6 | 6 → 0 |
| B.2.0 | 521/533 | 525/533 | 12 → 8 | 50 → 6 | 30 → 0 |
| C.1.0 | 151/183 | 152/183 | 32 → 31 | 12 → 4 | 50 → 0 |
| C.1.1 | 81/119 | 93/119 | 38 → 26 | 7 → 4 | 46 → 0 |
| C.2.0 | 171/171 | 171/171 | 0 → 0 | 6 → 0 | 4 → 0 |
| C.3.0 | 83/99 | 89/99 | 16 → 10 | 2 → 1 | 28 → 0 |
| C.4.0 | 265/306 | 299/306 | 41 → 7 | 26 → 0 | 56 → 0 |
| C.5.0 | 272/317 | 317/317 | 45 → 0 | 20 → 3 | 62 → 0 |
| C.6.0 | 169/182 | 182/182 | 13 → 0 | 19 → 0 | 28 → 0 |
| C.7.0 | 110/151 | 140/151 | 41 → 11 | 5 → 3 | 50 → 0 |
| C.8.0 | 106/159 | 109/159 | 53 → 50 | 2 → 0 | 94 → 0 |
| C.8.1 | 252/387 | 344/387 | 135 → 43 | 13 → 15 | 275 → 0 |
| C.9.0 | 168/191 | 187/191 | 23 → 4 | 6 → 0 | 31 → 0 |
| C.9.1 | 57/58 | 57/58 | 1 → 1 | 0 → 0 | 14 → 0 |
| C.9.2 | 120/123 | 120/123 | 3 → 3 | 0 → 0 | 6 → 0 |
| C.10.0 | 200/229 | 211/229 | 29 → 18 | 14 → 3 | 178 → 0 |
| **all** | **3259/3770** | **3533/3770** | **511 → 237** | **207 → 52** | **995 → 0** |

"Unreported" counts the lost, added and vendor facts that no import warning named.

## Not kept, and why

tok cannot express these; the import reports each of them.

| What | Models | Why |
|---|---|---|
| `isImmediate` of sequence flows (95 flows) | A.2.1, A.4.1, C.1.0, C.1.1, C.3.0, C.8.0, C.8.1, C.10.0 | tok does not record the open or closed reading of a sequence flow; most tools write it as a default. Reported once per model with the number of flows. |
| expression language of conditions and of the definitions, type language | A.2.1, C.7.0, C.8.1 | tok writes its expressions (FEEL) and types without a language attribute. |
| `implementation` of tasks and global tasks | B.1.0, B.2.0, C.1.0, C.1.1, C.3.0, C.8.1 | the technology of a task is outside the language; executable documents use `type=` and the Camunda extensions. |
| `startQuantity`, `completionQuantity` other than 1 | C.3.0 | not in the language. |
| `processType`, `isClosed="true"` | C.1.0, C.1.1, C.3.0 | not in the language. |
| `potentialOwner`, and `performer` on user tasks | C.1.0, C.1.1, C.3.0, C.7.0, C.8.1 | `by=` names the resource; the export writes `humanPerformer` on user and manual tasks and `performer` elsewhere. A resource assignment expression is not kept. |
| capacity and `isUnlimited="false"` of data stores, the item of a data store (as opposed to its references) | B.1.0, B.2.0, C.1.1, C.4.0 | `store` declares a data store reference; its data store is shared by label. |
| data stores and messages that no element uses | C.1.1, C.9.0 | tok declares data stores, messages, signals, errors and escalations through their uses. |
| the item of a message (`itemRef`) | C.8.1 | messages are declared by name only. |
| further input and output sets, `inputSetRefs`, `outputSetRefs` | C.1.1 | tok writes one input set with every input and one output set with every output. |
| transformations of data associations | C.8.1 | not in the language. |
| imports | C.1.1, C.8.0, C.8.1, C.9.0, C.9.2, C.10.0 | not in the language. |
| further collaborations, the name of a collaboration | C.4.0, C.10.0 | a document has one collaboration; the participants and message flows of the others are merged into it. |
| documentation formats other than plain text, documentation of the process of a pool | C.7.0, C.8.1 | `doc=` is plain text; a pool documents its participant. |
| Camunda extensions of a process that is not executable | C.8.0 | the text keeps them; the export writes the executable layer only for executable processes (`tok check` reports them, `R055`). |
| `zeebe:userTaskForm`, `formKey`, `zeebe:taskDefinition` of a business rule task | C.8.0, C.8.1, C.9.0, C.9.2 | embedded forms and forms by key are not in the language (`form=` names a form by its identifier); a rule task calls a decision. |
| correlation key of a message awaited by a receive task | C.9.1 | a correlation key belongs to a message catch event. |
| `relationship` | C.7.0, C.8.1 | not in the language. |
| extension elements and attributes of other tools (Signavio, W4, ITP Commerce, Trisotech, BOC, Activiti, KnowProcess, OMNINET, i18n translations) | 15 models | vendor extensions are not kept by design; the import names each extension element and attribute once. |

### Added by design

| What | Models | Why |
|---|---|---|
| a participant for a process that no participant references | A.4.0, B.1.0, B.2.0, C.5.0, C.10.0 | in a document with pools every process is a pool. |
| a message for a named message flow without a message | A.4.1 | `message A -> B "Label"` refers to the message of its label. |
| a data object reference for a data object without one, or named directly by a data association | C.8.1 | tok declares data objects through their references (`data … object=`). |
| `supportedInterfaceRef` of a process whose tasks use operations | C.8.1 | the export lists the interface of every operation used. |
| a message for an operation that names an unknown message | C.8.1 | messages are declared by name. |
| `humanPerformer` or `performer` instead of the role of the model | C.1.0, C.1.1, C.3.0, C.7.0 | see `potentialOwner` above. |

Not counted as added: `zeebe:userTask` on the user tasks of executable processes (tok exports Camunda user tasks), and the name of the definitions of a model whose definitions have no name (the document is titled after its process).

## What the gate added to tok

The language elements below (language 0.5, `spec.md` and `design-0.5.md` §13) and the import changes that came with them:

- `doc="…"` on every statement that exports a BPMN element: documentation text (C.3.0, C.7.0, C.8.x, C.9.0).
- triggers without a value: the flags `message`, `timer`, `signal`, `condition` on events, and `error`, `escalation` on every event that allows them (A.3.0, B.1.0, B.2.0, C.1.0, C.2.0, C.4.0, C.5.0, C.6.0, C.9.0, C.10.0); before, the import invented a name or a time.
- `process-name="…"` on the header and on pools: the name of the process where it differs from the title or the pool label.
- `namespace="…"` on `item`: the namespace of a prefixed structure name (`structure="feel:string"`, C.1.1, C.7.0, C.8.x, C.10.0); before, the prefix was dropped.
- `reads … input=ID`, `writes … output=ID`: a data association with a declared data input or output of the activity (B.1.0, B.2.0, C.4.0, C.5.0, C.7.0, C.8.x, C.10.0).
- `reads` and `writes` on events, and `input`/`output … on=EVENT`: data of throwing and catching events (C.4.0, C.8.1).
- `data … object="…"`: the name of the data object behind a reference, shared by the references with the same object name (C.5.0, and two models of the course).
- `when=""`: a condition without an expression (A.2.1).
- An explicit `boundary … compensate` on an activity with `compensate=` is the boundary event associated with the handler (C.6.0); before, the import replaced it by a generated one.
- The import merges the participants and message flows of all collaborations (C.4.0), imports data objects without references (C.8.1), reports every element, attribute and documentation it does not keep, and names every message, signal, error and escalation it takes over.
