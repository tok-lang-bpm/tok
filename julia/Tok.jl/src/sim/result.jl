# Runs, indicators and experiments of the native simulator.

"Indicators of one activity: completed instances, mean wait and mean processing time (s)."
struct ActivityKpi
    label::String
    count::Int
    mean_wait_s::Float64
    mean_processing_s::Float64
end

"""
    Run

One simulation run of the native simulator: `cycle_times` (seconds) and `end_events` of
the finished cases in case order, with their numbers in `case_ids`; `by_end_event`, the
`activities` and the `utilization` of every resource (as in `tok sim`), `horizon_s`,
`warnings`, and, when the run was asked to keep it, the event log `log` (one named tuple
per activity instance, times in seconds from the simulation start).
"""
struct Run
    scenario::String
    replication::Int
    seed::UInt64
    started::Int
    finished::Int
    case_ids::Vector{Int}
    cycle_times::Vector{Float64}
    end_events::Vector{String}
    by_end_event::Vector{Pair{String,Int}}
    activities::Vector{Pair{String,ActivityKpi}}
    utilization::Vector{Pair{String,Float64}}
    horizon_s::Float64
    warnings::Vector{String}
    log::Vector{NamedTuple{(:case, :activity, :label, :resource, :enqueue, :start, :end, :status),
                           Tuple{Int,String,String,String,Float64,Float64,Float64,String}}}
end

Base.show(io::IO, r::Run) =
    print(io, "Run(\"", r.scenario, "\", replication ", r.replication, ": ", r.started, " cases, mean cycle time ",
          isempty(r.cycle_times) ? "-" : format_duration(mean_of(r.cycle_times)), ")")

mean_of(x) = isempty(x) ? NaN : sum(x) / length(x)

"The quantile of `tok sim`: the element at position ceil(q n) of the sorted values."
function sim_quantile(x::Vector{Float64}, q)
    isempty(x) && return NaN
    v = sort(x)
    k = max(ceil(Int, q * length(v)), 1)
    return v[k]
end

"Mean, median and 90th percentile of the cycle times of a run, in seconds."
cycle_time(r::Run) = (mean=mean_of(r.cycle_times), median=sim_quantile(r.cycle_times, 0.5),
                      p90=sim_quantile(r.cycle_times, 0.9))

"Utilization of a resource in a run (busy time / (count × simulated period))."
utilization(r::Run, resource::AbstractString) = Dict(r.utilization)[resource]

function collect_run(st::State, scenario::String, replication::Int, seed::UInt64, keep_log::Bool)
    m = st.m
    finished = [c for c in st.cases if !isnan(c.finish)]
    by_end = Pair{String,Int}[]
    for c in finished
        k = findfirst(p -> p.first == c.end_event, by_end)
        k === nothing ? push!(by_end, c.end_event => 1) : (by_end[k] = c.end_event => by_end[k].second + 1)
    end
    acts = Pair{String,Vector{Float64}}[]   # id => [count, wait, processing]
    for r in st.rows
        r.completed || continue
        id = m.nodes[r.activity].id
        k = findfirst(p -> p.first == id, acts)
        if k === nothing
            push!(acts, id => zeros(3))
            k = length(acts)
        end
        s = isnan(r.start) ? r.enqueue : r.start
        v = acts[k].second
        v[1] += 1
        v[2] += s - r.enqueue
        v[3] += (isnan(r.finish) ? s : r.finish) - s
    end
    activities = [id => ActivityKpi(m.nodes[m.index[id]].label, Int(v[1]), v[2] / v[1], v[3] / v[1]) for (id, v) in acts]
    horizon = st.t
    util = [m.resource_ids[k] => (m.counts[k] * horizon > 0 ? st.busy_time[k] / (m.counts[k] * horizon) : 0.0)
            for k in eachindex(m.resource_ids)]
    warnings = unique(st.warnings)
    log = keep_log ? [(case=r.case, activity=m.nodes[r.activity].id, label=m.nodes[r.activity].label,
                       resource=(b = m.nodes[r.activity].by; b == 0 ? "" : m.resource_ids[b]),
                       enqueue=r.enqueue, start=r.start, var"end"=r.finish,
                       status=r.completed ? "completed" : "interrupted") for r in st.rows] :
          NamedTuple{(:case, :activity, :label, :resource, :enqueue, :start, :end, :status),
                     Tuple{Int,String,String,String,Float64,Float64,Float64,String}}[]
    return Run(scenario, replication, seed, length(st.cases), length(finished), [c.id for c in finished],
               [c.finish - c.start for c in finished], [c.end_event for c in finished], by_end, activities, util,
               horizon, warnings, log)
