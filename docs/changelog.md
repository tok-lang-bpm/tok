# Changelog

All notable changes to the tok language and its reference tools are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/). Until 1.0 a minor version may change the language; a patch version never does.

## [0.7.0] — 2026-10-28

Language 0.7 (`tok 0.7` in the header; documents written for 0.1, 0.3, 0.5 and 0.6 stay valid, mean the same and keep every output): decision models of DMN 1.5 — decision requirements graphs, boxed expressions, FEEL at conformance level 3 — with evaluation, DMN 1.5 export with diagrams and DMN import; the DMN TCK as a conformance gate. Reference: `docs/spec.md` §4, §9, §10.4, §11–§15; `docs/feel.md` §9; design notes and the decisions taken during implementation in `docs/design-0.7.md`.

### Added

- Decision requirements graphs: `inputdata`, `source` (knowledge source: `kind=`, `uri=`) and `bkm` (business knowledge model: `params=`, `type=`) statements; `requires="ID …"` on decisions, business knowledge models and knowledge sources gives information, knowledge and authority requirements by the kind of the element named; `name=` for FEEL names with spaces; `question=` and `answers=` on decisions; `type=` on every element.
- Boxed expressions in the block or on the line of a decision, a business knowledge model and each line: literal expressions (`expr=`), contexts (`entry NAME …` lines, optionally ending with `result …`), lists (`element …` lines), relations (table rows without `||`), invocations (`invoke=BKM with="param = FEEL; …"`) and function definitions (`params=` on a line); every line may hold its own nested box.
- Decision tables of 0.7: hit policies `rule-order` and `output-order`, `aggregation=sum|count|min|max` with `collect`, input and output values in the header (`name: type = "a", "b"`; `priority` and `output-order` order the rules by them), default outputs (`default=`), every FEEL type and DMN type in the header.
- DMN types: in a 0.7 document `item ID type= allowed= [collection]` with a block of components (`item NAME type=`), exported as `itemDefinition`s; values that do not conform to the type of an element are `null`, after the implicit conversions of DMN 1.5.
- FEEL at conformance level 3 for every expression of a 0.7 document (`docs/feel.md` §9): temporal values with their arithmetic and comparison, the standard function library of DMN 1.5 (string, list, numeric, boolean, temporal, range and context functions, regular expressions of XPath), contexts, lists, ranges, filters, paths, `for`, `some`, `every`, `instance of`, function definitions, named arguments, names with spaces resolved against the document, numbers with 34 significant digits.
- `tok eval` evaluates a decision of a 0.7 document with the decisions, input data and business knowledge models it requires and prints its DMN value; values on the command line may be JSON objects and lists, and text for input data of a temporal type is read as that type. `tok sim` evaluates the business rule tasks and the expressions of 0.7 documents the same way; `tok.api.eval_decision` (Python) and `tok_eval` (R) evaluate decision requirements graphs.
- DMN 1.5 export: `tok export` writes one DMN file per connected decision requirements graph of a 0.7 document (`NAME.dmn`, named after its first decision) with its DMN types and a DMNDI diagram laid out automatically, and its picture `NAME.drd.svg`; a decision model without a process needs no BPMN file. The Python `export` and the R `tok_export` return the same files.
- DMN import: `tok import FILE.dmn` reads DMN 1.1–1.5 into a 0.7 document — decisions with their logic, input data, business knowledge models, knowledge sources, requirements and item definitions; the boxed `conditional`, `filter`, `for`, `some` and `every` of DMN 1.5 become literal expressions; everything not kept is reported.
- Checker codes `R076`–`R081` (cycles of requirements, boxed expression forms, parameters and bindings, names used without a requirement, duplicate names, types), `D006`–`D009` (output values, aggregations, relation rows, header cells of 0.7) and `C130`–`C132` (business knowledge models with an invocation, function definitions with a body that is not a literal expression, decisions without logic in executable documents for Camunda); the levels `basic` and `analytic` reject the new statements (`R040`).
- DMN TCK conformance gate (`rust/tok-core/tests/dmn_tck.rs`): 143 models of compliance levels 2 and 3 of the DMN Technology Compatibility Kit (Apache-2.0) in `conformance/dmn-tck`, imported, printed, parsed again and evaluated for every test case; 126 of 126 results of level 2 and 3313 of 3331 results of level 3 pass, the others are listed with their reasons in the baseline. Pass counts per test in `docs/dmn-tck.md`.
- Conformance cases 53–61 (decision requirements graphs, boxed expressions and types, the decision tables of 0.7, FEEL of level 3 in a process, the Camunda codes `C130`–`C132`, the errors of 0.7, the statements of 0.7 under an older header, relation rows and header cells, an executable decision model for Camunda 8) with the DMN files of every graph among the expected outputs; the Julia core parses, prints and exports them (core level, 240 checks).
- DMN files of executable documents for Camunda 8 write FEEL names with spaces or symbols in backticks, the form Camunda's FEEL reads; the executable conformance runner `conformance/camunda/run.py` deploys the decisions of case 61 to Camunda 8.10 and compares their evaluation with `tok eval` (names of several words, a dash, a filter over a component of two words, an invocation, a context and a list).

