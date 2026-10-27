# Tok.jl

Julia implementation of the tok core: lexer, parser, text-layer diagnostics, canonical printer and semantic BPMN 2.0 export, for language versions 0.1 and 0.3 (including `adhoc` and the executable layer of spec §10.3). It produces byte-identical output to the Python reference implementation on the conformance corpus (`../conformance`): the canonical text (`expected/*.tok`), the core diagnostic codes (`expected/*.diag`) and the semantic XML (`expected/*.bpmn`).

The files `expected/*.levels` list every code of the full checker (BPMN rules, executable profile, FEEL, decisions, simulation) at the levels basic, analytic and full. The full checker is not part of the Julia core yet (roadmap 0.8), so the Julia tests do not read these files.

Requires Julia 1.10 or newer and no packages.

```
cd julia/Tok.jl
julia --project=. bin/tok.jl check  ../../examples/propozycja-zakupu.tok
julia --project=. bin/tok.jl fmt    ../../examples/propozycja-zakupu.tok
julia --project=. bin/tok.jl xml    ../../examples/propozycja-zakupu.tok
julia --project=. -e "using Pkg; Pkg.test()"      # conformance tests
```

From Julia code:

```julia
using Tok
doc, diags = Tok.parse_tok(read("model.tok", String))
print(Tok.fmt(doc))
```

Layout, SVG, FEEL, decisions, forms, import and simulation are provided by the Python implementation; see `../docs/roadmap.md` for the plan to bring them to Julia.
