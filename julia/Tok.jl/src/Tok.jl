"""
    Tok

The tok language in Julia. The independent core of the package (lexer, parser,
canonical printer `fmt` and semantic BPMN 2.0 export `semantic_xml`) is a second
implementation of the specification, checked against the conformance corpus in
`conformance/`. [`Tok.Engine`](@ref) gives the whole language through the Rust engine,
[`Tok.Sim`](@ref) the native simulator for large experiments and [`Tok.Fit`](@ref) the
calibration of simulation parameters on event logs.
"""
module Tok

const VERSION_STRING = "1.0.3"

include("odict.jl")
include("diagnostics.jl")
include("lexer.jl")
include("model.jl")
include("parser.jl")
include("parser_07.jl")
include("printer.jl")
include("printer_07.jl")
include("xml.jl")
include("cli.jl")
include("json.jl")
include("stats.jl")
include("engine.jl")
include("sim.jl")
include("fit.jl")
include("app.jl")

export parse_tok, fmt, semantic_xml, Diag, message, format_diag, severity

end # module
