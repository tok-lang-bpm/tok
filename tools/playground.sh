#!/usr/bin/env bash
# Builds the browser playground: the tok engine compiled to WebAssembly (crate tok-wasm),
# its JavaScript glue written by wasm-bindgen, the page of playground/ and the examples.
#
#   tools/playground.sh                 build into rust/target/playground
#   tools/playground.sh -o DIR          build into DIR
#   tools/playground.sh --test          also run the tests of the module under Node.js
#
# The folder is a static site: serve it from any static host, or locally with
#   python -m http.server -d rust/target/playground 8000
# and open http://localhost:8000/. It needs the Rust target wasm32-unknown-unknown and the
# wasm-bindgen command of the exact version of the wasm-bindgen crate in
# rust/tok-wasm/Cargo.toml; the script installs that command with cargo when it is missing.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/rust/target/playground"
TEST=0
while [ $# -gt 0 ]; do
  case "$1" in
    -o) OUT="$2"; shift 2 ;;
    --test) TEST=1; shift ;;
    *) echo "usage: tools/playground.sh [-o DIR] [--test]" >&2; exit 2 ;;
  esac
done

VERSION="$(sed -n 's/^wasm-bindgen = "=\(.*\)"/\1/p' "$ROOT/rust/tok-wasm/Cargo.toml")"
# the toolchain of rust/rust-toolchain.toml
if ! (cd "$ROOT/rust" && rustup target list --installed 2> /dev/null) | tr -d '\r' | grep -qx wasm32-unknown-unknown; then
  echo "playground: the Rust target is missing; install it with: rustup target add wasm32-unknown-unknown" >&2
  exit 1
fi
if [ "$(wasm-bindgen --version 2> /dev/null | tr -d '\r' || true)" != "wasm-bindgen $VERSION" ]; then
  echo "playground: installing wasm-bindgen-cli $VERSION"
  cargo install --locked wasm-bindgen-cli --version "$VERSION"
fi

(cd "$ROOT/rust" && cargo build --release -p tok-wasm --target wasm32-unknown-unknown)

# empty the folder, not remove it: a local server may be serving it
mkdir -p "$OUT"
find "$OUT" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
mkdir -p "$OUT/pkg" "$OUT/examples"
wasm-bindgen --target web --no-typescript --out-dir "$OUT/pkg" \
  "$ROOT/rust/target/wasm32-unknown-unknown/release/tok_wasm.wasm"
cp "$ROOT/playground/index.html" "$ROOT/playground/app.js" "$ROOT/playground/style.css" "$OUT/"
cp "$ROOT/LICENSE" "$OUT/LICENSE.txt"

# the examples and their list
names=()
for f in "$ROOT"/examples/*.tok; do
  cp "$f" "$OUT/examples/"
  names+=("\"$(basename "$f")\"")
done
(IFS=,; printf '[%s]\n' "${names[*]}") > "$OUT/examples/index.json"

if [ "$TEST" = 1 ]; then
  (cd "$ROOT/rust" && CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
    cargo test -p tok-wasm --target wasm32-unknown-unknown --test web)
fi

echo "playground: $OUT"
