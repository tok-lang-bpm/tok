# The simulation model, built from the description the engine gives (`Engine.sim_model`).

"An error of the simulation: a model that cannot run, or an invalid value."
struct SimError <: Exception
    msg::String
end
Base.showerror(io::IO, e::SimError) = print(io, "SimError: ", e.msg)

"""
    UnsupportedModel(features)

The model uses features the native simulator does not run; `features` lists them as
(element, feature) pairs. Such models run in the engine ([`Engine.simulate`](@ref)).
"""
struct UnsupportedModel <: Exception
    features::Vector{Tuple{String,String}}
end
function Base.showerror(io::IO, e::UnsupportedModel)
    print(io, "UnsupportedModel: the native simulator does not run ")
    print(io, join(["$f ('$id')" for (id, f) in e.features], ", "))
    print(io, "; use Tok.Engine.simulate for this model")
end

"A timer resolved by the engine: a duration, a date (seconds from the start), or not understood."
struct TimerSpec
    kind::Symbol        # :duration, :date, :invalid, :error
    seconds::Float64
    text::String
end

mutable struct SimNode
    id::String
    kind::Symbol
    label::String
    scope::String
    attrs::Dict{String,String}
    flags::Set{String}
    by::Int                      # index of the resource, 0 for none
    duration::Union{Nothing,Dist}
    delay::Union{Nothing,Dist}
    assign::Vector{Tuple{String,Dist}}
    timer::Union{Nothing,TimerSpec}
    start::Int                   # start node of a sub-process, 0 for none
    out::Vector{Int}             # outgoing flows, in document order
    nin::Int                     # number of incoming flows
    boundaries::Vector{Int}      # boundary events attached to the activity
    triggers::Int                # number of triggers (several on a multiple event)
end

struct SimFlow
    id::String                   # "None" for a flow without identifier, as in warnings
    source::Int
    target::Int
    when::Union{Nothing,String}
    p::Float64                   # NaN when the flow has no probability
    default::Bool
end

has_p(f::SimFlow) = !isnan(f.p)

"""
    SimModel

A process ready for the native simulator: built with [`model`](@ref) from the text of a
document. `unsupported` lists the features the native simulator does not run (empty
when it runs the model).
"""
mutable struct SimModel
    name::String
    process::String
    start::Int
    cases::Int
    seed::BigInt
    arrival::Dist
    variables::Vector{Tuple{String,Dist}}
    resource_ids::Vector{String}
    resource_labels::Vector{String}
    counts::Vector{Int}
    nodes::Vector{SimNode}
    flows::Vector{SimFlow}
    index::Dict{String,Int}
    lang07::Bool
    unsupported::Vector{Tuple{String,String}}
end

Base.show(io::IO, m::SimModel) =
    print(io, "SimModel(\"", m.name, "\", ", length(m.nodes), " nodes, ", length(m.resource_ids), " resources",
          isempty(m.unsupported) ? "" : ", unsupported", ")")

const ACTIVITY_KINDS = (:task, :user, :service, :manual, :script, :rule, :send, :receive, :call,
                        :sub, :eventsub, :adhoc, :transaction)

parse_count(x) = x isa Integer ? x : parse(BigInt, string(x))

function timer_spec(t)
    t === nothing && return nothing
    k = Symbol(t["kind"])
    return TimerSpec(k, Float64(get(t, "seconds", 0.0)), string(get(t, "text", get(t, "message", ""))))
end

"""
    model(text; name="model", cases=nothing, seed=nothing) -> SimModel

The simulation model of a tok document, read by the engine. `cases` and `seed` override
the attributes of `simulate`.
"""
function model(text::AbstractString; name="model", cases=nothing, seed=nothing)
    m = Engine.sim_model(text; cases=cases, seed=seed)
    return model(m; name=name)
end

