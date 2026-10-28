# Build the `tok` application of Tok.jl with PackageCompiler.
#
#   julia julia/app/build.jl [OUTPUT]          default OUTPUT: julia/app/build/tok
#
# The result is a folder with bin/tok (bin/tok.exe on Windows), the Julia runtime and the
# compiled code of Tok.jl. The engine library of the repository build
# (rust/target/release, built with `cargo build --release -p tok-capi`) or of TOK_LIBTOK
# is copied into bin/, where the program finds it. PackageCompiler is installed into a
# temporary environment; it needs a C compiler (on Windows it downloads one).

using Pkg

const HERE = @__DIR__
const APP = joinpath(HERE, "TokApp")
const OUTPUT = length(ARGS) >= 1 ? abspath(ARGS[1]) : joinpath(HERE, "build", "tok")

# the application environment, with Tok.jl from this repository
Pkg.activate(APP)
Pkg.develop(path=normpath(joinpath(HERE, "..", "Tok.jl")))
Pkg.instantiate()

Pkg.activate(; temp=true)
Pkg.add("PackageCompiler")
using PackageCompiler

# the library the application uses, for the workload and for the folder
const LIBNAME = Sys.iswindows() ? "tokengine.dll" : Sys.isapple() ? "libtokengine.dylib" : "libtokengine.so"
lib = get(ENV, "TOK_LIBTOK", normpath(joinpath(HERE, "..", "..", "rust", "target", "release", LIBNAME)))
lib = isfile(lib) ? lib : nothing
lib === nothing || (ENV["TOK_LIBTOK"] = lib)

create_app(APP, OUTPUT; executables=["tok" => "julia_main"], precompile_execution_file=joinpath(HERE, "precompile.jl"),
           force=true, include_lazy_artifacts=false)

if lib === nothing
    @warn "the engine library was not found: the commands of the engine need TOK_LIBTOK when the program runs"
else
    cp(lib, joinpath(OUTPUT, "bin", basename(lib)); force=true)
end
println("tok application: ", joinpath(OUTPUT, "bin"))
