"""
    Tok

Julia implementation of the core of the tok language: lexer, parser, canonical
printer (`fmt`) and semantic BPMN 2.0 export (`semantic_xml`). It follows the
Python reference implementation and is checked against the shared conformance
corpus in `conformance/`.
"""
module Tok

const VERSION_STRING = "0.5.0"

include("odict.jl")
include("diagnostics.jl")
include("lexer.jl")
include("model.jl")
include("parser.jl")
include("printer.jl")
include("xml.jl")
include("cli.jl")

export parse_tok, fmt, semantic_xml, Diag, message, format_diag, severity

end # module
