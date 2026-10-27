# tok (Rust command line)

A native build of the `tok` command. For language 0.3 it prints the same results as
the pure-Python implementation (`TOK_ENGINE=python python -m tok`): the same
diagnostics in English and Polish, the same canonical text from `fmt`, the same
semantic BPMN, DMN and form files, `eval` results and simulation reports. From 0.4
the diagram layout of BPMN and SVG output is the improved layout of `tok-bpmn`
(fewer crossings, no label collisions), so drawings differ from the Python ones.

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
| `sim FILE` | `--cases N`, `--seed N`, `--log PATH`, `--cases-csv PATH`, `--xes PATH`, `--lang en\|pl`, `--json` | available |
| `eval FILE DECISION name=value …` | `--lang en\|pl` | available |

Exit status: 0 when there are no errors, 1 when the document has errors (or a file
cannot be produced), 2 for a usage error.

## Crates

- `tok-core`: lexer, parser, checker (structure, FEEL, decision tables, forms,
  simulation settings), canonical printer, BPMN/DMN/form exporters and the BPMN
  importer.
- `tok-bpmn`: BPMN 2.0 rules (the course validator `bpmn.py`), automatic layout
  with route and label refinement, and SVG rendering.
- `tok-sim`: the discrete-event simulator (`sim.py`), with a bit-exact port of
  Python's `random.Random`.
- `tok-cli`: this command.

## How parity is verified

- `tok-core/tests/conformance.rs` runs the language conformance corpus
  (`conformance/cases`): canonical text, core diagnostic codes and semantic BPMN XML.
- `tok-core/tests/python_parity.rs` compares complete outputs with fixtures produced
  by the Python implementation (`tok-core/tests/gen_python_expected.py`): the
  diagnostics of `check --json` in both languages, BPMN XML without the diagram,
  whether drawing succeeds, DMN, forms, and `import` with and without flow
  identifiers; the layout itself is covered by the golden files and quality
  measures of `tok-bpmn` (see `tok-bpmn/PARITY.md`). To refresh the fixtures
  after a change in the Python reference:

  ```
  PYTHONPATH=python/src python rust/tok-core/tests/gen_python_expected.py ../tok-course/przyklady
  ```

- `tok-cli/tests/sim_parity.rs` runs `tok sim` with the arguments of the fixtures
  produced by `python -m tok sim` (`tok-sim/tests/gen_python_sim.py`, one file per
  platform because the C library's `log`, `exp`, `sin` and `cos` differ in the last
  bit) and compares standard output, standard error, the exit code and the CSV files.
  The Python command has no `--xes`; the test adds that option to the usage line and
  the help text that Python prints before comparing them.

## Known differences

- `--version` prints the version of the Rust build.
- `sim --xes PATH` writes the event log as XES (IEEE 1849-2016) for process-mining
  tools (see `docs/simulation.md`); the frozen Python command does not have it.
- The overlap analysis of decision tables (D010) uses exact decimals of up to 38
  significant digits; a number literal that is longer, or written with digits
  outside ASCII, is left out of that analysis.
- Identifiers derived from labels fold accented letters with a built-in table that
  covers Latin-1, Latin Extended-A, common ligatures and full-width forms; letters
  of other scripts become `_`.
- An exception that the Python command does not catch (for example `sim` on a process
  without a start event) is printed as the last line of Python's traceback, with exit
  status 1.
- `sim` follows Python 3.12 and newer, whose `sum()` adds floats with compensated
  summation (the mean cycle time); Python 3.11 can differ in the last digit of a
  rounded mean in rare cases. Python stops with `RecursionError` when a token passes
  several hundred gateways or events that take no time in one step; `tok sim`
  continues up to the loop limit of 10,000 elements per case.
- Argument errors follow Python 3.11's `argparse`; later versions word a few of them
  differently (for example `invalid choice: 'de' (choose from en, pl)` without quotes
  around the choices).