### Changed

- The decimal arithmetic of the engine rounds to a precision set per computation: 28 digits, as before, for documents of earlier languages, 34 digits for FEEL of language 0.7.
- The cheat sheet printed by `tok cheatsheet` describes language 0.7.
- The DMN export of documents of earlier languages is unchanged (DMN 1.3, one file per decision table).

## [0.6.0] — 2026-10-28

Language 0.6 (`tok 0.6` in the header; documents written for 0.1, 0.3 and 0.5 stay valid and mean the same): conversations, choreographies and participant multiplicity; BPMN conformance classes in the checker; a layout sidecar that keeps the drawing of imported files. Reference: `docs/spec.md` §4, §5.5, §5.7, §6.4, §10.4, §11, §13–§16; design notes and the decisions taken during implementation in `docs/design-0.6.md`.

### Added

- Conversations: `conversation`, `subconversation` (with a block of conversation nodes) and `callconversation … collaboration=ID` with `pools=`, `messages=` and `map="inner:outer; …"` (BPMN `conversation`, `subConversation`, `callConversation` with participant and message flow references, participant associations, and one `conversationLink` per pool). Drawn as hexagons in the gap between the linked pools (a thick border for a call, a `+` for a sub-conversation), with double lines to the pools.
- Choreographies: `choreography ID` blocks with start, intermediate, boundary and end events, `xor`, `and`, `or`, `eventgw`, `complex` gateways and sequence flows; `choreo … from= to= message= reply=` (`choreographyTask` with its participants, initiating participant and message flows), `subchoreo … pools= from=` with a block (`subChoreography`), `callchoreo … choreography=ID map=` (`callChoreography`); the flags `loop`, `parallel`, `sequential` (`loopType`). Exported as separate `choreography` root elements; each drawn in a diagram of its own, with participant bands (initiating white, others shaded), message envelopes and loop markers.
- Participant multiplicity: `min=` and `max=` on pools in 0.6 documents (`participantMultiplicity`), drawn with the multi-instance marker.
- Checker codes `R070`–`R074` (conversations, choreographies, multiplicity) and `C120`–`C123` for conversations, call conversations, choreographies and multiplicity in executable documents for Camunda; the levels `basic` and `analytic` reject the new statements (`R040`).
- BPMN conformance classes: `tok check --class descriptive|analytic|common-executable|full` reports every element outside the chosen class of BPMN 2.0.2 (clause 2) as `R075`; `tok check --json` reports the smallest class the document fits as `"bpmn_class"`. `docs/bpmn-rules.md` relates the tok levels and every construct to the classes.
- BPMN import reads conversations, conversation links, call conversations with participant associations, choreographies with their participants, tasks, sub-choreographies, calls and loop types, and participant multiplicity (a file that uses them is printed as `tok 0.6`); message flows that a conversation names keep their identifiers.
- Layout sidecar: `tok import FILE.bpmn -o FILE.tok` writes the diagram of the file to `FILE.tok.layout` (`tok-layout 1`; shapes, edges and labels by tok identifier, in whole units; `--no-layout` suppresses it). `tok bpmn`, `tok svg` and `tok export` read it when it exists next to the document (`--no-sidecar` ignores it): unchanged elements keep their geometry, new elements are placed next to their neighbours, containers grow around them and edges whose ends moved are routed again. Tested on the course files of `conformance/import` and the MIWG reference models: every shape, edge and label the import keeps is drawn as in the file.
- Conformance cases 47–52 (conversations, choreographies, choreography events, multiplicity, the Camunda codes `C120`–`C123`, the errors `R070`–`R074`); the Julia core parses, prints and exports them (core level, 206 checks).

