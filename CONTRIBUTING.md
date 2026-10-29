# Contributing to tok

Thank you for helping with tok. Reports of problems, models that are exported wrongly, missing BPMN or DMN constructs, unclear documentation and ideas are all welcome.

## Issues

Open an issue in https://github.com/tok-lang-bpm/tok/issues. For a problem with a model, attach the smallest `.tok` (or `.bpmn`, `.dmn`) file that shows it, the command you ran, its output, and the version (`tok --version`). Security problems are reported privately, as described in [SECURITY.md](SECURITY.md).

## Where the code lives

The repository https://github.com/tok-lang-bpm/tok is the public home of the project, with its issues, releases and workflows. Development happens in https://codeberg.org/MarekDeja/tok, which is mirrored to GitHub after every change; pull requests on GitHub are welcome and are applied there, with their authors kept.

## Building and testing

| Part | Needs | Commands |
|---|---|---|
| engine and programs | Rust 1.99 or newer (`rust/rust-toolchain.toml`) | `cd rust && cargo test --workspace`, `cargo build --release -p tok-cli` |
| Python package | Python 3.8 or newer, maturin | `bash tools/wheel.sh -o dist`, then `pip install dist/*.whl` |
| R package | R 4.2 or newer, Rust | `bash r/build.sh`, `R CMD check --as-cran dist/tokr_*.tar.gz` |
| Julia package | Julia 1.10 or newer, the engine library | `julia --project=julia/Tok.jl -e 'using Pkg; Pkg.test()'` |
| VS Code extension | Node.js 20 or newer | `cd editors/vscode && npm ci && npm test` |
| playground | the Rust target `wasm32-unknown-unknown`, wasm-bindgen | `bash tools/playground.sh` |

`tools/check.sh` runs all of them, as the release gates do, on Linux, macOS or Windows (Git Bash). The workflow `.github/workflows/ci.yml` runs the same gates on every push and pull request.

## Changes to the language

The specification [docs/spec.md](docs/spec.md) is normative and the conformance corpus [conformance/](conformance/) is the contract between implementations: a change to the language comes with the specification text, a conformance case (the input document and its expected outputs), the diagnostics in English and Polish ([docs/diagnostics.md](docs/diagnostics.md)) and the documentation. Language 1.x keeps every valid 1.0 document valid with the same meaning (spec §4.3); a change that would break this waits for 2.0.

## Style

- Rust: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`.
- Python: the standard library only in the package; tests with `unittest`.
- Documentation: plain English, one document per subject in `docs/`; every command, option, diagnostic code, keyword and attribute is documented (the test `rust/tok-cli/tests/docs.rs` checks this).

## Licence

Contributions are accepted under the licence of the project, the GNU General Public License v3.0 or later.
