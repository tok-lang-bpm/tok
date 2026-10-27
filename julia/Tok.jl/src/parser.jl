# Parser: lines of tokens -> Document (spec §4–§11).

const ID_RE = r"^[A-Za-z_][A-Za-z0-9_-]*$"

const NODE_LIKE = Set{String}([NODE_KINDS..., "flow", "message", "data", "store", "reads", "writes", "note",
                                "input", "output", "property"])
const ALLOWED = Dict{String,Set{String}}(
    "top" => union(NODE_LIKE, Set(["pool", "group", "decision", "form", "simulate", "resource", "variable",
                                   "item", "global", "interface", "key"])),
    "pool" => union(NODE_LIKE, Set(["lane"])),
    "lane" => union(NODE_LIKE, Set(["lane"])),
    "sub" => NODE_LIKE,
    "eventsub" => NODE_LIKE,
    "adhoc" => NODE_LIKE,
    "transaction" => NODE_LIKE,
    "decision" => Set{String}(),
    "form" => Set(["text", "field"]),
    "interface" => Set(["operation"]),
    "key" => Set(["prop"]),
)
const KEYWORDS = union(values(ALLOWED)..., Set(["tok"]))

struct Ctx
    kind::String
    scope::MaybeStr
    lane::MaybeStr
    pool::MaybeStr
    owner::Union{Nothing,Elem}
end
Ctx(kind; scope=nothing, lane=nothing, pool=nothing, owner=nothing) = Ctx(kind, scope, lane, pool, owner)

# result of a statement: element, whether it accepts a block, context of that block
const StResult = Tuple{Elem,Bool,Union{Nothing,Ctx}}

mutable struct Parser
    doc::Document
    diags::Vector{Diag}
    lines::Vector{Line}
    pending::Vector{String}
    flow_counter::Dict{String,Int}
end

function Parser(text::AbstractString)
    lines, errors = tokenize(text)
    diags = Diag[]
    for (code, line, msg) in errors
        push!(diags, Diag(code, line; detail=msg))
    end
    Parser(Document(), diags, lines, String[], Dict{String,Int}())
end

err!(p::Parser, code, line, id=nothing; kw...) = push!(p.diags, Diag(code, line, id; kw...))

function take_comments!(p::Parser, el::Elem, line::Line)
    el.comments = p.pending
    el.trailing = line.comment
    p.pending = String[]
end

"Record an identifier. Return false (after reporting R001) when it is already used."
function register!(p::Parser, el::Elem, line_no::Int)
    el.id === nothing && return true
    occursin(ID_RE, el.id) || err!(p, "R004", line_no, el.id; id=el.id)
    other = get(p.doc.ids, el.id, nothing)
    if other !== nothing
        err!(p, "R001", line_no, el.id; id=el.id, other=other.line)
        return false
    end
    p.doc.ids[el.id] = el
    return true
end

"""
Parse [STRING] attributes from toks[start:]. Return (label, attrs, flags, ok, extra).
A repeated key replaces the earlier value, except that with `repeat` the further
values of REPEATABLE_KEYS are kept in `extra` (language 0.5).
"""
function parse_tail(p::Parser, toks::Vector{Token}, start::Int, line_no::Int; allow_label=true, repeat=false)
    label = nothing
    attrs = Dict{String,String}()
    extra = Tuple{String,String}[]
    flags = String[]
    i = start
    n = length(toks)
    if allow_label && i <= n && toks[i].kind == STRING
        label = toks[i].value
        i += 1
    end
    while i <= n
        t = toks[i]
        if t.kind == WORD && i + 1 <= n && toks[i+1].kind == EQUALS
            if i + 2 > n || !(toks[i+2].kind in (WORD, NUMBER, STRING))
                err!(p, "P007", line_no; near=t.value)
                return label, attrs, flags, false, extra
            end
            if repeat && t.value in REPEATABLE_KEYS && haskey(attrs, t.value)
                push!(extra, (t.value, toks[i+2].value))
            else
                attrs[t.value] = toks[i+2].value
            end
            i += 3
        elseif t.kind == WORD
            push!(flags, t.value)
            i += 1
        else
            err!(p, "P007", line_no; near=t.value)
            return label, attrs, flags, false, extra
        end
    end
    return label, attrs, flags, true, extra
