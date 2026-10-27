# Dependencies of the Rust engine

The dependency tree is kept small so that the R package can vendor it for offline CRAN builds and the engine compiles to WebAssembly.

| Crate | Used by | Why |
|---|---|---|
| thiserror | all library crates | error types |
| roxmltree | tok-core (import), tok-bpmn | reading BPMN/DMN XML |
| serde, serde_json | tok-core, tok-cli, tok-bpmn, tok-sim (tests only) | JSON diagnostics, Camunda forms (tok-bpmn reads `.form` ids for the Camunda profile; its parity test compares JSON); the parity tests read their fixtures as JSON |
| indexmap | tok-core, tok-sim | ordered maps that keep declaration order (and the insertion order of Python dicts in case variables) |
| pyo3 (0.29, features `abi3-py38`, `extension-module`) | tok-py only | Python bindings (module `tok_engine`); 0.29 is the newest release that still builds an abi3 module for Python 3.8, the minimum version of `tok-lang` |

## Random numbers

`tok-sim` does not use `rand`/`rand_distr`. Its output must match `sim.py` for the same seed, so `tok_sim::pyrandom` implements CPython's `random.Random` itself (MT19937, integer seeding through `init_by_array`, and the distribution formulas of `Lib/random.py`). Its parity test reads `tests/fixtures/pyrandom-PLATFORM.txt`, written by `tests/gen_python_expected.py`: `gauss`, `normalvariate`, `lognormvariate` and `expovariate` go through the C library's `log`, `exp`, `sin` and `cos`, whose last bit differs between the Windows UCRT and glibc, so each platform has its own fixture.

The simulator's parity tests (`tok-sim/tests/sim_parity.rs`, `tok-cli/tests/sim_parity.rs`) read `tok-sim/tests/fixtures/sim/cases-PLATFORM.json`, written by `tok-sim/tests/gen_python_sim.py` for the same reason once per platform. The script needs Python 3.12 or newer: since 3.12 `sum()` adds floats with compensated summation, which the mean cycle time uses, and the engine follows it.

## Unsafe code

The workspace forbids `unsafe` code. The only exception is `tok-py`: the code that PyO3's `#[pymodule]` and `#[pyfunction]` macros generate contains `unsafe` blocks, so `tok-py` does not inherit the workspace lint table (it keeps the Clippy settings). `tok-py` itself contains no handwritten `unsafe` code.
