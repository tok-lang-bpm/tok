# Dependencies of the Rust engine

The dependency tree is kept small so that the R package can vendor it for offline CRAN builds and the engine compiles to WebAssembly.

| Crate | Used by | Why |
|---|---|---|
| thiserror | all library crates | error types |
| roxmltree | tok-core (import), tok-bpmn | reading BPMN/DMN XML |
| serde, serde_json | tok-core, tok-cli, tok-bpmn | JSON diagnostics, Camunda forms (tok-bpmn reads `.form` ids for the Camunda profile; its parity test compares JSON) |
| indexmap | tok-core | ordered maps that keep declaration order |
| pyo3 (0.29, features `abi3-py38`, `extension-module`) | tok-py only | Python bindings (module `tok_engine`); 0.29 is the newest release that still builds an abi3 module for Python 3.8, the minimum version of `tok-lang` |

## Unsafe code

The workspace forbids `unsafe` code. The only exception is `tok-py`: the code that PyO3's `#[pymodule]` and `#[pyfunction]` macros generate contains `unsafe` blocks, so `tok-py` does not inherit the workspace lint table (it keeps the Clippy settings). `tok-py` itself contains no handwritten `unsafe` code.