end

function run!(p::Parser)
    lines = p.lines
    i = 1
    while i <= length(lines) && lines[i].standalone
        push!(p.pending, lines[i].comment)
        i += 1
    end
    if i > length(lines) || isempty(lines[i].tokens) || lines[i].tokens[1].value != "tok"
        err!(p, "P005", i <= length(lines) ? lines[i].number : 1)
    else
        header!(p, lines[i])
        i += 1
    end
    block!(p, i, -1, Ctx("top"; scope=default_process_id(p.doc)))
    p.doc.end_comments = p.pending
    p.pending = String[]
    return p.doc, p.diags
end

"Parse sibling statements indented deeper than parent_indent. Return the next line index."
function block!(p::Parser, i::Int, parent_indent::Int, ctx::Ctx)
    lines = p.lines
    indent = nothing
    prev = nothing
    while i <= length(lines)
        line = lines[i]
        if line.standalone
            push!(p.pending, line.comment)
            i += 1
            continue
        end
        line.indent <= parent_indent && return i
        indent === nothing && (indent = line.indent)
        if line.indent > indent
            if prev !== nothing && prev[2] && prev[3] === nothing
                i = skip_block(p, i, indent)
                prev = (prev[1], false, nothing)
            elseif prev !== nothing && prev[2]
                if prev[1].kind == "pool"
                    prev[1].blackbox = false
                end
                i = block!(p, i, indent, prev[3])
                prev = (prev[1], false, nothing)
            else
                kw = prev !== nothing ? prev[1].kind : "?"
                err!(p, "P003", line.number; keyword=kw)
                i = skip_block(p, i, indent)
            end
            continue
        end
        if line.indent < indent
            err!(p, "T003", line.number)
            return i
        end
        prev = statement!(p, line, ctx)
        i += 1
    end
    return i
end

function skip_block(p::Parser, i::Int, indent::Int)
    while i <= length(p.lines) && (p.lines[i].standalone || p.lines[i].indent > indent)
        i += 1
    end
    return i
end

function header!(p::Parser, line::Line)
    toks = line.tokens
    d = p.doc
    d.header_line = line.number
    d.header_comments = p.pending
    d.header_trailing = line.comment
    p.pending = String[]
    if length(toks) < 2 || toks[2].kind != NUMBER
        err!(p, "P005", line.number)
        return
    end
    d.version = toks[2].value
    d.version in LANGUAGE_VERSIONS || err!(p, "P006", line.number; version=d.version)
    d.title, d.attrs, d.flags, _ = parse_tail(p, toks, 3, line.number)
end

noresult(kind) = (Elem(kind, nothing), false, nothing)

function statement!(p::Parser, line::Line, ctx::Ctx)::StResult
    toks = line.tokens
    first = toks[1]
    kw = first.kind == WORD ? first.value : nothing
    if first.kind == ROW
        if ctx.kind == "decision"
            row!(p, line, ctx.owner)
        else
            err!(p, "P004", line.number; keyword="|", context=ctx.kind)
        end
        return noresult("row")
    end
    if kw == "tok"
        err!(p, "P010", line.number)
        return noresult("tok")
    end
    if kw === nothing || !(kw in KEYWORDS)
        err!(p, "P001", line.number; word=first.value)
        return noresult("?")
    end
    if !(kw in ALLOWED[ctx.kind])
        err!(p, "P004", line.number; keyword=kw, context=ctx.kind)
        return noresult(kw)
    end
    kw == "pool" && return st_pool!(p, line, ctx)
    kw == "lane" && return st_lane!(p, line, ctx)
    kw == "flow" && return st_flow!(p, line, ctx)
    kw == "message" && return st_message!(p, line, ctx)
    kw in ("data", "store", "input", "output", "property") && return st_data!(p, line, ctx, kw)
    kw in ("item", "global") && return st_declaration!(p, line, kw)
    kw in ("interface", "key") && return st_block_declaration!(p, line, kw)
    kw in ("operation", "prop") && return st_member!(p, line, ctx, kw)
    kw == "reads" && return st_assoc!(p, line, ctx, "reads")
    kw == "writes" && return st_assoc!(p, line, ctx, "writes")
    kw == "note" && return st_simple!(p, line, "note", p.doc.notes)
    kw == "group" && return st_simple!(p, line, "group", p.doc.groups)
    kw == "decision" && return st_decision!(p, line, ctx)
    kw == "form" && return st_form!(p, line, ctx)
    kw == "text" && return st_text!(p, line, ctx)
    kw == "field" && return st_field!(p, line, ctx)
    kw == "simulate" && return st_simulate!(p, line, ctx)
    kw == "resource" && return st_resource!(p, line, ctx)
    kw == "variable" && return st_variable!(p, line, ctx)
    return st_node!(p, line, ctx)
