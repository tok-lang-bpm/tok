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

Documents with errors raise `ValueError` with the error diagnostics as text; `fmt` and `semantic_xml` need a document without parser errors, `bpmn`, `svg` and `export` a document without any errors.

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

`tests/test_engine_parity.py` compares, for every document in `examples/`, `conformance/cases/` and (when the course repository `tok-course` is checked out next to this one) its `przyklady/`: diagnostics in English and Polish, canonical text, semantic XML, BPMN with layout, SVG and every exported file; and `import_bpmn` (with and without flow identifiers) for every model in `conformance/import/`. The pure-Python reference runs in a fresh interpreter for each input. Without `tok_engine` the tests are skipped, so the normal Python test suite does not need Rust.

The crate has no Rust test harness (`test = false`): an extension module links against the Python runtime only when Python loads it, so `cargo test --workspace` builds but does not run it.
