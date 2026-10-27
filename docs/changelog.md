# Changelog

All notable changes to the tok language and its reference tools are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/). Until 1.0 a minor version may change the language; a patch version never does.

## [0.3.0] — 2026-10-27

Language 0.3 (`tok 0.3` in the header; documents written for 0.1 stay valid and mean the same). Reference: `docs/spec.md` §5.3, §10, §10.3, §13; design notes in `docs/design-0.3.md`.

### Added

- `adhoc` sub-process (BPMN `adHocSubProcess`): activities in any order, `sequential` ordering; checker rule `R050`.
- Compensation in any document: `compensate=HANDLER` on an activity (boundary compensation event and association), the handler flag `compensation`, `throw|end … compensate activity=ID` (`activityRef`); checker rule `R051`.
- Catch-all error and escalation events: the flags `error` and `escalation` without a name on boundary events and event sub-process starts.
- Executable layer for Camunda 8 (only in executable documents): multi-instance collections (`collection`, `item`, `output`, `output-item`, `until`), ad-hoc activation (`active`, `until`, `wait`), call activities (`binding`, `version`, `child-vars`, `parent-vars`, `business-id`), decision and form bindings, variable mappings (`inputs`, `outputs`), user task assignment and scheduling (`assignee`, `candidate-users`, `candidate-groups`, `due`, `follow-up`, `priority`), jobs (`retries`, `headers`, `type` on message throw and end events), the REST connector (`connector=rest`, `method`, `url`, `body`, `query`, `headers-in`, `result`), conditional filters (`events`), execution and task listeners (`listeners`), process version tags (`version-tag`).
- Checker codes `R052`–`R055`, `C022`, `C090`, `C091`; the levels `basic` and `analytic` reject the executable layer (`R040`); executable-layer attributes in documents that are not executable give `R055` and are ignored by the exporter.
- FEEL subset: `now()`, `today()`, `duration("PnDTnHnMnS")` and date/time arithmetic with durations.
- BPMN import reads all of the above back; an imported file that uses them is printed as `tok 0.3`.
- Simulation of multi-instance collections (parallel and sequential, `output`, `until`), ad-hoc sub-processes (activated elements start at once) and compensation (handlers of completed activities run when a compensation event is reached).
- Conformance: cases 19–32 and `expected/*.levels` (all diagnostic codes at the three levels); executable conformance on Camunda 8 Run 8.10.7 in `conformance/camunda/` (12 cases driven to their expected end events by `run.py`).

### Implementations

- Python, the Rust engine (`tok-core`, `tok-cli`, `tok-py`) and the Julia core implement language 0.3 and pass the whole conformance corpus byte for byte; Rust and Python are additionally compared on 62 documents (390 outputs).
- Plan (`docs/roadmap.md`): from 0.4 the Rust engine is the only engine — Python and R call it, the pure-Python implementation stays frozen at language 0.3 as a test oracle; Julia calls the Rust engine, keeps its independent core and gets native simulation and calibration.

### Not included

- Error code and message variables on catching error events: Camunda 8.10 accepts `errorCodeVariable`/`errorMessageVariable` in the XML but ignores them; variables sent with an error are merged with `outputs=` on the catching event.

## [0.2.0] — 2026-10-27

The language is unchanged (documents still start with `tok 0.1`); this release adds the Rust engine.

### Added

- Rust workspace `rust/`:
  - `tok-core`: lexer, parser, model, checker (all tok-level codes, FEEL subset checking), canonical printer, semantic BPMN export, DMN and form export, BPMN import;
  - `tok-bpmn`: the BPMN 2.0 rules (`E`, `W`, `L` and the Camunda `C` profile), automatic layout and SVG rendering, byte-identical to the Python validator;
  - `tok-cli`: native `tok` command with the same commands, options and output as the Python implementation (`sim` and `eval` follow in 0.4);
  - `tok-py`: Python module `tok_engine` on the Rust core (PyO3, abi3, Python ≥ 3.8), built with maturin, used for parity testing.
- Parity tests: the conformance corpus, full `check --json` output in English and Polish, BPMN, SVG, DMN and form files and BPMN import compared byte for byte between Python and Rust (Rust test suite and `python/tests/test_engine_parity.py`).

### Fixed

- `tok bpmn`, `tok svg` and `tok export` on a document without any flow elements or pools stop with the message "nothing to draw: the model has no elements" (Polish: „nie ma czego narysować: model nie zawiera elementów”) instead of a Python traceback.

### Packaging

- Version 0.2.0 of the Python package, the Julia package and the Rust crates. The installed Python package remains pure Python; the Rust build is optional until prebuilt wheels exist.

## [0.1.1] — 2026-10-27

### Changed

- Polish diagnostics (`--lang pl`) are now fully Polish: the detail text of FEEL errors and warnings (`F001`, `F002`) in conditions, expressions and decision table cells is translated.
- Overlap warnings for decision tables with `hit=unique` (`D010`) show a concrete example value for every input, also for cells written as `-` (a value used by another rule, a number inside the other rule's range, or `null` when nothing else fits).
- `tok export -o FOLDER` prints the written paths with the platform's native separator.
- The BPMN 2.0 rules (`E`, `W`, `L` codes) run only on documents without errors of any layer; previously errors of layers `F`, `D`, `M` and `C` did not stop them, so style notes appeared next to real errors.
- `exporterVersion` in exported BPMN XML is the language version of the document (`0.1`) instead of the tool version, so every implementation exports identical bytes.

### Packaging

- Python package and Julia package version 0.1.1. Until 1.0, packages are installed from the Codeberg archive; registry releases come with the stable version.

## [0.1.0] — 2026-10-27

First public version.

- Language specification 0.1: pools, lanes, all BPMN events, activities, sub-processes and gateways, sequence and message flows, data objects and stores, notes and groups, decision tables, forms, executable profile for Camunda 8, simulation parameters (`docs/spec.md`).
- Python reference implementation (`tok-lang`): `check` with diagnostics in English and Polish, canonical `fmt`, BPMN 2.0 export with automatic layout and SVG, DMN and Camunda form export, BPMN import, FEEL subset and decision evaluation, discrete-event simulation with event log and indicators, command line `tok`.
- Julia implementation of the core (`Tok.jl`): parsing, text-layer diagnostics, canonical printing and semantic BPMN export, byte-identical to Python on the conformance corpus.
- Conformance corpus of 18 documents with expected outputs; import corpus of BPMN models.
- Agent skill `tok-modelling`, cheat sheet, documentation of the command line, diagnostics, FEEL subset, simulation and import.
