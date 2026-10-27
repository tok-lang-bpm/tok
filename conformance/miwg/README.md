# BPMN MIWG reference models

This folder holds the reference BPMN 2.0 XML files of the BPMN Model Interchange
Test Suite, published by the BPMN Model Interchange Working Group (BPMN MIWG) at
the Object Management Group (OMG). They are used here as conformance inputs for
BPMN import and export.

## Source

- Repository: https://github.com/bpmn-miwg/bpmn-miwg-test-suite
- Folder: `Reference/`
- Commit: `85240b8689d835d8bd1a96fbea6af46558ca0568` (branch `master`, 2026-10-18).
  This commit is newer than the latest release tag, `2025`.
- Files taken: the 22 `Reference/*.bpmn` files only. Vendor test results, PNG/PDF
  pictures, Visio files and the `Work in Progress/` folder were left out.
- The files are unmodified copies with their original names.

## Licence and attribution

The test suite, reference models included, is licensed under the
[Creative Commons Attribution 3.0 Unported License](https://creativecommons.org/licenses/by/3.0/)
(CC BY 3.0), as stated in the suite's `LICENSE.txt` and `README.md`. CC BY 3.0
allows copying and redistribution, including in other projects, provided that
attribution is given, a link to the licence is included, and changes, if any,
are indicated.

These files keep their CC BY 3.0 licence. They are not covered by the GPL-3.0
licence of the rest of this repository; they are distributed alongside it as
separate, unmodified data files.

Attribution:

> BPMN Model Interchange Test Suite, reference models, by the BPMN Model
> Interchange Working Group (BPMN MIWG) at the OMG,
> https://github.com/bpmn-miwg/bpmn-miwg-test-suite (commit 85240b8),
> licensed under CC BY 3.0 (https://creativecommons.org/licenses/by/3.0/).
> No changes were made.

The vendor tool that contributed each reference model is listed in the suite's
`test-case-structure.json`.

## Files

Descriptions are taken from the suite's `README.md` and
`test-case-structure.json`. C.10.0 has no description in the suite yet; its
line is a summary of the file's content.

| File | What the model exercises |
|------|--------------------------|
| `A.1.0.bpmn` | Abstract elements on a single sequential flow |
| `A.2.0.bpmn` | Divergence and convergence of flows with unmarked gateways |
| `A.2.1.bpmn` | Divergence and convergence of flows with unmarked gateways, using different types of sequence flows |
| `A.3.0.bpmn` | Exception flows attached to boundary events |
| `A.4.0.bpmn` | Graphical elements contained in expanded sub-processes, lanes and pools |
| `A.4.1.bpmn` | Vertical modelling direction |
| `B.1.0.bpmn` | All elements of the Descriptive conformance sub-class of BPMN 2.0 |
| `B.2.0.bpmn` | All elements of the Analytic conformance sub-class of BPMN 2.0 |
| `C.1.0.bpmn` | Invoice collaboration: executable process with multiple extension elements and attributes (2013 Berlin demo) |
| `C.1.1.bpmn` | Invoice collaboration: process from the 2015 Berlin execution demo, executable in Camunda BPM, OmnyLink and W4 BPMN+ |
| `C.2.0.bpmn` | Buying at Amazon: collaboration from the 2015 Berlin diagram interchange demo |
| `C.3.0.bpmn` | Fridge repair service: process with hidden BPMN attributes and elements (2015 Berlin demo) |
| `C.4.0.bpmn` | New employee onboarding: signal and message events and multiple diagrams (2017 Brussels demo) |
| `C.5.0.bpmn` | New bank customer: data objects and data stores with their hidden attributes (2018 Seattle demo) |
| `C.6.0.bpmn` | Travel booking: data inputs/outputs and an event sub-process (2019 Amsterdam demo) |
| `C.7.0.bpmn` | Advertise a job vacancy: data inputs and a data object connected to a sequence flow (2020 virtual demo) |
| `C.8.0.bpmn` | Vacation request: colour and internationalisation (2022 demo) |
| `C.8.1.bpmn` | Vacation request: automation extensions (2022 demo) |
| `C.9.0.bpmn` | Event sub-processes (2023 demo) |
| `C.9.1.bpmn` | Timer boundary events (2023 demo) |
| `C.9.2.bpmn` | Non-interrupting start events (2023 demo) |
| `C.10.0.bpmn` | Travel planning: two-pool collaboration with message flows, lanes, a call activity, data objects, data stores and task I/O specifications (not yet described by the suite) |
