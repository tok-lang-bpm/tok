# Build the `tok` application of TokLang.jl with PackageCompiler.
#
#   julia julia/app/build.jl [OUTPUT]          default OUTPUT: julia/app/build/tok
#
# The result is a folder with bin/tok (bin/tok.exe on Windows), the Julia runtime and the
# compiled code of TokLang.jl, with the engine library of tok_jll as an artifact.
# PackageCompiler is installed into a temporary environment; it needs a C compiler (on
# Windows it downloads one).

using Pkg

const HERE = @__DIR__
const APP = joinpath(HERE, "TokApp")
const OUTPUT = length(ARGS) >= 1 ? abspath(ARGS[1]) : joinpath(HERE, "build", "tok")

# the application environment, with TokLang.jl from this repository
Pkg.activate(APP)
Pkg.develop(path=normpath(joinpath(HERE, "..", "TokLang.jl")))
Pkg.instantiate()

Pkg.activate(; temp=true)
Pkg.add("PackageCompiler")
using PackageCompiler

create_app(APP, OUTPUT; executables=["tok" => "julia_main"], precompile_execution_file=joinpath(HERE, "precompile.jl"),
           force=true, include_lazy_artifacts=false)

println("tok application: ", joinpath(OUTPUT, "bin"))
