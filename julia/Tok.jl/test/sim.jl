# The native simulator (Tok.Sim): what it runs, reproducibility, and a statistical
# comparison with the simulator of the engine.

const S = Tok.Sim
const SIM_FIXTURES = joinpath(ROOT, "rust", "tok-sim", "tests", "fixtures", "sim")
const KWERENDA = read(joinpath(ROOT, "examples", "kwerenda-symulacja.tok"), String)

@testset "native simulation: supported models" begin
    m = S.model(KWERENDA)
    @test S.supported(m) && m.cases == 300 && m.seed == 7
    multi = S.model(read(joinpath(SIM_FIXTURES, "sim-multi.tok"), String))
    @test !S.supported(multi)
    feats = Set(f for (_, f) in S.unsupported(multi))
    @test "multi-instance activity" in feats && "ad-hoc sub-process" in feats && "compensation" in feats
    @test_throws S.UnsupportedModel S.simulate(multi)
    rules = S.model(read(joinpath(SIM_FIXTURES, "sim-rules.tok"), String))
    @test ("r1", "decision of a rule task") in S.unsupported(rules)
    nostart = S.model(read(joinpath(SIM_FIXTURES, "sim-nostart.tok"), String))
    @test_throws S.SimError S.simulate(nostart)
end

@testset "native simulation: reproducible runs" begin
    m = S.model(KWERENDA)
    a = S.simulate(m; seed=5, replication=2)
    b = S.simulate(m; seed=5, replication=2)
    c = S.simulate(m; seed=5, replication=3)
    @test a.cycle_times == b.cycle_times && a.utilization == b.utilization
    @test a.cycle_times != c.cycle_times
    @test a.started == 300 && a.finished == 300
    @test sum(p.second for p in a.by_end_event) == 300
    @test length(a.log) == sum(x.second.count for x in a.activities)
    m3 = S.scenario(m; name="three", resources=Dict("archiwisci" => 3))
    e1 = S.experiment([m, m3]; replications=6, seed=1, threads=true)
    e2 = S.experiment([m, m3]; replications=6, seed=1, threads=false)
    @test [r.cycle_times for r in e1.runs] == [r.cycle_times for r in e2.runs]
    @test size(e1.runs) == (2, 6) && e1.scenarios == ["model", "three"]
    # common random numbers: a third archivist shortens every replication
    @test all(S.cycle_time(e1.runs[2, r]).mean < S.cycle_time(e1.runs[1, r]).mean for r in 1:6)
    @test all(S.utilization(e1.runs[2, r], "archiwisci") < S.utilization(e1.runs[1, r], "archiwisci") for r in 1:6)
    rows = S.summarize(e1)
    @test any(r -> r.scenario == "three" && r.indicator == "utilization:archiwisci" && r.n == 6, rows)
    @test length(S.indicators(e1)) > 12
    d = S.dist("lnorm(30m, 10m)")
    @test d.name === :lnorm && d.args == [1800.0, 600.0] && S.dist_text(d) == "lnorm(30m, 10m)"
    @test S.format_duration(1830.0) == "30.5m" && S.format_duration(5400.0) == "1.5h"
end

"Rounded as `tok sim` rounds its indicators."
r3(x) = round(x; digits=3)

"""
The samples compared for one model: per replication the mean cycle time, the cycle time
of the case in the middle of the run, and the utilization of every resource.
"""
function rust_samples(text, R, n)
    k = cld(n, 2)
    out = Dict{String,Vector{Float64}}("mean cycle time" => Float64[], "cycle time of case $k" => Float64[])
    warnings = Set{String}()
    for r in 1:R
        res = try
            E.simulate(text; seed=r, log=false)
        catch e
            # times beyond the calendar of the event log (year 9999): indicators only
            e isa E.TokError && occursin("OverflowError", e.message) || rethrow()
            delete!(out, "cycle time of case $k")
            E.simulate(text; seed=r, log=false, cases_table=false)
        end
        kp = res.kpi
        kp["cycle_time"]["mean_s"] === nothing || push!(out["mean cycle time"], kp["cycle_time"]["mean_s"])
        if haskey(out, "cycle time of case $k")
            c = res.cases[k]["cycle_time_s"]
            c === nothing || push!(out["cycle time of case $k"], c)
        end
        for (id, v) in kp["resources"]
            push!(get!(out, "utilization of $id", Float64[]), v["utilization"])
        end
        union!(warnings, kp["warnings"])
    end
    return out, warnings
end