### Changed

- The BPMN validator treats choreography activities as flow nodes and choreographies as containers of sequence flows (boundary events may be attached to choreography activities), and does not ask for diagram edges of the message flows of choreographies (`D02`).
- The cheat sheet printed by `tok cheatsheet` describes language 0.6.
- Layout: message flows between the same two black-box pools are drawn side by side instead of on one line.
- The command-line parity test with the pure-Python implementation (frozen at language 0.3) ignores `"bpmn_class"` of `tok check --json` and the option `--class`.

## [0.5.0] — 2026-10-27

Language 0.5 (`tok 0.5` in the header; documents written for 0.1 and 0.3 stay valid and mean the same): the elements of BPMN 2.0.2 process modelling that Camunda 8 does not execute. Reference: `docs/spec.md` §4, §5.1.1, §5.2.1, §5.3, §5.4, §5.6, §6.3, §7.1, §7.2, §10.4, §13–§15; design notes and the decisions taken during implementation in `docs/design-0.5.md`; the BPMN rules the checker enforces, with their clauses, in `docs/bpmn-rules.md`.

### Added

- `transaction` (BPMN `transaction`, `method=`), cancel end and boundary events (flag `cancel`); the simulator cancels the transaction, runs the compensation handlers of its completed activities and continues from the cancel boundary event.
- `complex` gateway with `activation="FEEL"` (`activationCondition`; the variable `_activationCount` in the simulator).
- Standard loop attributes `while=`, `test-before`, `loop-max=` (`loopCondition`, `testBefore`, `loopMaximum`), simulated.
- Multiple and parallel multiple events: repeated and combined trigger attributes, the flag `parallel`; simulated on catching events.
- Instantiation: `receive … instantiate`, `eventgw … instantiate [parallel]`.
- Process data: `item` (`itemDefinition`), `input`, `output` (`ioSpecification`), `property`, `type=` (`itemSubjectRef`); in 0.5 documents `reads` and `writes` connect data with generated data inputs and outputs of the activity.
- Resources and performers: in 0.5 documents `resource` exports a BPMN `resource` and `by=` a `performer` (`humanPerformer` on user and manual tasks).
- `global` tasks (`call … task=`), `interface` with `operation` lines (`operation=` on service, send and receive tasks, `supportedInterfaceRef`).
- Correlation keys: `key` with `prop` lines (`correlationKey`, `correlationProperty`).
- Checker codes `R060`–`R066`; `C100`–`C110` for elements Camunda 8.10 does not execute in executable documents; the levels `basic` and `analytic` reject every element of 0.5 (`R040`).
- BPMN import reads all of the above back (an imported file that uses them is printed as `tok 0.5`); import of `transaction`, `complexGateway`, multiple event definitions, `ioSpecification`, properties, global tasks and correlation keys no longer drops or simplifies them.
- Drawing: data input and output shapes, the parallel multiple marker, instantiating event-based gateways and receive tasks, call activities of global tasks.
- MIWG conformance gate (`rust/tok-core/tests/miwg.rs`): the 22 reference models of the BPMN Model Interchange Working Group test suite are imported, printed and exported, and their semantic content (flow elements and artifacts by identifier and kind, names, event definitions, gateways, flows, pools, lanes, data, declarations, loops, documentation, Camunda extensions) is compared with the reference; the test fails when something that survived before is lost or when a difference is not reported by the import. Coverage per model in `docs/miwg.md` (3533 of 3770 facts kept). The import corpus of the course survives the same round trip without losses.
- Additions for the MIWG round trip (`docs/design-0.5.md` §13): `doc=` (documentation) on every statement that exports an element; the trigger flags `message`, `timer`, `signal`, `condition` (and `error`, `escalation` on every event that allows them) for an event definition without a value; `process-name=` on the header and on pools; `namespace=` on `item` for a prefixed `structure=`; `reads … input=`, `writes … output=`; data of events (`reads` on throwing, `writes` on catching events, `input`/`output … on=EVENT`); `data … object=` for data objects shared by several references; `when=""`; an explicit compensation boundary event of an activity with `compensate=`. Conformance cases 44–46.
- BPMN import: documentation, data objects without references, the participants and message flows of every collaboration, named compensation boundary events and the data inputs and outputs of activities and events are kept; messages, signals, errors, escalations and times that a model does not give are no longer invented; every element, attribute, extension element and documentation that is not kept is reported.
- Conformance cases 33–43, with expected outputs produced by the Rust engine (`TOK_BLESS=1 cargo test -p tok-core --test conformance`); the Julia core passes cases 33–46 at the core level.

