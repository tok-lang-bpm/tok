#!/usr/bin/env bash
# Release gates of the tok repository, run from any directory on Linux, macOS or Windows (Git Bash).
#
#   tools/check.sh            formatting, lints and tests of Rust, Python, R and Julia; wheel of
#                             tok-lang with the Rust engine and its parity with the pure-Python
#                             implementation; native tok program; C library of the engine
#   tools/check.sh --cross    additionally cross-compiles wheels for the other platforms
#                             with zig (Linux host: Linux aarch64 and macOS universal2; not tested)
#
# Built wheels, the tok program and the C library of the engine (with tok.h) are written to
# dist/. Julia is skipped when it is not installed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CROSS=0
[ "${1:-}" = "--cross" ] && CROSS=1

step() { printf '\n== %s\n' "$*"; }

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) PLATFORM=windows; PY=python; BIN=Scripts; EXE=tok.exe; LIB=tokengine.dll ;;
  Darwin) PLATFORM=macos; PY=python3; BIN=bin; EXE=tok; LIB=libtokengine.dylib ;;
  *) PLATFORM=linux; PY=python3; BIN=bin; EXE=tok; LIB=libtokengine.so ;;
esac

rm -rf dist
mkdir -p dist

step "Rust: formatting, lints, tests"
(
  cd rust
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo build --release -p tok-cli -p tok-capi
)
cp "rust/target/release/$EXE" "rust/target/release/$LIB" rust/tok-capi/include/tok.h dist/

step "tok program on the examples"
for f in examples/*.tok; do "dist/$EXE" check "$f"; done

step "Python: pure-Python implementation"
PYTHONPATH=python/src "$PY" -m unittest discover -s python/tests

step "Wheel of tok-lang with the Rust engine ($PLATFORM); parity with the pure-Python implementation"
VENV=rust/target/check-venv
[ -x "$VENV/$BIN/python" ] || "$PY" -m venv "$VENV"
"$VENV/$BIN/python" -m pip install --quiet --disable-pip-version-check --upgrade "maturin>=1.7,<2" ziglang
# maturin finds zig through the Python of the virtual environment
export PATH="$ROOT/$VENV/$BIN:$PATH"
WHEELS=rust/target/wheels-check
rm -rf "$WHEELS"
if [ "$PLATFORM" = linux ]; then
  # zig links against an old glibc, so the wheel is manylinux2014 (glibc 2.17) whatever the host is
  bash tools/wheel.sh -o "$WHEELS" --zig --compatibility manylinux2014
else
  bash tools/wheel.sh -o "$WHEELS"
fi
"$VENV/$BIN/python" -m pip install --quiet --force-reinstall "$WHEELS"/*.whl
# the installed package (with tok._engine) is tested; the reference runs from python/src
"$VENV/$BIN/python" -m unittest python/tests/test_engine_parity.py
cp "$WHEELS"/*.whl dist/

if [ "$CROSS" = 1 ] && [ "$PLATFORM" = linux ]; then
  step "Cross-compiled wheels (not tested on their platforms)"
  for target in aarch64-unknown-linux-gnu universal2-apple-darwin; do
    case "$target" in
      universal2-apple-darwin) (cd rust && rustup target add aarch64-apple-darwin x86_64-apple-darwin) ;;
      *) (cd rust && rustup target add "$target") ;;
    esac
    extra=()
    [ "$target" = aarch64-unknown-linux-gnu ] && extra=(--compatibility manylinux2014)
    bash tools/wheel.sh -o dist --zig --target "$target" "${extra[@]}"
  done
fi

if command -v Rscript > /dev/null; then
  step "R package tokr"
  RLIB="$ROOT/rust/target/check-rlib"
  rm -rf "$RLIB"
  mkdir -p "$RLIB"
  R CMD INSTALL --no-multiarch --library="$RLIB" r/tokr > rust/target/check-rlib.log 2>&1 ||
    { tail -40 rust/target/check-rlib.log; exit 1; }
  (cd r/tokr && RLIB="$RLIB" Rscript -e '.libPaths(c(Sys.getenv("RLIB"), .libPaths())); testthat::test_local(".", load_package = "installed", reporter = "summary", stop_on_failure = TRUE)')
else
  step "R package tokr: skipped (R not installed)"
fi

if command -v julia > /dev/null; then
  step "Julia: core, engine through the C interface, native simulation, calibration"
  # the engine library just built; the tests use two threads for the parallel runs
  TOK_LIBTOK="$ROOT/rust/target/release/$LIB"
  [ "$PLATFORM" = windows ] && TOK_LIBTOK="$(cygpath -w "$TOK_LIBTOK")"
  TOK_LIBTOK="$TOK_LIBTOK" julia --project=julia/Tok.jl -e 'using Pkg; Pkg.test(julia_args=["--threads=2"])'
else
  step "Julia: skipped (julia not installed)"
fi

step "All checks passed"
ls dist
