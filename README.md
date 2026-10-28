# tok

**tok** is a plain-text language for information-process models. One short text file describes a BPMN 2.0 process — pools, lanes, events, tasks, gateways, flows and data — together with the DMN decisions it uses (decision tables, and from language 0.7 complete decision requirements graphs with FEEL), the forms people fill in and the parameters needed to simulate it. The `tok` tool checks the model and turns it into everything else: BPMN 2.0 XML with an automatic diagram layout (opens in Camunda Modeler and bpmn.io), SVG pictures, DMN files, Camunda forms and simulation results.

```
tok 1.0 "Wypożyczenie międzybiblioteczne"

pool biblioteka "Biblioteka uczelniana"
  lane czytelnik "Czytelnik"
    start potrzeba "Potrzeba książki"
    user zamowienie "Złożenie zamówienia"
  lane bibliotekarz "Bibliotekarz"
    user katalogi "Sprawdzenie katalogów"
    xor dostepna "Czy książka dostępna lokalnie?"
    user wskazanie "Wskazanie egzemplarza"
    send wyslanie "Zamówienie w innej bibliotece" message="Zamówienie"
    end wskazany "Egzemplarz wskazany"
    end zamowione "Zamówienie złożone"

pool inna "Biblioteka udostępniająca"

flow potrzeba -> zamowienie -> katalogi -> dostepna
flow dostepna -> wskazanie "tak"
flow dostepna -> wyslanie "nie"
flow wskazanie -> wskazany
flow wyslanie -> zamowione

message wyslanie -> inna "Zamówienie"
```

## Why a text language

- **Readable and reviewable.** Every line starts with a keyword and describes one element. Changes show up as clear diffs in version control.
- **Made for people and for AI agents.** The whole language fits on one page (`tok cheatsheet`). An agent can write a model, run `tok check --json` (or call the `check` tool of the MCP server `tok-mcp`), read the line-numbered diagnostics and fix them, without ever touching XML or coordinates.
- **Standards on the outside.** tok is a notation for BPMN 2.0 and DMN, not a replacement. Everything you write is exported to the OMG formats that modelling tools and process engines read; `tok import` converts existing BPMN and DMN files back to text.
- **Executable and measurable.** The same file can carry the details Camunda 8 needs to run the process, and the times, resources and probabilities a simulation needs to estimate cycle times and workloads.
- **Simple to implement.** A line-based grammar with six token types. Python, Rust and Julia implementations pass the same conformance corpus byte for byte.

## Install

The `tok` command is the Rust engine. It comes with the Python package `tok-lang` (Python 3.8 or newer; wheels for Windows x64, Linux x86_64 and aarch64, macOS; on Windows use `py -m pip` if `pip` is not on the path):

```
pip install tok-lang
tok --version
```

On a platform without a wheel pip builds the engine from the source archive, which needs Rust 1.99 or newer. The command alone, from crates.io or from a checkout of the repository:

```
cargo install tok-cli                 # the program tok
cd rust && cargo build --release -p tok-cli      # target/release/tok
```

Every way reads every version of the language, 0.1 to 1.0. The package also keeps the pure-Python implementation frozen at language 0.3, the reference of the parity tests: `TOK_ENGINE=python tok …` runs it (`docs/python.md`).

The R package `tokr` (`install.packages("tokr")`), the Julia package `Tok.jl` (below), the language server `tok-lsp` and the MCP server `tok-mcp` (`cargo install tok-lsp tok-mcp`) and the VS Code extension *tok* are described under Tools.

## Julia

`Tok.jl` gives the whole language to Julia through the C interface of the engine, keeps an independent implementation of the core of the specification, and adds a native simulator for large experiments and the calibration of simulation parameters on event logs:

```julia
using Pkg; Pkg.add("Tok")    # or Pkg.add(url="https://codeberg.org/MarekDeja/tok", subdir="julia/Tok.jl")
using Tok
text = read("model.tok", String)
Tok.Engine.check(text)                                  # the engine: every layer of the language
m = Tok.Sim.model(text)                                 # native simulation (julia -t auto)
e = Tok.Sim.experiment([m, Tok.Sim.scenario(m; name="+1", resources=Dict("staff" => 3))]; replications=200)
Tok.Sim.summarize(e)
f = Tok.Fit.fit(text, Tok.Fit.read_xes("log.xes"))      # calibration on an event log
Tok.Fit.apply(text, f)                                  # the document with the fitted parameters
```

The engine library is found through `TOK_LIBTOK`, the package `tok_jll`, or the repository build (`cargo build --release -p tok-capi`); see [docs/julia.md](docs/julia.md).

## Use

```
tok check model.tok              # diagnostics; --lang pl for Polish, --json for programs
tok fmt model.tok -w             # canonical form
tok bpmn model.tok               # model.bpmn with diagram layout
tok svg model.tok                # model.svg
tok export model.tok -o out/     # BPMN, SVG, DMN and forms (executable models for Camunda 8)
tok import diagram.bpmn          # BPMN 2.0 XML to tok
tok sim model.tok --seed 1       # simulation indicators; --log log.csv for the event log
tok eval model.tok DECISION a=1  # evaluate a decision
```

## Tools

