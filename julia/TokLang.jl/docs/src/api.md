# Stable API

TokLang.jl follows [semantic versioning](https://semver.org). Its public API is what the reference pages of this manual list:

- the names exported from `TokLang`: [Independent core](@ref);
- the functions and types of the modules `TokLang.Engine`, `TokLang.Sim` and `TokLang.Fit` on the pages [Engine](@ref), [Native simulation](@ref) and [Calibration](@ref), reached through the module (`TokLang.Engine.check`);
- the `fit_bayes` function of `TokLang.Fit`, available when Turing.jl is loaded.

Within 1.x, a public function keeps working with the same arguments and keyword arguments, and the fields of its documented result stay. New functions, new keyword arguments and new fields can be added in a minor version. Everything else (names that are not listed in the reference, including the modules `TokLang.JSON` and `TokLang.Stats` and the fields of `SimModel`) is internal and may change in any release.

The text of diagnostic messages and the exact numbers of a simulation with a given seed are not part of the API; the diagnostic codes are. The language itself has its own [compatibility promise](https://github.com/tok-lang-bpm/tok/blob/main/docs/spec.md), which holds for every release 1.x.
