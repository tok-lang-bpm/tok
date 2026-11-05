# Julia: TokLang.jl

`TokLang.jl` brings tok to Julia in three layers:

| Layer | Module | What it does | Needs |
|---|---|---|---|
| engine | `TokLang.Engine` | the whole language through the Rust engine: the checker of every layer, canonical text, BPMN, SVG, DMN and form export, BPMN and DMN import, decisions, simulation | the engine library |
| independent core | `TokLang` | a second implementation of the core of the specification: lexer, parser, canonical printer and semantic BPMN export, checked against the conformance corpus | no library |
| native | `TokLang.Sim`, `TokLang.Fit` | a discrete-event simulator for large experiments (many scenarios and replications, parallel runs) and the calibration of simulation parameters on event logs | the engine library (models are read by the engine); Turing.jl for Bayesian calibration |

The engine is the reference: every feature of the language is written once, in Rust, and `TokLang.Engine` gives it to Julia without a second implementation. The independent core exists because two implementations of one specification are the strongest test that the specification is unambiguous; it passes the core level of the conformance corpus (spec §15) for every case, and its output is compared with the engine's in the tests. Native Julia code is used only where Julia adds something: large simulation experiments and Bayesian calibration.

## Installing

TokLang.jl needs Julia 1.10 or newer. It is installed from the General registry, or from the repository:

```julia
using Pkg
Pkg.add("TokLang")
Pkg.add(url="https://github.com/tok-lang-bpm/tok", subdir="julia/TokLang.jl")   # from the repository
```

The engine library comes with the package: the binary package `tok_jll`, a dependency of TokLang.jl, holds it for Linux (x86_64 and aarch64, glibc and musl), macOS (x86_64 and Apple silicon) and Windows (x86_64). `TokLang.Engine.available()` tells whether the library can be loaded, `TokLang.Engine.library()` where it is. On any other platform the package still loads and the independent core works; the engine, the native simulator and the calibration report that the library is not available.

To use your own build of the engine, for example when you work on the engine itself, run `cargo build --release -p tok-capi` in `rust/` (see [c-api.md](c-api.md)) and point `tok_jll` to the library with a preference. The setting is written to `LocalPreferences.toml` of the active project and takes effect when Julia restarts:

```julia
using Pkg
Pkg.add("Preferences")
using Preferences, tok_jll
set_preferences!(tok_jll, "libtokengine_path" => "/path/to/libtokengine.so"; force=true)
```

The public API is listed in the reference at https://tok-lang-bpm.github.io/julia/, and TokLang.jl follows semantic versioning for it (page Stable API). The folder `examples` of the package holds runnable scripts, one for each module, with the models they read; in an installed package its path is `pkgdir(TokLang, "examples")`.

## The engine

The functions mirror the Python package (`tok.api`) and the R package (`tokr`). Every function takes the text of a document.

```julia
using TokLang
const E = TokLang.Engine

text = read("examples/kwerenda-symulacja.tok", String)
E.check(text)                       # diagnostics: Vector of Dicts (code, severity, line, id, message)
E.check(text; lang="pl", level="analytic", class="descriptive")
E.bpmn_class(text)                  # smallest BPMN conformance class
E.fmt(text)                         # canonical text
E.bpmn(text)                        # BPMN 2.0 XML with layout (layout=false, sidecar=...)
E.svg(text; title="Kwerenda")       # SVG picture
E.export_files(text; name="kwerenda")   # Dict file name => content, as tok export
E.import_bpmn(xml)                  # (text, warnings)
E.import_dmn(xml)                   # (text, warnings)
E.import_file(xml)                  # Dict: format, text, warnings, layout (the sidecar of a BPMN file)
E.eval_decision(text, "limit", Dict("wiek" => 30, "typ" => "student"))
r = E.simulate(text; cases=500, seed=1, xes=true)   # SimResult: kpi, report, log, cases, xes, ...
print(r.report)
E.sim_xes(text)                     # the XES event log, as tok sim --xes
E.cli(["check", "model.tok", "--lang", "pl"])       # the tok command line; returns the exit status
```

| Function | Python | R |
|---|---|---|
| `check` | `check` | `tok_check` |
| `fmt` | `fmt` | `tok_fmt` |
| `bpmn`, `svg` | `bpmn`, `svg` | `tok_bpmn`, `tok_plot` |
| `export_files` | `export` | `tok_export` |
| `import_bpmn`, `import_dmn`, `import_file` | `import_bpmn` | `tok_import` |
| `eval_decision` | `eval_decision` | `tok_eval` |
| `simulate`, `sim_xes` | `simulate`, `SimResult.to_xes` | `tok_sim`, `tok_xes` |

(`export`, `import` and `eval` are reserved words or functions of Julia, hence the longer names.)

Errors of the engine are thrown as `TokLang.Engine.TokError` with a `kind` (`"document"`, `"decision"`, `"simulation"`, … as listed in [c-api.md](c-api.md)), a `message` and, for a document with errors, its `diagnostics`. The functions may be called from several threads at once.

## The independent core

```julia
doc, diags = TokLang.parse_tok(text)
print(TokLang.fmt(doc))            # canonical text
print(TokLang.semantic_xml(doc))   # semantic BPMN XML of the conformance corpus
TokLang.format_diag(diags[1], "pl")
```