end

function check_supported(m::SimModel)
    isempty(m.unsupported) || throw(UnsupportedModel(m.unsupported))
end

"""
    simulate(m; seed=m.seed, replication=1, cases=m.cases, log=true) -> Run

One run of the native simulator. The random stream depends only on `seed` and
`replication`, so the same arguments always give the same run.
"""
function simulate(m::SimModel; seed::Integer=m.seed, replication::Integer=1, cases::Integer=m.cases, log::Bool=true,
             scenario::AbstractString=m.name, stream::UInt64=stream_seed(seed, replication))
    check_supported(m)
    st = State(m, Rng(stream))
    simulate_run!(st, Int(cases))
    return collect_run(st, String(scenario), Int(replication), stream, log)
end

"""
    Experiment

The runs of [`experiment`](@ref): `runs[s, r]` is replication `r` of scenario `s`;
`scenarios` holds the names.
"""
struct Experiment
    scenarios::Vector{String}
    runs::Matrix{Run}
end

Base.show(io::IO, e::Experiment) =
    print(io, "Experiment(", length(e.scenarios), " scenarios × ", size(e.runs, 2), " replications)")

"""
    experiment(models; replications=10, seed=nothing, cases=nothing, threads=true,
               common_random_numbers=true, log=false) -> Experiment

Run every scenario `replications` times. `models` is one [`SimModel`](@ref), a vector
of them, or pairs `name => model` (or `name => text of a document`). With
`threads=true` the runs are distributed over the threads of Julia (`julia -t N`); the
result does not depend on the number of threads, because every run has its own random
stream, derived from `seed` (default: the seed of the first model) and the replication
number, and with `common_random_numbers=false` also from the scenario number. Common
random numbers (the default) give every scenario the same streams, which makes
differences between scenarios more precise.
"""
function experiment(models; replications::Integer=10, seed=nothing, cases=nothing, threads::Bool=true,
                    common_random_numbers::Bool=true, log::Bool=false)
    list = scenario_list(models)
    names = [n for (n, _) in list]
    ms = [m for (_, m) in list]
    foreach(check_supported, ms)
    base = seed === nothing ? ms[1].seed : seed
    jobs = [(s, r) for s in eachindex(ms) for r in 1:replications]
    runs = Matrix{Run}(undef, length(ms), replications)
    work(j) = begin
        s, r = jobs[j]
        stream = common_random_numbers ? stream_seed(base, r) : stream_seed(base, s, r)
        runs[s, r] = simulate(ms[s]; replication=r, cases=cases === nothing ? ms[s].cases : cases, log=log,
                         scenario=names[s], stream=stream)
    end
    if threads && Threads.nthreads() > 1
        Threads.@threads :dynamic for j in eachindex(jobs)
            work(j)
        end
    else
        foreach(work, eachindex(jobs))
    end
    return Experiment(names, runs)
end

scenario_list(m::SimModel) = [(m.name, m)]
scenario_list(v::AbstractVector{SimModel}) = [(m.name, m) for m in v]
function scenario_list(v::AbstractVector)
    out = Tuple{String,SimModel}[]
    for x in v
        if x isa SimModel
            push!(out, (x.name, x))
        elseif x isa Pair
            name = string(x.first)
            m = x.second isa SimModel ? x.second : model(String(x.second); name=name)
            push!(out, (name, m))
        else
            throw(ArgumentError("a scenario is a SimModel or a pair name => model"))
        end
    end
    return out
end
scenario_list(p::Pair) = scenario_list([p])

