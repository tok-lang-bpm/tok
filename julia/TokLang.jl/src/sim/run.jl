# The discrete-event simulator. It follows the simulator of the engine (crate tok-sim)
# element by element: the same token rules for events, activities, gateways and
# sub-processes, the same queues of resources, the same order of events at equal times
# (by scheduling order), the same step limit and the same indicators. Only the random
# number generator differs, so runs agree in distribution, not number by number.

"A case that passes more elements than this is stopped (`loop-limit`)."
const MAX_STEPS = 10_000

const EV_ARRIVE = 0x01
const EV_WAKE = 0x02
const EV_DONE = 0x03
const EV_BOUNDARY = 0x04
const EV_RECEIVE = 0x05

struct Entry
    t::Float64
    seq::Int
    kind::UInt8
    a::Int
    b::Int
    c::Int
end

@inline before(x::Entry, y::Entry) = x.t == y.t ? x.seq < y.seq : x.t < y.t

function heap_push!(h::Vector{Entry}, e::Entry)
    push!(h, e)
    i = length(h)
    while i > 1
        p = i >> 1
        before(h[i], h[p]) || break
        h[i], h[p] = h[p], h[i]
        i = p
    end
end

function heap_pop!(h::Vector{Entry})
    top = h[1]
    last = pop!(h)
    n = length(h)
    if n > 0
        h[1] = last
        i = 1
        while true
            l = 2i
            l > n && break
            c = (l + 1 <= n && before(h[l+1], h[l])) ? l + 1 : l
            before(h[c], h[i]) || break
            h[i], h[c] = h[c], h[i]
            i = c
        end
    end
    return top
end

mutable struct Frame
    case::Int
    parent::Int
    ai::Int
    active::Int
    alive::Bool
    joins::Dict{Int,Int}
    or_stack::Vector{Int}
    ais::Vector{Int}
    waiters::Vector{Int}
    children::Vector{Int}
    last_end::Int
end

Frame(case, parent, ai) = Frame(case, parent, ai, 0, true, Dict{Int,Int}(), Int[], Int[], Int[], Int[], 0)

mutable struct CaseState
    id::Int
    start::Float64
    finish::Float64      # NaN while the case runs
    end_event::String
    vars::Dict{String,Any}
    steps::Int
    top::Int
end

mutable struct ActivityInstance
    frame::Int
    node::Int
    enqueue::Float64
    start::Float64       # NaN until a performer starts the work
    finish::Float64
    resource::Int
    alive::Bool
    released::Bool
    timers::Vector{Int}
    child::Int
end

"One row of the event log: an activity instance."
struct LogRow
    case::Int
    activity::Int        # index of the node in the model
    enqueue::Float64
    start::Float64       # NaN when interrupted while waiting
    finish::Float64
    completed::Bool
end

mutable struct State
    m::SimModel
    rng::Rng
    t::Float64
    seq::Int
    heap::Vector{Entry}
    frames::Vector{Frame}
    cases::Vector{CaseState}
    ais::Vector{ActivityInstance}
    waiters::Vector{Bool}
    busy::Vector{Int}
    queues::Vector{Vector{Int}}
    busy_time::Vector{Float64}
    rows::Vector{LogRow}
    warnings::Vector{String}
    conds::Dict{Tuple{Int,String},Union{Bool,String}}
end

function State(m::SimModel, rng::Rng)
    nr = length(m.resource_ids)
    State(m, rng, 0.0, 0, Entry[], Frame[], CaseState[], ActivityInstance[], Bool[], zeros(Int, nr),
          [Int[] for _ in 1:nr], zeros(nr), LogRow[], String[], Dict{Tuple{Int,String},Union{Bool,String}}())
end

# ---------------------------------------------------------------------- values

"`max(0, x)` of a sample as a waiting time."
function delay_of(s::Sample)
    s isa Bool && return s ? 1.0 : 0.0
    s isa Float64 && return s > 0 ? s : 0.0
    throw(SimError("'>' not supported between instances of 'str' and 'float'"))
