"""
    Tok.Fit

Calibration of simulation parameters on event logs (the `fit` layer, see
`docs/calibration.md`): processing times of activities, times between arrivals and
branch probabilities, estimated from the XES or CSV event logs of `tok sim` or of a
real process. [`fit`](@ref) gives maximum-likelihood estimates with confidence
intervals; `fit_bayes` gives posterior estimates with credible intervals when Turing.jl
is loaded (`using Turing`). [`apply`](@ref) writes the estimates into the document.

```julia
log = Tok.Fit.read_xes("log.xes")
f = Tok.Fit.fit(text, log)
Tok.Fit.table(f)
fitted = Tok.Fit.apply(text, f)
```
"""
module Fit

using ..Sim
using ..Stats
# the parser and printer of the independent core
const TokCore = parentmodule(@__MODULE__)

export EventLog, Estimate, FitResult

include("fit/log.jl")
include("fit/observe.jl")
include("fit/mle.jl")
include("fit/apply.jl")

"""
    fit_bayes(model, log; samples=1000, warmup=500, seed=1, level=0.95, families=Dict())
        -> FitResult

Bayesian calibration with Turing.jl: the same parameters as [`fit`](@ref), with weakly
informative priors, sampled with NUTS; the estimates are posterior means and the
intervals equal-tailed credible intervals. Available after `using Turing` (the package
extension `TokTuringExt`).
"""
function fit_bayes(args...; kwargs...)
    throw(ErrorException("fit_bayes needs Turing.jl: install it and load it with `using Turing`"))
end

end # module Fit
