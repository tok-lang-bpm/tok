# What an event log tells about the parameters of a model: processing times of the
# activities, times between arrivals, and the branches taken at gateways.

"Observed processing times of one activity: completed instances and interrupted (censored) ones."
struct DurationData
    activity::String
    family::Symbol
    times::Vector{Float64}
    censored::Vector{Float64}
end

"Counts of the branches of one gateway: visits `total` and the count of each flow with a probability."
struct BranchData
    gateway::String
    kind::Symbol                    # :xor or :or
    targets::Vector{String}         # the target of each flow with `p=`
    counts::Vector{Int}
    total::Int
end

"""
    Observations

The data [`fit`](@ref) estimates from: `durations` (per activity), `arrivals` (times
between consecutive arrivals) with the family of the arrival distribution, `branches`
(per gateway), and `notes` on what could not be observed.
"""
struct Observations
    durations::Vector{DurationData}
    arrival_family::Symbol
    arrivals::Vector{Float64}
    branches::Vector{BranchData}
    notes::Vector{String}
end

const LOGGED = (:task, :user, :service, :manual, :script, :rule, :send, :receive, :call, :sub, :adhoc, :transaction)

is_logged(n) = n.kind in LOGGED

"Activities and end events reached first from a node, through elements that are not logged."
function forward(m::Sim.SimModel, node::Int, acts=Set{Int}(), ends=Set{Int}(), seen=Set{Int}())
    node in seen && return (acts, ends)
    push!(seen, node)
    n = m.nodes[node]
    if is_logged(n)
        push!(acts, node)
    elseif n.kind === :end
        push!(ends, node)
    else
        for f in n.out
            forward(m, m.flows[f].target, acts, ends, seen)
        end
    end
    return (acts, ends)
end

"Whether every path into `node` through elements that are not logged comes from gateway `g` by flow `flow`."
function only_from(m::Sim.SimModel, node::Int, g::Int, flow::Int, seen=Set{Int}())
    node in seen && return true
    push!(seen, node)
    incoming = [k for (k, f) in enumerate(m.flows) if f.target == node]
    isempty(incoming) && return false
    for k in incoming
        s = m.flows[k].source
        if s == g
            k == flow || return false
        elseif is_logged(m.nodes[s]) || m.nodes[s].kind in (:start, :boundary)
            return false
        elseif !only_from(m, s, g, flow, seen)
            return false
        end
    end
    return true
end

"""
The elements whose completions bring tokens to gateway `g` and only to it: logged
activities and start events from which a single path of elements that are not logged
leads to `g`; `nothing` when the visits of `g` cannot be counted.
"""
function feeders(m::Sim.SimModel, g::Int)
    out = Int[]
    stack = [g]
    seen = Set{Int}()
    while !isempty(stack)
        x = pop!(stack)
        x in seen && continue
        push!(seen, x)
        incoming = [f for f in m.flows if f.target == x]
        isempty(incoming) && return nothing
        for f in incoming
            s = f.source
            sn = m.nodes[s]
            # the path from s to x must be the only way out of s
            length(sn.out) == 1 || return nothing
            if is_logged(sn) || sn.kind === :start
                push!(out, s)
            elseif sn.kind in (:boundary,)
                return nothing
            else
                push!(stack, s)
            end
        end
    end
    return out
end

family_of(d::Union{Nothing,Sim.Dist}) = d === nothing ? :lnorm : d.name in (:bern, :cat) ? :lnorm : d.name

