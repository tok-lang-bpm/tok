#!/usr/bin/env bash
# Release gates of the tok repository, run from any directory on Linux, macOS or Windows (Git Bash).
#
#   tools/check.sh            formatting, lints and tests of Rust, Python, R and Julia; wheel of
#                             tok-lang with the Rust engine and its parity with the pure-Python
#                             implementation; source archive of tok-lang and the packages
#                             of the published crates (dry run); native tok program; C library
#                             of the engine; language server and MCP server; VS Code
#                             extension; playground
#   tools/check.sh --cross    additionally cross-compiles wheels for the other platforms
#                             with zig (Linux host: Linux aarch64 and macOS universal2; not tested)
#
# Built wheels and the source archive of tok-lang, the tok, tok-lsp and tok-mcp programs, the C library of the engine (with
# tok.h) and the .vsix of the VS Code extension are written to dist/; the playground to
# rust/target/playground. R and Julia are skipped when they are not installed, the VS Code
# extension without npm, the playground without the Rust target wasm32-unknown-unknown and
# the wasm-bindgen command of the version tok-wasm uses.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CROSS=0
[ "${1:-}" = "--cross" ] && CROSS=1

step() { printf '\n== %s\n' "$*"; }

case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) PLATFORM=windows; PY=python; BIN=Scripts; EXE=tok.exe; LIB=tokengine.dll; SUFFIX=.exe ;;
  Darwin) PLATFORM=macos; PY=python3; BIN=bin; EXE=tok; LIB=libtokengine.dylib; SUFFIX= ;;
  *) PLATFORM=linux; PY=python3; BIN=bin; EXE=tok; LIB=libtokengine.so; SUFFIX= ;;
esac

rm -rf dist
mkdir -p dist

# Node.js and npm of this platform (in WSL the npm of Windows may be on the PATH; it cannot
# build here)
node_ok() {
  command -v node > /dev/null && command -v npm > /dev/null || return 1
  local want=linux
  [ "$PLATFORM" = windows ] && want=win32
  [ "$PLATFORM" = macos ] && want=darwin
  [ "$(node -p process.platform 2> /dev/null | tr -d '\r')" = "$want" ]
}

step "Rust: formatting, lints, tests"
(
  cd rust
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo build --release -p tok-cli -p tok-capi -p tok-lsp -p tok-mcp
)
cp "rust/target/release/$EXE" "rust/target/release/$LIB" rust/tok-capi/include/tok.h dist/
cp "rust/target/release/tok-lsp$SUFFIX" "rust/target/release/tok-mcp$SUFFIX" dist/

step "tok program on the examples"
for f in examples/*.tok; do "dist/$EXE" check "$f"; done

step "Python: pure-Python implementation"
PYTHONPATH=python/src "$PY" -m unittest discover -s python/tests

step "Wheel of tok-lang with the Rust engine ($PLATFORM); parity with the pure-Python implementation"
VENV=rust/target/check-venv
# reused between runs; made again when its pip does not work (a copy restored by a build
# cache, or a Python that has changed)
"$VENV/$BIN/python" -m pip --version > /dev/null 2>&1 || { rm -rf "$VENV"; "$PY" -m venv "$VENV"; }
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

step "Packages of the registries: source archive of tok-lang, metadata of the Python packages, crates"
# the source archive builds the engine with Rust when pip installs it; twine checks the
# metadata PyPI shows; cargo packages the published crates in dependency order and builds
# each from its package (nothing is uploaded)
"$VENV/$BIN/python" -m pip install --quiet --disable-pip-version-check --upgrade twine
maturin sdist -o dist
"$VENV/$BIN/python" -m twine check --strict dist/tok_lang-*
(cd rust && cargo publish --dry-run --allow-dirty --quiet -p tok-core -p tok-bpmn -p tok-sim -p tok-cli -p tok-lsp -p tok-mcp)

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

if node_ok; then
  step "VS Code extension: build, grammar tests, package, smoke test of the package"
  rm -f editors/vscode/*.vsix
  (
    cd editors/vscode
    npm ci --no-audit --no-fund --loglevel=error
    npm test
    npm run package
  )
  VSIX="$(ls editors/vscode/*.vsix)"
  node editors/vscode/test/smoke.mjs "$VSIX"
  cp "$VSIX" dist/
else
  step "VS Code extension: skipped (Node.js and npm for $PLATFORM not installed)"
fi

WASM_BINDGEN="$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' rust/tok-wasm/Cargo.toml)"
if ! (cd rust && rustup target list --installed 2> /dev/null) | grep -q '^wasm32-unknown-unknown'; then
  step "Playground: skipped (the Rust target wasm32-unknown-unknown is not installed)"
elif [ "$(wasm-bindgen --version 2> /dev/null | tr -d '\r')" != "wasm-bindgen $WASM_BINDGEN" ]; then
  step "Playground: skipped (wasm-bindgen $WASM_BINDGEN not installed: cargo install --locked wasm-bindgen-cli --version $WASM_BINDGEN)"
elif node_ok; then
  step "Playground: WebAssembly build of the engine and its tests under Node.js"
  bash tools/playground.sh --test
else
  step "Playground: WebAssembly build of the engine (its tests skipped: Node.js for $PLATFORM not installed)"
  bash tools/playground.sh
fi

step "All checks passed"
ls dist
