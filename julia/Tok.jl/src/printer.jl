# Canonical printer (`tok fmt`, spec §14).

const BARE = r"^(?:[A-Za-z_][A-Za-z0-9_.:/+-]*|-?[0-9]+(?:\.[0-9]+)?|[0-9][A-Za-z0-9_.:/+-]*)$"

# values of these keys are free text and always printed in quotes
const QUOTED_KEYS = Set(["when", "expr", "condition", "correlation", "duration", "assign", "delay", "arrival",
                         "options", "members", "message", "signal", "error", "escalation", "link", "code",
                         "start",
                         # language 0.3: expressions, lists and values of the executable layer
                         "collection", "output-item", "until", "active", "inputs", "outputs", "headers",
                         "listeners", "events", "url", "body", "query", "headers-in", "assignee",
                         "candidate-users", "candidate-groups", "due", "follow-up", "business-id", "version",
                         "version-tag",
                         # language 0.5
                         "while", "activation", "structure", "in", "out", "path", "doc", "process-name",
                         "object",
                         # language 0.6
                         "reply", "pools", "messages", "map"])

const EXECUTION_LISTENER_EVENTS = ("start", "end")

"Execution listeners first, then task listeners, each group in written order (canonical form)."
function normalize_listeners(text::AbstractString)
    items = String[]
    for x in split(text, ";")
        s = pystrip(x)
        isempty(s) || push!(items, s)
    end
    event(x) = pystrip(split(x, ":"; limit=2)[1])
    first = [x for x in items if event(x) in EXECUTION_LISTENER_EVENTS]
    rest = [x for x in items if !(x in first)]
    return join(vcat(first, rest), "; ")
end

q(text::AbstractString) = "\"" * replace(text, "\\" => "\\\\", "\"" => "\\\"", "\n" => "\\n") * "\""

function value(key, v::AbstractString)
    if key == "listeners"
        v = normalize_listeners(v)
    end
    if key in QUOTED_KEYS || !occursin(BARE, v) || occursin("->", v)
        return q(v)
    end
    return String(v)
end

ljust(s::AbstractString, w::Int) = s * " "^max(0, w - length(s))

"Label, flags and attributes of an element in canonical order."
function tail(el, attrs::Dict{String,String}, flags::Vector{String}, label, keys, cflags; quote_type=false,
              extra=Tuple{String,String}[])
    parts = String[]
    label !== nothing && push!(parts, q(label))
    for f in cflags
        f in flags && push!(parts, f)
    end
    for f in flags
        f in cflags || push!(parts, f)
    end
    for k in keys
        if haskey(attrs, k)
            push!(parts, k * "=" * ((k == "type" && quote_type) ? q(attrs[k]) : value(k, attrs[k])))
            repeated!(parts, extra, k)
        end
    end
    for k in sort(collect(keys_of(attrs)))
        if !(k in keys)
            push!(parts, k * "=" * value(k, attrs[k]))
            repeated!(parts, extra, k)
        end
    end
    return parts
end
tail(el::Elem, keys, cflags; quote_type=false) =
    tail(el, el.attrs, el.flags, el.label, keys, cflags; quote_type=quote_type, extra=el.extra)

"Further values of a repeated key, after its first value (language 0.5)."
function repeated!(parts::Vector{String}, extra, k)
    for (x, v) in extra
        x == k && push!(parts, k * "=" * value(k, v))
    end
end

keys_of(d::Dict) = collect(keys(d))

mutable struct Printer
    doc::Document
    out::Vector{String}
end

comment_line(pad, c) = pad * (isempty(c) ? "#" : "# " * c)

function emit!(pr::Printer, indent::Int, text::String, el=nothing)
    pad = "  "^indent
    if el !== nothing
        for c in el.comments
            push!(pr.out, comment_line(pad, c))
        end
    end
    line = pad * text
    if el !== nothing && el.trailing !== nothing && !isempty(el.trailing)
        line *= "  # " * el.trailing
    end
    push!(pr.out, line)
end

function blank!(pr::Printer)
    if !isempty(pr.out) && pr.out[end] != ""
        push!(pr.out, "")
    end
end

