#!/usr/bin/env bash
# Builds the self-contained source archive of the R package tokr (dist/tokr_VERSION.tar.gz).
#
# In the repository the package compiles against the engine crates in rust/. The archive
# instead carries copies of those crates and every external crate they need (vendored),
# so it installs without network access and without the rest of the repository, as CRAN
# requires. Needs R, cargo and xz.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="tok-core tok-bpmn tok-sim"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

PKG="$STAGE/tokr"
mkdir -p "$PKG"
(cd "$ROOT/r/tokr" && tar --exclude=src/rust/target --exclude=src/.cargo --exclude=src/vendor -cf - .) | tar -xf - -C "$PKG"

# value of a key of [workspace.package] in rust/Cargo.toml
ws() { sed -n '/^\[workspace.package\]/,/^\[/p' "$ROOT/rust/Cargo.toml" | sed -n "s/^$1 = //p"; }

mkdir -p "$PKG/src/rust/engine"
for crate in $CRATES; do
  dst="$PKG/src/rust/engine/$crate"
  mkdir -p "$dst"
  cp -r "$ROOT/rust/$crate/src" "$dst/"
  # inherit nothing from the workspace; drop test-only dependencies and workspace lints
  awk '
    /^\[/ { skip = ($0 == "[dev-dependencies]" || $0 == "[lints]") }
    !skip
  ' "$ROOT/rust/$crate/Cargo.toml" |
    sed -e "s|^version.workspace = true|version = $(ws version)|" \
        -e "s|^edition.workspace = true|edition = $(ws edition)|" \
        -e "s|^rust-version.workspace = true|rust-version = $(ws rust-version)|" \
        -e "s|^license.workspace = true|license = $(ws license)|" \
        -e "s|^authors.workspace = true|authors = $(ws authors)|" \
        -e "s|^repository.workspace = true|repository = $(ws repository)|" \
    > "$dst/Cargo.toml"
done
sed -i 's|path = "../../../../rust/|path = "engine/|' "$PKG/src/rust/Cargo.toml"

(
  cd "$PKG/src/rust"
  cargo vendor --quiet --versioned-dirs vendor > /dev/null
  # the archive is unpacked next to Makevars, one level above the manifest
  printf '%s\n' '[source.crates-io]' 'replace-with = "vendored-sources"' '' \
    '[source.vendored-sources]' 'directory = "vendor"' > vendor-config.toml
  tar -cJf vendor.tar.xz vendor
  rm -rf vendor
)

mkdir -p "$ROOT/dist"
(cd "$ROOT/dist" && R CMD build --no-manual "$PKG")
ls "$ROOT"/dist/tokr_*.tar.gz
