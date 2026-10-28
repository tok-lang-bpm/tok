# Statistical functions used by the native simulator and the calibration, without
# dependencies: the normal distribution, the two-sample Kolmogorov–Smirnov test, and
# Nelder–Mead minimization.

"""
    Tok.Stats

The normal distribution (`normal_cdf`, `normal_logccdf`, `normal_quantile`), the
two-sample Kolmogorov–Smirnov test (`ks_test`) and Nelder–Mead minimization
(`nelder_mead`).
"""
module Stats

export ks_test, normal_cdf, normal_quantile, nelder_mead

erfc(x::Float64) = ccall((:erfc, Base.Math.libm), Float64, (Float64,), x)

"Standard normal distribution function."
normal_cdf(z::Real) = 0.5 * erfc(-Float64(z) / sqrt(2.0))

"Logarithm of the standard normal survival function, accurate far in the tail."
function normal_logccdf(z::Real)
    z = Float64(z)
    z < 30 && return log(0.5 * erfc(z / sqrt(2.0)))
    # asymptotic expansion of the Mills ratio
    return -z^2 / 2 - log(z) - 0.5 * log(2π) + log(1 - 1 / z^2 + 3 / z^4)
end

"""
    normal_quantile(p) -> z

Quantile of the standard normal distribution (Acklam's approximation, refined by one
Halley step to full double precision).
"""
function normal_quantile(p::Real)
    p = Float64(p)
    0 < p < 1 || return p == 0 ? -Inf : p == 1 ? Inf : NaN
    a = (-3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02, 1.383577518672690e+02,
         -3.066479806614716e+01, 2.506628277459239e+00)
    b = (-5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02, 6.680131188771972e+01,
         -1.328068155288572e+01)
    c = (-7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00, -2.549732539343734e+00,
         4.374664141464968e+00, 2.938163982698783e+00)
    d = (7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00, 3.754408661907416e+00)
    plow = 0.02425
    x = if p < plow
        q = sqrt(-2log(p))
        (((((c[1]q + c[2])q + c[3])q + c[4])q + c[5])q + c[6]) / ((((d[1]q + d[2])q + d[3])q + d[4])q + 1)
    elseif p <= 1 - plow
        q = p - 0.5
        r = q * q
        (((((a[1]r + a[2])r + a[3])r + a[4])r + a[5])r + a[6])q / (((((b[1]r + b[2])r + b[3])r + b[4])r + b[5])r + 1)
    else
        q = sqrt(-2log(1 - p))
        -(((((c[1]q + c[2])q + c[3])q + c[4])q + c[5])q + c[6]) / ((((d[1]q + d[2])q + d[3])q + d[4])q + 1)
    end
    e = normal_cdf(x) - p
    u = e * sqrt(2π) * exp(x * x / 2)
    return x - u / (1 + x * u / 2)
end

"""
    kolmogorov_q(λ) -> probability

The limiting distribution of the Kolmogorov–Smirnov statistic: P(K > λ) =
2 Σ (-1)^(k-1) exp(-2 k² λ²).
"""
function kolmogorov_q(λ::Float64)
    λ < 1e-3 && return 1.0
    s = 0.0
    for k in 1:200
        term = 2 * (-1)^(k - 1) * exp(-2 * k^2 * λ^2)
        s += term
        abs(term) < 1e-12 && break
    end
    return clamp(s, 0.0, 1.0)
end

"""
    ks_test(x, y) -> (statistic, pvalue)

Two-sample Kolmogorov–Smirnov test of the hypothesis that `x` and `y` come from the
same continuous distribution. The statistic is the largest distance between the
empirical distribution functions; the p-value is the asymptotic one with the
correction of Stephens (1970): Q((√nₑ + 0.12 + 0.11/√nₑ) D) with nₑ = n m / (n + m).
"""
function ks_test(x::AbstractVector{<:Real}, y::AbstractVector{<:Real})
    a = sort(Float64.(x))
    b = sort(Float64.(y))
    n, m = length(a), length(b)
    (n == 0 || m == 0) && throw(ArgumentError("both samples must be non-empty"))
    i = j = 1
    d = 0.0
    while i <= n && j <= m
        v = min(a[i], b[j])
        while i <= n && a[i] == v
            i += 1
        end
        while j <= m && b[j] == v
            j += 1
        end
        d = max(d, abs((i - 1) / n - (j - 1) / m))
    end
    ne = n * m / (n + m)
    p = kolmogorov_q((sqrt(ne) + 0.12 + 0.11 / sqrt(ne)) * d)
    return (statistic=d, pvalue=p)
end

"""
    nelder_mead(f, x0; step=0.1, tol=1e-10, maxiter=5000) -> (x, fx)

Minimize `f` from the starting point `x0` with the simplex method of Nelder and Mead
(standard coefficients; the initial simplex has edges `step` along each coordinate,
relative to the coordinate when it is not zero).
"""
function nelder_mead(f, x0::AbstractVector{<:Real}; step=0.1, tol=1e-10, maxiter=5000)
    n = length(x0)
    pts = [Float64.(x0)]
    for k in 1:n
        x = Float64.(x0)
        x[k] = x[k] == 0 ? step : x[k] * (1 + step)
        push!(pts, x)
    end
    vals = [f(p) for p in pts]
    for _ in 1:maxiter
        order = sortperm(vals)
        pts, vals = pts[order], vals[order]
        abs(vals[end] - vals[1]) <= tol * (abs(vals[1]) + tol) && break
        c = sum(pts[1:n]) / n
        xr = c + (c - pts[end])
        fr = f(xr)
        if fr < vals[1]
            xe = c + 2 * (c - pts[end])
            fe = f(xe)
            pts[end], vals[end] = fe < fr ? (xe, fe) : (xr, fr)
        elseif fr < vals[n]
            pts[end], vals[end] = xr, fr
        else
            xc = fr < vals[end] ? c + 0.5 * (xr - c) : c + 0.5 * (pts[end] - c)
            fc = f(xc)
            if fc < min(fr, vals[end])
                pts[end], vals[end] = xc, fc
            else
                for k in 2:n+1
                    pts[k] = pts[1] + 0.5 * (pts[k] - pts[1])
                    vals[k] = f(pts[k])
                end
            end
        end
    end
    k = argmin(vals)
    return (pts[k], vals[k])
end

end # module Stats
