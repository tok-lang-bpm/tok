# tok (Rust command line)

A native build of the `tok` command. It reads the same files and prints the same
results as the Python reference implementation (`python -m tok`):
the same diagnostics in English and Polish, the same canonical text from `fmt`,
and byte-identical BPMN XML, SVG, DMN and form files.

```
cargo build --release -p tok-cli
target/release/tok check examples/wypozyczenie.tok --lang pl
```

## Commands

| Command | Options | Status |
|---|---|---|
| `check FILE` | `--lang en\|pl`, `--json`, `--level basic\|analytic\|full` | available |
| `fmt FILE` | `-w`, `--check` | available |
| `bpmn FILE` | `-o PATH`, `--no-layout` | available |
| `svg FILE` | `-o PATH` | available |
| `export FILE` | `-o FOLDER` | available |
| `import FILE.bpmn` | `-o PATH`, `--no-flow-ids` | available |
| `cheatsheet` |  | available |
| `--version` |  | available |
| `sim FILE` |  | planned for tok 0.4; use the Python implementation |
| `eval FILE DECISION name=value …` |  | planned for tok 0.4; use the Python implementation |

`sim` and `eval` print a short message and exit with status 2 in this build.

Exit status: 0 when there are no errors, 1 when the document has errors (or a file
cannot be produced), 2 for a usage error.

## Crates

- `tok-core`: lexer, parser, checker (structure, FEEL, decision tables, forms,
  simulation settings), canonical printer, BPMN/DMN/form exporters and the BPMN
  importer.
- `tok-bpmn`: BPMN 2.0 rules, automatic layout and SVG rendering (the course
  validator `bpmn.py`).
- `tok-cli`: this command.

## How parity is verified

- `tok-core/tests/conformance.rs` runs the language conformance corpus
  (`conformance/cases`): canonical text, core diagnostic codes and semantic BPMN XML.
- `tok-core/tests/python_parity.rs` compares complete outputs with fixtures produced
  by the Python implementation (`tok-core/tests/gen_python_expected.py`): the
  diagnostics of `check --json` in both languages, BPMN XML after layout, SVG, DMN,
  forms, and `import` with and without flow identifiers. To refresh the fixtures
  after a change in the Python reference:

  ```
  PYTHONPATH=python/src python rust/tok-core/tests/gen_python_expected.py ../tok-course/przyklady
  ```

## Known differences

- `--version` prints the version of the Rust build.
- The overlap analysis of decision tables (D010) uses exact decimals of up to 38
  significant digits; a number literal that is longer, or written with digits
  outside ASCII, is left out of that analysis.
- Identifiers derived from labels fold accented letters with a built-in table that
  covers Latin-1, Latin Extended-A, common ligatures and full-width forms; letters
  of other scripts become `_`.
