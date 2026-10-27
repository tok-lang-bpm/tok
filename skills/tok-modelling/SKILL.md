---
name: tok-modelling
description: Write, check and repair process models in the tok language (.tok files) and export them to BPMN 2.0 XML, SVG, DMN decision tables, Camunda forms and simulation results. Use for .tok files, for turning a process description (e.g. a table of steps) into a model, and for executable or simulated processes described in tok.
---

# Modelling processes in tok

tok is a plain-text language for BPMN 2.0 process models. You write the semantics; the `tok` tool checks the model, prints it in canonical form and produces every other format (BPMN XML with layout, SVG, DMN, forms, simulation). Never write BPMN XML or coordinates by hand.

The whole language fits on one page: read `references/cheatsheet.md` before writing the first line. The full specification is `references/spec.md`; read a section of it only when the cheat sheet is not enough.

## Workflow

1. **Read the process description completely.** Identify participants (pools), roles (lanes), the steps, decisions, deadlines, messages to other organisations and the documents.
2. **Write a mapping first** (`mapping.md`): one row per step with the tok element (kind and id), the lane, the words in the description that justify the choice, and doubts. The person you work for reviews your model through this table.
3. **Write the `.tok` file.** One statement per line, explicit identifiers (short, ASCII, e.g. `check-request`), labels in the language of the description. Declare nodes inside lanes; write all `flow` lines after the pools.
4. **Check:** `tok check FILE --json`. Fix every error and re-run until there are no errors. Each diagnostic has a code and a line; codes are explained in `references/diagnostics.md`. Warnings are fixed or justified in `mapping.md`.
5. **Format:** `tok fmt FILE -w`. Always commit the canonical form.
6. **Export** what the task asks for: `tok bpmn FILE` (opens in Camunda Modeler and bpmn.io), `tok svg FILE`, `tok export FILE -o out/` (BPMN, SVG, DMN, forms), `tok sim FILE` (indicators).
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

## Executable models

When the document is `executable` (header flag), every `user` task should have `form=`, every `rule` task `decision=` and `result=`, every `script` task `expr=` and `result=`, every flow leaving an `xor`/`or` gateway with several outgoing flows either `when="…"` or `default`. FEEL expressions are written without a leading `=`. Avoid `service` tasks: they need a separate worker program; use `script` or `user` tasks instead. Forms and decisions are written in the same file (`form`, `decision` blocks) and exported with `tok export`.

## BPMN beyond Camunda (language 0.5)

With the header `tok 0.5` the model may use the rest of BPMN 2.0.2 process modelling: `transaction` with `end … cancel` and `boundary … cancel`, `complex` gateways (`activation=`), standard loops (`loop while="…" test-before loop-max=N`), events with several triggers (`parallel` for all of them), `receive`/`eventgw … instantiate`, process data (`item`, `input`, `output`, `property`, `type=`), `global` tasks, `interface`/`operation` and correlation `key` blocks (in documents with pools). They belong to the level `full` (the levels `basic` and `analytic` reject them) and are not executed by Camunda 8: in executable documents they are reported as `C100`–`C110`.

## Collaboration and choreography (language 0.6)

With the header `tok 0.6` the model may describe the interactions between participants: `conversation`, `subconversation` and `callconversation` nodes between pools (`pools="a b"`, `messages="m1 m2"` naming message flows with `id=`), `choreography` blocks with `choreo ID "Label" from=a to=b message="M" reply="R"`, `subchoreo` and `callchoreo` activities, and `min=`/`max=` on pools for several participants of one kind. Their participants are pools, usually black-box pools. They belong to the level `full`; in executable documents they are reported as `C120`–`C123`. `tok check FILE --class descriptive|analytic|common-executable|full` checks the model against a conformance class of BPMN 2.0.2.

When the model comes from a BPMN file drawn in a modelling tool, import it with `tok import FILE.bpmn -o FILE.tok`: the drawing is kept in `FILE.tok.layout`, and `tok bpmn`, `tok svg` and `tok export` reuse it for every element you do not change. Keep that file next to the model.

## Simulation

Add `simulate`, `resource` and `variable` statements and `by=`, `duration=`, `p=` attributes; run `tok sim FILE --seed 1`. Report indicators with the seed you used.
