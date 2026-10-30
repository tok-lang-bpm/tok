---
name: bpmn-modelling
description: Model processes in BPMN 2.0 (OMG) XML. Turns a process description, e.g. a table of steps, into a valid .bpmn file with an SVG diagram that opens in bpmn.io and Camunda Modeler; checks and repairs existing .bpmn models; prepares executable models for Camunda 8 (zeebe extensions, forms, DMN, FEEL). Works with the tok command, which checks the model, lays it out and draws it. Use for BPMN 2.0 process models, .bpmn files, BPMN diagrams and Camunda 8 executable processes.
---

# BPMN 2.0 modelling from a process description

This skill divides the work into two parts:

- **You (the agent)** are responsible for the semantics: which BPMN elements fit the steps of the description, who does what, how the process flows.
- **The `tok` command** is responsible for the mechanics: checking the BPMN rules, the automatic layout (BPMNDI coordinates) and the SVG drawing.

Never write coordinates by hand. You write only the semantic part of the XML; the tok command generates the rest.

## The tok command

| Task | Command |
|---|---|
| check the model | `tok check FILE.bpmn` (`--steps`, `--steps-from DESC.md`, `--camunda`, `--json`) |
| lay out the diagram | `tok bpmn FILE.bpmn` (writes a fresh diagram, BPMNDI, into the file; `-o OUT.bpmn` writes it elsewhere) |
| draw the diagram | `tok svg FILE.bpmn` (writes `FILE.svg` next to the model) |

`tok check` exits with code 1 when the model has errors. Each finding has a code, the id of the element and the line of the element in the file. Messages are in English (`--lang pl` gives them in Polish); the codes are the same in both languages.

When your client has the **tok MCP server**, its tool `check_bpmn` does the same check: give the file as `path` (the server reads it), or the document as `xml` when you have no file tools; `steps`, `description` with the text of the process description, and `camunda` are the options of the command. If the tool answers that no document was given although you passed one, your client dropped the argument: write the file and give `path`. Layout and drawing still need the `tok` command.

If the command is missing, install it: `pip install tok-lang` (the Python package with the engine; it gives the `tok` command), or use the native `tok` program. `tok --version` shows whether it runs. If you cannot run `tok`, write the model by `references/elements.md` and say clearly that it has not been checked.

## Workflow

1. **Read the whole process description** (e.g. `process.md`). Identify the participants (pools) and the roles (lanes). An external company whose inner workings are unknown is a separate, black-box pool.

2. **Before writing XML, create `mapping.md`** with a table:

   | Step | BPMN element | Lane / pool | Cue in the description | Doubts |
   |---|---|---|---|---|

   For each step give the element type and **the words of the description** that decided it. The cues are described in `references/elements.md`, section A. Split a step with one description but several elements (e.g. a task and a boundary event) into separate rows. Write the doubts down; do not hide them.

3. **Write the `.bpmn` file (semantics only, without `bpmndi:`).** Start from `references/template.bpmn` and take the XML fragments from `references/elements.md`, section B. These conventions apply:
   - **The name of every element starts with the step number from the description**, e.g. `12. Request for quotation`, `5.1. Review of merit`, `5a. Intervention`. Only plain end events and start events inside a sub-process may have no number.
   - Name splitting gateways with a question (`Approved?`), and the branches with labels from the description (`yes`/`no`, `Y`/`N`).
   - Every element has a readable `id` (e.g. `Task_12`, `Gw_13`, `Catch_14`). Declare messages, signals, errors and escalations at the level of `<bpmn:definitions>` and refer to them with `messageRef`, `signalRef`, `errorRef`, `escalationRef`.
   - Put every node of the process (except the elements inside sub-processes) into exactly one lane (`<bpmn:flowNodeRef>`).
   - If the description has a column of documents, model at least **data stores** for permanent registers (records, database, budget) and **data objects** for documents whose state the description gives in brackets (`Application [submitted]` → a data object with the state `submitted`). You do not have to draw every document at every step. Choose those that show the flow of information, and record the choice in `mapping.md`.

4. **Run the tok command** in the folder of the model:

   ```
   tok check model.bpmn --steps --steps-from process.md
   ```

   and, when it reports no errors:

   ```
   tok bpmn model.bpmn
   tok svg model.bpmn
   ```

   `--steps-from` compares the step numbers in the model with the first column of the table in the description file and reports the missing (`K02`) and the extra (`K03`) ones. If the process description has no step numbers, leave out `--steps`, `--steps-from` and the numbering convention for names.

   `tok check` checks the model. If there are **errors**, it lists them; fix them before laying out. `tok bpmn` **rewrites the file**: it adds the coordinates (BPMNDI), tidies the formatting and removes XML comments. So do not keep notes in comments; keep them in `mapping.md`. `tok svg` writes `model.svg` next to the model. You do not need to look at the SVG, because the layout is generated deterministically.

