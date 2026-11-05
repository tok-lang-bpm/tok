#!/usr/bin/env bash
# Copies the models and the conformance corpus that TokLang.jl reads into the folder of the
# package: a package registered from a subfolder is installed without the rest of the
# repository, so its tests and examples cannot read files outside julia/TokLang.jl.
# Usage: bash tools/julia-data.sh           writes the copies
#        bash tools/julia-data.sh --check   fails when a copy differs from its source
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/julia/TokLang.jl"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/examples" "$TMP/data/conformance/cases" "$TMP/data/conformance/expected" \
  "$TMP/data/conformance/import" "$TMP/data/conformance/dmn" "$TMP/data/sim"
cp "$ROOT"/examples/*.tok "$TMP/examples/"
cp "$ROOT"/conformance/cases/*.tok "$TMP/data/conformance/cases/"
cp "$ROOT"/conformance/expected/* "$TMP/data/conformance/expected/"
cp "$ROOT/conformance/import/01-wypozyczenie-pierwszy-model.bpmn" "$TMP/data/conformance/import/"
cp "$ROOT/conformance/dmn-tck/compliance-level-3/0001-filter/0001-filter.dmn" "$TMP/data/conformance/dmn/"
cp "$ROOT"/rust/tok-sim/tests/fixtures/sim/*.tok "$TMP/data/sim/"

if [ "${1:-}" = --check ]; then
  mkdir -p "$TMP/current"
  cp "$PKG"/examples/*.tok "$TMP/current/"
  diff -r "$TMP/examples" "$TMP/current"
  diff -r "$TMP/data" "$PKG/test/data"
  echo "TokLang.jl: the copies of the models and of the corpus are current"
else
  rm -rf "$PKG/test/data"
  mkdir -p "$PKG/examples"
  rm -f "$PKG"/examples/*.tok
  cp "$TMP"/examples/*.tok "$PKG/examples/"
  cp -r "$TMP/data" "$PKG/test/data"
fi
