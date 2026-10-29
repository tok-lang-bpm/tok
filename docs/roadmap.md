# Roadmap

The goal of the 0.x series is a language and a tool chain that cover BPMN 2.0.2 completely and run the same way from Python, R and Julia. Conformance is built in two stages: first everything that **Camunda 8** executes, then the rest of **BPMN 2.0.2**. Version 1.0 freezes the language.

## Architecture target

| Component | Language | Role |
|---|---|---|
| `tok-core` | Rust | reference engine from 0.2: lexer, parser, IR, checker, printer, BPMN/DMN/form readers and writers, layout and SVG, FEEL, simulation; C ABI and WebAssembly builds |
| `tok-lang` (Python) | Rust core via PyO3, wheels built with maturin | command line `tok` and Python API; from 0.4 a thin layer over the Rust engine. The pure-Python implementation is frozen at language 0.3 and kept as the test oracle for what it covers |
| `tok` (R) | Rust core via extendr | R API, knitr engine `{tok}`, event logs as data frames |
| `Tok.jl` | Julia: binding to the Rust engine + independent core + native simulation | the language through the Rust engine (C ABI); an independent core (parser, printer, BPMN export) kept as a check of the specification; native high-performance simulation and calibration |
| conformance corpus | text files | the contract between implementations: every implementation passes it before a release |

Why Rust for the shared engine: mature bindings for both Python (PyO3) and R (extendr, with an established CRAN policy for Rust packages); one code base compiled for Windows, macOS, Linux and WebAssembly (the browser playground); established XML, random-number and property-testing libraries; compile-time memory safety for a large stateful simulator. Julia gets the language through the Rust engine, so no feature is implemented twice. Its own independent core (text layer and BPMN export) stays, because a second implementation of one specification is the strongest test that the specification is unambiguous. Native Julia code is written only where Julia adds something: large-scale simulation and Bayesian calibration.

## Released