"""
    indicators(e::Experiment) -> Vector{NamedTuple}

The indicators of every run in long form: one named tuple `(scenario, replication,
indicator, value)` per indicator — `cases_started`, `cases_finished`, `cycle_time_mean_s`,
`cycle_time_median_s`, `cycle_time_p90_s`, `utilization:RESOURCE`,
`mean_wait_s:ACTIVITY`, `mean_processing_s:ACTIVITY` and `end_event:EVENT` (the share of
finished cases).
"""
function indicators(e::Experiment)
    out = NamedTuple{(:scenario, :replication, :indicator, :value),Tuple{String,Int,String,Float64}}[]
    for s in axes(e.runs, 1), r in axes(e.runs, 2)
        run = e.runs[s, r]
        add(k, v) = push!(out, (scenario=e.scenarios[s], replication=r, indicator=k, value=Float64(v)))
        add("cases_started", run.started)
        add("cases_finished", run.finished)
        ct = cycle_time(run)
        add("cycle_time_mean_s", ct.mean)
        add("cycle_time_median_s", ct.median)
        add("cycle_time_p90_s", ct.p90)
        for (k, u) in run.utilization
            add("utilization:" * k, u)
        end
        for (k, a) in run.activities
            add("mean_wait_s:" * k, a.mean_wait_s)
            add("mean_processing_s:" * k, a.mean_processing_s)
        end
        for (k, n) in run.by_end_event
            add("end_event:" * k, n / max(run.finished, 1))
        end
    end
    return out
end

"""
    summarize(e::Experiment; level=0.95) -> Vector{NamedTuple}

For every scenario and indicator: the mean over the replications, its standard deviation,
the half-width of the confidence interval of the mean (normal approximation) and the
number of replications.
"""
function summarize(e::Experiment; level=0.95)
    z = normal_quantile(0.5 + level / 2)
    rows = indicators(e)
    keys_ = unique([(r.scenario, r.indicator) for r in rows])
    out = NamedTuple{(:scenario, :indicator, :mean, :sd, :half_width, :n),Tuple{String,String,Float64,Float64,Float64,Int}}[]
    for (s, k) in keys_
        v = [r.value for r in rows if r.scenario == s && r.indicator == k && !isnan(r.value)]
        n = length(v)
        μ = mean_of(v)
        sd = n > 1 ? sqrt(sum((v .- μ) .^ 2) / (n - 1)) : NaN
        push!(out, (scenario=s, indicator=k, mean=μ, sd=sd, half_width=n > 1 ? z * sd / sqrt(n) : NaN, n=n))
    end
    return out
end

"""
    scenario(m; name=m.name, resources=Dict(), arrival=nothing, durations=Dict(),
             probabilities=Dict(), cases=nothing) -> SimModel

A copy of a model with other parameters: resource counts (`"archiwisci" => 3`), the
time between arrivals, activity durations (`"ocena" => "lnorm(25m, 8m)"`) and branch
probabilities (`("mozliwa", "odmowa") => 0.2`, the flow from a gateway to a node).
Distributions are texts of the language or [`Dist`](@ref) values.
"""
function scenario(m::SimModel; name=m.name, resources=Dict(), arrival=nothing, durations=Dict(),
                  probabilities=Dict(), cases=nothing)
    asdist(d) = d isa Dist ? d : dist(String(d))
    nodes = [SimNode(n.id, n.kind, n.label, n.scope, n.attrs, n.flags, n.by, n.duration, n.delay, n.assign, n.timer,
                     n.start, n.out, n.nin, n.boundaries, n.triggers) for n in m.nodes]
    for (id, d) in durations
        haskey(m.index, id) || throw(ArgumentError("no node '$id'"))
        nodes[m.index[id]].duration = asdist(d)
    end
    counts = copy(m.counts)
    for (id, c) in resources
        k = findfirst(==(id), m.resource_ids)
        k === nothing && throw(ArgumentError("no resource '$id'"))
        counts[k] = c
    end
    flows = copy(m.flows)
    for ((src, dst), p) in probabilities
        k = findfirst(f -> m.nodes[f.source].id == src && m.nodes[f.target].id == dst, flows)
        k === nothing && throw(ArgumentError("no flow $src -> $dst"))
        f = flows[k]
        flows[k] = SimFlow(f.id, f.source, f.target, f.when, Float64(p), f.default)
    end
    return SimModel(String(name), m.process, m.start, cases === nothing ? m.cases : Int(cases), m.seed,
                    arrival === nothing ? m.arrival : asdist(arrival), m.variables, m.resource_ids, m.resource_labels,
                    counts, nodes, flows, m.index, m.lang07, m.unsupported)
end