end

function le_zero(s::Sample)
    s isa Bool && return !s
    s isa Float64 && return s <= 0
    throw(SimError("'<=' not supported between instances of 'str' and 'int'"))
end

function sample_lt(a::Sample, b::Sample)
    a isa String && b isa String && return a < b
    (a isa String || b isa String) && throw(SimError("'<' not supported between a text and a number"))
    return Float64(a) < Float64(b)
end

draw(st::State, d::Dist) = sample(d, st.rng)

function schedule!(st::State, dt::Sample, kind::UInt8, a::Int=0, b::Int=0, c::Int=0)
    st.seq += 1
    heap_push!(st.heap, Entry(st.t + delay_of(dt), st.seq, kind, a, b, c))
end

function new_waiter!(st::State)
    push!(st.waiters, true)
    return length(st.waiters)
end

function new_ai!(st::State, frame::Int, node::Int)
    push!(st.ais, ActivityInstance(frame, node, st.t, NaN, NaN, 0, true, false, Int[], 0))
    return length(st.ais)
end

function new_frame!(st::State, case::Int, parent::Int, ai::Int)
    push!(st.frames, Frame(case, parent, ai))
    return length(st.frames)
end

# ------------------------------------------------------------------ conditions

"The truth of the condition of a flow, evaluated by the engine as the simulator does."
function cond(st::State, flow::Int, vars::Dict{String,Any})
    f = st.m.flows[flow]
    key = (flow, write_json(vars))
    r = get(st.conds, key, nothing)
    if r === nothing
        r = try
            Engine.sim_eval(f.when, vars; lang07=st.m.lang07)[2]
        catch e
            e isa Engine.TokError && e.kind == "feel" || rethrow()
            e.message
        end
        st.conds[key] = r
    end
    if r isa String
        push!(st.warnings, "condition of flow '$(f.id)' could not be evaluated: $r")
        return false
    end
    return r
end

# ---------------------------------------------------------------------- driver

function simulate_run!(st::State, n_cases::Int)
    m = st.m
    m.start == 0 && throw(SimError("the process '$(m.process)' has no start event"))
    n_cases > 0 && schedule!(st, 0.0, EV_ARRIVE, 0)
    while !isempty(st.heap)
        e = heap_pop!(st.heap)
        st.t = e.t
        k = e.kind
        if k == EV_ARRIVE
            arrive!(st, e.a, n_cases)
        elseif k == EV_WAKE
            wake!(st, e.a, e.b, e.c)
        elseif k == EV_DONE
            done!(st, e.a)
        elseif k == EV_BOUNDARY
            boundary_fire!(st, e.a, e.b, e.c)
        else
            race_receive!(st, e.a, e.b, e.c)
        end
    end
    return st
end

function arrive!(st::State, k::Int, n_cases::Int)
    m = st.m
    vars = Dict{String,Any}()
    for (name, d) in m.variables
        vars[name] = draw(st, d)
    end
    push!(st.cases, CaseState(k + 1, st.t, NaN, "", vars, 0, 0))
    case = length(st.cases)
    frame = new_frame!(st, case, 0, 0)
    st.cases[case].top = frame
    st.frames[frame].active = 1
    enter!(st, frame, m.start)
    if k + 1 < n_cases
        schedule!(st, draw(st, m.arrival), EV_ARRIVE, k + 1)
    end
end

# ---------------------------------------------------------------------- tokens

function consume!(st::State, frame::Int)
    f = st.frames[frame]
    f.active -= 1
    if f.active <= 0 && f.alive
        frame_done!(st, frame)
    end
end

function frame_done!(st::State, frame::Int)
    f = st.frames[frame]
    f.alive = false
    if f.parent == 0
        c = st.cases[f.case]
        if isnan(c.finish)
            pending = any(v != 0 for v in values(f.joins))
            c.finish = st.t
            c.end_event = f.last_end > 0 ? st.m.nodes[f.last_end].id : pending ? "deadlock" : ""
        end
    elseif f.ai > 0
        finish_ai!(st, f.ai)
    end