### Changed

- The BPMN validator counts an instantiating receive task or event-based gateway as the start of a process (`W040`, `W041`) and accepts a complex gateway as the join of any split (`W051`).
- The cheat sheet printed by `tok cheatsheet` describes language 0.5.
- Layout: a label of a message flow that would stand inside a pool or lane holding neither end of the flow, or next to another flow, shape or label, moves to a free place along its flow (between the pools, or next to its own ends); no quality measure of the corpus gets worse.
- The BPMN parity tests with the pure-Python implementation (frozen at language 0.3) skip the BPMN files that the import reads as language 0.5.

## [0.4.0] — 2026-10-27

The language is unchanged (language 0.3). From this release the Rust engine is the only engine: every new feature is written once, in Rust, and Python and R call it.

### Added

- Rust engine: FEEL evaluation with Python's decimal semantics, decision table evaluation with all hit policies, and `tok eval`; the discrete-event simulator (`tok-sim`, with a bit-exact port of Python's `random.Random`) and `tok sim`. Both print exactly what the pure-Python commands print for language 0.3; simulation results are identical per platform (Python's own `gauss`, `lognormvariate` and `expovariate` differ in the last bit between Windows and Linux, and so does the engine, deliberately).
- XES event log of a simulation (IEEE 1849-2016): `tok sim FILE --xes PATH`, `SimResult.to_xes()` and `write_xes()` in Python, `tok_xes()` in R. One trace per case with typed case variables, lifecycle transitions `schedule`, `start`, `complete`, `ati_abort` and `withdraw`, resources, deterministic output. See `docs/simulation.md`.
- Python package `tok-lang` on the engine: wheels contain the extension module `tok._engine`; the `tok` command runs the Rust command line; the new module `tok.api` (`check`, `fmt`, `bpmn`, `svg`, `export`, `import_bpmn`, `eval_decision`, `simulate`, `format_kpi`) works on the engine, or on the pure-Python implementation where no wheel is installed or with `TOK_ENGINE=python`. `tools/wheel.sh` builds the wheels (abi3, Python 3.8 or newer).
- R package `tokr` (`r/tokr`, the CRAN name `tok` is taken) on the engine through extendr: `tok_read()`, `tok_check()` (diagnostics as a data frame), `tok_fmt()`, `tok_bpmn()`, `tok_svg()`, `tok_plot()` (viewer), `tok_export()`, `tok_import()`, `tok_eval()`, `tok_sim()` (indicators, an event log ready for bupaR, cases as data frames) and `tok_xes()`; knitr engine `{tok}` for R Markdown and Quarto. `r/build.sh` builds a source archive that carries every Rust crate it needs and installs without network access; it passes `R CMD check --as-cran`.
- Release gates in one script, `tools/check.sh`: formatting, lints and tests of Rust, Python, R and Julia, the wheel and its parity with the pure-Python implementation, the native `tok` program; checked on Windows and Linux, with Linux wheels for x86_64 and aarch64 (manylinux2014) and a cross-compiled macOS wheel (universal2).

### Changed

- Diagram layout of the engine (`tok bpmn`, `tok svg`, `tok export`, and every binding): after the classic layout, flows are rerouted and labels moved where this removes defects, and columns are reordered by barycentre sweeps when that is better. On the 65 models of the corpus edge crossings fell from 59 to 30, flows through shapes from 22 to 17, label collisions from 28 to 0, labels on edges from 112 to 3 and bends from 281 to 228; no model is worse on any measure, and a model that brings its own diagram is drawn as before. The layout stays deterministic. It takes about 1.6 ms per model instead of 0.7 ms. Drawings therefore differ from those of the pure-Python implementation; semantic BPMN, DMN and form files remain identical.
- The pure-Python implementation is frozen at language 0.3 and remains the reference of the parity tests.
- The argument parsing of the native `tok` command follows the Python command for every command, including help texts and error messages.

### Fixed

- `tok eval` reports a violated hit policy (two rules under `unique`, different outputs under `any`) as `tok: rules 1, 2 all match (hit policy unique)` with exit status 1 instead of a Python traceback.

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
