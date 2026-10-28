# tok_jll: the engine library for Julia

`build_tarballs.jl` is the BinaryBuilder recipe of `tok_jll`, the binary package that gives Tok.jl the C library of the tok engine (`tokengine`, crate `rust/tok-capi`, see `docs/c-api.md`) without a Rust compiler. It builds the library from the source of a release on Codeberg for:

| Platform | Triplet |
|---|---|
| Linux x86_64, glibc and musl | `x86_64-linux-gnu`, `x86_64-linux-musl` |
| Linux aarch64, glibc and musl | `aarch64-linux-gnu`, `aarch64-linux-musl` |
| macOS x86_64 and Apple silicon | `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-w64-mingw32` |

The package has one product, `libtokengine` (`libtokengine.so`, `libtokengine.dylib`, `tokengine.dll`), and installs the header `include/tok.h` and the license. Tok.jl finds the library of `tok_jll` when the package is installed in the active environment (after `TOK_LIBTOK`, before the build directory of the repository); it does not depend on `tok_jll`, so it works without it.

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

## Submitting to Yggdrasil

For every release from 0.8.0 on:

1. Tag the release on Codeberg and note the commit of the tag (`git rev-parse v0.8.0^{commit}`).
2. Fork [JuliaPackaging/Yggdrasil](https://github.com/JuliaPackaging/Yggdrasil) and create `T/tok/build_tarballs.jl` (later releases: edit it) from this recipe, with the source written out as Yggdrasil requires:

   ```julia
   version = v"0.8.0"
   sources = [GitSource("https://codeberg.org/MarekDeja/tok.git", "<commit of the tag>")]
   ```

   and without the `TOK_COMMIT`/`TOK_SOURCE` selection.
3. Check that it builds: `TOK_COMMIT=<commit> julia --project=ENV build_tarballs.jl --verbose` (every platform) or, for one platform, with its triplet as above.
4. Open a pull request to Yggdrasil. Its CI builds every platform and, when the pull request is merged, registers `tok_jll` in the General registry; later releases only change `version` and the commit.

Yggdrasil allows the recipe to download crates during the build (`cargo build --locked` uses `rust/Cargo.lock`), as other Rust packages there do.

Tok.jl itself is registered in the General registry from 1.0 (roadmap); until then users install it from the repository and `tok_jll` from the registry.
