# Builds the API reference of TokLang.jl into docs/build. Run from the package folder:
#
#   julia --project=docs -e 'using Pkg; Pkg.develop(path="."); Pkg.instantiate()'
#   julia --project=docs docs/make.jl
#
# The guide to tok publishes the result at https://tok-lang-bpm.github.io/julia/.

using Documenter
using TokLang

makedocs(;
    sitename="TokLang.jl",
    modules=[TokLang],
    authors="Marek Deja",
    repo=Remotes.GitHub("tok-lang-bpm", "tok"),
    format=Documenter.HTML(; canonical="https://tok-lang-bpm.github.io/julia/", edit_link="main"),
    checkdocs=:exports,
    checkdocs_ignored_modules=[TokLang.JSON, TokLang.Stats],
    doctest=false,
    pages=[
        "Home" => "index.md",
        "Stable API" => "api.md",
        "Reference" => ["core.md", "engine.md", "sim.md", "fit.md"],
    ],
)
