# tok_jll: the engine library for Julia

`build_tarballs.jl` is the BinaryBuilder recipe of `tok_jll`, the binary package that gives TokLang.jl the C library of the tok engine (`tokengine`, crate `rust/tok-capi`, see `docs/c-api.md`) without a Rust compiler. It builds the library from the source of a release on GitHub for:

| Platform | Triplet |
|---|---|
| Linux x86_64, glibc and musl | `x86_64-linux-gnu`, `x86_64-linux-musl` |
| Linux aarch64, glibc and musl | `aarch64-linux-gnu`, `aarch64-linux-musl` |
| macOS x86_64 and Apple silicon | `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-w64-mingw32` |

The package has one product, `libtokengine` (`libtokengine.so`, `libtokengine.dylib`, `tokengine.dll`), and installs the header `include/tok.h` and the license. TokLang.jl depends on `tok_jll`, so `Pkg.add("TokLang")` installs the library; a platform outside the table has `tok_jll.is_available() == false`, and the package still loads with its independent core.

## Building locally

BinaryBuilder runs on Linux (also in WSL 2), with user namespaces or Docker, and downloads its compiler shards (a few GB) on the first build:

```
julia --project=ENV -e 'using Pkg; Pkg.add("BinaryBuilder")'
mkdir /tmp/recipe && cp julia/binarybuilder/build_tarballs.jl /tmp/recipe/
cd /tmp/recipe
TOK_SOURCE=/path/to/clean/checkout julia --project=ENV build_tarballs.jl --verbose x86_64-linux-gnu
```

`TOK_SOURCE` is a checkout of the repository: BinaryBuilder copies it into its sandbox, so a clean checkout (without `rust/target`) is fast, and the recipe has to run from a folder outside it (BinaryBuilder writes its build folder next to the recipe). `TOK_COMMIT=<commit>` builds the commit of the repository on Codeberg instead. Without a platform argument the recipe builds every platform of the table. The tarballs are written to `products/`.

The recipe builds with the newest Rust toolchain of BinaryBuilder (1.94 at this release) and with the check of `rust-version` turned off: the workspace declares the Rust version it is developed with (`rust/rust-toolchain.toml`), and the engine compiles with the older toolchain. When the engine starts to use a newer Rust feature, `preferred_rust_version` must name a toolchain BinaryBuilder has (the error of a failed build lists them).

## In the General registry

The recipe is the one of the package `tok_jll` in [Yggdrasil](https://github.com/JuliaPackaging/Yggdrasil) (`T/tok/build_tarballs.jl`), which builds the library for every platform from the commit of a release tag of https://github.com/tok-lang-bpm/tok; the recipe in this folder carries the version of the release (`version = v"1.0.3"`) and builds from a local checkout as well. To use a build of your own, run `cargo build --release -p tok-capi` and point the preference `libtokengine_path` of `tok_jll` to the library (`docs/julia.md`).
