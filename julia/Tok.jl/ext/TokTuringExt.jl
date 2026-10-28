# Bayesian calibration of simulation parameters with Turing.jl (`Tok.Fit.fit_bayes`).
#
# Every parameter is sampled with NUTS from its own model: the likelihood of the observed
# processing times (interrupted instances as right-censored observations), of the times
# between arrivals, or of the branch counts of a gateway, with weakly informative priors
# on the scale of the data (docs/calibration.md lists them).

module TokTuringExt

using Tok
using Turing

const F = Tok.Fit
const Random = Turing.Random

logistic(v) = 1 / (1 + exp(-v))

@model function exp_model(x, c, scale)
    logmean ~ Normal(log(scale), 3)
    d = Exponential(exp(logmean))
    Turing.@addlogprob! sum(logpdf.(d, x)) + sum(logccdf.(d, c); init=0.0)
end

@model function lnorm_model(x, c, scale)
    μ ~ Normal(log(scale), 3)
    σ ~ truncated(Normal(0, 2); lower=0)
    d = LogNormal(μ, σ)
    Turing.@addlogprob! sum(logpdf.(d, x)) + sum(logccdf.(d, c); init=0.0)
end

@model function norm_model(x, c, scale)
    μ ~ Normal(scale, 10 * scale)
    σ ~ truncated(Normal(0, 10 * scale); lower=0)
    d = Normal(μ, σ)
    Turing.@addlogprob! sum(logpdf.(d, x)) + sum(logccdf.(d, c); init=0.0)
end

# bounds beyond the observed range, relative to its width: a = lo - span e^u, b = hi + span e^w
@model function unif_model(x, lo, hi)
    span = hi - lo
    u ~ Normal(-3, 2)
    w ~ Normal(-3, 2)
    d = Uniform(lo - span * exp(u), hi + span * exp(w))
    Turing.@addlogprob! sum(logpdf.(d, x))
end

@model function tri_model(x, lo, hi)
    span = hi - lo
    u ~ Normal(-3, 2)
    w ~ Normal(-3, 2)
    v ~ Normal(0, 1.5)
    a = lo - span * exp(u)
    b = hi + span * exp(w)
    d = TriangularDist(a, b, a + (b - a) * logistic(v))
    Turing.@addlogprob! sum(logpdf.(d, x))
end

@model function branch_model(counts, total, exclusive)
    if exclusive
        # an exclusive gateway: one category per flow with p=, and the rest (default flow)
        k = length(counts)
        rest = total - sum(counts)
        θ ~ Dirichlet(rest > 0 ? k + 1 : k, 1.0)
        Turing.@addlogprob! logpdf(Multinomial(total, θ), rest > 0 ? vcat(counts, rest) : counts)
    else
        # an inclusive gateway: every flow independently
        θ ~ filldist(Beta(1, 1), length(counts))
        Turing.@addlogprob! sum(logpdf.(Binomial.(total, θ), counts))
    end
end

draws(ch, name) = Float64.(vec(Array(ch[name])))

"Draws of element `j` of a vector-valued variable (chains store them as vectors or as `name[j]`)."
function vector_draws(ch, name::Symbol, j::Int)
    a = try
        Array(ch[name])
    catch
        nothing
    end
    a !== nothing && eltype(a) <: AbstractVector && return Float64[x[j] for x in vec(a)]
    return Float64.(vec(Array(ch[Symbol("$name[$j]")])))
end

function interval(v::Vector{Float64}, level)
    q = quantile(v, [(1 - level) / 2, (1 + level) / 2])
    return (sum(v) / length(v), q[1], q[2])
end

function run_nuts(model, seed, samples, warmup)
    return sample(Random.Xoshiro(seed), model, NUTS(warmup, 0.8), samples; progress=false, verbose=false)
end