end

function kill!(st::State, frame::Int)
    f = st.frames[frame]
    f.alive = false
    for w in f.waiters
        st.waiters[w] = false
    end
    for ai in f.ais
        a = st.ais[ai]
        if a.alive
            a.alive = false
            release!(st, ai)
            for w in a.timers
                st.waiters[w] = false
            end
        end
    end
    for ch in f.children
        st.frames[ch].alive && kill!(st, ch)
    end
end

function enter!(st::State, frame::Int, node::Int)
    st.frames[frame].alive || return
    c = st.cases[st.frames[frame].case]
    c.steps += 1
    if c.steps > MAX_STEPS
        kill!(st, c.top)
        if isnan(c.finish)
            c.finish = st.t
            c.end_event = "loop-limit"
        end
        return
    end
    n = st.m.nodes[node]
    k = n.kind
    if k === :end
        st.frames[frame].last_end = node
        if "terminate" in n.flags
            kill!(st, frame)
            f = st.frames[frame]
            f.alive = true
            f.active = 0
            frame_done!(st, frame)
        else
            consume!(st, frame)
        end
    elseif k === :start || k === :throw || k === :boundary
        leave!(st, frame, node)
    elseif k === :catch
        wait = event_wait!(st, n)
        if wait === nothing || le_zero(wait)
            leave!(st, frame, node)
        else
            w = new_waiter!(st)
            push!(st.frames[frame].waiters, w)
            schedule!(st, wait, EV_WAKE, frame, w, node)
        end
    elseif k in (:xor, :and, :or, :eventgw, :complex)
        gateway!(st, frame, node)
    elseif k === :sub
        start_sub!(st, frame, node)
    elseif k === :eventsub
        consume!(st, frame)
    else
        start_activity!(st, frame, node)
    end
end

function wake!(st::State, frame::Int, waiter::Int, node::Int)
    if st.waiters[waiter] && st.frames[frame].alive
        st.waiters[waiter] = false
        leave!(st, frame, node)
    end
end

"The seconds a timer waits now, or `nothing` when the engine does not understand it."
function timer_wait(st::State, t::TimerSpec)
    t.kind === :duration && return t.seconds
    t.kind === :date && return max(0.0, t.seconds - st.t)
    t.kind === :error && throw(SimError(t.text))
    return nothing
end

function event_wait!(st::State, n::SimNode)
    if n.timer !== nothing
        w = timer_wait(st, n.timer)
        w === nothing && push!(st.warnings, "timer of '$(n.id)' is not understood: $(n.attrs["timer"])")
        return w
    end
    n.delay !== nothing && return draw(st, n.delay)
    n.duration !== nothing && return draw(st, n.duration)
    return 0.0
end

function leave!(st::State, frame::Int, node::Int)
    m = st.m
    flows = m.nodes[node].out
    isempty(flows) && return consume!(st, frame)
    length(flows) == 1 && return enter!(st, frame, m.flows[flows[1]].target)
    vars = st.cases[st.frames[frame].case].vars
    taken = [i for i in flows if !m.flows[i].default && (m.flows[i].when === nothing || cond(st, i, vars))]
    if isempty(taken)
        taken = [i for i in flows if m.flows[i].default]
        isempty(taken) && (taken = [flows[1]])
    end
    route!(st, frame, [m.flows[i].target for i in taken])
end

function route!(st::State, frame::Int, targets::Vector{Int})
    isempty(targets) && return consume!(st, frame)
    st.frames[frame].active += length(targets) - 1
    for tg in targets
        enter!(st, frame, tg)
    end
end

# -------------------------------------------------------------------- gateways

