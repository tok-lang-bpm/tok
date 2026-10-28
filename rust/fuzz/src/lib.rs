//! Fuzz targets of the tok engine (cargo-fuzz, libFuzzer). The properties live in
//! [`oracles`]; each target in `fuzz_targets/` calls one of them. See `docs/fuzzing.md`.

pub mod oracles;
