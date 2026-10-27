# Parity with the Python reference

`tok-bpmn` is a port of the BPMN validator, layout engine and SVG renderer of the course tool `bpmn.py` 1.2, shipped with the Python implementation as `tok.bpmnlib`. The goal is identical output for identical input.

## How parity is checked

`tests/gen_expected.py` runs the Python reference on every fixture and stores its outputs in `tests/expected/`; `tests/parity.rs` runs the Rust functions on the same fixtures and requires equality:

| Output | Python | Rust | Comparison |
|---|---|---|---|
| diagnostics | `validate(m)` | `validate` | level, code, element id and Polish message of every issue, in order |
| diagnostics with step numbers | `validate(m, kroki=True)` | `validate_with` | same |
| diagnostics against a process description | `validate(m, kroki=True, opis_kroki=read_opis(...))` | `validate_with` + `read_opis` | same (fixtures with a `.opis.md` file) |
| Camunda 8 profile | `validate_camunda(m, folder)` | `validate_camunda` | same; the folder holds the fixture's `.form`/`.dmn` files |
| summary | `summarize(m)` | `summarize` | string equality |
| layout | `layout(m)` + `save()` | `layout` | byte-identical XML |
| drawing | `render_svg(m, "model.bpmn")` | `render_svg` | byte-identical SVG, or the same error message |

Regenerate after a deliberate change of `bpmnlib.py`:

```
PYTHONPATH=tok/python/src python tok/rust/tok-bpmn/tests/gen_expected.py
cd tok/rust && cargo test -p tok-bpmn
```

## Fixtures

| Group | Models | Source |
|---|---|---|
| `import` | 16 | course models drawn for *Modelowanie procesów informacyjnych* (`tok/conformance/import`) |
| `semantic` | 15 | semantic BPMN exported by tok from the conformance corpus (`tok/conformance/expected`), laid out from scratch |
| `camunda` | 1 | executable Camunda 8 model with its forms and decision table |
| `extra` | 3 | stress cases: default BPMN namespace, foreign namespace prefixes and `xsi:type` rewriting, comments and CDATA, attribute-value normalization, nested lanes, transactions and cancel events, complex gateway, link events, multiple triggers, compensation, id-less data associations, broken references, step numbers with ranges and a process description |

Result: 35 models × 6 outputs (7 with a process description), all identical.

## Behaviour that is reproduced on purpose

- **ElementTree serialization.** Namespace declarations collected over the whole tree and written on the root sorted by prefix; registered prefixes as `load()` leaves them (ElementTree defaults, then the input's prefixes, then `bpmn`, `bpmndi`, `dc`, `di`, `xsi`); `ns0`-style prefixes for unregistered namespaces; attribute order as parsed or inserted; `" />"` for empty elements; ElementTree's escaping of text and attributes (`&#10;`, `&#09;`, `&#13;`); `_indent` and the whitespace clean-up of `save()`; extra `xmlns:` declarations for prefixes used only inside attribute values.
- **Python numbers.** `"%.1f"`/`"%.0f"` and `round()` round half to even on the exact binary value, as Rust's formatting and `f64::round_ties_even` do; `str(int(round(x)))` turns `-0.0` into `0`. Trigonometry and square roots use the platform C library through `f64::cos`, `f64::sin` and `powf(0.5)`, as Python's `math` module and `** 0.5` do.
- **Python `None`.** Missing ids and references behave as Python's `None` dictionary key (one shared key), and are printed as `None` in messages exactly where Python prints them.
- **`str.capitalize()`** lowercases everything after the first character.

## Known differences (not exercised by the fixtures)

| Area | Python | Rust | Effect |
|---|---|---|---|
| Line endings of files written by `save()` | platform line ending (CRLF on Windows) | `\n` | the library returns text; the caller chooses the line ending. Expected files are stored with `\n`. |
| Step numbers (`--kroki`) with non-ASCII digits | `\d` matches any Unicode decimal digit | ASCII digits only | names starting with e.g. Arabic-Indic digits are not recognised as step numbers |
| Whitespace in `str.split()`/`strip()` | Python `isspace()` (includes U+001C–U+001F) | Unicode `White_Space` | only for those four control characters in names or texts |
| Sorting step numbers that mix numbers and letters at the same position | raises `TypeError` | numbers before letters | Python would crash; Rust reports |
| Elements without `id` in DI emission, `D02` with id-less elements | raise `TypeError` | skipped / printed as `None` | Python would crash |
| XML parser messages | expat wording | roxmltree wording | only in `Error::Xml` |
