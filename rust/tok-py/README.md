# tok-py: Python bindings of the tok engine

The crate builds the extension module `tok._engine` of the Python package `tok-lang` on top of `tok-core`, `tok-bpmn`, `tok-sim` and `tok-cli`. A wheel of `tok-lang` contains the Python package `python/src/tok` and this module:

- the `tok` command runs the Rust command line (`tok._engine.cli`), so its output is that of the native `tok` program;
- the functions of `tok.api` call the engine;
- with the environment variable `TOK_ENGINE=python` both use the pure-Python implementation instead, which is frozen at language 0.3 and serves as the reference of the parity tests.

The package is defined by `pyproject.toml` at the root of the repository (maturin backend, `python-source = "python/src"`, `manifest-path = "rust/tok-py/Cargo.toml"`): `pip install tok-lang` installs a wheel, and the source archive (`maturin sdist`, or `pip install` of a checkout or of the archive of a release on Codeberg) builds this crate with Rust.

## Functions

| Function | Result |
|---|---|
| `cli(argv)` | runs the `tok` command line with the arguments after the program name and returns the exit status |
| `version()` | version of the engine |
| `check(text, lang="en", level="full", bpmn=True)` | list of diagnostics as dicts with `code`, `severity`, `line`, `id`, `message`, sorted like `tok check` |
| `fmt(text)` | canonical text (`tok fmt`) |
| `semantic_xml(text)` | semantic BPMN XML without layout, attributes sorted (conformance format) |
| `bpmn(text)` | BPMN XML with diagram layout (`tok bpmn`) |
| `svg(text, title=None)` | SVG picture (`tok svg`) |
| `export(text, name="model")` | dict file name → content: `NAME.bpmn`, `NAME.svg`, one `.dmn` per decision, one `.form` per form (`tok export`) |
| `import_bpmn(xml, keep_ids=True)` | `(tok text, warnings)` (`tok import`) |
| `eval_decision(text, decision, values=None)` | result of a decision table for the variables in the dict `values`, as `dmn.evaluate_decision` returns it: a dict {output: value}, `None` when no rule matches, a list of dicts for hit policy `collect`; numbers as `int` or `float`, dates as ISO text (`tok eval`) |
| `simulate(text, cases=None, seed=None)` | a `SimResult` of the simulation (`tok sim`, `sim.simulate`); `cases` and `seed` override the attributes of `simulate` and, as in `sim.py`, may be `int`, `float` or `str` |
| `format_kpi(kpi, lang="en")` | the plain-text report of a dict of indicators as `SimResult.kpi()` returns it (`sim.format_kpi`) |
| `feel_eval(expr, context=None)` | value of a FEEL expression for the variables in the dict `context`, as `feel.eval_text` returns it: `decimal.Decimal`, `str`, `bool`, `None`, `list`, `dict`, `datetime.date`, `datetime.timedelta` or `datetime.datetime` |

`SimResult` has the members of `sim.Result`: `kpi()`, the properties `log` (list of dicts), `cases` (list of dicts with the case variables), `warnings`, `seed` and `horizon`, `to_json(include_log=False)`, `write_csv(path_log, path_cases=None)`, and `format_kpi(lang="en")`. It also has `to_xes()`, the event log as an XES document (IEEE 1849-2016) for process-mining tools such as ProM, pm4py, bupaR or Disco, and `write_xes(path)`, which writes that document to a file (UTF-8, `
` line endings; `path` is a `str` or an `os.PathLike`); the format is described in `docs/simulation.md`. These two are not in the pure-Python `sim.Result`, which is frozen at language 0.3, so with `TOK_ENGINE=python` the result of `tok.api.simulate` does not have them.

Documents with errors raise `ValueError` with the error diagnostics as text; `fmt` and `semantic_xml` need a document without parser errors, `bpmn`, `svg`, `export`, `eval_decision` and `simulate` a document without any errors (`eval_decision` checks without the BPMN rules, like `tok eval`).

Evaluation errors are raised as in the pure-Python implementation: `KeyError` for an unknown decision, `tok._engine.DecisionError` for a violated hit policy (two rules under `unique`, different outputs under `any`), `tok._engine.FeelSyntaxError` and `tok._engine.FeelUnsupported` (both subclasses of `tok._engine.FeelError`) for expressions that cannot be parsed, and the exceptions of the `decimal` module and the built-in ones (`ValueError`, `OverflowError`, `TypeError`) where Python's decimal and datetime arithmetic raises them. Variables may be `None`, `bool`, `str`, `int`, `float`, `decimal.Decimal`, `datetime.date`, `datetime.datetime`, `datetime.timedelta`, lists, tuples and dicts with string keys; other types raise `TypeError`.

## Build

Requirements: the Rust toolchain of the workspace, Python 3.8 or newer and maturin. From the repository root:

```
python -m venv .venv-engine
.venv-engine/Scripts/python -m pip install maturin          # macOS/Linux: .venv-engine/bin/...
PATH="$PWD/.venv-engine/Scripts:$PATH" bash tools/wheel.sh -o dist
.venv-engine/Scripts/python -m pip install dist/tok_lang-*.whl
```

`tools/wheel.sh` runs `maturin build --release` in the root of the repository; its arguments go to maturin (`--zig --compatibility manylinux2014` for Linux wheels that run on old distributions, `--target …` for other platforms). The wheel is abi3: one file per platform for every Python from 3.8.

## Parity tests

```
.venv-engine/Scripts/python -m unittest python/tests/test_engine_parity.py -v
```

The installed package is tested; the pure-Python reference runs from `python/src`. `tests/test_engine_parity.py` compares, for every document in `examples/`, `conformance/cases/` and (when the course repository `tok-course` is checked out next to this one) its `przyklady/`: diagnostics in English and Polish, canonical text, semantic XML, BPMN without the diagram, SVG by its drawn elements, DMN and form files exactly (the layout of the engine differs from Python from 0.4); `import_bpmn` (with and without flow identifiers) for every model in `conformance/import/`; `eval_decision` for every decision table of those documents and of `tok-core/tests/fixtures/eval/eval-tables.tok` over a grid of typed input values; `feel_eval` for the expression corpus `tok-core/tests/fixtures/eval/feel.json`; and `simulate` (indicators, log, cases, JSON and both text reports, or the exception) for every document and the synthetic simulation models `tok-sim/tests/fixtures/sim/sim-*.tok`, with several case counts and seeds. The pure-Python reference runs in a fresh interpreter for each input. The tests also compare the output and exit status of the `tok` command on the engine and with `TOK_ENGINE=python`, and the functions of `tok.api` on both. Without the engine the tests are skipped, so the normal Python test suite does not need Rust.

The crate has no Rust test harness (`test = false`): an extension module links against the Python runtime only when Python loads it, so `cargo test --workspace` builds but does not run it.
