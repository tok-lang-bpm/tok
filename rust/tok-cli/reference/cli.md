# Command line

```
tok check FILE [--level basic|analytic|full] [--class descriptive|analytic|common-executable|full] [--lang en|pl] [--json]
tok fmt FILE [-w|--write] [--check] [--lang en|pl] [--json]
tok bpmn FILE [-o|--output OUT.bpmn] [--no-layout] [--no-sidecar] [--lang en|pl] [--json]
tok svg FILE [-o|--output OUT.svg] [--no-sidecar] [--lang en|pl] [--json]
tok export FILE [-o|--output FOLDER] [--no-sidecar] [--lang en|pl] [--json]
tok import FILE.bpmn|FILE.dmn [-o|--output OUT.tok] [--no-flow-ids] [--no-layout] [--lang en|pl]
tok sim FILE [--cases N] [--seed N] [--log LOG.csv] [--cases-csv CASES.csv] [--xes LOG.xes] [--lang en|pl] [--json]
tok eval FILE DECISION name=value … [--lang en|pl]
tok cheatsheet
tok --version
tok [COMMAND] -h|--help
```

| Command | What it does |
|---|---|
| `check` | parses the document and lists diagnostics (codes in `diagnostics.md`). Exit code 1 if there are errors. `--level` restricts the accepted elements to a teaching level. `--class` checks the document against a conformance class of BPMN 2.0.2 and reports every element outside it (`R075`); `--json` also gives the smallest class the document fits (`"bpmn_class"`; see `bpmn-rules.md`). |
| `fmt` | prints the canonical form. `-w` rewrites the file, `--check` only reports whether the file is already canonical. |
| `bpmn` | writes BPMN 2.0 XML with an automatic diagram layout. The file opens in Camunda Modeler and bpmn.io. When the layout sidecar `FILE.layout` exists next to the document (written by `tok import`), the shapes and edges it names keep their geometry and new elements are placed around them; `--no-sidecar` ignores it. |
| `svg` | writes a picture of the diagram (with the layout sidecar, as `bpmn`). |
| `export` | writes the BPMN file, the SVG picture, the decision files and one `.form` file per form: everything Camunda 8 needs for an executable document (with the layout sidecar, as `bpmn`). The decision files are one DMN 1.3 file per decision table in documents of languages 0.1–0.6, and in a document of language 0.7 or 1.0 one DMN 1.5 file per decision requirements graph (`NAME.dmn`, named after its first decision, with a DMNDI diagram) and its picture (`NAME.drd.svg`); a 0.7 or 1.0 document without a process writes only the decision files and forms. In an executable document for Camunda the FEEL names with spaces or symbols are written in backticks (spec §9.5). |
| `import` | converts a BPMN 2.0 XML file (for example drawn in Camunda Modeler) to tok, or a DMN 1.1–1.5 file (`.dmn`, or any file whose root is a DMN `definitions`) to a document of language 0.7 (`import.md`). Elements tok does not cover are reported on standard error. With `-o OUT.tok` the diagram of the file is written to the layout sidecar `OUT.tok.layout` next to it, so that `tok bpmn OUT.tok` draws the model as it was drawn (`--no-layout`: no sidecar). `--no-flow-ids` drops the identifiers of sequence flows so they print as compact chains. |
| `sim` | runs the simulation described by `simulate`, `resource`, `variable` and the simulation attributes; prints indicators, optionally writes the event log and the per-case table as CSV (`--log`, `--cases-csv`) and the event log as XES, IEEE 1849-2016, for process-mining tools (`--xes`; see `simulation.md`). |
| `eval` | evaluates a decision for the given input values: `tok eval proposal.tok tryb-akceptacji rodzaj=e-book cena=120`. A value is `true`/`false`, a number, or else text. The result is printed as JSON. In documents of languages 0.1–0.6 the decision table alone is evaluated, and the result is an object of outputs, `null` when no rule matches, a list of objects for hit policy `collect`. In a document of language 0.7 or 1.0 the decision is evaluated with the decisions, input data and business knowledge models it requires; `name` is the FEEL name of input data (`"Zaległe opłaty=60"`), a value may also be a JSON object or list (`czytelnik='{"wiek": 20}'`), and text for input data of a temporal type is read as that type (`termin=2027-01-15`); the result is the value of the decision (spec §9.3), with dates, times and durations as their FEEL text. Exit code 1 when the document has errors or a hit policy is violated (several rules under `unique`, different outputs under `any`), in the decision or in a decision it requires; 2 for an unknown decision or an argument that is not `name=value`. |
| `cheatsheet` | prints the one-page language summary; useful as context for an AI agent. |

`--lang pl` (or the environment variable `TOK_LANG=pl`) switches messages to Polish. `--json` prints machine-readable output for programs: the diagnostics of `check`, the indicators of `sim`, and for `fmt`, `bpmn`, `svg` and `export` the diagnostics of a document with errors (`{"file", "errors", "diagnostics"}`) instead of the text report. `-h` or `--help` after a command prints its options. The exit code is 0 on success, 1 when the document has errors or the command fails, 2 for a usage error.

The layout sidecar is plain text and can be kept in version control next to the model; its format is described in spec §16:

```
tok import diagram.bpmn -o model.tok      # writes model.tok and model.tok.layout
# … edit model.tok …
tok bpmn model.tok                        # model.bpmn with the drawing of diagram.bpmn, new elements placed around it
```

Without installation the same commands run as `python -m tok …` with `PYTHONPATH` pointing to `python/src`. That pure-Python implementation is frozen at language 0.3 and has no `tok sim --xes`, no `tok check --class` and no layout sidecar; they need the Rust engine (the native `tok` program, or the `tok-lang` package with the engine).
