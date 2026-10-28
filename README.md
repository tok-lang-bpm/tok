# tok

**tok** is a plain-text language for information-process models. One short text file describes a BPMN 2.0 process — pools, lanes, events, tasks, gateways, flows and data — together with the DMN decisions it uses (decision tables, and from language 0.7 complete decision requirements graphs with FEEL), the forms people fill in and the parameters needed to simulate it. The `tok` tool checks the model and turns it into everything else: BPMN 2.0 XML with an automatic diagram layout (opens in Camunda Modeler and bpmn.io), SVG pictures, DMN files, Camunda forms and simulation results.

```
tok 0.1 "Wypożyczenie międzybiblioteczne"

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
- **Made for people and for AI agents.** The whole language fits on one page (`tok cheatsheet`). An agent can write a model, run `tok check --json`, read the line-numbered diagnostics and fix them, without ever touching XML or coordinates.
- **Standards on the outside.** tok is a notation for BPMN 2.0 and DMN, not a replacement. Everything you write is exported to the OMG formats that modelling tools and process engines read; `tok import` converts existing BPMN and DMN files back to text.
- **Executable and measurable.** The same file can carry the details Camunda 8 needs to run the process, and the times, resources and probabilities a simulation needs to estimate cycle times and workloads.
- **Simple to implement.** A line-based grammar with six token types. Python, Rust and Julia implementations pass the same conformance corpus byte for byte.

## Install

Python 3.8 or newer, no other dependencies (on Windows use `py -m pip` if `pip` is not on the path):

```
pip install https://codeberg.org/MarekDeja/tok/archive/main.zip
tok --version
```

Without installation: `PYTHONPATH=python/src python -m tok …`.

Native build from source (Rust 1.99 or newer):

```
cd rust
cargo build --release -p tok-cli      # target/release/tok
```

It gives the same output as the Python command.

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

## Documentation

| Document | Contents |
|---|---|
| [docs/spec.md](docs/spec.md) | the language specification (normative) |
| [docs/cheatsheet.md](docs/cheatsheet.md) | the language on one page |
| [docs/cli.md](docs/cli.md) | command line reference |
| [docs/diagnostics.md](docs/diagnostics.md) | diagnostic codes in English and Polish |
| [docs/feel.md](docs/feel.md) | FEEL: the subset of languages 0.1–0.6 and the full FEEL of language 0.7 |
| [docs/dmn-tck.md](docs/dmn-tck.md) | DMN TCK conformance results |
| [docs/simulation.md](docs/simulation.md) | simulation semantics, distributions and outputs |
| [docs/import.md](docs/import.md) | converting BPMN and DMN XML to tok |
| [docs/roadmap.md](docs/roadmap.md) | plans from 0.1 to 1.0: full Camunda 8 and BPMN 2.0.2 coverage, Rust engine for Python and R, Julia standalone |
| [skills/tok-modelling](skills/tok-modelling/SKILL.md) | a skill that teaches an AI agent to model in tok |
| [examples/](examples/) | example models |

## Repository

| Folder | Contents |
|---|---|
| `python/` | reference implementation (package `tok-lang`, command `tok`) and its tests |
| `rust/` | Rust engine: `tok-core`, `tok-bpmn` (BPMN rules, layout, SVG), `tok-sim` (simulator), `tok-cli` (native `tok`), `tok-py` (Python bindings) |
| `r/tokr` | R package `tokr` on the Rust engine: diagnostics as data frames, diagrams in the viewer, knitr engine `{tok}` |
| `julia/Tok.jl` | Julia implementation of the core |
| `conformance/` | shared test corpus: input documents and expected outputs for every implementation |
| `docs/`, `examples/`, `skills/` | documentation, examples, agent skill |

`tools/check.sh` runs every check of a release (Rust, Python, R and Julia tests, engine wheel and its parity with the Python implementation, the native `tok` program) on Linux, macOS or Windows with Git Bash.

The BPMN validation comes from the `bpmn.py` validator used in the course *Modelowanie procesów informacyjnych* (Jagiellonian University), shipped here as `tok.bpmnlib`; its rule messages are in Polish and `tok` adds English equivalents. The diagram layout and SVG rendering started as a port of `bpmn.py` and, from 0.4, are developed in the Rust engine (`rust/tok-bpmn`), in the same style: left to right, pools stacked, lanes as rows.

## License

GNU General Public License v3.0 or later, see [LICENSE](LICENSE).