function model(d::AbstractDict; name="model")
    index = Dict{String,Int}()
    for (k, n) in enumerate(d["nodes"])
        index[n["id"]] = k
    end
    rids = String[r["id"] for r in d["resources"]]
    rindex = Dict(id => k for (k, id) in enumerate(rids))
    nodes = SimNode[]
    for n in d["nodes"]
        attrs = Dict{String,String}(k => string(v) for (k, v) in n["attrs"])
        by = get(attrs, "by", "")
        assign = n["assign"] === nothing ? Tuple{String,Dist}[] :
                 [(a["name"], Dist(a["dist"])) for a in n["assign"]]
        push!(nodes, SimNode(n["id"], Symbol(n["kind"]), something(n["label"], ""), something(n["scope"], ""), attrs, Set{String}(n["flags"]),
                             isempty(by) ? 0 : get(rindex, by, 0),
                             n["duration"] === nothing ? nothing : Dist(n["duration"]),
                             n["delay"] === nothing ? nothing : Dist(n["delay"]),
                             assign, timer_spec(n["timer"]),
                             n["start"] === nothing ? 0 : index[n["start"]],
                             Int[], 0, Int[], sum(values(n["triggers"]); init=0)))
    end
    flows = SimFlow[]
    for f in d["flows"]
        s, t = index[f["source"]], index[f["target"]]
        push!(flows, SimFlow(something(f["id"], "None"), s, t, f["when"],
                             f["p"] === nothing ? NaN : Float64(f["p"]), f["default"]))
        push!(nodes[s].out, length(flows))
        nodes[t].nin += 1
    end
    for n in nodes
        if n.kind === :boundary && haskey(n.attrs, "on") && haskey(index, n.attrs["on"])
            push!(nodes[index[n.attrs["on"]]].boundaries, index[n.id])
        end
    end
    start = d["start_event"] === nothing ? 0 : index[d["start_event"]]
    m = SimModel(name, d["process"], start, Int(parse_count(d["cases"])), parse_count(d["seed"]),
                 Dist(d["arrival"]), [(v["name"], Dist(v["dist"])) for v in d["variables"]],
                 rids, String[something(r["label"], r["id"]) for r in d["resources"]],
                 Int[Int(parse_count(r["count"])) for r in d["resources"]],
                 nodes, flows, index, d["lang07"], Tuple{String,String}[])
    m.unsupported = find_unsupported(m, d["lang05"])
    return m
end

"""
    supported(m) -> Bool

Whether the native simulator runs the model; [`unsupported`](@ref) lists what it does not.
"""
supported(m::SimModel) = isempty(m.unsupported)

"The features of the model the native simulator does not run, as (element, feature) pairs."
unsupported(m::SimModel) = m.unsupported

"Nodes the simulator can reach: the process and its embedded sub-processes."
function simulated_nodes(m::SimModel)
    scopes = Set{String}([m.process])
    changed = true
    while changed
        changed = false
        for n in m.nodes
            # a sub-process without a start event runs as a task, its content does not run
            inner = n.kind in (:transaction, :adhoc) || (n.kind === :sub && n.start != 0)
            if inner && !(n.id in scopes) && node_scope(m, n) in scopes
                push!(scopes, n.id)
                changed = true
            end
        end
    end
    return [n for n in m.nodes if node_scope(m, n) in scopes]
end

node_scope(::SimModel, n::SimNode) = n.scope

function find_unsupported(m::SimModel, lang05::Bool)
    out = Tuple{String,String}[]
    m.start == 0 && return out  # reported as an error when the model runs
    for n in simulated_nodes(m)
        k = n.kind
        f = n.flags
        if k === :adhoc
            push!(out, (n.id, "ad-hoc sub-process"))
        elseif k === :transaction
            push!(out, (n.id, "transaction"))
        elseif k === :complex
            push!(out, (n.id, "complex gateway"))
        end
        if "compensate" in f || haskey(n.attrs, "compensate")
            push!(out, (n.id, "compensation"))
        end
        k === :end && "cancel" in f && push!(out, (n.id, "cancel end event"))
        k === :rule && haskey(n.attrs, "decision") && push!(out, (n.id, "decision of a rule task"))
        k === :script && haskey(n.attrs, "expr") && push!(out, (n.id, "expression of a script task"))
        if k in ACTIVITY_KINDS && ("parallel" in f || "sequential" in f) && haskey(n.attrs, "collection")
            push!(out, (n.id, "multi-instance activity"))
        end
        if lang05 && "loop" in f && k in ACTIVITY_KINDS && k !== :eventsub &&
           (haskey(n.attrs, "while") || haskey(n.attrs, "loop-max"))
            push!(out, (n.id, "standard loop"))
        end
        lang05 && n.triggers >= 2 && push!(out, (n.id, "multiple event"))
        k in (:receive, :eventgw) && "instantiate" in f && push!(out, (n.id, "instantiating element"))
        n.timer !== nothing && n.timer.kind === :error && push!(out, (n.id, "timer date not comparable with the start"))
    end
    return out
end
