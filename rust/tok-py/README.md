# tok-py: Python bindings of the tok engine

The crate builds the Python extension module `tok_engine` on top of `tok-core` and `tok-bpmn`. It is used to test that the Rust engine and the pure-Python implementation of tok produce the same results. It is not published: until the 1.0 registry release the installed package `tok-lang` stays pure Python, so users need no Rust compiler.

## Functions

| Function | Result |
|---|---|
| `version()` | version of the engine |
| `check(text, lang="en", level="full", bpmn=True)` | list of diagnostics as dicts with `code`, `severity`, `line`, `id`, `message`, sorted like `tok check` |
| `fmt(text)` | canonical text (`tok fmt`) |
| `semantic_xml(text)` | semantic BPMN XML without layout, attributes sorted (conformance format) |
| `bpmn(text)` | BPMN XML with diagram layout (`tok bpmn`) |
| `svg(text, title=None)` | SVG picture (`tok svg`) |
| `export(text, name="model")` | dict file name → content: `NAME.bpmn`, `NAME.svg`, one `.dmn` per decision, one `.form` per form (`tok export`) |
| `import_bpmn(xml, keep_ids=True)` | `(tok text, warnings)` (`tok import`) |
| `eval_decision(text, decision, values=None)` | result of a decision table for the variables in the dict `values`, as `dmn.evaluate_decision` returns it: a dict {output: value}, `None` when no rule matches, a list of dicts for hit policy `collect`; numbers as `int` or `float`, dates as ISO text (`tok eval`) |
| `feel_eval(expr, context=None)` | value of a FEEL expression for the variables in the dict `context`, as `feel.eval_text` returns it: `decimal.Decimal`, `str`, `bool`, `None`, `list`, `dict`, `datetime.date`, `datetime.timedelta` or `datetime.datetime` |

Documents with errors raise `ValueError` with the error diagnostics as text; `fmt` and `semantic_xml` need a document without parser errors, `bpmn`, `svg`, `export` and `eval_decision` a document without any errors (`eval_decision` checks without the BPMN rules, like `tok eval`).

Evaluation errors are raised as in the pure-Python implementation: `KeyError` for an unknown decision, `tok_engine.DecisionError` for a violated hit policy (two rules under `unique`, different outputs under `any`), `tok_engine.FeelSyntaxError` and `tok_engine.FeelUnsupported` (both subclasses of `tok_engine.FeelError`) for expressions that cannot be parsed, and the exceptions of the `decimal` module and the built-in ones (`ValueError`, `OverflowError`, `TypeError`) where Python's decimal and datetime arithmetic raises them. Variables may be `None`, `bool`, `str`, `int`, `float`, `decimal.Decimal`, `datetime.date`, `datetime.datetime`, `datetime.timedelta`, lists, tuples and dicts with string keys; other types raise `TypeError`.

## Build

Requirements: the Rust toolchain of the workspace and Python 3.8 or newer. From the repository root:

```
python -m venv .venv-engine
.venv-engine/Scripts/python -m pip install maturin          # macOS/Linux: .venv-engine/bin/python
.venv-engine/Scripts/maturin develop --release -m rust/tok-py/Cargo.toml
```

`maturin develop` compiles the module and installs it into the active virtual environment.

## Parity tests

```
cd python
PYTHONPATH=src ../.venv-engine/Scripts/python -m unittest tests.test_engine_parity -v
```

`tests/test_engine_parity.py` compares, for every document in `examples/`, `conformance/cases/` and (when the course repository `tok-course` is checked out next to this one) its `przyklady/`: diagnostics in English and Polish, canonical text, semantic XML, BPMN with layout, SVG and every exported file; `import_bpmn` (with and without flow identifiers) for every model in `conformance/import/`; `eval_decision` for every decision table of those documents and of `tok-core/tests/fixtures/eval/eval-tables.tok` over a grid of typed input values; and `feel_eval` for the expression corpus `tok-core/tests/fixtures/eval/feel.json`. The pure-Python reference runs in a fresh interpreter for each input. Without `tok_engine` the tests are skipped, so the normal Python test suite does not need Rust.

The crate has no Rust test harness (`test = false`): an extension module links against the Python runtime only when Python loads it, so `cargo test --workspace` builds but does not run it.