5. **Fix until there are 0 errors and 0 warnings.** Each message has a code (e.g. `E008`), the id of the element and its line; the meaning of the codes is in `references/elements.md`, section C. Fix every warning (`W…`, `K…`) as well: BPMN linters such as bpmnlint of bpmn.io report the `W…` rules as errors, so a model with a warning left is rejected by them. Keep a warning only when the description demands the construct, and say so in `mapping.md`. The notes (`L…`) concern modelling style.

   After **every** change of the model run the whole sequence again (`tok check`, then `tok bpmn` and `tok svg`), not `tok check` alone. `tok check` on a file with the old layout reports stale coordinates (`D01`, `D02`), and `tok bpmn` refreshes the layout.

   **0 errors does not mean the model is good.** The validator checks the rules of the notation and the consistency of the structure. It does not check whether the element type fits the description of the step — that is your responsibility, through the mapping.

6. **Finish with a short summary in the chat:** which files were created (`model.bpmn`, `model.svg`, `mapping.md`), how many elements there are, which decisions were ambiguous. Do not copy the whole mapping into the chat.

## Executable model for Camunda 8

If the request concerns a process that is to run in the engine (Camunda 8 Run, Tasklist, Operate), three steps are added:

1. In `mapping.md` add the column **Process data**: which variables are created in the step (e.g. `price`, `approved`) and who fills them in (a form, a DMN table, a FEEL script).
2. Add the `zeebe:` extensions according to `references/elements.md`, section D: forms for user tasks (`.form` files next to the model), `zeebe:calledDecision` for business rule tasks (a `.dmn` file next to the model), `zeebe:script` for script tasks, FEEL conditions `=…` on every outgoing flow of an XOR/OR gateway except the default one.
3. Run the check with the option `--camunda`:

   ```
   tok check process.bpmn --camunda
   ```

   and, when it reports no errors, `tok bpmn process.bpmn` and `tok svg process.bpmn`. The `C…` codes are described in `references/elements.md`, section D. Avoid service tasks (`serviceTask`): without a separate program (a worker) the instance stops at them. Use script tasks with a FEEL expression or user tasks instead.

The user deploys and starts the process in Camunda Modeler (the deploy and start buttons). You prepare the files and check them with the tok command.

## BPMN rules that models most often break

- **A message flow (`messageFlow`) connects only different pools.** Between the lanes of one pool always use a sequence flow. A message from an external company is modelled as a message flow from its pool.
- **A sequence flow does not leave its process or sub-process.** The inside of a sub-process has its own start and end events.
- **After an event-based gateway** there are only intermediate catch events (message, timer, signal, condition) or receive tasks. The gateway itself has no conditions.
- **An interrupting boundary event** (the default) takes the task away. **A non-interrupting one** (`cancelActivity="false"`) starts a parallel path, and the task goes on. An error boundary event always interrupts.
- **An escalation** is thrown inside a sub-process (a throw event or an end event) and caught by an escalation boundary event on that sub-process, with the same `escalationRef`.
- **An event sub-process** (`triggeredByEvent="true"`) has no incoming or outgoing flows. It is started by its only start event with a trigger (e.g. a signal).
- **A signal** is a broadcast to many recipients at once, and **a message** has a specific addressee.
- **An inclusive gateway (OR)** splits into "one, several or none" of the paths. It needs a default flow (`default`) and a join with the same OR gateway.
- **A parallel gateway (AND)** has no conditions on its outgoing flows. An AND join waits for all branches.

## What not to do

- Do not edit `bpmndi:BPMNDiagram` or the coordinates. After every change of the semantics run `tok bpmn` again, and the layout is generated anew.
- Do not remove steps of the description to get rid of a warning. Fix the structure.
- Do not add elements that the description does not justify. If you add something for the correctness of the notation (e.g. a start event in a sub-process, a joining gateway), note it in `mapping.md`.

## Files of the skill

- The `tok` command: `tok check`, `tok bpmn`, `tok svg` (`--help` shows the options, `--json` gives the result for further processing, `--steps-from` checks that every step is in the model, `--camunda` checks the executable model for Camunda 8).
- `references/elements.md`: how to recognise the element type in a description (A), XML templates (B), validator codes (C), the executable model for Camunda 8 (D).
- `references/template.bpmn`: a skeleton with a collaboration, a pool, lanes and an external pool.
- `references/symbols.md`: all BPMN 2.0.2 symbols with their names, the element of the specification and whether Camunda 8 executes them; when the user describes a symbol ("an envelope", "a clock on the edge of a task"), find the element here.
