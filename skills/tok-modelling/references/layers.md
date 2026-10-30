# The layers of the language beyond a process model

Read the section the task needs. The cheat sheet (`cheatsheet.md`) has the core language; the specification (`spec.md`) is normative and has the details of every element named here.

## The core language (0.1 and 0.3)

- **Events** `start`, `end`, `catch`, `throw`, `boundary` with one trigger: `message="Name"`, `timer=P3D` (duration, date-time or cycle), `signal=`, `error=`, `escalation=`, `condition="FEEL"`, `link=`, or the flags `terminate`, `compensate`.
- **Activities** `task`, `user`, `service`, `manual`, `script`, `rule`, `send`, `receive`, `call`; markers `loop`, `parallel`, `sequential`, `compensation`.
- **Sub-processes** `sub`, `eventsub`, `adhoc` with a block.
- **Gateways** `xor`, `and`, `or`, `eventgw`; flows take `when="FEEL"`, `default`, `p=` (simulation).
- **Pools and lanes** (`pool ID "Label"` with a block of lanes; a pool without a block is a black box), **message flows**, **data** (`data`, `store`, `reads`, `writes`, `state=`), **notes** and **groups**.
- **Decision tables** (`decision ID hit=first` with rows `| input: type || output: type |`), **forms** (`form` with `text` and `field` lines), and the **simulation** statements.

## Executable models (Camunda 8)

When the document is `executable` (header flag), every `user` task should have `form=`, every `rule` task `decision=` and `result=`, every `script` task `expr=` and `result=`, every flow leaving an `xor`/`or` gateway with several outgoing flows either `when="…"` or `default`. FEEL expressions are written without a leading `=`. Avoid `service` tasks: they need a separate worker program; use `script` or `user` tasks instead. Forms and decisions are written in the same file (`form`, `decision` blocks) and exported together. The executable layer (spec §10.3) adds what Camunda 8 runs: multi-instance (`collection=`, `item=`, `output=`), call activities (`process=`, `binding=`), variable mappings (`inputs=`, `outputs=`), user task assignment (`assignee=`, `candidate-groups=`, `due=`), job types and headers, the REST connector, compensation, conditional events and listeners. Diagnostics `C…` report what Camunda 8 does not accept.

## BPMN beyond Camunda (language 0.5)

With the header `tok 0.5` (or `tok 1.0`) the model may use the rest of BPMN 2.0.2 process modelling: `transaction` with `end … cancel` and `boundary … cancel`, `complex` gateways (`activation=`), standard loops (`loop while="…" test-before loop-max=N`), events with several triggers (`parallel` for all of them), `receive`/`eventgw … instantiate`, process data (`item`, `input`, `output`, `property`, `type=`), `global` tasks, `interface`/`operation` and correlation `key` blocks (in documents with pools), `doc="…"` documentation on any element. They belong to the level `full` (the levels `basic` and `analytic` reject them) and are not executed by Camunda 8: in executable documents they are reported as `C100`–`C110`.

## Collaboration and choreography (language 0.6)

With the header `tok 0.6` (or `tok 1.0`) the model may describe the interactions between participants: `conversation`, `subconversation` and `callconversation` nodes between pools (`pools="a b"`, `messages="m1 m2"` naming message flows with `id=`), `choreography` blocks with `choreo ID "Label" from=a to=b message="M" reply="R"`, `subchoreo` and `callchoreo` activities, and `min=`/`max=` on pools for several participants of one kind. Their participants are pools, usually black-box pools. They belong to the level `full`; in executable documents they are reported as `C120`–`C123`. Checking with `class` (`descriptive`, `analytic`, `common-executable`, `full`) tests the model against a conformance class of BPMN 2.0.2.

## Models from BPMN or DMN files

When the model comes from a BPMN file drawn in a modelling tool, import it (`import`, or `tok import FILE.bpmn -o FILE.tok`) and keep the layout sidecar `FILE.tok.layout` next to the model: export and the diagram reuse the drawing for every element you do not change. DMN files are imported the same way.

## Decision models (language 0.7)

With the header `tok 0.7` or `tok 1.0` decisions become DMN 1.5 decision models. Declare `inputdata ID "Label" type=…`, knowledge sources (`source`) and business knowledge models (`bkm ID params="a: number" expr="…"`), and connect them with `requires="ID …"` on decisions; a decision sees only what it requires, and FEEL refers to an element by its identifier (or `name="…"` for a name with spaces). Decision logic is a table (as before, plus `hit=rule-order|output-order`, `aggregation=`, output values `name: type = "a", "b"` and `default=`), `expr="FEEL"`, `invoke=BKM with="p = FEEL; …"`, a context (`entry NAME …` lines, optionally `result …` last), a list (`element …` lines) or a relation (rows without `||`); `params=` on a line makes it a function. DMN types are `item` blocks with `type=`, `allowed=` and components. Every expression of a 0.7 or 1.0 document is full FEEL (dates, durations, `for`, `some`, filters, contexts; `feel.md`). Check the model, test decisions with `eval` (or `tok eval FILE DECISION name=value …`; JSON objects and lists are accepted as values), export the DMN 1.5 files and their diagrams with `export`, and read existing DMN files with `import`. In executable documents `C130`–`C132` report what Camunda 8 does not execute.

## Simulation

Add `simulate cases=N seed=N arrival="exp(2h)"`, `resource ID count=N` and `variable NAME "DIST"` statements and `by=`, `duration="DIST"`, `assign=`, `delay=` and `p=` attributes; run the simulation (`sim` with a `seed`, or `tok sim FILE --seed 1`). Report the indicators with the seed you used. Ask for the event log only when the analysis needs it (`log: true`, or `tok sim --log FILE.csv`).