function gateway!(st::State, frame::Int, node::Int)
    m = st.m
    n = m.nodes[node]
    kind = n.kind
    nin = n.nin
    vars = st.cases[st.frames[frame].case].vars
    if nin > 1 && (kind === :and || kind === :or || kind === :complex)
        f = st.frames[frame]
        expected = kind === :and ? nin : (isempty(f.or_stack) ? nin : f.or_stack[end])
        got = get(f.joins, node, 0) + 1
        if got < expected
            f.joins[node] = got
            return consume!(st, frame)
        end
        f.joins[node] = 0
        kind !== :and && !isempty(f.or_stack) && pop!(f.or_stack)
    end
    flows = n.out
    length(flows) <= 1 && return leave!(st, frame, node)
    if kind === :and
        route!(st, frame, [m.flows[i].target for i in flows])
    elseif kind === :xor
        route!(st, frame, [xor_choice!(st, flows, vars)])
    elseif kind === :or || kind === :complex
        targets = or_choice!(st, flows, vars)
        push!(st.frames[frame].or_stack, length(targets))
        route!(st, frame, targets)
    else
        race!(st, frame, node, flows)
    end
end

function xor_choice!(st::State, flows::Vector{Int}, vars)
    fl = st.m.flows
    for i in flows
        f = fl[i]
        f.when !== nothing && !f.default && cond(st, i, vars) && return f.target
    end
    ps = [i for i in flows if has_p(fl[i])]
    if !isempty(ps)
        u = uniform(st.rng)
        acc = 0.0
        for i in ps
            acc += fl[i].p
            u < acc && return fl[i].target
        end
    end
    for i in flows
        fl[i].default && return fl[i].target
    end
    !isempty(ps) && return fl[ps[end]].target
    return fl[flows[1]].target
end

function or_choice!(st::State, flows::Vector{Int}, vars)
    fl = st.m.flows
    chosen = Int[]
    for i in flows
        f = fl[i]
        f.default && continue
        if f.when !== nothing
            cond(st, i, vars) && push!(chosen, f.target)
        elseif has_p(f) && uniform(st.rng) < f.p
            push!(chosen, f.target)
        end
    end
    if isempty(chosen)
        chosen = [fl[i].target for i in flows if fl[i].default]
        isempty(chosen) && (chosen = [fl[flows[1]].target])
    end
    return chosen
end

function race!(st::State, frame::Int, node::Int, flows::Vector{Int})
    m = st.m
    best = 0
    best_t = nothing
    for i in flows
        tn = m.flows[i].target
        target = m.nodes[tn]
        w = if target.kind === :receive
            target.delay !== nothing ? draw(st, target.delay) :
            target.duration !== nothing ? draw(st, target.duration) : nothing
        elseif target.timer !== nothing || target.delay !== nothing
            event_wait!(st, target)
        else
            nothing
        end
        if w !== nothing && (best_t === nothing || sample_lt(w, best_t))
            best = tn
            best_t = w
        end
    end
    if best == 0
        push!(st.warnings, "event-based gateway '$(m.nodes[node].id)': no event can happen (add delay= to message events)")
        return
    end
    w = new_waiter!(st)
    push!(st.frames[frame].waiters, w)
    if m.nodes[best].kind === :receive
        ai = new_ai!(st, frame, best)
        push!(st.frames[frame].ais, ai)
        schedule!(st, best_t, EV_RECEIVE, frame, w, ai)
    else
        schedule!(st, best_t, EV_WAKE, frame, w, best)
    end
end

function race_receive!(st::State, frame::Int, waiter::Int, ai::Int)
    if st.waiters[waiter] && st.frames[frame].alive
        st.ais[ai].start = st.t
        complete!(st, ai)
    end
end

# ------------------------------------------------------------------ activities

function start_activity!(st::State, frame::Int, node::Int)
    ai = new_ai!(st, frame, node)
    push!(st.frames[frame].ais, ai)
    arm_boundaries!(st, ai)
    claim!(st, ai)
end

function claim!(st::State, ai::Int)
    r = st.m.nodes[st.ais[ai].node].by
    if r > 0
        st.ais[ai].resource = r
        st.busy[r] < st.m.counts[r] && return begin_work!(st, ai)
        push!(st.queues[r], ai)
        return
    end
    begin_work!(st, ai)