end

function need_id(p::Parser, line::Line, pos=2)
    toks = line.tokens
    if length(toks) < pos || !(toks[pos].kind in (WORD, NUMBER))
        err!(p, "P002", line.number; keyword=toks[1].value)
        return nothing
    end
    return toks[pos].value
end

function add_child!(p::Parser, ctx::Ctx, id::String)
    if ctx.kind == "top"
        push!(p.doc.top, id)
    elseif ctx.kind in ("pool", "lane", SUB_KINDS...) && ctx.owner !== nothing
        push!(ctx.owner.children, id)
    end
end

function st_node!(p::Parser, line::Line, ctx::Ctx)::StResult
    kind = line.tokens[1].value
    nid = need_id(p, line)
    if nid === nothing
        return (Elem(kind, nothing), kind in SUB_KINDS, nothing)
    end
    # a key written twice in a 0.1 or 0.3 document keeps its last value, as before
    label, attrs, flags, _, extra = parse_tail(p, line.tokens, 3, line.number; repeat=lang_05(p.doc))
    node = Elem(kind, nid; label=label, attrs=attrs, flags=flags, line=line.number)
    node.extra = extra
    take_comments!(p, node, line)
    node.scope = ctx.scope
    node.pool = ctx.pool
    node.lane = ctx.kind == "lane" ? ctx.lane : nothing
    register!(p, node, line.number) || return (node, kind in SUB_KINDS, nothing)
    p.doc.nodes[nid] = node
    add_child!(p, ctx, nid)
    if kind in SUB_KINDS
        return (node, true, Ctx(kind; scope=nid, lane=nothing, pool=ctx.pool, owner=node))
    end
    return (node, false, nothing)
end

function st_pool!(p::Parser, line::Line, ctx::Ctx)::StResult
    pid = need_id(p, line)
    pid === nothing && return noresult("pool")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    pool = Elem("pool", pid; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, pool, line)
    register!(p, pool, line.number) || return (pool, true, nothing)
    push!(p.doc.pools, pool)
    return (pool, true, Ctx("pool"; scope=process_id(pool), lane=nothing, pool=pid, owner=pool))
end

function st_lane!(p::Parser, line::Line, ctx::Ctx)::StResult
    lid = need_id(p, line)
    lid === nothing && return noresult("lane")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    lane = Elem("lane", lid; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, lane, line)
    lane.pool = ctx.pool
    lane.parent = ctx.lane
    register!(p, lane, line.number) || return (lane, true, nothing)
    p.doc.lanes[lid] = lane
    push!(ctx.owner.children, lid)
    return (lane, true, Ctx("lane"; scope=ctx.scope, lane=lid, pool=ctx.pool, owner=lane))
end

