# Changelog

All notable changes to the tok language and its reference tools are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/). Until 1.0 a minor version may change the language; a patch version never does.

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