function run!(pr::Printer)
    d = pr.doc
    for c in d.header_comments
        push!(pr.out, comment_line("", c))
    end
    head = ["tok", d.version === nothing ? "0.1" : d.version]
    d.title !== nothing && push!(head, q(d.title))
    for f in HEADER_FLAGS
        f in d.flags && push!(head, f)
    end
    for f in d.flags
        f in HEADER_FLAGS || push!(head, f)
    end
    for k in HEADER_KEYS
        haskey(d.attrs, k) && push!(head, k * "=" * value(k, d.attrs[k]))
    end
    for k in sort(keys_of(d.attrs))
        k in HEADER_KEYS || push!(head, k * "=" * value(k, d.attrs[k]))
    end
    line = join(head, " ")
    if d.header_trailing !== nothing && !isempty(d.header_trailing)
        line *= "  # " * d.header_trailing
    end
    push!(pr.out, line)

    if !isempty(d.items) || !isempty(d.globals)
        blank!(pr)
        item_keys = lang_07(d) ? ITEM_KEYS_07 : ITEM_KEYS
        for it in ovalues(d.items)
            emit!(pr, 0, join(["item", it.id, tail(it, item_keys, ITEM_FLAGS)...], " "), it)
            components!(pr, it.items, 1)
        end
        for g in ovalues(d.globals)
            emit!(pr, 0, join(["global", g.id, tail(g, GLOBAL_KEYS, ())...], " "), g)
        end
    end
    for x in ovalues(d.interfaces)
        blank!(pr)
        emit!(pr, 0, join(["interface", x.id, tail(x, INTERFACE_KEYS, ())...], " "), x)
        for op in x.items
            emit!(pr, 1, join(["operation", op.id, tail(op, OPERATION_KEYS, ())...], " "), op)
        end
    end

    if !isempty(d.top)
        blank!(pr)
        children!(pr, d.top, 0)
    end
    for p in d.pools
        blank!(pr)
        emit!(pr, 0, join(["pool", p.id, tail(p, pool_keys(d), POOL_FLAGS)...], " "), p)
        children!(pr, p.children, 1)
    end
    # language 0.6: each choreography with its nodes and its sequence flows
    for c in ovalues(d.choreographies)
        blank!(pr)
        emit!(pr, 0, join(["choreography", c.id, tail(c, CHOREOGRAPHY_KEYS, ())...], " "), c)
        children!(pr, c.children, 1)
        flows!(pr, [f for f in d.flows if flow_choreography(d, f) == c.id], 1)
    end
    process_flows = [f for f in d.flows if flow_choreography(d, f) === nothing]
    if !isempty(process_flows)
        blank!(pr)
        flows!(pr, process_flows, 0)
    end
    # message flows named by a conversation print their identifier (language 0.6)
    referenced = Set{String}()
    for c in ovalues(d.conversations)
        union!(referenced, words(get(c.attrs, "messages", "")))
    end
    if !isempty(d.messages)
        blank!(pr)
        for m in d.messages
            parts = ["message", m.source, "->", m.target, tail(m, MESSAGE_KEYS, ())...]
            (m.explicit_id || m.id in referenced) && push!(parts, "id=" * m.id)
            emit!(pr, 0, join(parts, " "), m)
        end
    end
    top = [c for c in ovalues(d.conversations) if c.parent === nothing]
    if !isempty(top)
        blank!(pr)
        for c in top
            conversation!(pr, c, 0)
        end
    end
    for k in ovalues(d.keys)
        blank!(pr)
        emit!(pr, 0, join(["key", k.id, tail(k, (), ())...], " "), k)
        for x in k.items
            emit!(pr, 1, join(["prop", x.id, tail(x, PROP_KEYS, ())...], " "), x)
        end
    end
    if !isempty(d.assocs)
        blank!(pr)
        for a in d.assocs
            keys = a.kind == "reads" ? READS_KEYS : WRITES_KEYS
            emit!(pr, 0, join([a.kind, a.source, a.target, tail(a, keys, ())...], " "), a)
        end
    end
    if !isempty(d.notes) || !isempty(d.groups)
        blank!(pr)
        for n in d.notes
            emit!(pr, 0, join(["note", n.id, tail(n, NOTE_KEYS, ())...], " "), n)
        end
        for g in d.groups
            emit!(pr, 0, join(["group", g.id, tail(g, GROUP_KEYS, ())...], " "), g)
        end
    end
    if lang_07(d)
        drg!(pr)
    else
        for dec in ovalues(d.decisions)
            blank!(pr)
            decision!(pr, dec)
        end
    end
    for f in ovalues(d.forms)
        blank!(pr)
        form!(pr, f)
    end
    if d.simulate !== nothing || !isempty(d.resources) || !isempty(d.variables)
        blank!(pr)
        if d.simulate !== nothing
            emit!(pr, 0, join(["simulate", tail(d.simulate, SIMULATE_KEYS, ())...], " "), d.simulate)
        end
        for r in ovalues(d.resources)
            emit!(pr, 0, join(["resource", r.id, tail(r, RESOURCE_KEYS, ())...], " "), r)
        end
        for v in d.variables
            emit!(pr, 0, "variable $(v.id) $(q(v.label))", v)
        end
    end
    if !isempty(d.end_comments)
        blank!(pr)
        for c in d.end_comments
            push!(pr.out, comment_line("", c))
        end
    end
    while !isempty(pr.out) && pr.out[end] == ""
        pop!(pr.out)
    end
    return join(pr.out, "\n") * "\n"
end

function node!(pr::Printer, n::Elem, indent::Int)
    emit!(pr, indent, join([n.kind, n.id, tail(n, node_keys(n.kind), node_flags(n.kind); quote_type=true)...], " "), n)
    if n.kind in SUB_KINDS || n.kind == "subchoreo"
        children!(pr, n.children, indent + 1)
    end
end

