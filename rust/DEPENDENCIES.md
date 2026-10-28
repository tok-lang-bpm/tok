# Dependencies of the Rust engine

The dependency tree is kept small so that the R package can vendor it for offline CRAN builds and the engine compiles to WebAssembly.

| Crate | Used by | Why |
|---|---|---|
| thiserror | all library crates | error types |
| roxmltree | tok-core (import), tok-bpmn, tok-sim (tests only) | reading BPMN/DMN XML; the tests of the XES event log parse it back |
| serde, serde_json | tok-core, tok-cli, tok-bpmn, tok-lsp, tok-mcp, tok-capi (tests only), tok-sim (tests only), tok-wasm (tests only) | JSON diagnostics, Camunda forms (tok-bpmn reads `.form` ids for the Camunda profile; its parity test compares JSON); the parity tests read their fixtures as JSON; the functions of `tok_cli::api` (the C interface, the MCP server, the WebAssembly build) read their options; the language server and the MCP server read and write their JSON-RPC messages |
| indexmap | tok-core, tok-sim | ordered maps that keep declaration order (and the insertion order of Python dicts in case variables) |
| pyo3 (0.29, features `abi3-py38`, `extension-module`) | tok-py only | Python bindings (module `tok._engine` of the package `tok-lang`); 0.29 is the newest release that still builds an abi3 module for Python 3.8, the minimum version of `tok-lang` |
| wasm-bindgen (`=0.2.129`) | tok-wasm only | the JavaScript bindings of the WebAssembly module of the playground; the version is exact because the `wasm-bindgen` command that writes the glue code (`tools/playground.sh`) must be the same version as the crate |
| wasm-bindgen-test (`=0.3.79`) | tok-wasm (tests only) | runs the tests of `tok-wasm/tests/web.rs` inside the WebAssembly module under Node.js; the release of wasm-bindgen 0.2.129 |

## Protocols without dependencies

The language server (`tok-lsp`) and the MCP server (`tok-mcp`) need no crate beyond `serde_json`. Both protocols are JSON-RPC 2.0 over standard input and output: LSP frames each message with a `Content-Length` header, MCP writes one message per line, and each server answers a small, fixed set of methods. The messages are handled as `serde_json::Value`s with a few dozen lines of framing, which keeps the dependency tree as small as the rest of the engine. The alternatives were weighed: `lsp-server` with `lsp-types` would add `crossbeam-channel`, `log`, `serde` with its derive macros (and through them `syn`, `quote`, `proc-macro2`), `serde_repr`, `bitflags` and a URI crate, and the URI type of `lsp-types` has changed incompatibly between releases; `tower-lsp` would add an asynchronous runtime (`tokio`) that a server answering one editor at a time does not need; the MCP SDK for Rust (`rmcp`) is likewise built on `tokio` and the derive macros of `serde`. The tests of both crates drive the servers with the JSON messages of the protocols, so a typed layer would not add checks the tests do not already make.

## Random numbers

`tok-sim` does not use `rand`/`rand_distr`. Its output must match `sim.py` for the same seed, so `tok_sim::pyrandom` implements CPython's `random.Random` itself (MT19937, integer seeding through `init_by_array`, and the distribution formulas of `Lib/random.py`). Its parity test reads `tests/fixtures/pyrandom-PLATFORM.txt`, written by `tests/gen_python_expected.py`: `gauss`, `normalvariate`, `lognormvariate` and `expovariate` go through the C library's `log`, `exp`, `sin` and `cos`, whose last bit differs between the Windows UCRT and glibc, so each platform has its own fixture.

The simulator's parity tests (`tok-sim/tests/sim_parity.rs`, `tok-cli/tests/sim_parity.rs`) read `tok-sim/tests/fixtures/sim/cases-PLATFORM.json`, written by `tok-sim/tests/gen_python_sim.py` for the same reason once per platform. The script needs Python 3.12 or newer: since 3.12 `sum()` adds floats with compensated summation, which the mean cycle time uses, and the engine follows it.

## Unsafe code

The workspace forbids `unsafe` code; `tok-lsp`, `tok-mcp` and `tok-wasm` inherit the prohibition (the code that the `#[wasm_bindgen]` macro generates compiles under it). There are two exceptions, both at a foreign-function boundary:

- `tok-py`: the code that PyO3's `#[pymodule]` and `#[pyfunction]` macros generate contains `unsafe` blocks, so `tok-py` does not inherit the workspace lint table (it keeps the Clippy settings). `tok-py` itself contains no handwritten `unsafe` code.
- `tok-capi`, the C interface: reading the C strings passed in and taking back the strings handed out needs `unsafe`. The crate denies `unsafe` code and allows it only in its module `ffi`, which holds the exported functions; every `unsafe` block there has a `SAFETY` comment (Clippy `undocumented_unsafe_blocks` is denied), and every exported function catches panics so that none unwinds into C.
