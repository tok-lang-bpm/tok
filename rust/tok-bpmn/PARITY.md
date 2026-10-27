# Parity with the Python reference and the layout engine

`tok-bpmn` is a port of the BPMN validator, layout engine and SVG renderer of the course tool `bpmn.py` 1.2, shipped with the Python implementation as `tok.bpmnlib`. The validator stays identical to Python. From tok 0.4 the pure-Python implementation is frozen at language 0.3 and the layout engine is developed further in Rust: the diagram (BPMNDI) and the SVG of a model laid out by tok differ from Python, the semantic BPMN does not.

## What stays identical to Python

`tests/gen_expected.py` runs the Python reference on every fixture and stores its outputs in `tests/expected/`; `tests/parity.rs` runs the Rust functions on the same fixtures:

| Output | Python | Rust | Comparison |
|---|---|---|---|
| diagnostics | `validate(m)` | `validate` | level, code, element id and Polish message of every issue, in order |
| diagnostics with step numbers | `validate(m, kroki=True)` | `validate_with` | same |
| diagnostics against a process description | `validate(m, kroki=True, opis_kroki=read_opis(...))` | `validate_with` + `read_opis` | same (fixtures with a `.opis.md` file) |
| Camunda 8 profile | `validate_camunda(m, folder)` | `validate_camunda` | same; the folder holds the fixture's `.form`/`.dmn` files |
| summary | `summarize(m)` | `summarize` | string equality |
| layout | `layout(m)` + `save()` | `layout` | byte-identical XML without the `BPMNDiagram` element (the semantic part, including the ids given to data associations without one) |
| drawing of a model with its own diagram | `render_svg(m, "model.bpmn")` | `render_svg` | byte-identical SVG (a diagram without `BPMNLabel` bounds is drawn exactly as by `bpmn.py`) |
| drawing of a model without a diagram | `render_svg(m, "model.bpmn")` | `render_svg` | drawn, or the same error message |

Regenerate after a deliberate change of `bpmnlib.py`:

```
PYTHONPATH=tok/python/src python tok/rust/tok-bpmn/tests/gen_expected.py
cd tok/rust && cargo test -p tok-bpmn
```

The same split applies in the other parity tests: `tok-core/tests/python_parity.rs` compares the exported BPMN without the diagram, and `python/tests/test_engine_parity.py` compares BPMN without the diagram and SVG by its structure (the same drawn elements).

### Fixtures

| Group | Models | Source |
|---|---|---|
| `import` | 16 | course models drawn for *Modelowanie procesów informacyjnych* (`tok/conformance/import`) |
| `semantic` | 15 | semantic BPMN exported by tok from the conformance corpus (`tok/conformance/expected`), laid out from scratch |
| `camunda` | 1 | executable Camunda 8 model with its forms and decision table |
| `extra` | 3 | stress cases: default BPMN namespace, foreign namespace prefixes and `xsi:type` rewriting, comments and CDATA, attribute-value normalization, nested lanes, transactions and cancel events, complex gateway, link events, multiple triggers, compensation, id-less data associations, broken references, step numbers with ranges and a process description |

## The layout engine

The layout keeps the style of `bpmn.py` that students know: columns from left to right by the longest path, pools stacked, lanes as rows, the main path straight, flows leaving to the right and entering from the left, splits leaving from the corners of gateways. It runs in three stages (`src/layout.rs`, `src/refine.rs`):

1. **Classic layout** — the port of `bpmn.py`: columns, rows by the first predecessor, a few candidate routes per flow.
2. **Refinement** — every sequence or message flow with a defect (crossing another edge, passing through a shape or a label, running on top of another flow or along a pool or lane border, leaving its pool, a message flow crossing a pool or lane that holds neither of its ends, a boundary flow that does not leave downwards) and every flow with two or more bends gets alternative routes: the classic candidates, vertical channels for message flows (several positions along an activity, never on a port used by another flow), and orthogonal shortest paths (A*) on a grid of the free channels between shapes, which keep a clearance from shapes and pay for bends, crossings, overlaps, labels, borders and foreign pools. Labels of events, gateways, data and flows are moved to the best free position among candidates around their element (below an element and next to the first long segment of a flow, as before, come first; narrower or wider wrapping is tried), never on the title band of a pool or lane, and written as `BPMNLabel` bounds, which `render_svg` and Camunda Modeler use.
3. **Order of the nodes** — a second layout orders the nodes of every column of a lane by barycentre sweeps (the rows a column occupies stay the same) and is refined in the same way; it is used only when it is better.

A change is accepted only when it lowers the weighted cost and none of the measures below (computed for the whole diagram) becomes higher than in the classic layout, and the drawing does not grow; so the result is never worse than the layout of tok 0.3 on any measure. All decisions are made in a fixed order on integer coordinates, without hashing order or randomness: the same input gives the same bytes (checked by `tests/golden.rs`).

### Quality measures

