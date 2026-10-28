# Conformance corpus

Every implementation of tok must reproduce these outputs exactly.

| Folder | Contents |
|---|---|
| `cases/` | input documents, including documents with errors |
| `expected/NAME.tok` | canonical text (`tok fmt`) |
| `expected/NAME.diag` | diagnostic codes of the core level (lexer, parser, identifiers, decision rows), sorted, one per line |
| `expected/NAME.bpmn` | semantic BPMN 2.0 XML without layout, attributes sorted (only for documents without errors) |
| `expected/NAME.levels` | every diagnostic code of `tok check` at the levels `basic`, `analytic` and `full` (full conformance level) |
| `expected/NAME.GRAPH.dmn` | for documents of languages 0.7 and 1.0 without parse errors: the DMN 1.5 file of each decision requirements graph (named after its first decision), with its diagram |
| `miwg/` | reference models of the BPMN Model Interchange Working Group test suite (see `miwg/README.md`); their semantic content must survive import, printing and export (`rust/tok-core/tests/miwg.rs`, results in `docs/miwg.md`) |
| `dmn-tck/` | test cases of the DMN Technology Compatibility Kit, compliance levels 2 and 3 (see `dmn-tck/README.md`); every model must import, print and evaluate to the expected results, apart from the cases listed with a reason in `rust/tok-core/tests/fixtures/dmn-tck/baseline.tsv` (`rust/tok-core/tests/dmn_tck.rs`, results in `docs/dmn-tck.md`) |
| `import/` | BPMN 2.0 files drawn in Camunda Modeler for the course *Modelowanie procesów informacyjnych*; `tok import` must convert each of them to tok and back without BPMN errors |
| `camunda/` | executable conformance (language 0.3): tok documents that `run.py` deploys to a running Camunda 8 and drives to their expected end events; for language 1.0 it also runs the process of case 63, exported with the native `tok`, and for languages 0.7 and 1.0 it deploys the DMN files of cases 61 and 63 and compares Camunda's decision results with `tok eval` |

Cases 01–32 are written for languages 0.1 and 0.3; cases 33–46 for language 0.5 (one or more per section of the language: transaction and cancel events, complex gateway, standard loop, multiple events, instantiation, process data, resources and performers, global tasks and interfaces, correlation keys, the Camunda codes `C100`–`C110`, the errors `R060`–`R066`, and the additions of `docs/design-0.5.md` §13 in cases 44–46); cases 47–52 for language 0.6 (conversations, choreographies, the events of choreographies, participant multiplicity, the Camunda codes `C120`–`C123` and the errors `R070`–`R074`); cases 53 and following for language 0.7 (decision requirements graphs, boxed expressions and DMN types, the decision tables of 0.7, FEEL of conformance level 3 in a process, the Camunda codes `C130`–`C132`, the errors of 0.7, the statements of 0.7 under an older header, the rows of relations and header cells, and an executable decision model for Camunda 8); cases 62 and following for language 1.0 (a document with the elements of every layer, an executable document for Camunda 8 with a process, forms and decisions, the errors of the header and of the version, and the deprecation `R090`).

The expected files are produced by two generators. Run them after a deliberate change of the language and review the differences before committing:

- `python conformance/generate.py` regenerates the files of the cases whose header declares language 0.1 or 0.3, with the pure-Python implementation (frozen at language 0.3). It skips every other case.
- `TOK_BLESS=1 cargo test -p tok-core --test conformance` (in `rust/`) regenerates, with the Rust engine, the files of every case `generate.py` skips: the cases whose header declares a newer language or is not a version at all (case 66). It never writes the files of the older cases.

Without `TOK_BLESS` the same test compares the Rust engine with every expected file, old and new; `julia/Tok.jl` checks the core level (`.tok`, `.diag`, `.bpmn`) of all cases, and the Python test suite only the cases of language 0.1 and 0.3.