"A conversation node and, for a sub-conversation, the nodes of its block."
function conversation!(pr::Printer, c::Elem, indent::Int)
    emit!(pr, indent, join([c.kind, c.id, tail(c, conversation_keys(c.kind), ())...], " "), c)
    for k in c.children
        conversation!(pr, pr.doc.conversations[k], indent + 1)
    end
end

"The choreography of a sequence flow: that of its source, else of its target."
function flow_choreography(d::Document, f::Elem)
    haskey(d.nodes, f.source) && return d.nodes[f.source].choreography
    haskey(d.nodes, f.target) && return d.nodes[f.target].choreography
    return nothing
end

function children!(pr::Printer, ids::Vector{String}, indent::Int)
    d = pr.doc
    datas = Elem[]
    for i in ids
        if haskey(d.nodes, i)
            node!(pr, d.nodes[i], indent)
        elseif haskey(d.lanes, i)
            lane = d.lanes[i]
            emit!(pr, indent, join(["lane", lane.id, tail(lane, LANE_KEYS, ())...], " "), lane)
            children!(pr, lane.children, indent + 1)
        elseif haskey(d.data, i)
            push!(datas, d.data[i])
        end
    end
    for x in datas
        keys, flags = x.kind in IO_KINDS ? (IO_KEYS, IO_FLAGS) : (DATA_KEYS, DATA_FLAGS)
        emit!(pr, indent, join([x.kind, x.id, tail(x, keys, flags)...], " "), x)
    end
end

isplain(f::Elem) = f.label === nothing && isempty(f.attrs) && isempty(f.flags) && !f.explicit_id

function flows!(pr::Printer, flows::Vector{Elem}, indent::Int)
    out_count = Dict{String,Int}()
    for f in flows
        out_count[f.source] = get(out_count, f.source, 0) + 1
    end
    chain = nothing
    chain_el = nothing
    for f in flows
        if chain !== nothing && isplain(f) && f.source == chain[end] && get(out_count, f.source, 0) == 1 &&
           isempty(f.comments) && (f.trailing === nothing || isempty(f.trailing))
            push!(chain, f.target)
            continue
        end
        if chain !== nothing
            emit!(pr, indent, "flow " * join(chain, " -> "), chain_el)
            chain = nothing
        end
        if isplain(f)
            chain, chain_el = [f.source, f.target], f
        else
            parts = ["flow", f.source, "->", f.target, tail(f, FLOW_KEYS, FLOW_FLAGS)...]
            f.explicit_id && push!(parts, "id=" * f.id)
            emit!(pr, indent, join(parts, " "), f)
        end
    end
    chain !== nothing && emit!(pr, indent, "flow " * join(chain, " -> "), chain_el)
end

function decision!(pr::Printer, dec::Elem)
    emit!(pr, 0, join(["decision", dec.id, tail(dec, DECISION_KEYS, ())...], " "), dec)
    hin = ["$(a): $(b)" for (a, b) in dec.inputs]
    hout = ["$(a): $(b)" for (a, b) in dec.outputs]
    rows = Any[(hin, hout, dec.has_annotations ? something(dec.annotation_title, "") : nothing, nothing)]
    ann = any(r -> r.annotation !== nothing && !isempty(r.annotation), dec.rules) || dec.has_annotations
    for r in dec.rules
        push!(rows, (r.inputs, r.outputs, ann ? something(r.annotation, "") : nothing, r))
    end
    nin = length(dec.inputs)
    ncol = nin + length(dec.outputs)
    widths = zeros(Int, ncol)
    awidth = 0
    for (ins, outs, a, _) in rows
        for (k, c) in enumerate(vcat(ins, outs))
            k <= ncol && (widths[k] = max(widths[k], length(c)))
        end
        a !== nothing && (awidth = max(awidth, length(a)))
    end
    for (ins, outs, a, r) in rows
        cells = vcat(ins, outs)
        left = join([ljust(cells[k], widths[k]) for k in 1:nin], " | ")
        right = join([ljust(cells[k], widths[k]) for k in nin+1:ncol], " | ")
        text = "| $(left) || $(right) |"
        if ann
            text *= "| " * ljust(something(a, ""), awidth) * " |"
        end
        if r !== nothing
            for c in r.comments
                push!(pr.out, "  # " * c)
            end
            if r.trailing !== nothing && !isempty(r.trailing)
                text *= "  # " * r.trailing
            end
        end
        push!(pr.out, "  " * pyrstrip(text))
    end
end

function form!(pr::Printer, f::Elem)
    emit!(pr, 0, join(["form", f.id, tail(f, (), ())...], " "), f)
    for it in f.items
        if it.kind == "text"
            emit!(pr, 1, "text " * q(it.label), it)
        else
            emit!(pr, 1, join(["field", it.id, tail(it, FIELD_KEYS, FIELD_FLAGS)...], " "), it)
        end
    end
end

"""
    fmt(doc) -> String

Canonical text of a parsed document.
"""
fmt(doc::Document) = run!(Printer(doc, String[]))
