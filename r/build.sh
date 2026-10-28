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
        -e "s|^homepage.workspace = true|homepage = $(ws homepage)|" \
    > "$dst/Cargo.toml"
  cp "$ROOT/rust/$crate/README.md" "$dst/"
done
sed -i 's|path = "../../../../rust/|path = "engine/|' "$PKG/src/rust/Cargo.toml"

(
  cd "$PKG/src/rust"
  cargo vendor --quiet --versioned-dirs vendor > /dev/null
  # the archive is unpacked next to Makevars, one level above the manifest
  printf '%s\n' '[source.crates-io]' 'replace-with = "vendored-sources"' '' \
    '[source.vendored-sources]' 'directory = "vendor"' > vendor-config.toml
  # the copyright holders of the bundled crates, as CRAN asks for code of other authors
  mkdir -p "$PKG/inst"
  {
    printf '%s\n' 'tokr bundles the Rust engine of tok (Marek Deja, GPL-3.0-or-later) and, in' \
      'src/rust/vendor.tar.xz, these Rust crates; the copyright of each crate belongs to its' \
      'authors, named in the Cargo.toml of the crate, and each crate is used under its licence.' ''
    cargo tree --offline --edges normal --prefix none --format '{p}: {l}, {r}' |
      sed -e 's/ (\*)$//' | grep -v '^tok' | sort -u
  } > "$PKG/inst/AUTHORS"
  printf '%s\n' 'Copyright: Marek Deja; the authors of the bundled Rust crates listed in inst/AUTHORS.' >> "$PKG/DESCRIPTION"
  tar -cJf vendor.tar.xz vendor
  rm -rf vendor
)

mkdir -p "$ROOT/dist"
(cd "$ROOT/dist" && R CMD build --no-manual "$PKG")
# R CMD build leaves out some directory names (for example check/); every engine source
# file must still be in the archive
VER="$(sed -n 's/^Version: //p' "$PKG/DESCRIPTION")"
missing="$(comm -23 \
  <(cd "$PKG/src/rust/engine" && find . -name '*.rs' | sed 's|^\./|tokr/src/rust/engine/|' | sort) \
  <(tar -tzf "$ROOT/dist/tokr_$VER.tar.gz" | sort))"
if [ -n "$missing" ]; then
  printf 'missing from the archive:\n%s\n' "$missing" >&2
  exit 1
fi
ls "$ROOT"/dist/tokr_*.tar.gz
