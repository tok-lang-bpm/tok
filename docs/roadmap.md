# Roadmap

tok 0.1 has two implementations that read the same text and pass the same conformance corpus:

| Implementation | Folder | Status in 0.1 |
|---|---|---|
| **Python** (reference) | `python/` | full: check, fmt, BPMN/SVG/DMN/form export, BPMN import, FEEL subset, decision evaluation, simulation, CLI |
| **Julia** (port) | `julia/Tok.jl` | core: parsing, diagnostics of the text layer, canonical printing, semantic BPMN export |

The Python version is the reference because it installs with one command on any computer that has Python and needs no compiler. The Julia port checks that the language is simple enough to be implemented twice from the specification and that both implementations agree byte for byte.

## Version 0.2 — language

- Syntax for choreographies and conversations (BPMN conformance class "full").
- Working-time calendars and resource shifts in the simulation layer.
- Data inputs and outputs of processes (`ioSpecification`) for executable models.
- Decision requirements diagrams: decisions that call other decisions and input data elements.
- Assertions for exercises: `expect` statements that `tok check` verifies (for example "every path reaches an end event", "uses an event-based gateway").

## Version 0.3 — Julia port to full

- FEEL subset, decision evaluation and simulation in Julia, compared with Python on fixed seeds through the conformance corpus (statistical comparison of indicators, since random number streams differ).
- Precompiled application (`juliac` or a Pkg app) so that `tok` starts quickly from the command line.

## Version 1.0 — one engine for Python and R

The long-term plan is a single engine with thin bindings, instead of separate implementations per language:

- **Core in Rust or Zig.** The parser, checker, printer, exporters, FEEL evaluator and simulator move to one compiled library with a C interface. The choice between Rust and Zig is made with the experience of the Python and Julia implementations: Rust offers mature bindings for both target languages (PyO3 for Python, extendr for R) and an existing DMN/FEEL ecosystem; Zig offers a native C ABI and a small toolchain (rzig for R).
- **Python package** on top of the core: the same command line and API as the 0.1 reference implementation, so existing scripts keep working.
- **R package `tok`**: `tok_read()`, `tok_check()` (diagnostics as a data frame), `tok_fmt()`, `tok_plot()` (SVG in the RStudio viewer and in documents), `tok_sim()` (event log as a data frame, compatible with process-mining packages), and a knitr engine so that a `{tok}` chunk in R Markdown or Quarto renders the diagram.
- **Conformance corpus** as the contract: every implementation, old or new, must pass it.