The core reads every language version (0.1 to 1.0) and reports the core diagnostics (codes `T`, `P` and the structural `R` codes of the corpus). The command line `julia --project=julia/TokLang.jl julia/TokLang.jl/bin/tok.jl fmt|xml|check FILE` runs it.

## Native simulation

`TokLang.Sim` runs the processes the engine reads, with the semantics of the engine's simulator ([simulation.md](simulation.md#native-simulation-in-julia) lists what it runs), at the speed of compiled Julia and in parallel. The engine reads the document (`TokLang.Sim.model`) and evaluates the conditions of flows; everything else runs in Julia.

```julia
const S = TokLang.Sim
m = S.model(text)                       # SimModel; S.supported(m), S.unsupported(m)
run = S.simulate(m; seed=7)             # one run: cycle_times, by_end_event, activities, utilization, log
S.cycle_time(run)                       # (mean, median, p90) in seconds
S.utilization(run, "archiwisci")

# scenarios: another number of performers, other durations or probabilities, another arrival rate
three = S.scenario(m; name="three archivists", resources=Dict("archiwisci" => 3))
faster = S.scenario(m; name="shorter search", durations=Dict("poszukiwania" => "lnorm(5h, 2h)"))
busier = S.scenario(m; name="busier", arrival="exp(3h)")

e = S.experiment([m, three, faster, busier]; replications=200, seed=1)
S.summarize(e)       # per scenario and indicator: mean, sd, half-width of the 95 % interval, n
S.indicators(e)      # every indicator of every run, in long form (for DataFrames)
```

Start Julia with several threads (`julia -t 8`, or `-t auto`) and `experiment` distributes the runs over them. Each run has its own random stream, derived from the seed and the number of the replication (and of the scenario with `common_random_numbers=false`), so the results do not depend on the number of threads or the order in which the runs finish, and a replication can be repeated on its own (`S.simulate(m; seed=1, replication=17)`). By default all scenarios use the same streams (common random numbers), which makes the differences between scenarios more precise than the scenarios themselves. The generator is xoshiro256** written in the package, so the streams are the same on every platform and Julia version.

A model with features the native simulator does not run (decisions of rule tasks, script expressions, multi-instance activities, ad-hoc sub-processes, compensation, transactions, complex gateways, standard loops, multiple events, instantiating elements) is rejected with `UnsupportedModel`, which names the elements; such models run in the engine (`TokLang.Engine.simulate`).

## Calibration

`TokLang.Fit` estimates the parameters of a simulation model from an event log: processing times of the activities, the time between arrivals and the probabilities of branches ([calibration.md](calibration.md)).

```julia
const F = TokLang.Fit
log = F.read_xes("log.xes")                          # or F.read_csv_log("log.csv"; cases="cases.csv")
f = F.fit(text, log)                                 # maximum likelihood, 95 % intervals
F.table(f)                                           # one row per parameter
write("fitted.tok", F.apply(text, f))                # the document with the fitted parameters

using Turing                                         # loads the extension TokLangTuringExt
b = F.fit_bayes(text, log; samples=1000)             # posterior means, 95 % credible intervals
```

Turing.jl is a weak dependency: TokLang.jl does not install it, and `fit_bayes` becomes available when Turing is loaded in the same session.

## The tok application

`julia/app` builds a `tok` program for Julia users with PackageCompiler:

```
julia julia/app/build.jl [OUTPUT]       # default: julia/app/build/tok
```

The folder it writes holds `bin/tok` (`bin\tok.exe` on Windows) with the Julia runtime, the compiled TokLang.jl and the engine library of `tok_jll` (the folder takes about 400 MB, and the build several minutes). The program has every command of the `tok` program (they run in the engine) and three of its own:

```
tok experiment model.tok --replications 200 --resource archiwisci=3     # native simulation, scenarios
tok fit model.tok log.xes -o fitted.tok                                 # calibration
tok core fmt model.tok                                                  # the independent core
```

`tok experiment` runs on as many threads as `JULIA_NUM_THREADS` gives (the compiled program reads it), `--json` prints the table as JSON. The same commands are available in Julia as `TokLang.app_main(["experiment", …])`.

## Tests

```
cd julia/TokLang.jl
julia --project=. -e 'using Pkg; Pkg.test(julia_args=["--threads=2"])'
```

The tests also run from an installed package (`Pkg.test("TokLang")`): the models and the conformance corpus they read are copies inside the package, in `examples/` and `test/data/`, written by `tools/julia-data.sh`, and `tools/check.sh` checks that they are current. The conformance test of the core always runs. The tests of the engine, the native simulator, the calibration, the application and the examples run when the engine library is available; `tools/check.sh` builds the library and points `tok_jll` to it with a `LocalPreferences.toml` in the folder of the package. They include the statistical comparison of the native simulator with the engine's ([simulation.md](simulation.md#native-simulation-in-julia)); `TOK_SIM_REPORT=1` prints its table. The tests also check the package with Aqua.jl. The Bayesian calibration is tested in an environment with Turing:

```
julia --project=ENV -e 'using Pkg; Pkg.develop(path="julia/TokLang.jl"); Pkg.add("Turing")'
julia --project=ENV julia/TokLang.jl/test/turing.jl
```

The API reference is built with Documenter.jl, from the folder of the package:

```
julia --project=docs -e 'using Pkg; Pkg.develop(path="."); Pkg.instantiate()'
julia --project=docs docs/make.jl
```