function st_flow!(p::Parser, line::Line, ctx::Ctx)::StResult
    toks = line.tokens
    ids = String[]
    i = 2
    while true
        if i > length(toks) || !(toks[i].kind in (WORD, NUMBER))
            err!(p, "P008", line.number; keyword="flow", detail="expected an identifier")
            return noresult("flow")
        end
        push!(ids, toks[i].value)
        i += 1
        if i <= length(toks) && toks[i].kind == ARROW
            i += 1
            continue
        end
        break
    end
    if length(ids) < 2
        err!(p, "P008", line.number; keyword="flow", detail="expected '->'")
        return noresult("flow")
    end
    label, attrs, flags, _ = parse_tail(p, toks, i, line.number)
    if (label !== nothing || !isempty(attrs) || !isempty(flags)) && length(ids) > 2
        err!(p, "R020", line.number)
        label, attrs, flags = nothing, Dict{String,String}(), String[]
    end
    first = nothing
    for k in 1:length(ids)-1
        src, tgt = ids[k], ids[k+1]
        explicit = k == 1 ? get(attrs, "id", nothing) : nothing
        fid = (explicit !== nothing && !isempty(explicit)) ? explicit : derive_flow_id!(p, src, tgt)
        f = Elem("flow", fid; label=label, attrs=copy(attrs), flags=copy(flags), line=line.number)
        f.source, f.target = src, tgt
        f.explicit_id = explicit !== nothing
        delete!(f.attrs, "id")
        f.chain_start = k == 1
        if k == 1
            take_comments!(p, f, line)
            first = f
        end
        register!(p, f, line.number)
        push!(p.doc.flows, f)
    end
    return (first, false, nothing)
end

derive_flow_id!(p::Parser, src, tgt) = derive_id!(p, "f-$(src)-$(tgt)")

function derive_id!(p::Parser, base::String)
    n = get(p.flow_counter, base, 0) + 1
    p.flow_counter[base] = n
    fid = n == 1 ? base : "$(base)-$(n)"
    while haskey(p.doc.ids, fid)
        n += 1
        p.flow_counter[base] = n
        fid = "$(base)-$(n)"
    end
    return fid
end

function st_message!(p::Parser, line::Line, ctx::Ctx)::StResult
    toks = line.tokens
    if length(toks) < 4 || !(toks[2].kind in (WORD, NUMBER)) || toks[3].kind != ARROW ||
       !(toks[4].kind in (WORD, NUMBER))
        err!(p, "P008", line.number; keyword="message", detail="expected 'message A -> B'")
        return noresult("message")
    end
    label, attrs, flags, _ = parse_tail(p, toks, 5, line.number)
    src, tgt = toks[2].value, toks[4].value
    mid = pop!(attrs, "id", nothing)
    id = (mid !== nothing && !isempty(mid)) ? mid : derive_id!(p, "m-$(src)-$(tgt)")
    m = Elem("message", id; label=label, attrs=attrs, line=line.number)
    m.source, m.target = src, tgt
    m.explicit_id = mid !== nothing
    for fl in flags
        err!(p, "R011", line.number; flag=fl, kind="message")
    end
    take_comments!(p, m, line)
    register!(p, m, line.number)
    push!(p.doc.messages, m)
    return (m, false, nothing)
end

function st_data!(p::Parser, line::Line, ctx::Ctx, kind::String)::StResult
    did = need_id(p, line)
    did === nothing && return noresult(kind)
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    d = Elem(kind, did; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, d, line)
    if ctx.kind == "top"
        d.block = nothing
        d.scope = nothing
    else
        d.scope = ctx.scope
        d.block = ctx.kind == "lane" ? ctx.pool : ctx.owner.id
    end
    register!(p, d, line.number) || return (d, false, nothing)
    p.doc.data[did] = d
    if ctx.kind == "lane"
        for pool in p.doc.pools
            if pool.id == ctx.pool
                push!(pool.children, did)
                break
            end
        end
    else
        add_child!(p, ctx, did)
    end
    return (d, false, nothing)
end

function st_assoc!(p::Parser, line::Line, ctx::Ctx, kind::String)::StResult
    toks = line.tokens
    if length(toks) < 3 || toks[2].kind != WORD || toks[3].kind != WORD
        err!(p, "P008", line.number; keyword=kind, detail="expected '$(kind) ACTIVITY DATA'")
        return noresult(kind)
    end
    # language 0.5: `input=` / `output=` name the data input or output of the activity
    _, attrs, flags, _ = parse_tail(p, toks, 4, line.number; allow_label=false)
    a = Elem(kind, "$(kind)-$(toks[2].value)-$(toks[3].value)"; attrs=attrs, flags=flags, line=line.number)
    a.source, a.target = toks[2].value, toks[3].value     # activity, data
    take_comments!(p, a, line)
    push!(p.doc.assocs, a)
    return (a, false, nothing)
