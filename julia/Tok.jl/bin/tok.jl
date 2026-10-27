# Command line interface of the Julia implementation of tok.
#
#   julia --project=<path to Tok.jl> bin/tok.jl fmt FILE      canonical text
#   julia --project=<path to Tok.jl> bin/tok.jl xml FILE      semantic BPMN 2.0 XML
#   julia --project=<path to Tok.jl> bin/tok.jl check FILE    diagnostics (--lang pl for Polish)

pushfirst!(LOAD_PATH, joinpath(@__DIR__, ".."))
using Tok

exit(Tok.main(ARGS))
