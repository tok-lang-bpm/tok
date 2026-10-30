---
name: tok-modelling
description: Write, check and repair process models in the tok language (.tok files) and export them to BPMN 2.0 XML, SVG, DMN decision models, Camunda forms and simulation results; import BPMN and DMN files into tok. Use for .tok files, for turning a process description (e.g. a table of steps) into a model, for decision models (DMN), and for executable or simulated processes described in tok. Works with the tok MCP server when it is configured and with the tok command otherwise.
---

# Modelling processes in tok

tok is a plain-text language for BPMN 2.0 process models with their DMN decisions, Camunda forms and simulation parameters (language 1.0, the stable language; documents of 0.1–0.7 stay valid and mean the same). You write the semantics; the tok engine checks the model, prints it in canonical form and produces every other format (BPMN XML with layout, SVG, DMN, forms, simulation). Never write BPMN or DMN XML or diagram coordinates by hand.

The whole language fits on one page: read `references/cheatsheet.md` before writing the first line. The full specification is `references/spec.md` (normative; read the section you need when the cheat sheet is not enough), FEEL is in `references/feel.md`, and every diagnostic code is explained in `references/diagnostics.md`.

## Tools: the MCP server or the tok command

Use the **tok MCP server** when your client has it (tools `check`, `fmt`, `export`, `import`, `sim`, `eval`, `reference`, and `check_bpmn` for BPMN XML files, usually shown with the server name `tok`). Otherwise use the **`tok` command** in a shell. Both run the same engine and give the same results.

| Task | MCP tool | Command |
|---|---|---|
| check, list diagnostics | `check` with `path` (or `text`); `lang`, `level`, `class` | `tok check FILE --json` (`--lang pl`, `--level`, `--class`) |
| canonical form | `fmt` → write its `text` to the file | `tok fmt FILE -w` |
| BPMN, SVG, DMN, forms | `export` with `path`, `formats: ["bpmn", "svg", "dmn", "forms"]` → write each returned file | `tok bpmn FILE`, `tok svg FILE`, `tok export FILE -o out/` |
| BPMN or DMN to tok | `import` with `path` → write `text` to `FILE.tok` and `layout` to `FILE.tok.layout` | `tok import FILE.bpmn -o FILE.tok` |
| simulation | `sim` with `path`, `seed` (and `cases`) | `tok sim FILE --seed 1` |
| evaluate a decision | `eval` with `path`, `decision`, `values` (JSON object) | `tok eval FILE DECISION name=value …` |
| language reference | `reference` (`cheatsheet`, `spec`, `diagnostics`, `feel`, `cli`) or the resources `tok://reference/…` | `tok cheatsheet`; the files in `references/` |

The MCP tools return file contents instead of writing files: write what they return with your file tools. A tool error (`isError`) explains what to fix — a document with errors lists them. `check` reports errors as its normal result (`errors` > 0). Paths are read relative to the working directory of the server; give absolute paths when unsure.

Without the MCP server, install the command if it is missing: `pip install tok-lang` (the engine; on a platform without a wheel pip builds it from source and needs Rust) or `cargo install tok-cli`, or in a checkout of the repository `cargo build --release -p tok-cli` in `rust/`. The same Python package runs the MCP server, `tok mcp`, which a client can be configured to start (`"command": "tok", "args": ["mcp"]`). If you can run neither, write the model by the cheat sheet and say clearly that it has not been checked.

## Workflow

