# Random numbers and the distributions of the simulation statements.

"""
    Rng

The random number generator of the native simulator: xoshiro256** (Blackman and
Vigna), seeded through SplitMix64. Its stream depends only on the seed, not on the Julia
version or the platform, so a replication gives the same result everywhere.
"""
mutable struct Rng
    s0::UInt64
    s1::UInt64
    s2::UInt64
    s3::UInt64
end

@inline function splitmix(x::UInt64)
    x += 0x9e3779b97f4a7c15
    z = x
    z = (z ⊻ (z >> 30)) * 0xbf58476d1ce4e5b9
    z = (z ⊻ (z >> 27)) * 0x94d049bb133111eb
    return x, z ⊻ (z >> 31)
end

function Rng(seed::UInt64)
    x, a = splitmix(seed)
    x, b = splitmix(x)
    x, c = splitmix(x)
    _, d = splitmix(x)
    return Rng(a, b, c, d)
end

@inline rotl(x::UInt64, k) = (x << k) | (x >> (64 - k))

@inline function next!(r::Rng)
    result = rotl(r.s1 * 0x0000000000000005, 7) * 0x0000000000000009
    t = r.s1 << 17
    r.s2 ⊻= r.s0
    r.s3 ⊻= r.s1
    r.s1 ⊻= r.s2
    r.s0 ⊻= r.s3
    r.s2 ⊻= t
    r.s3 = rotl(r.s3, 45)
    return result
end

"A uniform number in [0, 1) with 53 random bits."
@inline uniform(r::Rng) = (next!(r) >> 11) * 0x1.0p-53

"A standard normal number (Box–Muller)."
@inline function normal(r::Rng)
    u1 = 1.0 - uniform(r)  # in (0, 1]
    u2 = uniform(r)
    return sqrt(-2.0 * log(u1)) * cospi(2.0 * u2)
end

"""
    stream_seed(seed, parts...) -> UInt64

The seed of one independent stream: a base seed (any integer) mixed with the indices of
a scenario and a replication.
"""
function stream_seed(seed::Integer, parts::Integer...)
    h = UInt64(mod(seed, BigInt(2)^64))
    _, h = splitmix(h)
    for p in parts
        _, h = splitmix(h ⊻ UInt64(mod(p, BigInt(2)^64)))
    end
    return h
end

"""
    Dist(name, kind, args, labels)

A distribution of the simulation statements as the engine parses it: `name` is one of
`:fixed`, `:unif`, `:tri`, `:norm`, `:lnorm`, `:exp`, `:bern`, `:cat`; `kind` is
`:duration` (arguments in seconds), `:number`, `:bool` or `:cat`; `labels` are the
labels of `cat` (text, booleans or numbers) with the probabilities in `args`.
"""
struct Dist
    name::Symbol
    kind::Symbol
    args::Vector{Float64}
    labels::Vector{Any}
end

Dist(d::AbstractDict) = Dist(Symbol(d["name"]), Symbol(d["kind"]), Float64[Float64(a) for a in d["args"]],
                             Vector{Any}(d["labels"]))

"""
    dist(text) -> Dist

Parse a distribution with the engine, for example `dist("lnorm(30m, 10m)")`.
"""
dist(text::AbstractString) = Dist(Engine.sim_dist(text))

"A value drawn from a distribution: a number (seconds for durations), a boolean or a label."
const Sample = Union{Float64,Bool,String}

label_sample(l) = l isa Bool ? l : l isa Number ? Float64(l) : String(l)

function sample(d::Dist, rng::Rng)::Sample
    a = d.args
    n = d.name
    x = if n === :fixed
        a[1]
    elseif n === :unif
        a[1] + (a[2] - a[1]) * uniform(rng)
    elseif n === :tri
        lo, mode, hi = a[1], a[2], a[3]
        hi == lo && return d.kind === :duration && lo < 0 ? 0.0 : lo
        u = uniform(rng)
        c = (mode - lo) / (hi - lo)
        u < c ? lo + sqrt(u * (hi - lo) * (mode - lo)) : hi - sqrt((1 - u) * (hi - lo) * (hi - mode))
    elseif n === :norm
        a[1] + a[2] * normal(rng)
    elseif n === :lnorm
        mean, sd = a[1], a[2]
        s2 = log(1.0 + (sd * sd) / (mean * mean))
        exp(log(mean) - s2 / 2 + sqrt(s2) * normal(rng))
    elseif n === :exp
        -a[1] * log(1.0 - uniform(rng))
    elseif n === :bern
        return uniform(rng) < a[1]
    elseif n === :cat
        u = uniform(rng)
        acc = 0.0
        for (label, p) in zip(d.labels, a)
            acc += p
            u < acc && return label_sample(label)
        end
        return isempty(d.labels) ? 0.0 : label_sample(d.labels[end])
    else
        throw(SimError("unknown distribution $n"))
    end
    return d.kind === :duration && x < 0 ? 0.0 : x
end

"Mean of a distribution (numbers and durations; `bern` gives its probability)."
function dist_mean(d::Dist)
    a = d.args
    d.name === :fixed && return a[1]
    d.name === :unif && return (a[1] + a[2]) / 2
    d.name === :tri && return (a[1] + a[2] + a[3]) / 3
    d.name in (:norm, :lnorm, :exp) && return a[1]
    d.name === :bern && return a[1]
    return NaN
end

const UNITS = ((86400.0, "d"), (3600.0, "h"), (60.0, "m"), (1.0, "s"))

"A number with at most `digits` significant digits, without a trailing `.0`."
function short_number(x::Float64; digits=4)
    x == 0 && return "0"
    r = round(x; sigdigits=digits)
    t = string(r)
    if occursin('e', t)
        # the language reads plain decimals only: digits and one decimal point
        t = Base.Ryu.writefixed(r, max(0, digits - 1 - floor(Int, log10(abs(r)))))
        occursin('.', t) && (t = rstrip(rstrip(t, '0'), '.'))
    end
    endswith(t, ".0") && (t = t[1:end-2])
    return String(t)
end

"""
    format_duration(seconds; digits=4) -> String

A duration in the notation of the language with the largest unit that keeps the value at
least 1, for example `format_duration(1830.0) == "30.5m"`.
"""
function format_duration(s::Real; digits=4)
    s = Float64(s)
    s <= 0 && return "0s"
    for (f, u) in UNITS
        s >= f && return short_number(s / f; digits=digits) * u
    end
    return short_number(s; digits=digits) * "s"
end

"""
    dist_text(d; digits=4) -> String

A distribution in the notation of the language, for example `"lnorm(30.5m, 9.8m)"`.
"""
function dist_text(d::Dist; digits=4)
    arg(x) = d.kind === :duration ? format_duration(x; digits=digits) : short_number(x; digits=digits)
    if d.name === :cat
        parts = String[]
        for (l, p) in zip(d.labels, d.args)
            push!(parts, string(l isa Bool ? (l ? "true" : "false") : l isa Number ? short_number(Float64(l)) : l,
                                "=", short_number(p; digits=digits)))
        end
        return "cat(" * join(parts, ", ") * ")"
    end
    d.name === :fixed && return arg(d.args[1])
    return string(d.name, "(", join(arg.(d.args), ", "), ")")
end

Base.show(io::IO, d::Dist) = print(io, "Dist(\"", dist_text(d), "\")")
