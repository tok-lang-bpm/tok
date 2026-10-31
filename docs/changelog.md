# Changelog

All notable changes to the tok language and its reference tools are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [Semantic Versioning](https://semver.org/). Until 1.0 a minor version could change the language; from 1.0 the compatibility promise of the specification (`docs/spec.md` §4.3) holds for every release 1.x, and a patch version never changes the language.

## [Unreleased]

Changes to the agent skills only (`skills/`); the programs and the language are those of 1.0.3. The agent bundle https://github.com/tok-lang-bpm/tok-agent carries them as plugin version 1.0.3.1.

### Changed

- The agent skill `tok-modelling` has a reference policy and a shorter workflow, from the third evidence study of tok (four language models as agents on 87 process descriptions of two datasets, with the skill and the MCP server). Read the cheat sheet once and nothing else before the first check; open the specification only for the one section a diagnostic needs; write the model, check it, fix every error and every warning, format, export, report; the mapping table is written when the description is a numbered table of steps or the person asks for a review of the choices. The sections on the layers of the language (the core language, executable models, BPMN beyond Camunda, collaboration and choreography, models from BPMN or DMN files, decision models, simulation) moved to `references/layers.md`. With the skill of 1.0.3 the agents read the whole reference through the tool `reference` and carried it in every later turn, at 4.4 to 6.5 times the tokens of a run from the one-page reference; the new workflow keeps their validity (97.6–100 %) at 36–60 % of those tokens.
- The agent skill `bpmn-modelling` fixes every warning as well as every error (BPMN linters such as bpmnlint report the `W…` rules as errors; the previous text let an agent justify a warning instead, and the models left `W022`), and gives a BPMN file to the MCP tool `check_bpmn` as `path`, with `xml` for clients without file tools. The tool calls of one of the studied models (Beam, through Ollama) arrive without an argument named `xml`; a later release of the MCP server may accept `text` as well.

## [1.0.3] — 2026-10-30

A release of the checker and the command line, from the second evidence study of tok (language models of other makers writing process models with the checker as their feedback, and a scan of every rule of the BPMN linter of bpmn.io against the checker); the language stays 1.0. Outputs change only where listed below: the conformance case 03-xor gets the new warning `W044`, and the cases 36, 44, 62 and 67 the new warning `W047`.

### Added

- Four warnings of the BPMN rules, for what bpmnlint reports and the checker did not (`docs/bpmn-rules.md`); with them the checker reports every rule of bpmnlint that concerns the model rather than its style:
  - `W044` — an activity or an event with several outgoing sequence flows without conditions: all of them are taken (an implicit split), which a parallel gateway states explicitly (bpmnlint `no-implicit-split`);
  - `W045`, `W046` — an embedded sub-process or a transaction without its own start or end event; BPMN 2.0.2 allows them to be left out, bpmnlint does not (`start-event-required`, `end-event-required`);
  - `W047` — an event with several triggers (a multiple event): valid BPMN, reported as an error by bpmnlint (`single-event-definition`); keep it when the process needs it.
  They are warnings, so that every valid document stays valid. The pure-Python reference `tok.bpmnlib` (1.4) has the same rules.
- The agent skill `tok-modelling` explains the new warnings.
- `tok check FILE.bpmn` checks a BPMN 2.0 XML file, from tok or from any modelling tool, with the BPMN rules of the checker, and reports each finding with the line of its element; `--steps` and `--steps-from DESCRIPTION.md` check the step numbers of a process description (`K01`–`K03`), `--camunda` the Camunda 8 profile of executable models (`C…`). `tok bpmn FILE.bpmn` lays out a BPMN file and `tok svg FILE.bpmn` draws it. These are the commands of the course tool `bpmn.py`, run by the engine, so that agents and courses need one program; the codes of BPMN files are listed in `docs/diagnostics.md`.
- The MCP server has the tool `check_bpmn`, the same check of BPMN 2.0 XML for agents (`docs/mcp.md`).

## [1.0.2] — 2026-10-29

A release of the checker and the exporters, from the findings of the first evidence study of tok (fault injection, and language models writing process models with the checker as their feedback); the language stays 1.0. Outputs change only where listed below: the conformance case 03-xor gets the new warning `W023`, and the exports of cases 45 and 62 carry the documentation of the header in the collaboration.

### Added

- Three warnings of the BPMN rules, for what the BPMN linter of bpmn.io (bpmnlint) reports and the checker did not (`docs/bpmn-rules.md`):
  - `W022` — a branch without a condition at an element that splits conditionally (it has a default flow, or other branches have conditions): BPMN takes the default only when no condition is true, so the branch needs one (`when="…"`, or `when=""` while it is not known);
  - `W023` — a sequence flow that repeats another one (same source, target and condition);
  - `W043` — a sequence flow from an element back to itself.
  They are warnings, so that every valid document stays valid (the compatibility promise of 1.x). The course tool `bpmn.py` (1.3) has the same rules, as before.
- The agent skill `tok-modelling` fixes warnings as well as errors and explains the warnings of the BPMN rules.

### Fixed

- The documentation of the header (`doc=` on the first line) of a document with pools was written into `bpmn:definitions`, which has no documentation in BPMN 2.0.2: the file was not valid against the XML Schema of the OMG. It is written into the collaboration; the import reads it from there, and from the definitions in files of earlier releases.
- In documents of language 0.7 and 1.0, a condition with two comparison operators in a row (`age >= > 18`) was accepted; it is the syntax error `F002`, as in documents of 0.1–0.6.
- `P005` and `P006` named version 0.1 only: they name `tok 1.0` and the versions the engine reads (0.1, 0.3, 0.5, 0.6, 0.7 and 1.0). The pure-Python implementation keeps its messages (it reads 0.1 and 0.3).
- `tok bpmn`, `tok svg`, `tok export` and `tok import` with `-o` into a folder that does not exist failed; the folder is made.
- Camunda forms carry the language version of the document as the version of the exporter, as the BPMN and DMN files do (documents of 0.1 and 0.3 keep 0.1.0, the output of the pure-Python implementation).

### Changed

- The generator of the simulation fixtures (`rust/tok-sim/tests/gen_python_sim.py`) takes the documents of an existing fixture, so that a regeneration on another platform gives the same runs; `TOK_SIM_ALL=1` takes every document.
- The comment of `julia/binarybuilder/build_tarballs.jl` names the repository on GitHub.

## [1.0.1] — 2026-10-29

A release of the tools and the packages; the language stays 1.0 and every output for the conformance cases, the examples, the MIWG models and the DMN TCK stays as it was.

### Added

- `tok mcp`: the Python package runs the MCP server of the engine (`tok mcp`, `tok mcp --version`), so `pip install tok-lang` gives the language, the command line and the server for AI agents in one installation, and `uvx tok-lang mcp` runs the server without one. The package also has the command `tok-lang`, the same as `tok`. `server.json` registers the server in the MCP Registry as `io.github.tok-lang-bpm/tok` (`docs/mcp.md`).
- The catalogue of BPMN symbols, `docs/symbols.md` and `docs/symbols/`: every symbol the engine draws (events by trigger and position, activities, markers, gateways, data, connections, pools and lanes, conversations, choreographies), with its BPMN element, the tok statement that produces it, its execution on Camunda 8, and short descriptions in English and Polish (`symbols.json`). `tools/symbols.py` draws them from small models with `tok bpmn` and `tok svg`, so they are exactly what the engine draws.
- The example `examples/interlibrary-loan.tok` (the model of the README).
- `CONTRIBUTING.md`, `SECURITY.md` and issue forms for the GitHub repository.

### Fixed

- Playground: the tabs *Decisions* and *Files* and the import of BPMN and DMN files failed, because the WebAssembly module exports the functions `export`, `import` and `eval` as `_export`, `_import` and `_eval` (reserved words of JavaScript) and the page called them without the underscore. `docs/playground.md` gives the exported names.

### Changed

- The project home is the GitHub organisation `tok-lang-bpm`: the repository https://github.com/tok-lang-bpm/tok with its issues, the guide and the playground at https://tok-lang-bpm.github.io/. The metadata of every package points there; the VS Code extension is published as `tok-lang-bpm.tok`. Development continues in the Codeberg repository, which is mirrored to GitHub.
- The README is organised by what the reader wants to do, with an example in English and links that work on PyPI as well.
- The R package `tokr`: every exported function has examples, the title and description of `DESCRIPTION` follow the conventions of CRAN, and `NEWS.md` lists the changes of the package; it is also built by R-universe (https://tok-lang-bpm.r-universe.dev).
- The publishing sections of `editors/vscode/README.md`, `julia/binarybuilder/README.md` and `docs/playground.md` describe where users get the extension, `tok_jll` and the playground.

## [1.0.0] — 2026-10-28

The stable release: language 1.0 (`tok 1.0` in the header) — the content of language 0.7 with its limits as rules, the deprecation `R090` and the compatibility promise of 1.x — the hardening of the engine (fuzzing, performance, the documentation checked against the sources, conformance tables for Camunda 8 and BPMN 2.0.2), and the packages prepared for their registries: PyPI, crates.io, CRAN, the Julia General registry, the VS Code Marketplace and Open VSX, Zenodo. Documents of 0.1, 0.3, 0.5, 0.6 and 0.7 stay valid and mean the same; every output of 0.9.0 for the conformance cases 01–61, the examples, the MIWG models and the DMN TCK stays as it was, and outputs change only where 0.9.0 crashed or wrote a file that is wrong or does not read back (below, Fixed). Reference: `docs/spec.md` §2.1, §4.1–§4.3; design notes and the decisions taken during implementation in `docs/design-1.0.md`; `docs/fuzzing.md`, `docs/performance.md`, `docs/conformance-camunda.md`, `docs/conformance-bpmn.md`, `docs/python.md`.

### Added

- Language 1.0: the header `tok 1.0` next to `0.1`, `0.3`, `0.5`, `0.6` and `0.7`. A 1.0 document is read, checked, printed, exported, evaluated and simulated as the same document under `tok 0.7`; its BPMN and DMN files carry `exporterVersion="1.0"`, and its decisions are DMN 1.5. The specification is version 1.0: the bounds of nesting are rules (§2.1: blocks 64 levels, FEEL 160 levels, imported BPMN and DMN files 64 levels, XML of the BPMN tools 256 levels), and §4.3 states what every release 1.x keeps — every valid 1.0 document stays valid and prints identically, outputs change only for documented bug fixes, new revisions of Camunda 8 and of DMN come in minor releases with a new language version in the header.
- Deprecation warning `R090` in 1.0 documents: an attribute key or a flag given more than once on one line, of which only the last value has ever counted (the triggers of events and the errors of operations, which may repeat, are not reported). Documents of earlier versions get no warning. The review of the language for 1.0 and why the other names and forms stay is in `docs/design-1.0.md` §4.
- Conformance cases 62–67 of language 1.0: a document with the elements of every layer, an executable document for Camunda 8 with a process, forms and decisions, the errors of the header and the version (`P005`, `P006`, `P010`), and `R090`. `conformance/camunda/run.py` runs the process of case 63, exported with the native `tok`, on Camunda 8 Run and evaluates its decisions, with the process cases first and the decision cases last. The Julia core reads `tok 1.0` and passes the core level for the new cases; the pure-Python implementation, frozen at language 0.3, reports a 1.0 header as an unknown version, as it reports 0.5–0.7.
- The example `examples/limit-wypozyczen.tok`: a process of language 1.0 with the decision requirements graph its business rule task uses, and simulation parameters.
- Packages for the registries. Python: `pyproject.toml` at the root builds `tok-lang` with maturin, so `pip install tok-lang` installs the engine — abi3 wheels for Windows x64, Linux x86_64 and aarch64 (manylinux2014) and macOS universal2, and a source archive that builds the engine with Rust; the pure-Python implementation of language 0.3 stays in the package as the reference of the parity tests (`TOK_ENGINE=python`, `docs/python.md`). crates.io: `tok-core`, `tok-bpmn`, `tok-sim`, `tok-cli` (the program `tok`; the crate name `tok` is taken), `tok-lsp` and `tok-mcp`, with descriptions, keywords, categories, READMEs and versioned dependencies; `tok-cli` carries copies of the reference documents (`rust/tok-cli/reference`), which `tok cheatsheet` and the MCP server use and the test suite compares with `docs/`. CRAN: the authors and licences of the bundled crates in `inst/AUTHORS` of the archive, software names quoted in `DESCRIPTION`. Julia: the compat entries of `Tok.jl` for the General registry, the recipe of `tok_jll` 1.0.0. `CITATION.cff` and `.zenodo.json` for citation and the Zenodo archive.
- Hosted continuous integration for a GitHub mirror: `.github/workflows/ci.yml` runs `tools/check.sh` on Linux, Windows and macOS and keeps the wheels, programs and `.vsix` as artifacts; `.github/workflows/release.yml` builds the release artifacts of a tag (wheels, the source archive of tok-lang, the programs and the C library, the `.vsix` files, the archive of tokr, the playground), creates the GitHub release and publishes to PyPI, crates.io, the VS Code Marketplace and Open VSX only when their secrets are set. The workflows run only on GitHub.
- `tools/check.sh` builds the source archive of tok-lang, checks the metadata of the Python packages with `twine check --strict`, and packages the six published crates with `cargo publish --dry-run`.

- Fuzz targets (`rust/fuzz`, cargo-fuzz with libFuzzer, a cargo workspace of its own): the lexer, parser and checker at every level and conformance class with the BPMN rules; the canonical printer (parse → print → parse → print is a fixed point and keeps what the document means); FEEL parsing and evaluation of languages 0.1–0.6 and of language 0.7; BPMN import with the layout sidecar; DMN import; the layout sidecar reader with layout and SVG; export with layout, SVG and every file; the simulator. `rust/fuzz/seeds.py` builds the seed corpora and dictionaries from the conformance cases, the examples, the MIWG models and the DMN TCK. Every input a target failed on is a regression test of the normal test suite (`rust/tok-cli/tests/fuzz_regressions.rs`), run on a stack of the size of a Windows main thread.
- Benchmarks (`rust/tok-bench`): generators of large valid models (processes of 10 000 elements, collaborations of many pools and lanes, deeply nested sub-processes, decision tables of 10 000 rules, decision requirements graphs, long FEEL expressions; `cargo run -p tok-bench --example generate`) and `cargo bench -p tok-bench` for parsing, checking, formatting, export, layout, SVG, import, decision evaluation and simulation. Results in `docs/performance.md`.
- Documentation checks (`rust/tok-cli/tests/docs.rs`): the test suite fails when a diagnostic code, keyword, attribute key or flag, command or option of the command line, function of the C interface, of `tok.api` or of the R package, tool of the MCP server or method of the language server is not documented, when a keyword, attribute or flag has no example in the conformance cases or the examples, or when a copy of a reference document differs from `docs/`.
- `docs/conformance-camunda.md`: every BPMN, DMN and FEEL feature Camunda 8.10 executes, how tok expresses it, the conformance case and the Camunda 8 Run case that verify it, and the elements Camunda does not execute with their codes. `docs/conformance-bpmn.md`: every element class of the BPMN 2.0.2 and DMN 1.5 metamodels with its tok construct, export, import, MIWG and DMN TCK coverage, and the gaps.
- `docs/python.md`: the functions of the Python module `tok.api`. `docs/diagnostics.md` lists the BPMN rules `E`, `W`, `L` with their English texts and the messages of `C022`, `C090` and `C091`; `docs/cli.md` gives every option of every command, the meaning of `--json` and the exit codes; `docs/lsp.md` names every method of the language server.

### Changed

- The VS Code extension is bundled with esbuild 0.28.2 into one file, `out/extension.js`: the `.vsix` holds 9 files instead of 234 and no `node_modules`; the smoke test checks it.
- The automatic layout of large models is much faster, with the same drawing: the obstacles of a route, and the shapes, routes and labels near a route or a label, are found through grids of their boxes instead of a scan of the whole diagram, and the barycentre sweeps group the nodes of each column once. A process of 10 000 elements is laid out in about 1.3 s instead of 93 s, one of 1 000 elements in 0.09 s instead of 0.66 s.
- The cycle check of decision requirements graphs (`R076`) runs once over the strongly connected components instead of a search from every element: 23 ms instead of 0.94 s for a graph of 500 decisions; the reported cycles are the same.
- A FEEL iteration (`for`) copies the results so far (`partial`) only when its body reads `partial`, so an iteration takes linear time.
- The cheat sheet shows `candidate-users=`, `follow-up=`, `business-id=` and `code=`, and the specification, the cheat sheet, the skill `tok-modelling`, the playground and the descriptions of the MCP server name language 1.0 as the newest.
- `TOK_BLESS=1 cargo test -p tok-core --test conformance` writes the expected files of every case the Python generator skips, by the same rule as `conformance/generate.py`, so that a case whose header is not a version (66) has expected files.
- `tools/wheel.sh` runs `maturin build` in the root of the repository; the separate `pyproject.toml` of `rust/tok-py` and the copy of the Python package next to it are gone. Installing the source archive of the repository builds the engine with Rust instead of installing the pure-Python implementation.
- The R package unpacks its vendored crates with R's own `untar`, so its installation needs no `xz` program.

### Fixed

- Deep nesting no longer overflows the stack: blocks of a document nested more than 64 levels deep are `P008`; FEEL expressions nested more than 160 levels deep (parentheses, lists, contexts, calls, `if`, unary minus, type expressions, and long operator chains such as `1 + 1 + …`) are `F002` ("the expression is nested too deeply"); BPMN and DMN files whose elements are nested more than 64 levels deep are refused by the import, and the BPMN tools refuse XML nested more than 256 levels deep. Before, each of these crashed the program.
- FEEL of language 0.7: `for`, `some` or `every` not followed by a name (`for[`, `some""`) recursed without end; it is a syntax error now.
- FEEL of language 0.7: `time("…")`, `@"…"` and the other readers of times panicked on text with characters outside ASCII where the hours, minutes or seconds stand; such text is not a time (`null`).
- FEEL of languages 0.1–0.6: a nested target of `in` (`a in [b in [c in …]]`) was parsed a number of times exponential in its depth.
- BPMN import: an element without an identifier, or with the identifier of an element before it, gave text that does not parse (an empty identifier, `on=None`), and a container sharing its identifier with its content made the export recurse without end; such elements get identifiers of their own, and the import reports them. A message flow without both ends is dropped and reported instead of giving a line that does not parse.
- The canonical form of a decision table without input columns or without output columns left an empty cell on that side, which reads back as a column without a name (`D009`); the empty side is left out now (`|| out: number |`, `| in: number ||`).
- BPMN export: in a document with groups, the conversations of the collaboration were written into the `category` of the groups, because the category was inserted at the start of the definitions while the later steps of the export still wrote into the collaboration and the processes by their positions; the category is now placed with the declarations at the end. No conformance case, example or MIWG model had groups together with conversations, so no expected file changes; the Julia core already placed them correctly.
- Language server: renaming an element of a decision requirements graph also renamed the first word of a longer FEEL name (`kara` in the entry `kara za tydzień`) and the keys of contexts (`{kara: …}`), and renaming a DMN type did not rename it in the header of a decision table; FEEL names of several words and context keys are no longer references, and the column types of tables are.
- DMN import: a decision table or relation with a cell tok cannot write in a row (an output name or input expression with `=`, a cell with `|` or `#` outside a string, a type with a prefix) gave text that reads back differently; it is not kept and is reported.

## [0.9.0] — 2026-10-28

Tooling of 0.9: a language server, a VS Code extension, an MCP server and a browser playground, all on the engine itself. The language does not change (language 0.7); every output of 0.8.0 stays as it was. Reference: `docs/lsp.md`, `docs/mcp.md`, `docs/playground.md`, `editors/vscode/README.md`.

### Added

- Language server `tok-lsp` (crate `rust/tok-lsp`, LSP 3.17 over standard input and output): diagnostics of every layer of the checker on open and on every change, in English or Polish (locale of the editor or the setting `lang`), with the teaching level and the BPMN conformance class as settings; formatting to the canonical form, refused for a document with syntax errors; go to definition, find references, highlight and rename of identifiers — the node names of flows and message flows, the ends of associations, the attributes that name elements, and in language 0.7 the FEEL names of a decision requirements graph in the expressions that see them — with a rename checked as an identifier and against the identifiers in use, and the derived identifiers of flows following their ends; outline; hover for elements, keywords and diagnostic codes; completion of keywords, identifiers and attributes; positions in UTF-16 code units. The requests `tok/check`, `tok/svg`, `tok/bpmn` and `tok/export` give editor extensions the results of `tok check`, `tok svg`, `tok bpmn` and `tok export`, with the layout sidecar of the file. Configuration for Neovim, Helix and Emacs (eglot) in `docs/lsp.md`.
- VS Code extension (`editors/vscode`): TextMate grammar of `.tok` files, language configuration (comments, brackets, folding by indentation), the language client of the bundled `tok-lsp` (or `tok.server.path`, or `tok-lsp` on the `PATH`), the settings `tok.lang`, `tok.level` and `tok.class`, and the commands *tok: Show diagram* (a live SVG preview beside the editor), *tok: Export BPMN*, *tok: Export all files*, *tok: Check* and *tok: Restart language server*. `npm run package` writes a `.vsix` per platform; grammar tests over the constructs of the language and over every example and conformance document, a smoke test of the packaged extension without a display, and a test inside VS Code.
- MCP server `tok-mcp` (crate `rust/tok-mcp`, stdio transport) for AI agents: the tools `check`, `fmt`, `export` (BPMN, SVG, DMN, forms, returning the contents of the files), `import` (BPMN and DMN to tok, with the layout sidecar), `sim`, `eval` and `reference`, with exact input and output schemas and errors of a tool kept apart from errors of the protocol; the language reference also as resources `tok://reference/…`. It speaks MCP revision 2026-07-28 (per-request metadata, `server/discover`) and the handshake revisions 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05.
- Browser playground: the crate `rust/tok-wasm` compiles the engine to WebAssembly with wasm-bindgen 0.2.129 (`check`, `fmt`, `bpmn`, `svg`, `export`, `import`, `eval`, `sim`, with the options and results of the C interface); the page in `playground/` has an editor with live diagnostics, the diagram, the bpmn-js 18.31.0 viewer, simulation, decisions, downloads of every exported file, import of BPMN and DMN files and links that carry the document. `tools/playground.sh` builds it as a static site; its tests run inside the WebAssembly module under Node.js.
- Documentation: `docs/lsp.md`, `docs/mcp.md`, `docs/playground.md`, `editors/vscode/README.md`; the skill `skills/tok-modelling` covers the whole of language 0.7 and uses the MCP server when the agent has it, the `tok` command otherwise.

### Changed

- The functions of the C interface on strings (`tok_cli::api`, formerly a private module of `tok-capi`) are now a public module of `tok-cli`, shared by the C interface, the language server, the MCP server and the WebAssembly build; the C interface is unchanged.
- The lexer records the column of every token (`Line::spans`), which the language server uses; the parser and every output are unchanged.
- FEEL `now()` and `today()` read a clock that a host can set (`tok_core::pytime::set_clock`); the WebAssembly build sets the clock of the page, where the system clock is not available. Elsewhere they read the system clock as before.
- The installation section of the README says which way of installing gives the engine (the wheel of `tok-lang`, the native build) and that the source archive without a wheel installs the pure-Python implementation of language 0.3.
- `tools/check.sh` also builds `tok-lsp` and `tok-mcp` into `dist/`, builds, tests, packages and smoke-tests the VS Code extension when npm is available (the `.vsix` goes to `dist/`), and builds and tests the playground when the target `wasm32-unknown-unknown` and the matching `wasm-bindgen` command are installed; otherwise it says why it skips them.

## [0.8.0] — 2026-10-28

Julia on the Rust engine: a C interface of the engine, `Tok.jl` with the whole language through it, a native simulator for large experiments checked statistically against the engine's, and the calibration of simulation parameters on event logs. The language does not change (language 0.7); every output of 0.7.0 stays as it was. Reference: `docs/c-api.md`, `docs/julia.md`, `docs/calibration.md`, `docs/simulation.md` (native simulation in Julia).

### Added

- C interface of the engine: the crate `rust/tok-capi` builds the shared and static library `tokengine` (`libtokengine.so`, `libtokengine.dylib`, `tokengine.dll`) with the header `rust/tok-capi/include/tok.h`. Every function takes UTF-8 strings (the text and the options as a JSON object) and returns a JSON object `{"ok": …}` or `{"error": {"kind", "message"}}`, freed with `tok_free`: `tok_check`, `tok_fmt`, `tok_bpmn`, `tok_svg`, `tok_export`, `tok_import` (BPMN and DMN, with the layout sidecar), `tok_eval`, `tok_sim` (indicators, report, event log, cases, CSV and XES on request), `tok_sim_model` (the simulation model as data), `tok_sim_dist`, `tok_sim_eval` (expressions as the simulator evaluates them), `tok_cli` (the command line), `tok_version` and `tok_abi_version` (1). No panic crosses the boundary; the functions are reentrant. `tools/check.sh` builds the library and copies it with the header to `dist/`; the tests of the crate call every function through its C signature and, on Unix systems with a C compiler, build and run a C program.
- `Tok.Engine`: the whole language in Julia through the C interface (`ccall`) — `check`, `fmt`, `bpmn`, `svg`, `export_files`, `import_bpmn`, `import_dmn`, `import_file`, `eval_decision`, `simulate`, `sim_xes`, `cli` — returning Julia values, with the errors of the engine as `TokError`. The library is found through `TOK_LIBTOK`, the binary package `tok_jll` when it is installed, or the build directory of the repository. The tests check that the engine and the independent core print every case of the conformance corpus identically.
- BinaryBuilder recipe of `tok_jll` (`julia/binarybuilder`): Linux x86_64 and aarch64 (glibc and musl), macOS x86_64 and aarch64, Windows x86_64, with the steps of its submission to Yggdrasil.
- `Tok.Sim`, a native discrete-event simulator in Julia for the models the engine reads, with the semantics of the engine's simulator for every element it runs (events, activities with resources, durations and assignments, gateways with conditions and probabilities, event-based gateways, timers and delays, boundary events, sub-processes); models with other features are reported (`UnsupportedModel`) and run in the engine. Scenarios (`scenario`: resource counts, durations, probabilities, arrivals), experiments with many replications in parallel on the threads of Julia with one reproducible random stream per replication and common random numbers across scenarios, indicators in long form and summaries with confidence intervals.
- Statistical check of the native simulator: on the example and on the models of the simulation fixtures it runs, 200 replications of each simulator are compared with two-sample Kolmogorov–Smirnov tests (mean cycle time, cycle time of a case, utilization of every resource; α = 0.01 per model, Bonferroni), with fixed seeds; a model with 10 % longer durations is rejected.
- Calibration, the `fit` layer (`Tok.Fit`): processing times of activities (with interrupted instances as censored observations), times between arrivals and branch probabilities of `xor` and `or` gateways, estimated from XES logs and from the CSV logs of `tok sim`; maximum-likelihood estimates with confidence intervals (Wald, delta method, Wilson, parametric bootstrap for `unif` and `tri`); Bayesian estimates with Turing.jl in the package extension `TokTuringExt` (Turing is a weak dependency); `apply` writes the estimates into the document. The tests simulate models with known parameters and recover them from the logs.
- `tok` application for Julia users (`julia/app`, built with PackageCompiler): the commands of the `tok` program, `tok experiment` (native simulation of scenarios), `tok fit` (calibration) and `tok core` (the independent core); the same commands as `Tok.app_main`.
- Documentation: `docs/c-api.md`, `docs/julia.md`, `docs/calibration.md`, and the native simulation in `docs/simulation.md`.

### Changed

- The command line of the engine reports an unreadable input file with exit status 2 as before, now by returning the status instead of ending the process, so that `tok_cli` and the Python binding can run it inside another program.
- `tools/check.sh` builds the C library before the Julia step and runs the tests of Tok.jl with it, on two threads.

## [0.7.0] — 2026-10-28

Language 0.7 (`tok 0.7` in the header; documents written for 0.1, 0.3, 0.5 and 0.6 stay valid, mean the same and keep every output): decision models of DMN 1.5 — decision requirements graphs, boxed expressions, FEEL at conformance level 3 — with evaluation, DMN 1.5 export with diagrams and DMN import; the DMN TCK as a conformance gate. Reference: `docs/spec.md` §4, §9, §10.4, §11–§15; `docs/feel.md` §9; design notes and the decisions taken during implementation in `docs/design-0.7.md`.

### Added

- Decision requirements graphs: `inputdata`, `source` (knowledge source: `kind=`, `uri=`) and `bkm` (business knowledge model: `params=`, `type=`) statements; `requires="ID …"` on decisions, business knowledge models and knowledge sources gives information, knowledge and authority requirements by the kind of the element named; `name=` for FEEL names with spaces; `question=` and `answers=` on decisions; `type=` on every element.
- Boxed expressions in the block or on the line of a decision, a business knowledge model and each line: literal expressions (`expr=`), contexts (`entry NAME …` lines, optionally ending with `result …`), lists (`element …` lines), relations (table rows without `||`), invocations (`invoke=BKM with="param = FEEL; …"`) and function definitions (`params=` on a line); every line may hold its own nested box.
- Decision tables of 0.7: hit policies `rule-order` and `output-order`, `aggregation=sum|count|min|max` with `collect`, input and output values in the header (`name: type = "a", "b"`; `priority` and `output-order` order the rules by them), default outputs (`default=`), every FEEL type and DMN type in the header.
- DMN types: in a 0.7 document `item ID type= allowed= [collection]` with a block of components (`item NAME type=`), exported as `itemDefinition`s; values that do not conform to the type of an element are `null`, after the implicit conversions of DMN 1.5.
- FEEL at conformance level 3 for every expression of a 0.7 document (`docs/feel.md` §9): temporal values with their arithmetic and comparison, the standard function library of DMN 1.5 (string, list, numeric, boolean, temporal, range and context functions, regular expressions of XPath), contexts, lists, ranges, filters, paths, `for`, `some`, `every`, `instance of`, function definitions, named arguments, names with spaces resolved against the document, numbers with 34 significant digits.
- `tok eval` evaluates a decision of a 0.7 document with the decisions, input data and business knowledge models it requires and prints its DMN value; values on the command line may be JSON objects and lists, and text for input data of a temporal type is read as that type. `tok sim` evaluates the business rule tasks and the expressions of 0.7 documents the same way; `tok.api.eval_decision` (Python) and `tok_eval` (R) evaluate decision requirements graphs.
- DMN 1.5 export: `tok export` writes one DMN file per connected decision requirements graph of a 0.7 document (`NAME.dmn`, named after its first decision) with its DMN types and a DMNDI diagram laid out automatically, and its picture `NAME.drd.svg`; a decision model without a process needs no BPMN file. The Python `export` and the R `tok_export` return the same files.
- DMN import: `tok import FILE.dmn` reads DMN 1.1–1.5 into a 0.7 document — decisions with their logic, input data, business knowledge models, knowledge sources, requirements and item definitions; the boxed `conditional`, `filter`, `for`, `some` and `every` of DMN 1.5 become literal expressions; everything not kept is reported.
- Checker codes `R076`–`R081` (cycles of requirements, boxed expression forms, parameters and bindings, names used without a requirement, duplicate names, types), `D006`–`D009` (output values, aggregations, relation rows, header cells of 0.7) and `C130`–`C132` (business knowledge models with an invocation, function definitions with a body that is not a literal expression, decisions without logic in executable documents for Camunda); the levels `basic` and `analytic` reject the new statements (`R040`).
- DMN TCK conformance gate (`rust/tok-core/tests/dmn_tck.rs`): 143 models of compliance levels 2 and 3 of the DMN Technology Compatibility Kit (Apache-2.0) in `conformance/dmn-tck`, imported, printed, parsed again and evaluated for every test case; 126 of 126 results of level 2 and 3313 of 3331 results of level 3 pass, the others are listed with their reasons in the baseline. Pass counts per test in `docs/dmn-tck.md`.
- Conformance cases 53–61 (decision requirements graphs, boxed expressions and types, the decision tables of 0.7, FEEL of level 3 in a process, the Camunda codes `C130`–`C132`, the errors of 0.7, the statements of 0.7 under an older header, relation rows and header cells, an executable decision model for Camunda 8) with the DMN files of every graph among the expected outputs; the Julia core parses, prints and exports them (core level, 240 checks).
- DMN files of executable documents for Camunda 8 write FEEL names with spaces or symbols in backticks, the form Camunda's FEEL reads; the executable conformance runner `conformance/camunda/run.py` deploys the decisions of case 61 to Camunda 8.10 and compares their evaluation with `tok eval` (names of several words, a dash, a filter over a component of two words, an invocation, a context and a list).

### Changed

- The decimal arithmetic of the engine rounds to a precision set per computation: 28 digits, as before, for documents of earlier languages, 34 digits for FEEL of language 0.7.
- The cheat sheet printed by `tok cheatsheet` describes language 0.7.
- The DMN export of documents of earlier languages is unchanged (DMN 1.3, one file per decision table).

## [0.6.0] — 2026-10-28

Language 0.6 (`tok 0.6` in the header; documents written for 0.1, 0.3 and 0.5 stay valid and mean the same): conversations, choreographies and participant multiplicity; BPMN conformance classes in the checker; a layout sidecar that keeps the drawing of imported files. Reference: `docs/spec.md` §4, §5.5, §5.7, §6.4, §10.4, §11, §13–§16; design notes and the decisions taken during implementation in `docs/design-0.6.md`.

### Added

- Conversations: `conversation`, `subconversation` (with a block of conversation nodes) and `callconversation … collaboration=ID` with `pools=`, `messages=` and `map="inner:outer; …"` (BPMN `conversation`, `subConversation`, `callConversation` with participant and message flow references, participant associations, and one `conversationLink` per pool). Drawn as hexagons in the gap between the linked pools (a thick border for a call, a `+` for a sub-conversation), with double lines to the pools.
- Choreographies: `choreography ID` blocks with start, intermediate, boundary and end events, `xor`, `and`, `or`, `eventgw`, `complex` gateways and sequence flows; `choreo … from= to= message= reply=` (`choreographyTask` with its participants, initiating participant and message flows), `subchoreo … pools= from=` with a block (`subChoreography`), `callchoreo … choreography=ID map=` (`callChoreography`); the flags `loop`, `parallel`, `sequential` (`loopType`). Exported as separate `choreography` root elements; each drawn in a diagram of its own, with participant bands (initiating white, others shaded), message envelopes and loop markers.
- Participant multiplicity: `min=` and `max=` on pools in 0.6 documents (`participantMultiplicity`), drawn with the multi-instance marker.
- Checker codes `R070`–`R074` (conversations, choreographies, multiplicity) and `C120`–`C123` for conversations, call conversations, choreographies and multiplicity in executable documents for Camunda; the levels `basic` and `analytic` reject the new statements (`R040`).
- BPMN conformance classes: `tok check --class descriptive|analytic|common-executable|full` reports every element outside the chosen class of BPMN 2.0.2 (clause 2) as `R075`; `tok check --json` reports the smallest class the document fits as `"bpmn_class"`. `docs/bpmn-rules.md` relates the tok levels and every construct to the classes.
- BPMN import reads conversations, conversation links, call conversations with participant associations, choreographies with their participants, tasks, sub-choreographies, calls and loop types, and participant multiplicity (a file that uses them is printed as `tok 0.6`); message flows that a conversation names keep their identifiers.
- Layout sidecar: `tok import FILE.bpmn -o FILE.tok` writes the diagram of the file to `FILE.tok.layout` (`tok-layout 1`; shapes, edges and labels by tok identifier, in whole units; `--no-layout` suppresses it). `tok bpmn`, `tok svg` and `tok export` read it when it exists next to the document (`--no-sidecar` ignores it): unchanged elements keep their geometry, new elements are placed next to their neighbours, containers grow around them and edges whose ends moved are routed again. Tested on the course files of `conformance/import` and the MIWG reference models: every shape, edge and label the import keeps is drawn as in the file.
- Conformance cases 47–52 (conversations, choreographies, choreography events, multiplicity, the Camunda codes `C120`–`C123`, the errors `R070`–`R074`); the Julia core parses, prints and exports them (core level, 206 checks).

### Changed

- The BPMN validator treats choreography activities as flow nodes and choreographies as containers of sequence flows (boundary events may be attached to choreography activities), and does not ask for diagram edges of the message flows of choreographies (`D02`).
- The cheat sheet printed by `tok cheatsheet` describes language 0.6.
- Layout: message flows between the same two black-box pools are drawn side by side instead of on one line.
- The command-line parity test with the pure-Python implementation (frozen at language 0.3) ignores `"bpmn_class"` of `tok check --json` and the option `--class`.

## [0.5.0] — 2026-10-27

Language 0.5 (`tok 0.5` in the header; documents written for 0.1 and 0.3 stay valid and mean the same): the elements of BPMN 2.0.2 process modelling that Camunda 8 does not execute. Reference: `docs/spec.md` §4, §5.1.1, §5.2.1, §5.3, §5.4, §5.6, §6.3, §7.1, §7.2, §10.4, §13–§15; design notes and the decisions taken during implementation in `docs/design-0.5.md`; the BPMN rules the checker enforces, with their clauses, in `docs/bpmn-rules.md`.

### Added

- `transaction` (BPMN `transaction`, `method=`), cancel end and boundary events (flag `cancel`); the simulator cancels the transaction, runs the compensation handlers of its completed activities and continues from the cancel boundary event.
- `complex` gateway with `activation="FEEL"` (`activationCondition`; the variable `_activationCount` in the simulator).
- Standard loop attributes `while=`, `test-before`, `loop-max=` (`loopCondition`, `testBefore`, `loopMaximum`), simulated.
- Multiple and parallel multiple events: repeated and combined trigger attributes, the flag `parallel`; simulated on catching events.
- Instantiation: `receive … instantiate`, `eventgw … instantiate [parallel]`.
- Process data: `item` (`itemDefinition`), `input`, `output` (`ioSpecification`), `property`, `type=` (`itemSubjectRef`); in 0.5 documents `reads` and `writes` connect data with generated data inputs and outputs of the activity.
- Resources and performers: in 0.5 documents `resource` exports a BPMN `resource` and `by=` a `performer` (`humanPerformer` on user and manual tasks).
- `global` tasks (`call … task=`), `interface` with `operation` lines (`operation=` on service, send and receive tasks, `supportedInterfaceRef`).
- Correlation keys: `key` with `prop` lines (`correlationKey`, `correlationProperty`).
- Checker codes `R060`–`R066`; `C100`–`C110` for elements Camunda 8.10 does not execute in executable documents; the levels `basic` and `analytic` reject every element of 0.5 (`R040`).
- BPMN import reads all of the above back (an imported file that uses them is printed as `tok 0.5`); import of `transaction`, `complexGateway`, multiple event definitions, `ioSpecification`, properties, global tasks and correlation keys no longer drops or simplifies them.
- Drawing: data input and output shapes, the parallel multiple marker, instantiating event-based gateways and receive tasks, call activities of global tasks.
- MIWG conformance gate (`rust/tok-core/tests/miwg.rs`): the 22 reference models of the BPMN Model Interchange Working Group test suite are imported, printed and exported, and their semantic content (flow elements and artifacts by identifier and kind, names, event definitions, gateways, flows, pools, lanes, data, declarations, loops, documentation, Camunda extensions) is compared with the reference; the test fails when something that survived before is lost or when a difference is not reported by the import. Coverage per model in `docs/miwg.md` (3533 of 3770 facts kept). The import corpus of the course survives the same round trip without losses.
- Additions for the MIWG round trip (`docs/design-0.5.md` §13): `doc=` (documentation) on every statement that exports an element; the trigger flags `message`, `timer`, `signal`, `condition` (and `error`, `escalation` on every event that allows them) for an event definition without a value; `process-name=` on the header and on pools; `namespace=` on `item` for a prefixed `structure=`; `reads … input=`, `writes … output=`; data of events (`reads` on throwing, `writes` on catching events, `input`/`output … on=EVENT`); `data … object=` for data objects shared by several references; `when=""`; an explicit compensation boundary event of an activity with `compensate=`. Conformance cases 44–46.
- BPMN import: documentation, data objects without references, the participants and message flows of every collaboration, named compensation boundary events and the data inputs and outputs of activities and events are kept; messages, signals, errors, escalations and times that a model does not give are no longer invented; every element, attribute, extension element and documentation that is not kept is reported.
- Conformance cases 33–43, with expected outputs produced by the Rust engine (`TOK_BLESS=1 cargo test -p tok-core --test conformance`); the Julia core passes cases 33–46 at the core level.

### Changed

- The BPMN validator counts an instantiating receive task or event-based gateway as the start of a process (`W040`, `W041`) and accepts a complex gateway as the join of any split (`W051`).
- The cheat sheet printed by `tok cheatsheet` describes language 0.5.
- Layout: a label of a message flow that would stand inside a pool or lane holding neither end of the flow, or next to another flow, shape or label, moves to a free place along its flow (between the pools, or next to its own ends); no quality measure of the corpus gets worse.
- The BPMN parity tests with the pure-Python implementation (frozen at language 0.3) skip the BPMN files that the import reads as language 0.5.

## [0.4.0] — 2026-10-27

The language is unchanged (language 0.3). From this release the Rust engine is the only engine: every new feature is written once, in Rust, and Python and R call it.

### Added

- Rust engine: FEEL evaluation with Python's decimal semantics, decision table evaluation with all hit policies, and `tok eval`; the discrete-event simulator (`tok-sim`, with a bit-exact port of Python's `random.Random`) and `tok sim`. Both print exactly what the pure-Python commands print for language 0.3; simulation results are identical per platform (Python's own `gauss`, `lognormvariate` and `expovariate` differ in the last bit between Windows and Linux, and so does the engine, deliberately).
- XES event log of a simulation (IEEE 1849-2016): `tok sim FILE --xes PATH`, `SimResult.to_xes()` and `write_xes()` in Python, `tok_xes()` in R. One trace per case with typed case variables, lifecycle transitions `schedule`, `start`, `complete`, `ati_abort` and `withdraw`, resources, deterministic output. See `docs/simulation.md`.
- Python package `tok-lang` on the engine: wheels contain the extension module `tok._engine`; the `tok` command runs the Rust command line; the new module `tok.api` (`check`, `fmt`, `bpmn`, `svg`, `export`, `import_bpmn`, `eval_decision`, `simulate`, `format_kpi`) works on the engine, or on the pure-Python implementation where no wheel is installed or with `TOK_ENGINE=python`. `tools/wheel.sh` builds the wheels (abi3, Python 3.8 or newer).
- R package `tokr` (`r/tokr`, the CRAN name `tok` is taken) on the engine through extendr: `tok_read()`, `tok_check()` (diagnostics as a data frame), `tok_fmt()`, `tok_bpmn()`, `tok_svg()`, `tok_plot()` (viewer), `tok_export()`, `tok_import()`, `tok_eval()`, `tok_sim()` (indicators, an event log ready for bupaR, cases as data frames) and `tok_xes()`; knitr engine `{tok}` for R Markdown and Quarto. `r/build.sh` builds a source archive that carries every Rust crate it needs and installs without network access; it passes `R CMD check --as-cran`.
- Release gates in one script, `tools/check.sh`: formatting, lints and tests of Rust, Python, R and Julia, the wheel and its parity with the pure-Python implementation, the native `tok` program; checked on Windows and Linux, with Linux wheels for x86_64 and aarch64 (manylinux2014) and a cross-compiled macOS wheel (universal2).

### Changed

- Diagram layout of the engine (`tok bpmn`, `tok svg`, `tok export`, and every binding): after the classic layout, flows are rerouted and labels moved where this removes defects, and columns are reordered by barycentre sweeps when that is better. On the 65 models of the corpus edge crossings fell from 59 to 30, flows through shapes from 22 to 17, label collisions from 28 to 0, labels on edges from 112 to 3 and bends from 281 to 228; no model is worse on any measure, and a model that brings its own diagram is drawn as before. The layout stays deterministic. It takes about 1.6 ms per model instead of 0.7 ms. Drawings therefore differ from those of the pure-Python implementation; semantic BPMN, DMN and form files remain identical.
- The pure-Python implementation is frozen at language 0.3 and remains the reference of the parity tests.
- The argument parsing of the native `tok` command follows the Python command for every command, including help texts and error messages.

### Fixed

- `tok eval` reports a violated hit policy (two rules under `unique`, different outputs under `any`) as `tok: rules 1, 2 all match (hit policy unique)` with exit status 1 instead of a Python traceback.

## [0.3.0] — 2026-10-27

Language 0.3 (`tok 0.3` in the header; documents written for 0.1 stay valid and mean the same). Reference: `docs/spec.md` §5.3, §10, §10.3, §13; design notes in `docs/design-0.3.md`.

### Added

- `adhoc` sub-process (BPMN `adHocSubProcess`): activities in any order, `sequential` ordering; checker rule `R050`.
- Compensation in any document: `compensate=HANDLER` on an activity (boundary compensation event and association), the handler flag `compensation`, `throw|end … compensate activity=ID` (`activityRef`); checker rule `R051`.
- Catch-all error and escalation events: the flags `error` and `escalation` without a name on boundary events and event sub-process starts.
- Executable layer for Camunda 8 (only in executable documents): multi-instance collections (`collection`, `item`, `output`, `output-item`, `until`), ad-hoc activation (`active`, `until`, `wait`), call activities (`binding`, `version`, `child-vars`, `parent-vars`, `business-id`), decision and form bindings, variable mappings (`inputs`, `outputs`), user task assignment and scheduling (`assignee`, `candidate-users`, `candidate-groups`, `due`, `follow-up`, `priority`), jobs (`retries`, `headers`, `type` on message throw and end events), the REST connector (`connector=rest`, `method`, `url`, `body`, `query`, `headers-in`, `result`), conditional filters (`events`), execution and task listeners (`listeners`), process version tags (`version-tag`).
- Checker codes `R052`–`R055`, `C022`, `C090`, `C091`; the levels `basic` and `analytic` reject the executable layer (`R040`); executable-layer attributes in documents that are not executable give `R055` and are ignored by the exporter.
- FEEL subset: `now()`, `today()`, `duration("PnDTnHnMnS")` and date/time arithmetic with durations.
- BPMN import reads all of the above back; an imported file that uses them is printed as `tok 0.3`.
- Simulation of multi-instance collections (parallel and sequential, `output`, `until`), ad-hoc sub-processes (activated elements start at once) and compensation (handlers of completed activities run when a compensation event is reached).
- Conformance: cases 19–32 and `expected/*.levels` (all diagnostic codes at the three levels); executable conformance on Camunda 8 Run 8.10.7 in `conformance/camunda/` (12 cases driven to their expected end events by `run.py`).

### Implementations

- Python, the Rust engine (`tok-core`, `tok-cli`, `tok-py`) and the Julia core implement language 0.3 and pass the whole conformance corpus byte for byte; Rust and Python are additionally compared on 62 documents (390 outputs).
- Plan (`docs/roadmap.md`): from 0.4 the Rust engine is the only engine — Python and R call it, the pure-Python implementation stays frozen at language 0.3 as a test oracle; Julia calls the Rust engine, keeps its independent core and gets native simulation and calibration.

### Not included

- Error code and message variables on catching error events: Camunda 8.10 accepts `errorCodeVariable`/`errorMessageVariable` in the XML but ignores them; variables sent with an error are merged with `outputs=` on the catching event.

## [0.2.0] — 2026-10-27

The language is unchanged (documents still start with `tok 0.1`); this release adds the Rust engine.

### Added

- Rust workspace `rust/`:
  - `tok-core`: lexer, parser, model, checker (all tok-level codes, FEEL subset checking), canonical printer, semantic BPMN export, DMN and form export, BPMN import;
  - `tok-bpmn`: the BPMN 2.0 rules (`E`, `W`, `L` and the Camunda `C` profile), automatic layout and SVG rendering, byte-identical to the Python validator;
  - `tok-cli`: native `tok` command with the same commands, options and output as the Python implementation (`sim` and `eval` follow in 0.4);
  - `tok-py`: Python module `tok_engine` on the Rust core (PyO3, abi3, Python ≥ 3.8), built with maturin, used for parity testing.
- Parity tests: the conformance corpus, full `check --json` output in English and Polish, BPMN, SVG, DMN and form files and BPMN import compared byte for byte between Python and Rust (Rust test suite and `python/tests/test_engine_parity.py`).

### Fixed

- `tok bpmn`, `tok svg` and `tok export` on a document without any flow elements or pools stop with the message "nothing to draw: the model has no elements" (Polish: „nie ma czego narysować: model nie zawiera elementów”) instead of a Python traceback.

### Packaging

- Version 0.2.0 of the Python package, the Julia package and the Rust crates. The installed Python package remains pure Python; the Rust build is optional until prebuilt wheels exist.

## [0.1.1] — 2026-10-27

### Changed

- Polish diagnostics (`--lang pl`) are now fully Polish: the detail text of FEEL errors and warnings (`F001`, `F002`) in conditions, expressions and decision table cells is translated.
- Overlap warnings for decision tables with `hit=unique` (`D010`) show a concrete example value for every input, also for cells written as `-` (a value used by another rule, a number inside the other rule's range, or `null` when nothing else fits).
- `tok export -o FOLDER` prints the written paths with the platform's native separator.
- The BPMN 2.0 rules (`E`, `W`, `L` codes) run only on documents without errors of any layer; previously errors of layers `F`, `D`, `M` and `C` did not stop them, so style notes appeared next to real errors.
- `exporterVersion` in exported BPMN XML is the language version of the document (`0.1`) instead of the tool version, so every implementation exports identical bytes.

### Packaging

- Python package and Julia package version 0.1.1. Until 1.0, packages are installed from the Codeberg archive; registry releases come with the stable version.

## [0.1.0] — 2026-10-27

First public version.

- Language specification 0.1: pools, lanes, all BPMN events, activities, sub-processes and gateways, sequence and message flows, data objects and stores, notes and groups, decision tables, forms, executable profile for Camunda 8, simulation parameters (`docs/spec.md`).
- Python reference implementation (`tok-lang`): `check` with diagnostics in English and Polish, canonical `fmt`, BPMN 2.0 export with automatic layout and SVG, DMN and Camunda form export, BPMN import, FEEL subset and decision evaluation, discrete-event simulation with event log and indicators, command line `tok`.
- Julia implementation of the core (`Tok.jl`): parsing, text-layer diagnostics, canonical printing and semantic BPMN export, byte-identical to Python on the conformance corpus.
- Conformance corpus of 18 documents with expected outputs; import corpus of BPMN models.
- Agent skill `tok-modelling`, cheat sheet, documentation of the command line, diagnostics, FEEL subset, simulation and import.