1. **Read the process description completely.** Identify participants (pools), roles (lanes), the steps, decisions, deadlines, messages to other organisations and the documents.
2. **Write a mapping first** (`mapping.md`): one row per step with the tok element (kind and id), the lane, the words in the description that justify the choice, and doubts. The person you work for reviews your model through this table.
3. **Write the `.tok` file.** One statement per line, explicit identifiers (short, ASCII, e.g. `check-request`), labels in the language of the description. Declare nodes inside lanes; write all `flow` lines after the pools. Write new models with the header `tok 1.0`: it has every element of the language. When you change an existing model, keep its header (`tok 0.1`, `0.3`, `0.5`, `0.6` or `0.7`) unless it needs elements of a later version (below); a model written for an earlier version means the same under `tok 1.0`.
4. **Check** (`check`, or `tok check FILE --json`). Fix every error and every warning, and check again until there are none. Each diagnostic has a code and a line; codes are explained in `references/diagnostics.md`. The warnings of the BPMN rules (`W…`) are what BPMN linters such as bpmnlint of bpmn.io report as errors: an element without a start or an end, a branch without a condition next to a default flow (`W022`; write `when="…"`, or `when=""` while the condition is not known yet), a flow written twice (`W023`), a flow back to its own element (`W043`; use a loop or a gateway), several outgoing flows from a task or an event without a gateway (`W044`; split with an `and` gateway, or give the flows conditions), a sub-process without its own start or end event (`W045`, `W046`), an event with several triggers (`W047`). Keep a warning only when the description demands it, and say why in `mapping.md`.
5. **Format** (`fmt`, or `tok fmt FILE -w`). Always keep the canonical form.
6. **Export** what the task asks for (`export`, or `tok bpmn` / `tok svg` / `tok export`): the BPMN file opens in Camunda Modeler and bpmn.io.
7. **Report briefly:** files created, number of elements, decisions that were ambiguous. Do not repeat the mapping in the chat.

No errors means the model follows the notation rules. It does not mean the model matches the description; that is what the mapping is for.

## Rules that models most often break

- A **message flow** (`message A -> B`) connects different pools. Between lanes of one pool use `flow`.
- A **splitting gateway** has a question as its label; its outgoing flows carry the answers: `flow g -> a "yes"`.
- **Join with the same kind of gateway** you split with (`and` with `and`, `or` with `or`).
- **After `eventgw`** only `catch` events or `receive` tasks.
- **Deadlines and failures during a task** are `boundary` events (`on=TASK timer=P3D`), not extra gateways. `noninterrupting` keeps the task running.
- A `sub` has its own `start` and `end` inside its block; flows do not cross its border.
- An `eventsub` has no incoming or outgoing flows; its first statement is a start event with a trigger.
- Identifiers are unique in the document and ASCII (`[A-Za-z_][A-Za-z0-9_-]*`); labels carry the text in any language.
- A line with a label or attributes on a `flow` has exactly one arrow: `flow g -> a "tak" when="zgoda = true"`; chains (`flow a -> b -> c`) have no attributes.
- Give each attribute and flag once per line. Only the triggers of an event (`message="A" message="B"`) may repeat; in a 1.0 document any other repetition is the deprecation warning `R090`, and only the last value counts.

## The core language (0.1 and 0.3)

- **Events** `start`, `end`, `catch`, `throw`, `boundary` with one trigger: `message="Name"`, `timer=P3D` (duration, date-time or cycle), `signal=`, `error=`, `escalation=`, `condition="FEEL"`, `link=`, or the flags `terminate`, `compensate`.
- **Activities** `task`, `user`, `service`, `manual`, `script`, `rule`, `send`, `receive`, `call`; markers `loop`, `parallel`, `sequential`, `compensation`.
- **Sub-processes** `sub`, `eventsub`, `adhoc` with a block.
- **Gateways** `xor`, `and`, `or`, `eventgw`; flows take `when="FEEL"`, `default`, `p=` (simulation).
- **Pools and lanes** (`pool ID "Label"` with a block of lanes; a pool without a block is a black box), **message flows**, **data** (`data`, `store`, `reads`, `writes`, `state=`), **notes** and **groups**.
- **Decision tables** (`decision ID hit=first` with rows `| input: type || output: type |`), **forms** (`form` with `text` and `field` lines), and the **simulation** statements.

## Executable models

