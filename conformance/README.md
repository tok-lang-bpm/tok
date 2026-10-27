# Conformance corpus

Every implementation of tok must reproduce these outputs exactly.

| Folder | Contents |
|---|---|
| `cases/` | input documents, including documents with errors |
| `expected/NAME.tok` | canonical text (`tok fmt`) |
| `expected/NAME.diag` | diagnostic codes of the core level (lexer, parser, identifiers, decision rows), sorted, one per line |
| `expected/NAME.bpmn` | semantic BPMN 2.0 XML without layout, attributes sorted (only for documents without errors) |
| `expected/NAME.levels` | every diagnostic code of `tok check` at the levels `basic`, `analytic` and `full` (full conformance level) |
| `miwg/` | reference models of the BPMN Model Interchange Working Group test suite (see `miwg/README.md`); their semantic content must survive import, printing and export (`rust/tok-core/tests/miwg.rs`, results in `docs/miwg.md`) |
| `import/` | BPMN 2.0 files drawn in Camunda Modeler for the course *Modelowanie procesów informacyjnych*; `tok import` must convert each of them to tok and back without BPMN errors |
| `camunda/` | executable conformance (language 0.3): tok documents that `run.py` deploys to a running Camunda 8 and drives to their expected end events |

Cases 01–32 are written for languages 0.1 and 0.3; cases 33 and following for language 0.5 (one or more per section of the language: transaction and cancel events, complex gateway, standard loop, multiple events, instantiation, process data, resources and performers, global tasks and interfaces, correlation keys, the Camunda codes `C100`–`C110`, the errors `R060`–`R066`, and the additions of `docs/design-0.5.md` §13 in cases 44–46).

The expected files are produced by two generators. Run them after a deliberate change of the language and review the differences before committing:

- `python conformance/generate.py` regenerates the files of the cases whose header declares language 0.1 or 0.3, with the pure-Python implementation (frozen at language 0.3). It skips every other case.
- `TOK_BLESS=1 cargo test -p tok-core --test conformance` (in `rust/`) regenerates the files of the cases whose header declares a newer language, with the Rust engine. It never writes the files of the older cases.

Without `TOK_BLESS` the same test compares the Rust engine with every expected file, old and new; `julia/Tok.jl` checks the core level (`.tok`, `.diag`, `.bpmn`) of all cases, and the Python test suite only the cases of language 0.1 and 0.3.
