# Maximum-likelihood estimates of the parameters, with confidence intervals.

"""
    Estimate

One fitted distribution or probability: `element` (an activity identifier, `"arrival"`,
or a flow `"GATEWAY -> TARGET"`), `parameter` (`:duration`, `:arrival` or `:p`),
`family` (`:exp`, `:norm`, `:lnorm`, `:unif`, `:tri`, `:fixed`, or `:p` for a
probability), the parameter `names` of the language (`mean`, `sd`, `a`, `mode`, `b`,
`x`, `p`) with their `values`, `lower` and `upper` bounds of the interval (seconds for
durations), the number of observations `n` (for a probability, the visits of the
gateway), `censored` (interrupted instances used as censored observations) and `method`
(`:mle` or `:bayes`).
"""
struct Estimate
    element::String
    parameter::Symbol
    family::Symbol
    names::Vector{String}
    values::Vector{Float64}
    lower::Vector{Float64}
    upper::Vector{Float64}
    n::Int
    censored::Int
    method::Symbol
end

"""
    FitResult

The estimates of [`fit`](@ref) (or of `fit_bayes`), the `level` of their intervals and
`notes` on parameters that could not be estimated.
"""
struct FitResult
    estimates::Vector{Estimate}
    level::Float64
    notes::Vector{String}
end

Base.show(io::IO, f::FitResult) =
    print(io, "FitResult(", length(f.estimates), " estimates, ",
          isempty(f.estimates) ? "none" : String(f.estimates[1].method), ", ", length(f.notes), " notes)")

"""
    table(f::FitResult) -> Vector{NamedTuple}

The estimates in long form, one row per parameter: `(element, parameter, family, name,
estimate, lower, upper, n, censored, method)`.
"""
function table(f::FitResult)
    out = NamedTuple{(:element, :parameter, :family, :name, :estimate, :lower, :upper, :n, :censored, :method),
                     Tuple{String,Symbol,Symbol,String,Float64,Float64,Float64,Int,Int,Symbol}}[]
    for e in f.estimates, k in eachindex(e.names)
        push!(out, (element=e.element, parameter=e.parameter, family=e.family, name=e.names[k], estimate=e.values[k],
                    lower=e.lower[k], upper=e.upper[k], n=e.n, censored=e.censored, method=e.method))
    end
    return out
end

"The distribution of the language for a fitted duration (`nothing` for a probability)."
function distribution(e::Estimate)
    e.family === :p && return nothing
    args = e.family === :fixed ? e.values[1:1] : e.values
    return Sim.Dist(e.family === :fixed ? :fixed : e.family, :duration, args, Any[])
end

param_names(f::Symbol) = f === :exp ? ["mean"] : f in (:norm, :lnorm) ? ["mean", "sd"] :
                         f === :unif ? ["a", "b"] : f === :tri ? ["a", "mode", "b"] : ["x"]

# ---------------------------------------------------------------- likelihoods

"Negative log-likelihood of (μ, log σ) for normal data with right-censored observations."
function normal_nll(θ, x, c)
    μ, σ = θ[1], exp(θ[2])
    s = 0.0
    for v in x
        z = (v - μ) / σ
        s += 0.5 * z * z + log(σ) + 0.5 * log(2π)
    end
    for v in c
        s -= Stats.normal_logccdf((v - μ) / σ)
    end
    return s
end

"Numerical Hessian of `f` at `θ` (central differences)."
function hessian(f, θ::Vector{Float64})
    n = length(θ)
    H = zeros(n, n)
    h = [1e-4 * max(abs(t), 1.0) for t in θ]
    for i in 1:n, j in i:n
        e_i = zeros(n); e_i[i] = h[i]
        e_j = zeros(n); e_j[j] = h[j]
        v = (f(θ + e_i + e_j) - f(θ + e_i - e_j) - f(θ - e_i + e_j) + f(θ - e_i - e_j)) / (4h[i] * h[j])
        H[i, j] = H[j, i] = v
    end
    return H
end

"Invert a small symmetric matrix; `nothing` when it is not positive definite."
function covariance(H)
    try
        C = inv(H)
        all(C[i, i] > 0 for i in axes(C, 1)) || return nothing
        return C
    catch
        return nothing
    end
end

