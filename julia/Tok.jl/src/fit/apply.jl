# A document with the fitted parameters, printed by the independent core.

"""
    apply(text, f::FitResult; digits=4) -> String

The document `text` with the estimates of `f` written into it: `duration=` of the
activities, `arrival=` of `simulate` (added when the document has none) and `p=` of the
flows, rounded to `digits` significant digits. The document is read and printed in
canonical form by the core of Tok.jl; comments are kept.
"""
function apply(text::AbstractString, f::FitResult; digits::Int=4)
    doc, diags = TokCore.parse_tok(text)
    errors = [d for d in diags if TokCore.severity(d) == "error"]
    isempty(errors) || throw(ArgumentError("the document has errors: " *
                                           join([TokCore.format_diag(d, "en") for d in errors], "; ")))
    for e in f.estimates
        if e.parameter === :duration
            haskey(doc.nodes, e.element) || continue
            doc.nodes[e.element].attrs["duration"] = Sim.dist_text(distribution(e); digits=digits)
        elseif e.parameter === :arrival
            if doc.simulate === nothing
                doc.simulate = TokCore.Elem("simulate", nothing)
            end
            doc.simulate.attrs["arrival"] = Sim.dist_text(distribution(e); digits=digits)
        elseif e.parameter === :p
            g, t = split(e.element, " -> ")
            for fl in doc.flows
                if fl.source == g && fl.target == t
                    fl.attrs["p"] = Sim.short_number(e.values[1]; digits=digits)
                end
            end
        end
    end
    return TokCore.fmt(doc)
end
