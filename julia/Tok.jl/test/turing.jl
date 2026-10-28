# Bayesian calibration with Turing.jl (the extension TokTuringExt). Not part of
# `Pkg.test()`: run it in an environment that has Tok and Turing,
#
#   julia --project=ENV -e 'using Pkg; Pkg.develop(path="julia/Tok.jl"); Pkg.add("Turing")'
#   julia --project=ENV julia/Tok.jl/test/turing.jl

using Test
using Tok
using Turing

const ROOT = normpath(joinpath(@__DIR__, "..", "..", ".."))
const F = Tok.Fit

@testset "Bayesian calibration (Turing)" begin
    text = read(joinpath(ROOT, "examples", "kwerenda-symulacja.tok"), String)
    r = Tok.Engine.simulate(text; cases=800, seed=5, log=false, cases_table=false, xes=true)
    log = F.read_xes(r.xes)
    f = F.fit_bayes(text, log; samples=400, warmup=300, seed=1)
    m = F.fit(text, log; bootstrap=0)
    value_of(f, el, name) = only(e.values[findfirst(==(name), e.names)] for e in f.estimates if e.element == el)
    getiv(f, el, name) = only((e.lower[k], e.upper[k]) for e in f.estimates if e.element == el
                              for k in (findfirst(==(name), e.names),))
    @test all(e -> e.method === :bayes, f.estimates)
    for (el, name, truth, rel) in (("ocena", "mean", 1800, 0.05), ("poszukiwania", "mean", 21600, 0.08),
                                   ("arrival", "mean", 14400, 0.08), ("mozliwa -> poszukiwania", "p", 0.85, 0.04))
        @test abs(value_of(f, el, name) - truth) <= rel * truth
        lo, hi = getiv(f, el, name)
        @test lo <= truth <= hi
        # with weakly informative priors the posterior mean is close to the likelihood estimate
        @test abs(value_of(f, el, name) - value_of(m, el, name)) <= 0.03 * truth
    end
    @test abs(value_of(f, "rejestracja", "mode") - 600) < 0.1 * 900
    fitted = F.apply(text, f)
    @test isempty(filter(d -> d["severity"] == "error", Tok.Engine.check(fitted)))
end
