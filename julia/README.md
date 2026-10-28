# Julia

| Folder | Contents |
|---|---|
| `Tok.jl/` | the package `Tok`: the whole language through the Rust engine (`Tok.Engine`), the independent core (lexer, parser, canonical printer, semantic BPMN export), the native simulator (`Tok.Sim`) and the calibration on event logs (`Tok.Fit`, with Bayesian estimates in the extension `TokTuringExt`) |
| `app/` | the `tok` application for Julia users, built with PackageCompiler (`julia app/build.jl`) |
| `binarybuilder/` | the BinaryBuilder recipe of `tok_jll`, the engine library for every platform, and how it is submitted to Yggdrasil |

The package and its use are described in [../docs/julia.md](../docs/julia.md), the calibration in [../docs/calibration.md](../docs/calibration.md), the C interface it calls in [../docs/c-api.md](../docs/c-api.md).

Requires Julia 1.10 or newer. The independent core needs no other package and no library:

```
cd julia/Tok.jl
julia --project=. bin/tok.jl check  ../../examples/propozycja-zakupu.tok
julia --project=. bin/tok.jl fmt    ../../examples/propozycja-zakupu.tok
julia --project=. bin/tok.jl xml    ../../examples/propozycja-zakupu.tok
```

It produces byte-identical output to the conformance corpus (`../conformance`): the canonical text (`expected/*.tok`), the core diagnostic codes (`expected/*.diag`) and the semantic XML (`expected/*.bpmn`), for every language version. The engine, the native simulator and the calibration need the engine library (`cargo build --release -p tok-capi` in `../rust`, the environment variable `TOK_LIBTOK`, or the package `tok_jll`).

```
julia --project=. -e 'using Pkg; Pkg.test(julia_args=["--threads=2"])'
```

runs the conformance tests of the core and, when the library is found, the tests of the engine, the native simulator (with its statistical comparison with the engine's), the calibration and the application.
