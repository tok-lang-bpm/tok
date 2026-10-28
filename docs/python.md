# Python

The package `tok-lang` gives the `tok` command and the module `tok.api` for programs and notebooks. Both run on the Rust engine, the extension module `tok._engine`. With the environment variable `TOK_ENGINE=python` they run on the pure-Python implementation instead, which is frozen at language 0.3; both give the same results for documents of languages 0.1 and 0.3, and everything added later exists only in the engine.

## Installation and packaging

```
pip install tok-lang
```

PyPI has abi3 wheels (one per platform for every Python from 3.8): Windows x64, Linux x86_64 and aarch64 (manylinux2014, glibc 2.17 or newer) and macOS (universal2, Intel and Apple silicon). Elsewhere pip builds the wheel from the source archive `tok_lang-VERSION.tar.gz`, which contains the Python package and the crates of the engine and needs Rust 1.99 or newer (`rust/rust-toolchain.toml`); maturin is installed by pip for the build. The package is built by maturin from `pyproject.toml` at the root of the repository: the Python sources `python/src/tok`, the extension module from the crate `rust/tok-py`.

The pure-Python implementation is part of every wheel and of the source archive: the modules of `tok` besides `api` and `_engine` (`parser`, `check`, `printer`, `bpmn_export`, `dmn`, `forms`, `sim`, …) are that implementation, unchanged since 0.3. It is kept because it is the reference of the parity tests of the engine for languages 0.1 and 0.3, not as a second engine for users: `TOK_ENGINE=python` selects it, and `tok.api.engine()` tells which one runs. It reads the headers `tok 0.1` and `tok 0.3`; any newer header is an unknown version to it (`P006`), and the statements added after 0.3 are unknown keywords.

```python
from tok import api

text = open("model.tok", encoding="utf-8").read()
api.check(text)                          # [] or a list of diagnostics
open("model.bpmn", "w", encoding="utf-8").write(api.bpmn(text))
api.eval_decision(text, "tryb-akceptacji", {"rodzaj": "e-book", "cena": 120})
result = api.simulate(text, cases=200, seed=1)
print(api.format_kpi(result.kpi()))
```

## Functions

Every function takes the text of a document (or of a BPMN file). The functions that need a valid document raise `ValueError` with the error diagnostics, one per line, as `tok check` prints them.

| Function | Command | Result |
|---|---|---|
| `engine()` | | `"rust"` when the Rust engine is used, `"python"` otherwise |
| `version()` | `tok --version` | the version of the engine in use |
| `check(text, lang="en", level="full", bpmn=True)` | `tok check` | the diagnostics: a list of dicts with the keys `code`, `severity`, `line`, `id` and `message`, sorted by line and code; `lang` is `en` or `pl`, `level` one of `basic`, `analytic`, `full`, and `bpmn=False` leaves out the BPMN rules (E, W, L) |
| `fmt(text)` | `tok fmt` | the canonical text; `ValueError` on syntax errors |
| `bpmn(text)` | `tok bpmn` | BPMN 2.0 XML with diagram layout |
| `svg(text, title=None)` | `tok svg` | the SVG picture of the diagram |
| `export(text, name="model")` | `tok export` | every file of `tok export` as a dict file name → content: `NAME.bpmn`, `NAME.svg`, the decision files and one `.form` per form |
| `import_bpmn(xml, keep_ids=True)` | `tok import` | `(text, warnings)`: the document and the list of what the import did not keep; `keep_ids=False` drops the identifiers of sequence flows (`--no-flow-ids`) |
| `eval_decision(text, decision, values=None)` | `tok eval` | the value of a decision for a dict of input values (spec §9.5): in a 0.7 or 1.0 document the DMN value of the decision with everything it requires, in earlier documents the outputs of the table (a dict, a list of dicts for `collect`, or `None`) |
| `simulate(text, cases=None, seed=None)` | `tok sim` | the result of the simulation (below); `cases` and `seed` override the `simulate` statement |
| `format_kpi(kpi, lang="en")` | `tok sim` | the text report of the indicators `simulate(...).kpi()` |

The result of `simulate` has `kpi()` (the indicators as a dict), `log` (the event log, one row per activity instance), `cases` (one row per case), `warnings`, `seed`, `horizon`, `to_json(include_log=False)` and `write_csv(path_log, path_cases=None)`; on the engine also `to_xes()` and `write_xes(path)`, the event log as an XES document (IEEE 1849-2016). `docs/simulation.md` describes the indicators and the columns.

The engine module `tok._engine` itself is an implementation detail of these functions; the C interface (`c-api.md`) offers the same functions with options and results as JSON for other languages.
