#!/usr/bin/env bash
# Builds a wheel of tok-lang with the Rust engine from the repository root (pyproject.toml): the
# Python package python/src/tok plus the extension module tok._engine. Arguments are passed to
# `maturin build` (e.g. -o DIR, --zig, --target …). Needs maturin (pip install maturin) and the
# Rust toolchain of rust/. `maturin sdist` (or `python -m build --sdist`) in the root writes
# the source archive, which builds the engine with Rust when it is installed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
maturin build --release "$@"