When the document is `executable` (header flag), every `user` task should have `form=`, every `rule` task `decision=` and `result=`, every `script` task `expr=` and `result=`, every flow leaving an `xor`/`or` gateway with several outgoing flows either `when="…"` or `default`. FEEL expressions are written without a leading `=`. Avoid `service` tasks: they need a separate worker program; use `script` or `user` tasks instead. Forms and decisions are written in the same file (`form`, `decision` blocks) and exported together. The executable layer (spec §10.3) adds what Camunda 8 runs: multi-instance (`collection=`, `item=`, `output=`), call activities (`process=`, `binding=`), variable mappings (`inputs=`, `outputs=`), user task assignment (`assignee=`, `candidate-groups=`, `due=`), job types and headers, the REST connector, compensation, conditional events and listeners. Diagnostics `C…` report what Camunda 8 does not accept.

## BPMN beyond Camunda (language 0.5)

With the header `tok 0.5` (or `tok 1.0`) the model may use the rest of BPMN 2.0.2 process modelling: `transaction` with `end … cancel` and `boundary … cancel`, `complex` gateways (`activation=`), standard loops (`loop while="…" test-before loop-max=N`), events with several triggers (`parallel` for all of them), `receive`/`eventgw … instantiate`, process data (`item`, `input`, `output`, `property`, `type=`), `global` tasks, `interface`/`operation` and correlation `key` blocks (in documents with pools), `doc="…"` documentation on any element. They belong to the level `full` (the levels `basic` and `analytic` reject them) and are not executed by Camunda 8: in executable documents they are reported as `C100`–`C110`.

## Collaboration and choreography (language 0.6)

With the header `tok 0.6` (or `tok 1.0`) the model may describe the interactions between participants: `conversation`, `subconversation` and `callconversation` nodes between pools (`pools="a b"`, `messages="m1 m2"` naming message flows with `id=`), `choreography` blocks with `choreo ID "Label" from=a to=b message="M" reply="R"`, `subchoreo` and `callchoreo` activities, and `min=`/`max=` on pools for several participants of one kind. Their participants are pools, usually black-box pools. They belong to the level `full`; in executable documents they are reported as `C120`–`C123`. Checking with `class` (`descriptive`, `analytic`, `common-executable`, `full`) tests the model against a conformance class of BPMN 2.0.2.

When the model comes from a BPMN file drawn in a modelling tool, import it (`import`, or `tok import FILE.bpmn -o FILE.tok`) and keep the layout sidecar `FILE.tok.layout` next to the model: export and the diagram reuse the drawing for every element you do not change.

## Decision models (language 0.7)

With the header `tok 0.7` or `tok 1.0` decisions become DMN 1.5 decision models. Declare `inputdata ID "Label" type=…`, knowledge sources (`source`) and business knowledge models (`bkm ID params="a: number" expr="…"`), and connect them with `requires="ID …"` on decisions; a decision sees only what it requires, and FEEL refers to an element by its identifier (or `name="…"` for a name with spaces). Decision logic is a table (as before, plus `hit=rule-order|output-order`, `aggregation=`, output values `name: type = "a", "b"` and `default=`), `expr="FEEL"`, `invoke=BKM with="p = FEEL; …"`, a context (`entry NAME …` lines, optionally `result …` last), a list (`element …` lines) or a relation (rows without `||`); `params=` on a line makes it a function. DMN types are `item` blocks with `type=`, `allowed=` and components. Every expression of a 0.7 or 1.0 document is full FEEL (dates, durations, `for`, `some`, filters, contexts; `references/feel.md`). Check the model, test decisions with `eval` (or `tok eval FILE DECISION name=value …`; JSON objects and lists are accepted as values), export the DMN 1.5 files and their diagrams with `export`, and read existing DMN files with `import`. In executable documents `C130`–`C132` report what Camunda 8 does not execute.

## Simulation

Add `simulate cases=N seed=N arrival="exp(2h)"`, `resource ID count=N` and `variable NAME "DIST"` statements and `by=`, `duration="DIST"`, `assign=`, `delay=` and `p=` attributes; run the simulation (`sim` with a `seed`, or `tok sim FILE --seed 1`). Report the indicators with the seed you used. Ask for the event log only when the analysis needs it (`log: true`, or `tok sim --log FILE.csv`).