end

function begin_work!(st::State, ai::Int)
    a = st.ais[ai]
    a.start = st.t
    a.resource > 0 && (st.busy[a.resource] += 1)
    n = st.m.nodes[a.node]
    dur = n.duration === nothing ? 0.0 : draw(st, n.duration)
    schedule!(st, dur, EV_DONE, ai)
end

function done!(st::State, ai::Int)
    a = st.ais[ai]
    a.alive && st.frames[a.frame].alive && complete!(st, ai)
end

function complete!(st::State, ai::Int)
    a = st.ais[ai]
    n = st.m.nodes[a.node]
    frame = a.frame
    a.alive = false
    a.finish = st.t
    release!(st, ai)
    for w in a.timers
        st.waiters[w] = false
    end
    if !isempty(n.assign)
        vars = st.cases[st.frames[frame].case].vars
        for (name, d) in n.assign
            vars[name] = draw(st, d)
        end
    end
    log_row!(st, ai, true)
    leave!(st, frame, a.node)
end

function release!(st::State, ai::Int)
    a = st.ais[ai]
    r = a.resource
    r == 0 && return
    if isnan(a.start) || a.released
        if isnan(a.start)
            q = st.queues[r]
            k = findfirst(==(ai), q)
            k !== nothing && deleteat!(q, k)
        end
        return
    end
    a.released = true
    st.busy[r] -= 1
    st.busy_time[r] += st.t - a.start
    q = st.queues[r]
    while !isempty(q) && st.busy[r] < st.m.counts[r]
        nxt = popfirst!(q)
        b = st.ais[nxt]
        b.alive && st.frames[b.frame].alive && begin_work!(st, nxt)
    end
end

function log_row!(st::State, ai::Int, completed::Bool)
    a = st.ais[ai]
    push!(st.rows, LogRow(st.cases[st.frames[a.frame].case].id, a.node, a.enqueue, a.start, a.finish, completed))
end

function arm_boundaries!(st::State, ai::Int)
    m = st.m
    for b in m.nodes[st.ais[ai].node].boundaries
        bn = m.nodes[b]
        "cancel" in bn.flags && continue
        w = if bn.timer !== nothing
            timer_wait(st, bn.timer)
        elseif bn.delay !== nothing
            draw(st, bn.delay)
        else
            nothing
        end
        w === nothing && continue
        wt = new_waiter!(st)
        push!(st.ais[ai].timers, wt)
        schedule!(st, w, EV_BOUNDARY, ai, b, wt)
    end
end

function boundary_fire!(st::State, ai::Int, b::Int, waiter::Int)
    a = st.ais[ai]
    frame = a.frame
    (st.waiters[waiter] && a.alive && st.frames[frame].alive) || return
    st.waiters[waiter] = false
    if "noninterrupting" in st.m.nodes[b].flags
        st.frames[frame].active += 1
        return leave!(st, frame, b)
    end
    a.alive = false
    a.finish = st.t
    release!(st, ai)
    for w in a.timers
        st.waiters[w] = false
    end
    a.child > 0 && kill!(st, a.child)
    log_row!(st, ai, false)
    leave!(st, frame, b)
end

function start_sub!(st::State, frame::Int, node::Int)
    start = st.m.nodes[node].start
    start == 0 && return start_activity!(st, frame, node)
    ai = new_ai!(st, frame, node)
    st.ais[ai].start = st.t
    push!(st.frames[frame].ais, ai)
    arm_boundaries!(st, ai)
    case = st.frames[frame].case
    child = new_frame!(st, case, frame, ai)
    st.ais[ai].child = child
    push!(st.frames[frame].children, child)
    st.frames[child].active = 1
    enter!(st, child, start)
end

function finish_ai!(st::State, ai::Int)
    a = st.ais[ai]
    a.alive && st.frames[a.frame].alive && complete!(st, ai)
end