- **0.1.0, 0.1.1** — language 0.1 with the Python reference implementation and the Julia core.
- **0.2.0** — Rust engine at parity with Python: `tok-core`, `tok-bpmn`, native `tok`, Python bindings for parity testing.
- **0.3.0** — language 0.3: complete Camunda 8 coverage as an optional executable layer (multi-instance, ad-hoc sub-processes, call activities, variable mappings, user task assignment and scheduling, jobs and REST connector, compensation, catch-all errors and escalations, conditional filters, listeners, version tags); Python, Rust and the Julia core at parity; executable conformance on Camunda 8 Run.
- **0.4.0** — the Rust engine is the only engine: `eval` and `sim` in Rust, XES event logs, improved layout; `tok-lang` wheels on the engine with the module `tok.api`; R package `tokr`; release gates in `tools/check.sh`.
- **0.5.0** — language 0.5: the BPMN 2.0.2 process elements Camunda 8 does not execute (transaction and cancel, complex gateway, standard loop, multiple events, instantiation, process data, resources, global tasks, interfaces, correlation keys), documentation, MIWG round-trip gate with coverage per model (`docs/miwg.md`).
- **0.6.0** — language 0.6: conversations, choreographies, participant multiplicity; BPMN conformance classes (`tok check --class`); layout sidecar keeping the drawing of imported files.
- **0.7.0** — language 0.7: decision requirements graphs (input data, knowledge sources, business knowledge models), boxed expressions, FEEL of DMN 1.5 conformance level 3, DMN 1.5 export and DMN import, `tok eval` for graphs; DMN TCK gate (level 2: all, level 3: 99.5 %); decisions verified on Camunda 8 Run.
- **0.8.0** — Julia on the Rust engine: the C interface `tokengine` (`docs/c-api.md`), `Tok.Engine` with the whole language in Julia, the native simulator `Tok.Sim` checked statistically against the engine (Kolmogorov–Smirnov, 44 models), calibration on event logs (`Tok.Fit`, Bayesian with Turing.jl), the BinaryBuilder recipe of `tok_jll` and the precompiled `tok` application.
- **0.9.0** — tooling: the language server `tok-lsp` and the VS Code extension with a live diagram, the MCP server `tok-mcp`, the browser playground on WebAssembly with the bpmn-js viewer; the agent skill for the whole of language 0.7. Fuzzing, benchmarks and the frozen specification follow as the 1.0 release candidate.
- **1.0.0** — language 1.0, the stable language: the content of 0.7 with its limits as rules, the deprecation `R090` and the compatibility promise of 1.x (`design-1.0.md`); fuzzing of the engine, benchmarks and the faster layout of large models, the documentation checked by the test suite, conformance tables for Camunda 8 and BPMN 2.0.2; the packages prepared for their registries (PyPI with the engine in the wheels, crates.io, CRAN, the Julia General registry, `tok_jll`, the VS Code Marketplace and Open VSX), continuous integration and release workflows for a GitHub mirror, and citation metadata for Zenodo.
- **1.0.1** — the tools and packages of language 1.0 at their public home (the GitHub organisation `tok-lang-bpm`, the guide at https://tok-lang-bpm.github.io/): the MCP server in the Python package (`tok mcp`) and in the MCP Registry, the catalogue of BPMN symbols drawn by the engine (`docs/symbols.md`).
- **1.0.2** — the findings of the first evidence study of tok (S2 fault injection, S3 language models writing process models): three warnings of the BPMN rules for what the BPMN linter of bpmn.io reports and `tok check` did not (`W022`, `W023`, `W043`), the documentation of the header written where the BPMN schema allows it, the FEEL check of conditions in documents of 0.7 and 1.0, the messages of `P005` and `P006`; the agent skill fixes warnings as well as errors.

## State at 0.1

Text layer, checker with diagnostics in English and Polish, canonical printer, BPMN export with layout and SVG, BPMN import, FEEL subset, decision tables, Camunda forms, discrete-event simulation (Python); core in Julia; conformance corpus of 18 cases.

Camunda 8.10 execution coverage, as published by Camunda: supported — pools, lanes, embedded, call, event and ad-hoc sub-processes, all task types, exclusive, parallel, inclusive and event-based gateways, parallel and sequential multi-instance, compensation, data objects and stores (modelling only), annotations and groups, none, message, timer, error, signal, conditional, escalation, compensation, terminate and link events; not supported — transaction, cancel events, complex gateway, standard loop, instantiating receive task, multiple and parallel multiple events.

## 0.1.1 — release hygiene

- Polish detail text in FEEL syntax errors (F002); concrete example values in overlap warnings (D010); platform-native paths in `tok export`; BPMN style notes not reported next to FEEL errors.
- Changelog and versioning policy (`docs/changelog.md`). Until 1.0 the packages are installed from the Codeberg archive; registry releases come with the stable version.

## 0.2 — Rust core at parity with 0.1

- `rust/tok-core`: lexer, parser, IR, diagnostics catalogue (EN/PL), checker (T, P, R, C, D, M codes), canonical printer, semantic BPMN export, BPMN import.
- Port of the BPMN rules and layout/SVG of `bpmnlib` to Rust (E, W, L codes identical).
- `rust/tok-cli`: native `tok` binary with the same commands and output as the Python 0.1 CLI.
- Python bindings on the Rust core (PyO3, built with maturin) used for parity testing; until the 1.0 registry release the installed `tok-lang` stays pure Python, so users need no Rust compiler.
- Conformance: the 18-case corpus extended with BPMN import cases and diagnostics of all layers; Python, Rust and Julia pass the core level byte for byte.

## 0.3 — complete Camunda 8 coverage

Language additions (each with checker rules, export, import, conformance cases and an executable test on Camunda 8 Run):

- **Sub-processes:**
  - `adhoc` sub-process with active elements, completion condition and cancellation of remaining instances;
  - call activity with propagation of variables and input/output mappings;
  - event sub-processes with every start trigger Camunda supports (message, timer, signal, error, escalation, conditional; interrupting and non-interrupting).
- **Multi-instance in full:** input collection, input element, output collection and element, completion condition (`parallel`/`sequential` with `collection=`, `item=`, `output=`, `until=`).
- **Compensation:** handlers bound to activities (`compensate=HANDLER`), throw and end compensation events with an activity reference, compensation inside sub-processes.
- **Errors and escalations:** codes, catch-all boundary events, variables carried by errors.
- **Messages and signals:** message start events, correlation on boundary and receive tasks, throwing signal and end events.
- **Timers:** expressions in durations, dates and cycles (`timer="=expr"`).
- **Conditional events:** in all positions Camunda supports, with variable filters.
- **Link events:** pairing rules.
- **Variable mappings** on activities and events (`inputs="target=expr; …"`, `outputs="…"`).
- **User tasks:** assignee, candidate users and groups, due and follow-up dates, priority.
- **Service and send tasks:** job type, retries, task headers; Camunda connector templates (REST) as a typed attribute set.
- **Forms:** all Camunda form components used in practice (groups, dynamic lists, tables, validation rules, conditional visibility).
- `tok check --target camunda`: every construct Camunda 8 does not execute is reported (C-codes), with the version of Camunda the rule refers to.
- Executable conformance: a script deploys every executable case to Camunda 8 Run and drives instances to their expected end events through the REST API.
- Simulation of every construct above (Python reference and Rust); `sim` and `eval` in the Rust engine.

## 0.4 — Rust as the only engine; R package, simulation and layout in Rust

From 0.4 every new feature is written once, in Rust. Python, R and Julia call the Rust engine; the Julia core stays as an independent check of the specification.

- **Build infrastructure first:** the release gates are one script, `tools/check.sh`: formatting, lints and tests of Rust, Python and Julia, the engine wheel (abi3) and its parity with the Python implementation, and the native `tok` program. Every release is checked with it on Windows and on Linux; Linux wheels are manylinux2014 (built with zig), and the macOS wheel (universal2) is cross-compiled. Wheels are attached to the release archives on Codeberg; registry releases remain for 1.0.
- **Python on Rust:** `tok-lang` becomes a thin layer over `tok-py` with the same command line and API. Where no wheel exists for a platform yet, the archive installs the frozen pure-Python implementation (language 0.3).
- **Pure Python frozen:** no new features after 0.3; it stays in the repository as the test oracle for language 0.3 (parity tests keep running against it).

- R package `tok` on the Rust core: `tok_read()`, `tok_check()` (diagnostics as a data frame), `tok_fmt()`, `tok_export()`, `tok_plot()` (SVG in the RStudio viewer and documents), `tok_sim()` (event log and cases as data frames, compatible with process-mining packages), `tok_eval()`; knitr engine `{tok}`; vendored crates for offline CRAN builds.
- Simulation engine in Rust with identical semantics to the Python reference; statistical comparison of indicators on fixed seeds; XES event-log export.
- Layout engine in Rust: deterministic, with better routing of message flows and boundary paths and without label collisions.

## 0.5 — BPMN 2.0.2 process modelling conformance

Elements beyond Camunda, as defined by the BPMN 2.0.2 specification:

- `transaction` sub-process with cancel end events, cancel boundary events and the compensation protocol;
- complex gateway with activation condition;
- standard loop: loop condition, test before or after, maximum;
- multiple and parallel multiple events (several triggers on one event);
- instantiating receive task and instantiating event-based gateway;
- process data: data inputs and outputs, input and output sets, properties, item definitions, data state transitions;
- resources and performers (`by=` mapped to resource roles), global tasks, interfaces and operations, correlation keys and properties;
- full checker coverage of the semantic constraints of BPMN 2.0.2 with codes traceable to specification clauses;
- conformance gate: the BPMN Model Interchange Working Group (MIWG) reference models import, export and round-trip without semantic loss; results published as a coverage table. Done: `rust/tok-core/tests/miwg.rs`, coverage in `miwg.md`; what tok cannot express is listed there and reported by the import.

## 0.6 — collaboration, choreography and diagram interchange

- Conversations, sub-conversations, call conversations, conversation links.
- Choreographies: choreography tasks, sub-choreographies, call choreographies, initiating and non-initiating participants.
- Participant multiplicity and participant associations.
- Diagram interchange round trip: `tok import` keeps the layout of a file drawn in a modelling tool in a sidecar file (`model.tok.layout`), and `tok bpmn` reuses it for unchanged elements, so a model can move between text and Camunda Modeler without losing its drawing.
- BPMN Descriptive, Analytic, Common Executable and Full conformance classes reported by `tok check --class` (and the smallest class of a document in `tok check --json`).

## 0.7 — decisions and expressions

- Decision requirements diagrams: decisions that depend on other decisions, input data, business knowledge models, knowledge sources.
- Boxed expressions: literal expressions, contexts, invocations, lists, relations.
- FEEL extended towards conformance level 3: temporal types and arithmetic, the standard function library, decimal semantics; DMN TCK subset as a conformance gate.
- `tok eval` for decision requirements graphs; DMN import.

## 0.8 — Julia: Rust engine, independent core, native simulation and calibration

- `Tok.jl` calls the Rust engine through its C ABI (`ccall`), distributed as a prebuilt binary package (JLL, built with BinaryBuilder in the 0.4 CI): the whole language — checker of all layers, FEEL, decisions, forms, BPMN import, layout and SVG — is available in Julia without a second implementation.
- The independent Julia core (lexer, parser, printer, semantic BPMN export) keeps passing the core conformance level with every language change.
- Native simulation in Julia for large experiments (many scenarios, many replications, parallel runs), checked statistically against the Rust simulator; calibration of simulation parameters on event logs (the `fit` layer) with Turing.jl.
- Precompiled `tok` application for Julia users; registration in the Julia General registry at 1.0.

## 0.9 — tooling and release candidates

- Language server (diagnostics, formatting, go to definition, rename of identifiers) and a VS Code extension with syntax highlighting and live diagram preview.
- MCP server exposing `check`, `fmt`, `export`, `import`, `sim`, `eval` for AI agents; agent skill updated for the full language.
- Browser playground on the WebAssembly build with the bpmn-js viewer.
- Fuzzing of the parser and importers (`fuzzing.md`); performance benchmarks on large models (`performance.md`); documentation complete for every construct, checked by the test suite.
- Specification frozen as 1.0 release candidate; deprecation of anything that will not survive into 1.0.

## 1.0 — stable

- Specification 1.0: every BPMN 2.0.2 element reachable from the language, conformance tables for Camunda 8 and BPMN 2.0.2 published with the release (`conformance-camunda.md`, `conformance-bpmn.md`).
- Hosted continuous integration running `tools/check.sh` on Windows, macOS and Linux for every change, so that the macOS wheel is tested on its platform before the first registry release.
- First registry releases, from one version number: crates.io, PyPI (`tok-lang`), CRAN and the Julia General registry; archived release with a DOI on Zenodo.
- Semantic versioning promise: 1.x keeps every valid 1.0 document valid and prints it identically; new Camunda versions and DMN revisions are tracked in minor releases.

Done in 1.0.0: the specification 1.0 with its compatibility promise (`spec.md` §4.3), the conformance tables, the workflows of the GitHub mirror (`.github/workflows`), and every package built and checked for its registry. The registry releases (PyPI, crates.io, CRAN, the Julia General registry, the VS Code Marketplace and Open VSX), the hosted runs on the GitHub mirror, the playground on Codeberg Pages and the archived release with a DOI follow from the 1.0.0 tag.