`tests/quality/mod.rs` computes, for every model of the corpus (language examples, conformance cases, the course diagrams of `conformance/import` — the same files as `diagramy/` of the course repository — the `extra` and `camunda` fixtures and, when the course repository is checked out next to this one, `tok-course/przyklady`), these counts of defects:

| Measure | Counts |
|---|---|
| `cross` | crossings of two edges |
| `hits` | edges passing through a shape they do not connect |
| `shapes` | pairs of overlapping shapes |
| `labels` | labels overlapping another label, a shape or the border of a pool |
| `lbl_edge` | labels crossed by an edge |
| `bends` | bends of sequence and message flows |
| `msg` | message flows passing through a pool or lane that holds neither of their ends |
| `flow_ovl` | pairs of edges running on top of each other |
| `bnd` | boundary flows not leaving downwards, crossing their host or going back |
| `area` | area of the drawing, in thousands of square units |

Totals over 65 models, the layout of tok 0.3 and now:

| | cross | hits | shapes | labels | lbl_edge | bends | msg | flow_ovl | bnd | area |
|---|---|---|---|---|---|---|---|---|---|---|
| 0.3 (`bpmn.py`) | 59 | 22 | 0 | 28 | 112 | 281 | 54 | 11 | 0 | 30193 |
| 0.4 | 30 | 17 | 0 | 0 | 3 | 228 | 53 | 8 | 0 | 30185 |

No model is worse on any measure. The remaining defects come mostly from the placement of nodes (a crossing that only a longer detour could avoid would cost more bends than the classic layout has) and from message flows that must cross the lanes of their own pool. Laying out and drawing takes about 1.6 ms per model of the corpus on average (0.7 ms in tok 0.3).

```
cargo run --release -p tok-bpmn --example layout_metrics tok-bpmn/tests/layout/baseline-0.3.tsv
```

prints the table per model with the values of tok 0.3 in brackets and the time; with `TOK_LAYOUT_DUMP=DIR` it also writes the laid-out XML and the SVG of every model.

### Tests of the layout

| Test | Checks | Regenerate |
|---|---|---|
| `tests/layout_quality.rs` | no measure worse than in `tests/layout/metrics.tsv` (the current values) and in `tests/layout/baseline-0.3.tsv` (tok 0.3); the laid-out XML reports the same BPMN errors and warnings as its input and has diagram interchange for every element (no `D01`, `D02`) | `TOK_BLESS=1 cargo test -p tok-bpmn --test layout_quality` |
| `tests/golden.rs` | laid-out XML and SVG of all `semantic` and `extra` fixtures, the Camunda model and four course diagrams equal `tests/golden/`; two runs give the same bytes | `TOK_BLESS=1 cargo test -p tok-bpmn --test golden` |

## Behaviour that is reproduced on purpose

- **ElementTree serialization.** Namespace declarations collected over the whole tree and written on the root sorted by prefix; registered prefixes as `load()` leaves them (ElementTree defaults, then the input's prefixes, then `bpmn`, `bpmndi`, `dc`, `di`, `xsi`); `ns0`-style prefixes for unregistered namespaces; attribute order as parsed or inserted; `" />"` for empty elements; ElementTree's escaping of text and attributes (`&#10;`, `&#09;`, `&#13;`); `_indent` and the whitespace clean-up of `save()`; extra `xmlns:` declarations for prefixes used only inside attribute values.
- **Python numbers.** `"%.1f"`/`"%.0f"` and `round()` round half to even on the exact binary value, as Rust's formatting and `f64::round_ties_even` do; `str(int(round(x)))` turns `-0.0` into `0`. Trigonometry and square roots use the platform C library through `f64::cos`, `f64::sin` and `powf(0.5)`, as Python's `math` module and `** 0.5` do.
- **Python `None`.** Missing ids and references behave as Python's `None` dictionary key (one shared key), and are printed as `None` in messages exactly where Python prints them.
- **`str.capitalize()`** lowercases everything after the first character.

## Known differences

| Area | Python | Rust | Effect |
|---|---|---|---|
| Automatic layout (from 0.4) | `bpmn.py` layout | refined layout with `BPMNLabel` bounds | different diagram and drawing of a model laid out by tok; semantic XML identical |
| Line endings of files written by `save()` | platform line ending (CRLF on Windows) | `\n` | the library returns text; the caller chooses the line ending. Expected files are stored with `\n`. |
| Step numbers (`--kroki`) with non-ASCII digits | `\d` matches any Unicode decimal digit | ASCII digits only | names starting with e.g. Arabic-Indic digits are not recognised as step numbers |
| Whitespace in `str.split()`/`strip()` | Python `isspace()` (includes U+001C–U+001F) | Unicode `White_Space` | only for those four control characters in names or texts |
| Sorting step numbers that mix numbers and letters at the same position | raises `TypeError` | numbers before letters | Python would crash; Rust reports |
| Elements without `id` in DI emission, `D02` with id-less elements | raise `TypeError` | skipped / printed as `None` | Python would crash |
| XML parser messages | expat wording | roxmltree wording | only in `Error::Xml` |
