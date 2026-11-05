# Julia

| Folder | Contents |
|---|---|
| `TokLang.jl/` | the package `TokLang` (its own [README](TokLang.jl/README.md)): the whole language through the Rust engine (`TokLang.Engine`), the independent core (lexer, parser, canonical printer, semantic BPMN export), the native simulator (`TokLang.Sim`) and the calibration on event logs (`TokLang.Fit`, with Bayesian estimates in the extension `TokLangTuringExt`) |
| `app/` | the `tok` application for Julia users, built with PackageCompiler (`julia app/build.jl`) |
| `binarybuilder/` | the BinaryBuilder recipe of `tok_jll`, the engine library for every platform, and how it is submitted to Yggdrasil |

The package and its use are described in [../docs/julia.md](../docs/julia.md), the calibration in [../docs/calibration.md](../docs/calibration.md), the C interface it calls in [../docs/c-api.md](../docs/c-api.md).

Requires Julia 1.10 or newer. The independent core uses no library:

```
cd julia/TokLang.jl
julia --project=. bin/tok.jl check  ../../examples/propozycja-zakupu.tok
julia --project=. bin/tok.jl fmt    ../../examples/propozycja-zakupu.tok
julia --project=. bin/tok.jl xml    ../../examples/propozycja-zakupu.tok
```

It produces byte-identical output to the conformance corpus (`../conformance`): the canonical text (`expected/*.tok`), the core diagnostic codes (`expected/*.diag`) and the semantic XML (`expected/*.bpmn`), for every language version. The engine, the native simulator and the calibration use the engine library of the package `tok_jll`, a dependency of TokLang.jl; a build of your own replaces it through a preference ([../docs/julia.md](../docs/julia.md)).

```
julia --project=. -e 'using Pkg; Pkg.test(julia_args=["--threads=2"])'
```

runs the conformance tests of the core and, when the library is available, the tests of the engine, the native simulator (with its statistical comparison with the engine's), the calibration, the application and the examples.