function native_samples(m, R, n)
    k = cld(n, 2)
    out = Dict{String,Vector{Float64}}("mean cycle time" => Float64[], "cycle time of case $k" => Float64[])
    warnings = Set{String}()
    e = S.experiment(m; replications=R, seed=20271, log=false)
    for run in e.runs
        isempty(run.cycle_times) || push!(out["mean cycle time"], r3(S.cycle_time(run).mean))
        j = findfirst(==(k), run.case_ids)
        j === nothing || push!(out["cycle time of case $k"], r3(run.cycle_times[j]))
        for (id, u) in run.utilization
            push!(get!(out, "utilization of $id", Float64[]), round(u; digits=4))
        end
        union!(warnings, run.warnings)
    end
    return out, warnings
end

# Significance level of the comparison of one model; the tests of a model share it
# (Bonferroni), so that a model passes with probability at least 1 - ALPHA when both
# simulators have the same semantics.
const ALPHA = 0.01

# Replications per model; the model whose cases all run into the step limit is slow in both
# simulators and its cycle times are all zero, so a few replications suffice.
const REPLICATIONS = 200
const FEW_REPLICATIONS = Dict("03-kwerenda-bramki.tok" => 10)

@testset "native simulation: same distributions as the engine" begin
    models = [("examples/kwerenda-symulacja.tok", KWERENDA, REPLICATIONS)]
    for f in sort(readdir(SIM_FIXTURES))
        endswith(f, ".tok") || continue
        push!(models, ("sim/" * f, read(joinpath(SIM_FIXTURES, f), String), get(FEW_REPLICATIONS, f, REPLICATIONS)))
    end
    compared = 0
    report = String[]
    for (name, text, R) in models
        m = try
            S.model(text)
        catch e
            # documents with errors and parameters the engine rejects
            e isa E.TokError || rethrow()
            continue
        end
        (S.supported(m) && m.start != 0 && 0 < m.cases <= 5000) || continue
        n = m.cases
        rs, rw = try
            rust_samples(text, R, n)
        catch e
            # a model the engine cannot run (a label as a duration): neither can the native simulator
            e isa E.TokError && e.kind == "simulation" || rethrow()
            @test_throws S.SimError S.simulate(m)
            continue
        end
        ns, nw = native_samples(m, R, n)
        filter!(p -> haskey(rs, p.first), ns)
        @testset "$name" begin
            @test rw == nw
            @test Set(keys(rs)) == Set(keys(ns))
            tests = [k for k in keys(rs) if !isempty(rs[k]) || !isempty(ns[k])]
            for k in sort(tests)
                @test !isempty(rs[k]) && !isempty(ns[k])
                (isempty(rs[k]) || isempty(ns[k])) && continue
                t = Tok.Stats.ks_test(rs[k], ns[k])
                push!(report, rpad(name, 42) * rpad(k, 30) * " D = " * rpad(string(round(t.statistic; digits=3)), 6) *
                              " p = " * string(round(t.pvalue; digits=3)))
                @test t.pvalue > ALPHA / length(tests)
            end
        end
        compared += 1
    end
    @test compared >= 15
    if get(ENV, "TOK_SIM_REPORT", "") != ""
        println("Kolmogorov–Smirnov tests of the native simulator against the engine ($compared models, α = $ALPHA per model):")
        foreach(println, report)
    end
end

@testset "native simulation: the comparison detects a different model" begin
    # durations of the search 10% longer: the comparison must reject it
    slower = replace(KWERENDA, "duration=\"lnorm(6h, 3h)\"" => "duration=\"lnorm(6.6h, 3.3h)\"")
    rs, _ = rust_samples(KWERENDA, 200, 300)
    ns, _ = native_samples(S.model(slower), 200, 300)
    @test Tok.Stats.ks_test(rs["utilization of archiwisci"], ns["utilization of archiwisci"]).pvalue < 1e-6
    @test Tok.Stats.ks_test(rs["mean cycle time"], ns["mean cycle time"]).pvalue < 1e-6
end

@testset "statistics" begin
    @test Tok.Stats.normal_quantile(0.975) ≈ 1.959963984540054 atol = 1e-12
    @test Tok.Stats.normal_cdf(1.959963984540054) ≈ 0.975 atol = 1e-12
    t = Tok.Stats.ks_test([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])
    @test t.statistic == 0 && t.pvalue == 1
    t = Tok.Stats.ks_test(collect(1.0:100.0), collect(51.0:150.0))
    @test t.statistic ≈ 0.5 && t.pvalue < 1e-8
    x, fx = Tok.Stats.nelder_mead(v -> (v[1] - 3)^2 + (v[2] + 1)^2, [0.0, 0.0])
    @test x ≈ [3.0, -1.0] atol = 1e-4
end
