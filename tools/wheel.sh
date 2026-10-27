#!/usr/bin/env bash
# Builds a wheel of tok-lang with the Rust engine: the Python package python/src/tok plus the
# extension module tok._engine. Arguments are passed to `maturin build` (e.g. -o DIR, --zig,
# --target …). Needs maturin (pip install maturin) and the Rust toolchain of rust/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE="$ROOT/rust/tok-py/python"
rm -rf "$STAGE"
mkdir -p "$STAGE"
(cd "$ROOT/python/src" && tar --exclude=__pycache__ --exclude='*.pyc' --exclude='_engine*' -cf - tok) | tar -xf - -C "$STAGE"
maturin build --release -m "$ROOT/rust/tok-py/Cargo.toml" "$@"