"Numerical gradient of a scalar function."
function gradient(g, θ::Vector{Float64})
    out = similar(θ)
    for i in eachindex(θ)
        h = 1e-6 * max(abs(θ[i]), 1.0)
        e = zeros(length(θ)); e[i] = h
        out[i] = (g(θ + e) - g(θ - e)) / (2h)
    end
    return out
end

"A Wald interval of a positive quantity g(θ), on the log scale (delta method)."
function log_interval(g, θ, C, z)
    v = g(θ)
    C === nothing && return (v, NaN, NaN)
    d = gradient(t -> log(g(t)), θ)
    se = sqrt(max(d' * C * d, 0.0))
    return (v, v * exp(-z * se), v * exp(z * se))
end

"Fit (μ, log σ) of normal data, closed form without censoring, numerical with it."
function normal_mle(x, c)
    μ = sum(x) / length(x)
    σ = sqrt(sum((x .- μ) .^ 2) / length(x))
    σ = σ > 0 ? σ : max(abs(μ) * 1e-6, 1e-9)
    θ = [μ, log(σ)]
    if !isempty(c)
        θ, _ = Stats.nelder_mead(t -> normal_nll(t, x, c), θ; step=0.05)
    end
    C = covariance(hessian(t -> normal_nll(t, x, c), θ))
    return θ, C
end

# --------------------------------------------------------------------- families

"Maximum-likelihood fit of one family: (values, lower, upper) in the parameters of the language."
function mle_family(family::Symbol, x::Vector{Float64}, c::Vector{Float64}, z::Float64; bootstrap::Int, rng)
    n = length(x)
    if family === :exp
        m = (sum(x) + sum(c)) / n
        return ([m], [m * exp(-z / sqrt(n))], [m * exp(z / sqrt(n))])
    elseif family === :norm
        θ, C = normal_mle(x, c)
        σ = exp(θ[2])
        se_μ = C === nothing ? NaN : sqrt(C[1, 1])
        se_l = C === nothing ? NaN : sqrt(C[2, 2])
        return ([θ[1], σ], [θ[1] - z * se_μ, σ * exp(-z * se_l)], [θ[1] + z * se_μ, σ * exp(z * se_l)])
    elseif family === :lnorm
        θ, C = normal_mle(log.(x), log.(c))
        mean_of(t) = exp(t[1] + exp(2t[2]) / 2)
        sd_of(t) = mean_of(t) * sqrt(expm1(exp(2t[2])))
        m, ml, mu = log_interval(mean_of, θ, C, z)
        s, sl, su = log_interval(sd_of, θ, C, z)
        return ([m, s], [ml, sl], [mu, su])
    elseif family === :unif || family === :tri
        v = family === :unif ? [minimum(x), maximum(x)] : tri_mle(x)
        lo, hi = bootstrap_interval(family, v, n, z, bootstrap, rng)
        return (v, lo, hi)
    elseif family === :fixed
        m = sum(x) / n
        return ([m], [m], [m])
    end
    throw(ArgumentError("unknown family $family"))
end

"""
Maximum-likelihood estimate of a triangular distribution (a, mode, b): for given bounds
the most likely mode is one of the observations (Oliver 1972), so the bounds are found by
Nelder–Mead on the profile likelihood.
"""
function tri_mle(x::Vector{Float64})
    v = sort(x)
    n = length(v)
    lo, hi = v[1], v[end]
    span = hi - lo
    span > 0 || return [lo, lo, hi]
    function best_mode(a, b)
        # log-likelihood for mode = v[k], with prefix sums of log(x - a) and suffix sums of log(b - x)
        la = log.(v .- a)
        lb = log.(b .- v)
        pre = cumsum(la)
        suf = reverse(cumsum(reverse(lb)))
        best, bk = -Inf, 1
        # a < v[1] and b > v[end], so every logarithm is finite
        for k in 1:n
            c = v[k]
            ll = (k > 1 ? pre[k-1] : 0.0) - (k - 1) * log(c - a) + suf[k] - (n - k + 1) * log(b - c)
            if ll > best
                best, bk = ll, k
            end
        end
        return best + n * log(2.0) - n * log(b - a), v[bk]
    end
    # bounds outside the data: a = lo - exp(u) span, b = hi + exp(w) span
    f(t) = -best_mode(lo - exp(t[1]) * span, hi + exp(t[2]) * span)[1]
    t, _ = Stats.nelder_mead(f, [log(1 / n), log(1 / n)]; step=0.5, tol=1e-12)
    a, b = lo - exp(t[1]) * span, hi + exp(t[2]) * span
    return [a, best_mode(a, b)[2], b]
end

"Percentile interval from the parametric bootstrap of a uniform or triangular fit."
function bootstrap_interval(family, v, n, z, B, rng)
    B > 0 || return (fill(NaN, length(v)), fill(NaN, length(v)))
    d = Sim.Dist(family, :number, v, Any[])
    reps = Vector{Vector{Float64}}()
    for _ in 1:B
        y = [Sim.sample(d, rng)::Float64 for _ in 1:n]
        push!(reps, family === :unif ? [minimum(y), maximum(y)] : tri_mle(y))
    end
    α = 2 * (1 - Stats.normal_cdf(z))
    lo = [Sim.sim_quantile([r[k] for r in reps], α / 2) for k in eachindex(v)]
    hi = [Sim.sim_quantile([r[k] for r in reps], 1 - α / 2) for k in eachindex(v)]
    return (lo, hi)
end

"Wilson score interval of a proportion."
function wilson(k, n, z)
    p = k / n
    d = 1 + z^2 / n
    c = (p + z^2 / (2n)) / d
    h = z * sqrt(p * (1 - p) / n + z^2 / (4n^2)) / d
    return (p, max(0.0, c - h), min(1.0, c + h))
end

positive_family(f) = f in (:exp, :lnorm)

"""
    fit(model, log; level=0.95, families=Dict(), bootstrap=200, seed=1) -> FitResult

Maximum-likelihood calibration of a simulation model on an event log (see
`docs/calibration.md`). `model` is a [`Sim.SimModel`](@ref) or the text of a document,
`log` an [`EventLog`](@ref). The estimates are the processing time of every activity
with observed instances (in the family the model declares, or the one in `families`),
the time between arrivals, and the probabilities `p=` of the branches of `xor` and `or`
gateways whose branches can be told apart in the log. Instances interrupted by a
boundary event are right-censored observations. Intervals are Wald intervals at `level`
(on the log scale for positive parameters, Wilson intervals for probabilities) and, for
the uniform and triangular families, percentile intervals of a parametric bootstrap with
`bootstrap` replications drawn from the stream `seed`.
"""
function fit(m::Sim.SimModel, log::EventLog; level=0.95, families=Dict(), bootstrap::Int=200, seed::Integer=1)
    obs = observations(m, log; families=families)
    return fit(obs; level=level, bootstrap=bootstrap, seed=seed)
end

fit(text::AbstractString, log::EventLog; kw...) = fit(Sim.model(text), log; kw...)

function fit(obs::Observations; level=0.95, bootstrap::Int=200, seed::Integer=1)
    z = Stats.normal_quantile(0.5 + level / 2)
    rng = Sim.Rng(Sim.stream_seed(seed, 0))
    notes = copy(obs.notes)
    est = Estimate[]
    function add(element, parameter, family, x, c)
        x = copy(x)
        c = copy(c)
        if all(==(0.0), x)
            return
        end
        if positive_family(family)
            k = count(<=(0.0), x)
            k > 0 && push!(notes, "$element: $k observations of zero length left out of the $family fit")
            filter!(>(0.0), x)
            filter!(>(0.0), c)
        end
        if family in (:unif, :tri, :fixed) && !isempty(c)
            push!(notes, "$element: $(length(c)) interrupted instances are not used by the $family fit")
            empty!(c)
        end
        length(x) >= 2 || (push!(notes, "$element: too few observations"); return)
        v, lo, hi = mle_family(family, x, c, z; bootstrap=bootstrap, rng=rng)
        push!(est, Estimate(element, parameter, family, param_names(family), v, lo, hi, length(x), length(c), :mle))
    end
    for d in obs.durations
        add(d.activity, :duration, d.family, d.times, d.censored)
    end
    isempty(obs.arrivals) || add("arrival", :arrival, obs.arrival_family, obs.arrivals, Float64[])
    for b in obs.branches
        for (t, k) in zip(b.targets, b.counts)
            p, lo, hi = wilson(k, b.total, z)
            push!(est, Estimate("$(b.gateway) -> $t", :p, :p, ["p"], [p], [lo], [hi], b.total, 0, :mle))
        end
    end
    return FitResult(est, Float64(level), notes)
end
