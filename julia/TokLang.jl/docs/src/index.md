# TokLang.jl

[tok](https://github.com/tok-lang-bpm/tok) is a plain-text language for BPMN 2.0 process models and DMN decisions. TokLang.jl is its Julia package. These pages are the reference of its public API. The [guide to the package](https://github.com/tok-lang-bpm/tok/blob/main/docs/julia.md) explains how the parts work together, and [the guide to tok](https://tok-lang-bpm.github.io/) teaches the language.

```julia
using Pkg
Pkg.add("TokLang")
```

| Module | What it does | Reference |
|---|---|---|
| `TokLang` | the independent core: parser, canonical printer, semantic BPMN export | [Independent core](@ref) |
| `TokLang.Engine` | the whole language through the Rust engine | [Engine](@ref) |
| `TokLang.Sim` | the native simulator for large experiments | [Native simulation](@ref) |
| `TokLang.Fit` | calibration of simulation parameters on event logs | [Calibration](@ref) |

The folder `examples` of the package holds runnable scripts, one for each module; `pkgdir(TokLang, "examples")` gives its path.

## The engine library

The engine comes with the package: `tok_jll` is a dependency of TokLang.jl and holds the prebuilt library for Linux (x86_64 and aarch64, glibc and musl), macOS (x86_64 and Apple silicon) and Windows (x86_64). On any other platform [`TokLang.Engine.available`](@ref) returns `false`. The independent core works, and the functions of `Engine`, `Sim` and `Fit` raise an error that says so.

To use your own build of the engine, for example when you work on the engine itself, build it with `cargo build --release -p tok-capi` in the folder `rust` of the repository and override the library of `tok_jll`:

```julia
using Pkg
Pkg.add("Preferences")
using Preferences, tok_jll
set_preferences!(tok_jll, "libtokengine_path" => "/path/to/libtokengine.so"; force=true)
```

The setting is written to `LocalPreferences.toml` of the active project and takes effect when Julia restarts. The library must implement the version of the C interface that TokLang.jl is written for ([`TokLang.Engine.abi_version`](@ref)).