| Tool | What it gives | Documentation |
|---|---|---|
| `tok` | the command line: check, format, export, import, simulate, evaluate | [docs/cli.md](docs/cli.md) |
| `tok-lsp` | language server (LSP 3.17) for any editor: diagnostics, formatting, go to definition, references, rename, outline, hover, completion, the diagram | [docs/lsp.md](docs/lsp.md) |
| VS Code extension | syntax highlighting, the language server, a live diagram preview, export of BPMN and every file | [editors/vscode](editors/vscode/README.md) |
| `tok-mcp` | MCP server for AI agents: the tools `check`, `fmt`, `export`, `import`, `sim`, `eval` and the language reference | [docs/mcp.md](docs/mcp.md) |
| playground | the engine in the browser (WebAssembly): editor, diagnostics, diagram, bpmn-js viewer, simulation, decisions, downloads | [docs/playground.md](docs/playground.md) |
| Python, R, Julia | `tok-lang` (Python), `tokr` (R), `Tok.jl` (Julia) on the same engine | [docs/julia.md](docs/julia.md), `r/tokr` |
| C library | `tokengine` with `tok.h`, for any language that calls C | [docs/c-api.md](docs/c-api.md) |

```
cd rust
cargo build --release -p tok-lsp -p tok-mcp      # target/release/tok-lsp, target/release/tok-mcp
bash tools/playground.sh                          # the playground in rust/target/playground
```

## Documentation

| Document | Contents |
|---|---|
| [docs/spec.md](docs/spec.md) | the language specification (normative) |
| [docs/cheatsheet.md](docs/cheatsheet.md) | the language on one page |
| [docs/cli.md](docs/cli.md) | command line reference |
| [docs/diagnostics.md](docs/diagnostics.md) | diagnostic codes in English and Polish |
| [docs/feel.md](docs/feel.md) | FEEL: the subset of languages 0.1–0.6 and the full FEEL of language 0.7 |
| [docs/dmn-tck.md](docs/dmn-tck.md) | DMN TCK conformance results |
| [docs/conformance-camunda.md](docs/conformance-camunda.md) | every BPMN, DMN and FEEL feature of Camunda 8.10, how tok expresses it, the case and the Camunda run that verify it |
| [docs/conformance-bpmn.md](docs/conformance-bpmn.md) | every element of the BPMN 2.0.2 and DMN 1.5 metamodels: tok construct, export, import, MIWG and DMN TCK coverage |
| [docs/simulation.md](docs/simulation.md) | simulation semantics, distributions and outputs |
| [docs/import.md](docs/import.md) | converting BPMN and DMN XML to tok |
| [docs/python.md](docs/python.md) | the Python module `tok.api` |
| [docs/julia.md](docs/julia.md) | Tok.jl: the engine in Julia, the independent core, native simulation, calibration |
| [docs/calibration.md](docs/calibration.md) | the `fit` layer: estimating simulation parameters from event logs |
| [docs/c-api.md](docs/c-api.md) | the C interface of the engine (library `tokengine`, header `tok.h`) |
| [docs/lsp.md](docs/lsp.md) | the language server `tok-lsp` and its use in VS Code, Neovim, Helix and Emacs |
| [docs/mcp.md](docs/mcp.md) | the MCP server `tok-mcp`: tools, resources, configuration of MCP clients |
| [docs/playground.md](docs/playground.md) | the browser playground on the WebAssembly build of the engine |
| [docs/fuzzing.md](docs/fuzzing.md) | the fuzz targets of the engine, how to run them, what they found |
| [docs/performance.md](docs/performance.md) | benchmarks on large generated models and their results |
| [docs/design-1.0.md](docs/design-1.0.md) | language 1.0: decisions, deprecations, the compatibility of 1.x (earlier versions: `design-0.3.md` to `design-0.7.md`) |
| [docs/roadmap.md](docs/roadmap.md) | plans from 0.1 to 1.0: full Camunda 8 and BPMN 2.0.2 coverage, Rust engine for Python and R, Julia standalone |
| [skills/tok-modelling](skills/tok-modelling/SKILL.md) | a skill that teaches an AI agent to model in tok |
| [examples/](examples/) | example models |

## Repository

| Folder | Contents |
|---|---|
| `python/` | reference implementation (package `tok-lang`, command `tok`) and its tests |
| `rust/` | Rust engine: `tok-core`, `tok-bpmn` (BPMN rules, layout, SVG), `tok-sim` (simulator), `tok-cli` (native `tok`), `tok-py` (Python bindings), `tok-capi` (C interface), `tok-lsp` (language server), `tok-mcp` (MCP server), `tok-wasm` (WebAssembly build), `tok-bench` (generated large models and benchmarks); `rust/fuzz` holds the fuzz targets (a cargo workspace of its own) |
| `editors/vscode` | VS Code extension: grammar, language client, diagram preview |
| `playground/` | the page of the browser playground |
| `r/tokr` | R package `tokr` on the Rust engine: diagnostics as data frames, diagrams in the viewer, knitr engine `{tok}` |
| `julia/Tok.jl` | Julia package: the engine through the C interface, the independent core, native simulation and calibration; `julia/app` (the `tok` application), `julia/binarybuilder` (the recipe of `tok_jll`) |
| `conformance/` | shared test corpus: input documents and expected outputs for every implementation |
| `docs/`, `examples/`, `skills/` | documentation, examples, agent skill |

`tools/check.sh` runs every check of a release (Rust, Python, R and Julia tests, engine wheel and its parity with the Python implementation, the native `tok` program, the C library of the engine, the language and MCP servers, the VS Code extension and the playground) on Linux, macOS or Windows with Git Bash.

The BPMN validation comes from the `bpmn.py` validator used in the course *Modelowanie procesów informacyjnych* (Jagiellonian University), shipped here as `tok.bpmnlib`; its rule messages are in Polish and `tok` adds English equivalents. The diagram layout and SVG rendering started as a port of `bpmn.py` and, from 0.4, are developed in the Rust engine (`rust/tok-bpmn`), in the same style: left to right, pools stacked, lanes as rows.

## License

GNU General Public License v3.0 or later, see [LICENSE](LICENSE).
