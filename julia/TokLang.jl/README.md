# TokLang.jl

[tok](https://github.com/tok-lang-bpm/tok) is a plain-text language for BPMN 2.0 process models and DMN decisions. TokLang.jl is its Julia package: it checks models, exports them to BPMN, SVG, DMN and Camunda forms, simulates processes and calibrates simulation parameters on event logs.

**Documentation:** [the API reference](https://tok-lang-bpm.github.io/julia/) of this package, [docs/julia.md](https://github.com/tok-lang-bpm/tok/blob/main/docs/julia.md) (the guide to the package), and [the guide to tok](https://tok-lang-bpm.github.io/).

This folder is one package of the [tok repository](https://github.com/tok-lang-bpm/tok), which also holds the language specification, the Rust engine and the packages for Python and R. TokLang.jl is registered from this subfolder.

## Installing

TokLang.jl needs Julia 1.10 or newer:

```julia
using Pkg
Pkg.add("TokLang")
```

This also installs `tok_jll`, the prebuilt library of the Rust engine, for Linux (x86_64 and aarch64, glibc and musl), macOS (x86_64 and Apple silicon) and Windows (x86_64). You need no Rust compiler. On any other platform the package still loads and its independent core works; `TokLang.Engine.available()` returns `false`.

## A first example

```julia
using TokLang

text = """
tok 1.0 "Book request"

pool library "Library"
  lane desk "Desk"
    start received "Request received"
    user lookup "Look up the book"
    end done "Request handled"

flow received -> lookup -> done
"""

const E = TokLang.Engine
E.check(text)                          # diagnostics, empty when the model is correct
write("request.bpmn", E.bpmn(text))    # BPMN 2.0 XML with diagram layout
write("request.svg", E.svg(text))      # the diagram as a picture
```

## What the package contains

| Layer | Module | What it does |
|---|---|---|
| engine | `TokLang.Engine` | the whole language through the Rust engine: the checker, canonical text, BPMN, SVG, DMN and form export, BPMN and DMN import, decisions, simulation |
| independent core | `TokLang` | a second implementation of the core of the specification (parser, canonical printer, semantic BPMN export), written in Julia and checked against the conformance corpus |
| native simulation | `TokLang.Sim` | a discrete-event simulator for large experiments: many scenarios and replications, in parallel on the threads of Julia |
| calibration | `TokLang.Fit` | estimates of processing times, arrival rates and branch probabilities from XES or CSV event logs; Bayesian estimates when Turing.jl is loaded |

## Examples

The folder [`examples`](https://github.com/tok-lang-bpm/tok/tree/main/julia/TokLang.jl/examples) holds runnable scripts and the models they read. The tests of the package run every script.

| Script | Shows |
|---|---|
| `01-core.jl` | the independent core: parsing, canonical text, semantic BPMN |
| `02-engine.jl` | checking, the files of `tok export`, a DMN decision |
| `03-simulation.jl` | a simulation run and scenarios compared over replications |
| `04-calibration.jl` | parameters estimated from an event log and written back to the model |

In an installed package, `pkgdir(TokLang, "examples")` is the path of the folder.

## Tests

`Pkg.test("TokLang")` runs the conformance tests of the core, and the tests of the engine, the native simulator (with its statistical comparison with the engine's simulator), the calibration, the application and the examples. The models and the corpus are copies inside the package, in `examples/` and `test/data/`.

## Licence

GPL-3.0-or-later.
