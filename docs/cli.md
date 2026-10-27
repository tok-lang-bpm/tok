# Command line

```
tok check FILE [--level basic|analytic|full] [--lang en|pl] [--json]
tok fmt FILE [-w] [--check]
tok bpmn FILE [-o OUT.bpmn] [--no-layout]
tok svg FILE [-o OUT.svg]
tok export FILE [-o FOLDER]
tok import FILE.bpmn [-o OUT.tok]
tok sim FILE [--cases N] [--seed N] [--log LOG.csv] [--cases-csv CASES.csv] [--json]
tok eval FILE DECISION name=value …
tok cheatsheet
tok --version
```

| Command | What it does |
|---|---|
| `check` | parses the document and lists diagnostics (codes in `diagnostics.md`). Exit code 1 if there are errors. `--level` restricts the accepted elements to a teaching level. |
| `fmt` | prints the canonical form. `-w` rewrites the file, `--check` only reports whether the file is already canonical. |
| `bpmn` | writes BPMN 2.0 XML with an automatic diagram layout. The file opens in Camunda Modeler and bpmn.io. |
| `svg` | writes a picture of the diagram. |
| `export` | writes the BPMN file, the SVG picture, one `.dmn` file per decision and one `.form` file per form: everything Camunda 8 needs for an executable document. |
| `import` | converts a BPMN 2.0 XML file (for example drawn in Camunda Modeler) to tok. Elements tok 0.1 does not cover are reported on standard error. |
| `sim` | runs the simulation described by `simulate`, `resource`, `variable` and the simulation attributes; prints indicators, optionally writes the event log and the per-case table as CSV. |
| `eval` | evaluates a decision table for the given input values: `tok eval proposal.tok tryb-akceptacji rodzaj=e-book cena=120`. |
| `cheatsheet` | prints the one-page language summary; useful as context for an AI agent. |

`--lang pl` (or the environment variable `TOK_LANG=pl`) switches messages to Polish. `--json` prints machine-readable output for programs and agents.

Without installation the same commands run as `python -m tok …` with `PYTHONPATH` pointing to `python/src`.
