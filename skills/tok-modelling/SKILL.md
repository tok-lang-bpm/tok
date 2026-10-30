---
name: tok-modelling
description: Write, check and repair process models in the tok language (.tok files) and export them to BPMN 2.0 XML, SVG, DMN decision models, Camunda forms and simulation results; import BPMN and DMN files into tok. Use for .tok files, for turning a process description (e.g. a table of steps) into a model, for decision models (DMN), and for executable or simulated processes described in tok. Works with the tok MCP server when it is configured and with the tok command otherwise.
---

# Modelling processes in tok

tok is a plain-text language for BPMN 2.0 process models with their DMN decisions, Camunda forms and simulation parameters (language 1.0, the stable language; documents of 0.1–0.7 stay valid and mean the same). You write the semantics; the tok engine checks the model, prints it in canonical form and produces every other format (BPMN XML with layout, SVG, DMN, forms, simulation). Never write BPMN or DMN XML or diagram coordinates by hand.

## What to read, and when

The whole language fits on one page, `references/cheatsheet.md` (through the MCP server: the tool `reference` with `topic: cheatsheet`, or the resource `tok://reference/cheatsheet`). Read it once, before the first model of a session, and read nothing else before the first check. The checker is the teacher: every diagnostic names a code, a line and an element, and its message says what to change. Open `references/diagnostics.md` (`topic: diagnostics`) only for a message you cannot act on. The specification `references/spec.md` (`topic: spec`) is normative and long: open the one section you need (the executable layer, decisions, simulation), never the whole file, and never before a model has been checked. FEEL expressions are in `references/feel.md` (`topic: feel`). Every page you read comes back with every later turn of the conversation, so one page read once is cheap, and the specification read before the first check is the most expensive thing you can do.

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
| language reference | `reference` with `topic` (`cheatsheet`, `diagnostics`, `feel`, `spec`, `cli`) | `tok cheatsheet`; the files in `references/` |

The MCP tools return file contents instead of writing files: write what they return with your file tools. A tool error (`isError`) explains what to fix; a document with errors is a successful call that lists them (`errors` > 0). Paths are read relative to the working directory of the server; give absolute paths when unsure. Without the MCP server, install the command if it is missing: `pip install tok-lang` (the engine; the same package runs the server, `tok mcp`) or `cargo install tok-cli`. If you can run neither, write the model by the cheat sheet and say clearly that it has not been checked.

## Workflow

1. **Read the description.** List the participants (pools) and the roles (lanes), the activities in their order, the decisions, the parallel work, the deadlines and messages to other organisations, and the start and the end of every path.
2. **Write the model** in a `.tok` file. New models carry the header `tok 1.0`; an existing model keeps its header (`tok 0.1`, `0.3`, `0.5`, `0.6` or `0.7`) unless it needs elements of a later version. One pool for the organisation, one lane for each actor, one activity for each step named with the words of the description, `xor` for a decision with a label on every outgoing flow, `and` for parallel work with a matching join, a `start` event, and an `end` event on every path. One statement per line, short ASCII identifiers (`check-request`), labels in the language of the description; nodes declared inside lanes, all `flow` lines after the pools.
3. **Check** (`check`, or `tok check FILE --json`). Fix every error and every warning and check again, until the tool reports no errors and no warnings. Keep the model as small as the description allows. The warnings of the BPMN rules (`W…`) are what BPMN linters such as bpmnlint of bpmn.io report as errors, so a model with a warning left is rejected by them; the message of each warning says what to change (a condition `when="…"` on a branch next to a default flow, a gateway where several flows leave a task, a start and an end event inside a sub-process). Keep a warning only when the description demands the construct, and say so in the report.
4. **Format** (`fmt`, or `tok fmt FILE -w`). Always keep the canonical form.
5. **Export** what the task asks for (`export`, or `tok bpmn` / `tok svg` / `tok export`): the BPMN file opens in Camunda Modeler and bpmn.io.
6. **Report briefly:** the files written, the number of elements, the decisions that were ambiguous. When the description is a numbered table of steps, or the person asks for a review of your choices, add `mapping.md` next to the model: one row per step with the tok element (kind and id), the lane, the words of the description that justify the choice, and doubts. The person reviews your decisions through the table and your model through the diagram; do not repeat the table in the chat. Without file tools, answer with the final model in one fenced code block marked ```tok.

No errors and no warnings mean the model follows the notation rules. They do not mean the model matches the description; that is what the mapping and the diagram are for.

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

## Beyond a process model

The cheat sheet covers the core language (events, activities, sub-processes, gateways, pools and lanes, message flows, data, notes, decision tables, forms, simulation statements). When the task asks for more, read `references/layers.md`, one section at a time: executable models for Camunda 8 (forms, decisions and scripts on the tasks, the `C…` diagnostics), the rest of BPMN 2.0.2 beyond Camunda (transactions, complex gateways, standard loops, multiple events, process data), collaboration and choreography between pools, DMN 1.5 decision models (`inputdata`, `bkm`, `requires`, contexts, `eval`), models imported from BPMN or DMN files with their layout, and simulation (`simulate`, `resource`, `variable`, `sim`).
