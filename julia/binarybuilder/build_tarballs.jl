# BinaryBuilder recipe of tok_jll: the C library of the tok engine (crate tok-capi) for
# every platform Julia supports, in the form of the Yggdrasil repository.
#
#   TOK_COMMIT=<sha> julia build_tarballs.jl --verbose                 the release commit
#   TOK_SOURCE=<dir> julia build_tarballs.jl --verbose x86_64-linux-gnu   a local checkout
#
# TOK_COMMIT selects a commit of the repository on Codeberg (the commit of the release tag),
# as the recipe submitted to Yggdrasil does (README.md). TOK_SOURCE selects a checkout of
# the repository on disk instead; BinaryBuilder copies it and writes its build folder next
# to this file, so the checkout must not contain the folder this recipe runs from.

using BinaryBuilder, Pkg

name = "tok"
version = v"1.0.0"

const REPO = "https://codeberg.org/MarekDeja/tok.git"

sources = if haskey(ENV, "TOK_COMMIT")
    [GitSource(REPO, ENV["TOK_COMMIT"])]
elseif haskey(ENV, "TOK_SOURCE")
    [DirectorySource(ENV["TOK_SOURCE"]; target="tok")]
else
    error("set TOK_COMMIT to the commit of the release tag, or TOK_SOURCE to a checkout of the repository")
end

script = raw"""
cd ${WORKSPACE}/srcdir/tok/rust
# the Rust toolchain of BinaryBuilder builds the crate (rust-toolchain.toml would ask rustup
# for another one); it is older than the rust-version of the workspace, which the engine
# does not need yet, so the check of rust-version is turned off
rm -f rust-toolchain.toml
if [[ "${target}" == *-musl* ]]; then
    # a shared library needs the C runtime linked dynamically on musl
    export RUSTFLAGS="-C target-feature=-crt-static"
fi
cargo build --release --locked --ignore-rust-version -p tok-capi
if [[ "${target}" == *-mingw* ]]; then
    lib=tokengine.${dlext}
else
    lib=libtokengine.${dlext}
fi
install -Dvm 755 "target/${rust_target}/release/${lib}" "${libdir}/${lib}"
install -Dvm 644 tok-capi/include/tok.h "${includedir}/tok.h"
install_license ../LICENSE
"""

platforms = [
    Platform("x86_64", "linux"; libc="glibc"),
    Platform("aarch64", "linux"; libc="glibc"),
    Platform("x86_64", "linux"; libc="musl"),
    Platform("aarch64", "linux"; libc="musl"),
    Platform("x86_64", "macos"),
    Platform("aarch64", "macos"),
    Platform("x86_64", "windows"),
]

products = [
    LibraryProduct(["libtokengine", "tokengine"], :libtokengine),
]

dependencies = Dependency[]

# the newest Rust toolchain of BinaryBuilder at the time of the release
build_tarballs(ARGS, name, version, sources, script, platforms, products, dependencies;
               julia_compat="1.10", compilers=[:c, :rust], preferred_rust_version=v"1.94")