end

function st_simple!(p::Parser, line::Line, kind::String, list::Vector{Elem})::StResult
    id = need_id(p, line)
    id === nothing && return noresult(kind)
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    e = Elem(kind, id; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    register!(p, e, line.number)
    push!(list, e)
    return (e, false, nothing)
end

function st_decision!(p::Parser, line::Line, ctx::Ctx)::StResult
    did = need_id(p, line)
    did === nothing && return noresult("decision")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    d = Elem("decision", did; label=label, attrs=attrs, line=line.number)
    for fl in flags
        err!(p, "R011", line.number; flag=fl, kind="decision")
    end
    take_comments!(p, d, line)
    register!(p, d, line.number)
    p.doc.decisions[did] = d
    d.header_done = false
    return (d, true, Ctx("decision"; owner=d))
end

function row!(p::Parser, line::Line, d::Elem)
    cells = split_row(line.tokens[1].value)
    if cells === nothing
        err!(p, "P009", line.number; detail="unbalanced quotes")
        return
    end
    groups = [String[]]
    for c in cells
        if c == ""
            push!(groups, String[])
        else
            push!(groups[end], pystrip(c))
        end
    end
    if length(groups) < 2 || length(groups) > 3
        err!(p, "P009", line.number; detail="expected inputs || outputs [|| annotation]")
        return
    end
    ins, outs = groups[1], groups[2]
    ann = length(groups) == 3 ? join([x for x in groups[3] if !isempty(x)], " ") : nothing
    if !d.header_done
        d.header_done = true
        for cell in ins
            hc = split_header_cell(cell)
            hc === nothing ? err!(p, "D004", line.number, d.id; cell=cell) : push!(d.inputs, hc)
        end
        for cell in outs
            hc = split_header_cell(cell)
            hc === nothing ? err!(p, "D004", line.number, d.id; cell=cell) : push!(d.outputs, hc)
        end
        d.has_annotations = length(groups) == 3
        d.annotation_title = ann
        return
    end
    if length(ins) != length(d.inputs) || length(outs) != length(d.outputs)
        err!(p, "D002", line.number, d.id; got=length(ins) + length(outs),
             expected=length(d.inputs) + length(d.outputs))
        return
    end
    r = Rule(ins, outs, (ann === nothing || isempty(ann)) ? nothing : ann, line.number, p.pending, line.comment)
    p.pending = String[]
    push!(d.rules, r)
end

function st_form!(p::Parser, line::Line, ctx::Ctx)::StResult
    fid = need_id(p, line)
    fid === nothing && return noresult("form")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    f = Elem("form", fid; label=label, attrs=attrs, line=line.number)
    take_comments!(p, f, line)
    register!(p, f, line.number)
    p.doc.forms[fid] = f
    return (f, true, Ctx("form"; owner=f))
end

function st_text!(p::Parser, line::Line, ctx::Ctx)::StResult
    toks = line.tokens
    if length(toks) != 2 || toks[2].kind != STRING
        err!(p, "P008", line.number; keyword="text", detail="expected text \"…\"")
        return noresult("text")
    end
    t = Elem("text", nothing; label=toks[2].value, line=line.number)
    take_comments!(p, t, line)
    push!(ctx.owner.items, t)
    return (t, false, nothing)
end

function st_field!(p::Parser, line::Line, ctx::Ctx)::StResult
    key = need_id(p, line)
    key === nothing && return noresult("field")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    f = Elem("field", key; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, f, line)
    push!(ctx.owner.items, f)
    return (f, false, nothing)
end

function st_simulate!(p::Parser, line::Line, ctx::Ctx)::StResult
    _, attrs, flags, _ = parse_tail(p, line.tokens, 2, line.number; allow_label=false)
    s = Elem("simulate", nothing; attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, s, line)
    p.doc.simulate = s
    return (s, false, nothing)
end

function st_resource!(p::Parser, line::Line, ctx::Ctx)::StResult
    rid = need_id(p, line)
    rid === nothing && return noresult("resource")
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    r = Elem("resource", rid; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, r, line)
    register!(p, r, line.number)
    p.doc.resources[rid] = r
    return (r, false, nothing)
end

function st_variable!(p::Parser, line::Line, ctx::Ctx)::StResult
    toks = line.tokens
    if length(toks) != 3 || toks[2].kind != WORD || toks[3].kind != STRING
        err!(p, "P008", line.number; keyword="variable", detail="expected variable NAME \"DIST\"")
        return noresult("variable")
    end
    v = Elem("variable", toks[2].value; label=toks[3].value, line=line.number)
    take_comments!(p, v, line)
    push!(p.doc.variables, v)
    return (v, false, nothing)
end

"`item` and `global` (language 0.5): top-level declarations without a block."
function st_declaration!(p::Parser, line::Line, kind::String)::StResult
    id = need_id(p, line)
    id === nothing && return noresult(kind)
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    e = Elem(kind, id; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    if register!(p, e, line.number)
        (kind == "item" ? p.doc.items : p.doc.globals)[id] = e
    end
    return (e, false, nothing)
end

"`interface` and `key` (language 0.5): declarations with a block of `operation` or `prop` lines."
function st_block_declaration!(p::Parser, line::Line, kind::String)::StResult
    id = need_id(p, line)
    id === nothing && return (Elem(kind, nothing), true, nothing)
    label, attrs, flags, _ = parse_tail(p, line.tokens, 3, line.number)
    e = Elem(kind, id; label=label, attrs=attrs, flags=flags, line=line.number)
    take_comments!(p, e, line)
    register!(p, e, line.number) || return (e, true, nothing)
    (kind == "interface" ? p.doc.interfaces : p.doc.keys)[id] = e
    return (e, true, Ctx(kind; owner=e))
end

"`operation` of an interface and `prop` of a correlation key (language 0.5)."
function st_member!(p::Parser, line::Line, ctx::Ctx, kind::String)::StResult
    id = need_id(p, line)
    id === nothing && return noresult(kind)
    label, attrs, flags, _, extra = parse_tail(p, line.tokens, 3, line.number; repeat=kind == "operation")
    e = Elem(kind, id; label=label, attrs=attrs, flags=flags, line=line.number)
    e.extra = extra
    take_comments!(p, e, line)
    if kind == "operation"
        register!(p, e, line.number) && push!(ctx.owner.items, e)
    else
        occursin(ID_RE, id) || err!(p, "R004", line.number, id; id=id)
        push!(ctx.owner.items, e)
    end
    return (e, false, nothing)
end

"Split a table row into cells by '|' outside quotes. Empty cells mark '||'. `nothing` on unbalanced quotes."
function split_row(text::AbstractString)
    s = collect(pystrip(text))
    if !isempty(s) && s[1] == '|'
        s = s[2:end]
    end
    if !isempty(s) && s[end] == '|' && !(length(s) >= 2 && s[end-1] == '\\')
        s = s[1:end-1]
    end
    cells = String[]
    buf = Char[]
    in_str = false
    i = 1
    while i <= length(s)
        c = s[i]
        if c == '\\' && in_str && i + 1 <= length(s)
            push!(buf, c, s[i+1])
            i += 2
            continue
        end
        c == '"' && (in_str = !in_str)
        if c == '|' && !in_str
            push!(cells, String(buf))
            empty!(buf)
        else
            push!(buf, c)
        end
        i += 1
    end
    in_str && return nothing
    push!(cells, String(buf))
    return cells
end

function split_header_cell(cell::AbstractString)
    occursin(':', cell) || return nothing
    k = findlast(':', cell)
    name = pystrip(cell[1:prevind(cell, k)])
    typ = pystrip(cell[nextind(cell, k):end])
    (isempty(name) || !(typ in ("string", "number", "boolean", "date"))) && return nothing
    return (name, typ)
end

"""
    parse_tok(text) -> (Document, Vector{Diag})

Parse a tok document.
"""
parse_tok(text::AbstractString) = run!(Parser(text))