"""
    observations(m, log; families=Dict()) -> Observations

Extract the data for the calibration of model `m` (a [`Sim.SimModel`](@ref) or the text
of a document) from an event log. The family of each duration is the one the model
declares (`lnorm` for an activity without `duration=`), or the one given in `families`
(activity identifier, or `"arrival"`, => `:exp`, `:norm`, `:lnorm`, `:unif`, `:tri` or
`:fixed`).
"""
function observations(m::Sim.SimModel, log::EventLog; families=Dict())
    notes = String[]
    fams = Dict{String,Symbol}(string(k) => Symbol(v) for (k, v) in families)
    # processing times
    durations = DurationData[]
    by_act = Dict{String,Tuple{Vector{Float64},Vector{Float64}}}()
    counts = Dict{String,Int}()          # completed instances per activity
    for t in log.traces, i in t.instances
        key = haskey(m.index, i.activity) ? i.activity : i.label
        haskey(m.index, key) || continue
        i.completed && (counts[key] = get(counts, key, 0) + 1)
        isnan(i.start) && continue
        x, c = get!(by_act, key) do
            (Float64[], Float64[])
        end
        isnan(i.finish) && continue
        push!(i.completed ? x : c, i.finish - i.start)
    end
    for n in m.nodes
        n.kind in (:task, :user, :service, :manual, :script, :rule, :send, :call, :receive) || continue
        haskey(by_act, n.id) || continue
        x, c = by_act[n.id]
        isempty(x) && continue
        push!(durations, DurationData(n.id, get(fams, n.id, family_of(n.duration)), x, c))
    end
    # arrivals
    starts = sort([t.arrival for t in log.traces if !isnan(t.arrival)])
    arrivals = diff(starts)
    arrival_family = get(fams, "arrival", family_of(m.arrival))
    # branches
    branches = BranchData[]
    ntraces = length(log.traces)
    ends = Dict{String,Int}()
    for t in log.traces
        isempty(t.end_event) || (ends[t.end_event] = get(ends, t.end_event, 0) + 1)
    end
    for (gi, g) in enumerate(m.nodes)
        g.kind in (:xor, :or) && length(g.out) >= 2 || continue
        pflows = [k for k in g.out if Sim.has_p(m.flows[k])]
        isempty(pflows) && continue
        if any(m.flows[k].when !== nothing for k in g.out)
            push!(notes, "gateway '$(g.id)': flows with conditions; probabilities not estimated")
            continue
        end
        if g.kind === :or && !any(m.flows[k].default for k in g.out)
            # without a default flow a case in which no branch is drawn takes the first flow,
            # so the counts of that flow do not estimate its probability
            push!(notes, "gateway '$(g.id)': an inclusive gateway without a default flow; probabilities not estimated")
            continue
        end
        known = Dict{Int,Int}()
        for k in g.out
            acts, endset = forward(m, m.flows[k].target)
            if !isempty(acts) && all(a -> only_from(m, a, gi, k), acts)
                known[k] = sum(get(counts, m.nodes[a].id, 0) + count_interrupted(log, m.nodes[a].id) for a in acts)
            elseif isempty(acts) && !isempty(endset) && log.has_end_events && all(e -> only_from(m, e, gi, k), endset)
                known[k] = sum(get(ends, m.nodes[e].id, 0) for e in endset)
            end
        end
        fed = feeders(m, gi)
        total = if fed === nothing
            nothing
        else
            sum(m.nodes[s].kind === :start ? ntraces : get(counts, m.nodes[s].id, 0) for s in fed; init=0)
        end
        unknown = [k for k in g.out if !haskey(known, k)]
        if g.kind === :xor
            if isempty(unknown)
                total = sum(values(known))
            elseif length(unknown) == 1 && total !== nothing
                known[unknown[1]] = total - sum(values(known); init=0)
            else
                push!(notes, "gateway '$(g.id)': the branches taken cannot be told apart in the log")
                continue
            end
        else
            if !isempty(setdiff(pflows, keys(known))) || total === nothing
                push!(notes, "gateway '$(g.id)': the branches taken cannot be told apart in the log")
                continue
            end
        end
        total > 0 || continue
        push!(branches, BranchData(g.id, g.kind, [m.nodes[m.flows[k].target].id for k in pflows],
                                   [known[k] for k in pflows], total))
    end
    return Observations(durations, arrival_family, arrivals, branches, notes)
end

observations(text::AbstractString, log::EventLog; kw...) = observations(Sim.model(text), log; kw...)

count_interrupted(log::EventLog, id) = count(i -> i.activity == id && !i.completed, (i for t in log.traces for i in t.instances))
