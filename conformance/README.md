# Conformance corpus

Every implementation of tok must reproduce these outputs exactly.

| Folder | Contents |
|---|---|
| `cases/` | input documents, including documents with errors |
| `expected/NAME.tok` | canonical text (`tok fmt`) |
| `expected/NAME.diag` | diagnostic codes of the core level (lexer, parser, identifiers, decision rows), sorted, one per line |
| `expected/NAME.bpmn` | semantic BPMN 2.0 XML without layout, attributes sorted (only for documents without errors) |
| `expected/NAME.levels` | every diagnostic code of `tok check` at the levels `basic`, `analytic` and `full` (full conformance level) |
| `import/` | BPMN 2.0 files drawn in Camunda Modeler for the course *Modelowanie procesów informacyjnych*; `tok import` must convert each of them to tok and back without BPMN errors |
| `camunda/` | executable conformance (language 0.3): tok documents that `run.py` deploys to a running Camunda 8 and drives to their expected end events |

`generate.py` regenerates `expected/` with the Python reference implementation. Run it after a deliberate change of the language and review the differences before committing.