function duration_estimate(element, parameter, family, x, c, seed, samples, warmup, level)
    scale = sort(x)[cld(length(x), 2)]
    scale > 0 || (scale = 1.0)
    lo, hi = minimum(x), maximum(x)
    vals = Vector{Vector{Float64}}()
    if family === :exp
        ch = run_nuts(exp_model(x, c, scale), seed, samples, warmup)
        vals = [exp.(draws(ch, :logmean))]
    elseif family === :lnorm
        ch = run_nuts(lnorm_model(x, c, scale), seed, samples, warmup)
        μ, σ = draws(ch, :μ), draws(ch, :σ)
        m = exp.(μ .+ σ .^ 2 ./ 2)
        vals = [m, m .* sqrt.(expm1.(σ .^ 2))]
    elseif family === :norm
        ch = run_nuts(norm_model(x, c, scale), seed, samples, warmup)
        vals = [draws(ch, :μ), draws(ch, :σ)]
    elseif family === :unif
        ch = run_nuts(unif_model(x, lo, hi), seed, samples, warmup)
        span = hi - lo
        vals = [lo .- span .* exp.(draws(ch, :u)), hi .+ span .* exp.(draws(ch, :w))]
    elseif family === :tri
        ch = run_nuts(tri_model(x, lo, hi), seed, samples, warmup)
        span = hi - lo
        a = lo .- span .* exp.(draws(ch, :u))
        b = hi .+ span .* exp.(draws(ch, :w))
        vals = [a, a .+ (b .- a) .* logistic.(draws(ch, :v)), b]
    else
        m = sum(x) / length(x)
        return F.Estimate(element, parameter, family, F.param_names(family), [m], [m], [m], length(x), length(c), :bayes)
    end
    iv = [interval(v, level) for v in vals]
    return F.Estimate(element, parameter, family, F.param_names(family), [i[1] for i in iv], [i[2] for i in iv],
                      [i[3] for i in iv], length(x), length(c), :bayes)
end

function F.fit_bayes(m::Tok.Sim.SimModel, log::F.EventLog; samples::Integer=1000, warmup::Integer=500,
                     seed::Integer=1, level=0.95, families=Dict())
    obs = F.observations(m, log; families=families)
    notes = copy(obs.notes)
    est = F.Estimate[]
    k = 0
    stream() = (k += 1; Tok.Sim.stream_seed(seed, k))
    function add(element, parameter, family, x, c)
        x, c = copy(x), copy(c)
        all(==(0.0), x) && return
        if F.positive_family(family)
            n0 = count(<=(0.0), x)
            n0 > 0 && push!(notes, "$element: $n0 observations of zero length left out of the $family fit")
            filter!(>(0.0), x)
            filter!(>(0.0), c)
        end
        if family in (:unif, :tri, :fixed) && !isempty(c)
            push!(notes, "$element: $(length(c)) interrupted instances are not used by the $family fit")
            empty!(c)
        end
        length(x) >= 2 || (push!(notes, "$element: too few observations"); return)
        push!(est, duration_estimate(element, parameter, family, x, c, stream(), samples, warmup, level))
    end
    for d in obs.durations
        add(d.activity, :duration, d.family, d.times, d.censored)
    end
    isempty(obs.arrivals) || add("arrival", :arrival, obs.arrival_family, obs.arrivals, Float64[])
    for b in obs.branches
        ch = run_nuts(branch_model(b.counts, b.total, b.kind === :xor), stream(), samples, warmup)
        for (j, t) in enumerate(b.targets)
            p, lo, hi = interval(vector_draws(ch, :θ, j), level)
            push!(est, F.Estimate("$(b.gateway) -> $t", :p, :p, ["p"], [p], [lo], [hi], b.total, 0, :bayes))
        end
    end
    return F.FitResult(est, Float64(level), notes)
end

F.fit_bayes(text::AbstractString, log::F.EventLog; kw...) = F.fit_bayes(Tok.Sim.model(text), log; kw...)

end # module TokTuringExt
