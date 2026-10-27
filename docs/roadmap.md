# Roadmap

The goal of the 0.x series is a language and a tool chain that cover BPMN 2.0.2 completely and run the same way from Python, R and Julia. Conformance is built in two stages: first everything that **Camunda 8** executes, then the rest of **BPMN 2.0.2**. Version 1.0 freezes the language.

## Architecture target

| Component | Language | Role |
|---|---|---|
| `tok-core` | Rust | reference engine from 0.2: lexer, parser, IR, checker, printer, BPMN/DMN/form readers and writers, layout and SVG, FEEL, simulation; C ABI and WebAssembly builds |
| `tok-lang` (Python) | Rust core via PyO3, wheels built with maturin | command line `tok` and Python API; the pure-Python implementation is the installed package until prebuilt wheels exist (0.4) and stays the executable specification |
| `tok` (R) | Rust core via extendr | R API, knitr engine `{tok}`, event logs as data frames |
| `Tok.jl` | Julia, standalone | an independent second implementation of the specification, no bindings to the Rust core |
| conformance corpus | text files | the contract between implementations: every implementation passes it before a release |

Why Rust for the shared engine: mature bindings for both Python (PyO3) and R (extendr, with an established CRAN policy for Rust packages); one code base compiled for Windows, macOS, Linux and WebAssembly (the browser playground); established XML, random-number and property-testing libraries; compile-time memory safety for a large stateful simulator. Julia stays standalone on purpose: two independent implementations of one specification are the strongest test that the specification is unambiguous.

## Released

- **0.1.0, 0.1.1** — language 0.1 with the Python reference implementation and the Julia core.
- **0.2.0** — Rust engine at parity with Python: `tok-core`, `tok-bpmn`, native `tok`, Python bindings for parity testing.

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
- Simulation of every construct above (Python reference and Rust).

## 0.4 — R package, simulation and layout in Rust

- R package `tok` on the Rust core: `tok_read()`, `tok_check()` (diagnostics as a data frame), `tok_fmt()`, `tok_export()`, `tok_plot()` (SVG in the RStudio viewer and documents), `tok_sim()` (event log and cases as data frames, compatible with process-mining packages), `tok_eval()`; knitr engine `{tok}`; vendored crates for offline CRAN builds.
- Simulation engine in Rust with identical semantics to the Python reference; statistical comparison of indicators on fixed seeds; XES event-log export.
- Layout engine in Rust: deterministic, faster, with better routing of message flows and boundary paths.
- Prebuilt Python wheels on the Rust core produced for every release archive (abi3, Windows/macOS/Linux); the pure-Python implementation becomes the specification test oracle only.

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
- conformance gate: the BPMN Model Interchange Working Group (MIWG) reference models import, export and round-trip without semantic loss; results published as a coverage table.

## 0.6 — collaboration, choreography and diagram interchange

- Conversations, sub-conversations, call conversations, conversation links.
- Choreographies: choreography tasks, sub-choreographies, call choreographies, initiating and non-initiating participants.
- Participant multiplicity and participant associations.
- Diagram interchange round trip: `tok import` keeps the layout of a file drawn in a modelling tool in a sidecar file (`model.tok.layout`), and `tok bpmn` reuses it for unchanged elements, so a model can move between text and Camunda Modeler without losing its drawing.
- BPMN Descriptive, Analytic, Common Executable and Full conformance classes reported by `tok check --level`.

## 0.7 — decisions and expressions

- Decision requirements diagrams: decisions that depend on other decisions, input data, business knowledge models, knowledge sources.
- Boxed expressions: literal expressions, contexts, invocations, lists, relations.
- FEEL extended towards conformance level 3: temporal types and arithmetic, the standard function library, decimal semantics; DMN TCK subset as a conformance gate.
- `tok eval` for decision requirements graphs; DMN import.

## 0.8 — Julia to full parity

- Tok.jl as a complete standalone implementation: checker of all layers, FEEL, decisions, forms, BPMN import, layout and SVG, simulation.
- Full conformance level passed by Julia and Rust; simulation compared statistically.
- Precompiled `tok` application for Julia users; registration in the Julia General registry.

## 0.9 — tooling and release candidates

- Language server (diagnostics, formatting, go to definition, rename of identifiers) and a VS Code extension with syntax highlighting and live diagram preview.
- MCP server exposing `check`, `fmt`, `export`, `import`, `sim`, `eval` for AI agents; agent skill updated for the full language.
- Browser playground on the WebAssembly build with the bpmn-js viewer.
- Fuzzing of the parser and importers; performance benchmarks on large models; documentation complete for every construct.
- Specification frozen as 1.0 release candidate; deprecation of anything that will not survive into 1.0.

## 1.0 — stable

- Specification 1.0: every BPMN 2.0.2 element reachable from the language, conformance tables for Camunda 8 and BPMN 2.0.2 published with the release.
- First registry releases, from one version number: crates.io, PyPI (`tok-lang`), CRAN and the Julia General registry; archived release with a DOI on Zenodo.
- Semantic versioning promise: 1.x keeps every valid 1.0 document valid and prints it identically; new Camunda versions and DMN revisions are tracked in minor releases.
