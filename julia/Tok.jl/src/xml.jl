# Semantic BPMN 2.0 export (no diagram interchange) and an XML writer that
# reproduces the output of Python's xml.etree.ElementTree byte for byte.



const BPMN_NS = "http://www.omg.org/spec/BPMN/20100524/MODEL"
const XSI_NS = "http://www.w3.org/2001/XMLSchema-instance"
const ZEEBE_NS = "http://camunda.org/schema/zeebe/1.0"
const MODELER_NS = "http://camunda.org/schema/modeler/1.0"
const PREFIXES = Dict(BPMN_NS => "bpmn", XSI_NS => "xsi", ZEEBE_NS => "zeebe", MODELER_NS => "modeler")

const TAGS = Dict(
    "start" => "startEvent", "end" => "endEvent", "catch" => "intermediateCatchEvent",
    "throw" => "intermediateThrowEvent", "boundary" => "boundaryEvent",
    "task" => "task", "user" => "userTask", "service" => "serviceTask", "manual" => "manualTask",
    "script" => "scriptTask", "rule" => "businessRuleTask", "send" => "sendTask", "receive" => "receiveTask",
    "call" => "callActivity", "sub" => "subProcess", "eventsub" => "subProcess",
    "xor" => "exclusiveGateway", "and" => "parallelGateway", "or" => "inclusiveGateway",
    "eventgw" => "eventBasedGateway",
)
const DEFINITION = Dict(
    "message" => "messageEventDefinition", "timer" => "timerEventDefinition", "signal" => "signalEventDefinition",
    "error" => "errorEventDefinition", "escalation" => "escalationEventDefinition",
    "condition" => "conditionalEventDefinition", "link" => "linkEventDefinition",
    "terminate" => "terminateEventDefinition", "compensate" => "compensateEventDefinition",
)

# ------------------------------------------------------------------ tiny DOM

mutable struct XElem
    ns::String
    name::String
    attrs::Vector{Tuple{String,String,String}}   # (namespace, local name, value)
    children::Vector{XElem}
    text::Union{Nothing,String}
    tail::Union{Nothing,String}
end

XElem(ns, name) = XElem(ns, name, Tuple{String,String,String}[], XElem[], nothing, nothing)

function setattr!(e::XElem, name::String, v::AbstractString; ns::String="")
    for (k, (ans, an, _)) in enumerate(e.attrs)
        if ans == ns && an == name
            e.attrs[k] = (ns, name, String(v))
            return e
        end
    end
    push!(e.attrs, (ns, name, String(v)))
    return e
end

function getattr(e::XElem, name::String)
    for (ans, an, v) in e.attrs
        ans == "" && an == name && return v
    end
    return nothing
end

function subelem!(parent::XElem, ns::String, name::String, attrs::Pair...)
    e = XElem(ns, name)
    for (k, v) in attrs
        v === nothing || setattr!(e, String(k), string(v))
    end
    push!(parent.children, e)
    return e
end

"Insert like Python's list.insert (index 0-based, clamped)."
function insert_at!(parent::XElem, pos::Int, child::XElem)
    pos = clamp(pos, 0, length(parent.children))
    insert!(parent.children, pos + 1, child)
end

function iter_elems(f, e::XElem)
    f(e)
    for c in e.children
        iter_elems(f, c)
    end
end

clark(ns, name) = ns == "" ? name : "{" * ns * "}" * name

# ------------------------------------------------------------------ helpers

function slug(text::AbstractString)
    t = replace(text, "ł" => "l", "Ł" => "L")
    t = Base.Unicode.normalize(t, :NFKD)
    t = filter(isascii, t)
    t = replace(t, r"[^A-Za-z0-9]+" => "_")
    t = String(strip(==('_'), t))
    if isempty(t) || !occursin(r"^[A-Za-z_]", t)
        t = "_" * t
    end
    return t
end

function timer_kind(v::AbstractString)
    v = pystrip(v)
    startswith(v, "R") && return "timeCycle"
    startswith(v, "P") && return "timeDuration"
    return "timeDate"
end

mutable struct Decl
    id::String
    code::MaybeStr
    correlation::MaybeStr
end

mutable struct Exporter
    doc::Document
    camunda::Bool
    decls::Dict{String,ODict{Decl}}
    stores::ODict{String}
    containers::ODict{XElem}
    root::XElem
    collab::Union{Nothing,XElem}
end

function Exporter(doc::Document, target=nothing)
    t = target !== nothing ? target : get(doc.attrs, "target", "camunda")
    camunda = executable_any(doc) && t == "camunda"
    decls = Dict(k => ODict{Decl}() for k in ("message", "signal", "error", "escalation"))
    Exporter(doc, camunda, decls, ODict{String}(), ODict{XElem}(), XElem(BPMN_NS, "definitions"), nothing)
end

bsub!(parent, name, attrs::Pair...) = subelem!(parent, BPMN_NS, name, attrs...)

function ext!(e::XElem)
    for c in e.children
        c.ns == BPMN_NS && c.name == "extensionElements" && return c
    end
    x = XElem(BPMN_NS, "extensionElements")
    insert_at!(e, 0, x)
    return x
end

feel(ex::Exporter, expr::AbstractString) = ex.camunda ? "=" * expr : String(expr)

function declare!(ex::Exporter, kind::String, name::String, code=nothing)
    reg = ex.decls[kind]
    if !haskey(reg, name)
        prefix = Dict("message" => "Message", "signal" => "Signal", "error" => "Error", "escalation" => "Escalation")[kind]
        reg[name] = Decl("$(prefix)_$(slug(name))", code, nothing)
    elseif code !== nothing && !isempty(code) && (reg[name].code === nothing || isempty(reg[name].code))
        reg[name].code = code
    end
    return reg[name]
end

nonempty(s) = s !== nothing && !isempty(s)

function run!(ex::Exporter)
    d = ex.doc
    root = ex.root
    setattr!(root, "id", get(d.attrs, "id", "definitions"))
    nonempty(d.title) && setattr!(root, "name", d.title)
    setattr!(root, "targetNamespace", get(d.attrs, "namespace", "http://bpmn.io/schema/bpmn"))
    setattr!(root, "exporter", "tok")
    setattr!(root, "exporterVersion", d.version === nothing ? "0.1" : d.version)
    if ex.camunda
        setattr!(root, "executionPlatform", "Camunda Cloud"; ns=MODELER_NS)
        setattr!(root, "executionPlatformVersion", "8.10.0"; ns=MODELER_NS)
    end
    decl_slot = length(root.children)

    processes = Tuple{String,MaybeStr,Bool,Union{Nothing,Elem}}[]
    if !isempty(d.pools)
        collab = bsub!(root, "collaboration", "id" => get(d.attrs, "collaboration", "collaboration"))
        inner = [p for p in d.pools if !p.blackbox]
        for p in d.pools
            part = bsub!(collab, "participant", "id" => p.id, "name" => p.label)
            if !p.blackbox
                setattr!(part, "processRef", process_id(p))
                pname = (length(inner) == 1 && nonempty(d.title)) ? d.title : p.label
                push!(processes, (process_id(p), pname, has(p, "executable") || is_executable(d), p))
            end
        end
        for m in d.messages
            mf = bsub!(collab, "messageFlow", "id" => m.id, "name" => m.label, "sourceRef" => m.source,
                       "targetRef" => m.target)
            name = nonempty(get(m.attrs, "message", nothing)) ? m.attrs["message"] : m.label
            nonempty(name) && setattr!(mf, "messageRef", declare!(ex, "message", name).id)
        end
        ex.collab = collab
    else
        ex.collab = nothing
        push!(processes, (default_process_id(d), d.title, is_executable(d), nothing))
    end

    for (pid, name, executable, pool) in processes
        proc = bsub!(root, "process", "id" => pid, "name" => name, "isExecutable" => executable ? "true" : "false")
        ex.containers[pid] = proc
        if pool !== nothing
            lanes = [d.lanes[c] for c in pool.children if haskey(d.lanes, c)]
            if !isempty(lanes)
                ls = bsub!(proc, "laneSet", "id" => "$(pool.id)-lanes")
                for lane in lanes
                    export_lane!(ex, ls, lane)
                end
            end
            export_block!(ex, proc, ordered_nodes(ex, pool.children))
        else
            export_block!(ex, proc, d.top)
        end
    end

    export_flows!(ex)
    export_data!(ex)
    export_artifacts!(ex)

    decl = XElem[]
    for kind in ("message", "signal", "error", "escalation")
        reg = ex.decls[kind]
        for name in reg.keys
            info = reg[name]
            e = XElem(BPMN_NS, kind)
            setattr!(e, "id", info.id)
            setattr!(e, "name", name)
            kind == "error" && nonempty(info.code) && setattr!(e, "errorCode", info.code)
            kind == "escalation" && nonempty(info.code) && setattr!(e, "escalationCode", info.code)
            if kind == "message" && ex.camunda && nonempty(info.correlation)
                x = bsub!(e, "extensionElements")
                subelem!(x, ZEEBE_NS, "subscription", "correlationKey" => feel(ex, info.correlation))
            end
            push!(decl, e)
        end
    end
    for name in ex.stores.keys
        e = XElem(BPMN_NS, "dataStore")
        setattr!(e, "id", ex.stores[name])
        setattr!(e, "name", name)
        push!(decl, e)
    end
    for (k, e) in enumerate(decl)
        insert_at!(root, decl_slot + k - 1, e)
    end
    return root
end

function ordered_nodes(ex::Exporter, children::Vector{String})
    d = ex.doc
    out = String[]
    function walk(ids)
        for i in ids
            if haskey(d.lanes, i)
                walk(d.lanes[i].children)
            elseif haskey(d.nodes, i)
                push!(out, i)
            end
        end
    end
    walk(children)
    return out
end

function export_lane!(ex::Exporter, parent::XElem, lane::Elem)
    d = ex.doc
    e = bsub!(parent, "lane", "id" => lane.id, "name" => lane.label)
    sublanes = [d.lanes[c] for c in lane.children if haskey(d.lanes, c)]
    for c in lane.children
        if haskey(d.nodes, c)
            r = bsub!(e, "flowNodeRef")
            r.text = c
        end
    end
    if !isempty(sublanes)
        cls = bsub!(e, "childLaneSet", "id" => "$(lane.id)-lanes")
        for s in sublanes
            export_lane!(ex, cls, s)
        end
    end
end

function export_block!(ex::Exporter, container::XElem, ids::Vector{String})
    for i in ids
        n = get(ex.doc.nodes, i, nothing)
        n === nothing || export_node!(ex, container, n)
    end
end

function export_node!(ex::Exporter, container::XElem, n::Elem)
    e = bsub!(container, TAGS[n.kind], "id" => n.id, "name" => n.label)
    a = n.attrs
    if n.kind == "boundary"
        setattr!(e, "attachedToRef", get(a, "on", ""))
        has(n, "noninterrupting") && setattr!(e, "cancelActivity", "false")
    end
    n.kind == "start" && has(n, "noninterrupting") && setattr!(e, "isInterrupting", "false")
    n.kind == "eventsub" && setattr!(e, "triggeredByEvent", "true")
    n.kind == "call" && haskey(a, "process") && setattr!(e, "calledElement", a["process"])
    has(n, "compensation") && setattr!(e, "isForCompensation", "true")
    if n.kind in ("send", "receive") && haskey(a, "message")
        setattr!(e, "messageRef", declare!(ex, "message", a["message"]).id)
    end
    if n.kind == "script" && !ex.camunda && haskey(a, "expr")
        setattr!(e, "scriptFormat", "feel")
        s = bsub!(e, "script")
        s.text = a["expr"]
    end
    ex.camunda && export_zeebe!(ex, e, n)
    n.kind in EVENT_KINDS && export_event_definition!(ex, e, n)
    if has(n, "loop")
        bsub!(e, "standardLoopCharacteristics")
    elseif has(n, "parallel") || has(n, "sequential")
        bsub!(e, "multiInstanceLoopCharacteristics", "isSequential" => has(n, "sequential") ? "true" : "false")
    end
    if n.kind in SUB_KINDS
        ex.containers[n.id] = e
        export_block!(ex, e, n.children)
    end
end

function export_zeebe!(ex::Exporter, e::XElem, n::Elem)
    a = n.attrs
    if n.kind == "user"
        x = ext!(e)
        subelem!(x, ZEEBE_NS, "userTask")
        haskey(a, "form") && subelem!(x, ZEEBE_NS, "formDefinition", "formId" => a["form"])
    elseif n.kind == "start" && haskey(a, "form")
        subelem!(ext!(e), ZEEBE_NS, "formDefinition", "formId" => a["form"])
    elseif n.kind == "rule" && haskey(a, "decision")
        res = nonempty(get(a, "result", nothing)) ? a["result"] : default_result(ex, a["decision"])
        subelem!(ext!(e), ZEEBE_NS, "calledDecision", "decisionId" => a["decision"], "resultVariable" => res)
    elseif n.kind == "script" && haskey(a, "expr")
        subelem!(ext!(e), ZEEBE_NS, "script", "expression" => feel(ex, a["expr"]),
                 "resultVariable" => get(a, "result", "result"))
    elseif n.kind in ("service", "send") && haskey(a, "type")
        subelem!(ext!(e), ZEEBE_NS, "taskDefinition", "type" => a["type"])
    end
end

function default_result(ex::Exporter, decision_id::String)
    dec = get(ex.doc.decisions, decision_id, nothing)
    if dec !== nothing && length(dec.outputs) == 1
        return dec.outputs[1][1]
    end
    return "result"
end

function export_event_definition!(ex::Exporter, e::XElem, n::Elem)
    a = n.attrs
    for key in TRIGGERS
        haskey(a, key) || continue
        df = bsub!(e, DEFINITION[key], "id" => "$(n.id)-def")
        if key == "message"
            info = declare!(ex, "message", a["message"])
            haskey(a, "correlation") && (info.correlation = a["correlation"])
            setattr!(df, "messageRef", info.id)
        elseif key == "signal"
            setattr!(df, "signalRef", declare!(ex, "signal", a["signal"]).id)
        elseif key == "error"
            setattr!(df, "errorRef", declare!(ex, "error", a["error"], get(a, "code", nothing)).id)
        elseif key == "escalation"
            setattr!(df, "escalationRef", declare!(ex, "escalation", a["escalation"], get(a, "code", nothing)).id)
        elseif key == "timer"
            t = bsub!(df, timer_kind(a["timer"]))
            setattr!(t, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            t.text = a["timer"]
        elseif key == "condition"
            c = bsub!(df, "condition")
            setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            c.text = feel(ex, a["condition"])
        elseif key == "link"
            setattr!(df, "name", a["link"])
        end
        return
    end
    for flag in TRIGGER_FLAGS
        if has(n, flag)
            bsub!(e, DEFINITION[flag], "id" => "$(n.id)-def")
            return
        end
    end
end

function find_id(ex::Exporter, id)
    found = nothing
    iter_elems(ex.root) do e
        if found === nothing && getattr(e, "id") == id
            found = e
        end
    end
    return found
end

function export_flows!(ex::Exporter)
    d = ex.doc
    for f in d.flows
        src = get(d.nodes, f.source, nothing)
        tgt = get(d.nodes, f.target, nothing)
        (src === nothing || tgt === nothing || src.scope != tgt.scope) && continue
        cont = get(ex.containers, something(src.scope, ""), nothing)
        cont === nothing && continue
        e = bsub!(cont, "sequenceFlow", "id" => f.id, "name" => f.label, "sourceRef" => f.source,
                  "targetRef" => f.target)
        if haskey(f.attrs, "when")
            c = bsub!(e, "conditionExpression")
            setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            c.text = feel(ex, f.attrs["when"])
        end
        if has(f, "default")
            srcel = find_id(ex, src.id)
            srcel === nothing || setattr!(srcel, "default", f.id)
        end
    end
end

function data_scope(ex::Exporter, x::Elem)
    d = ex.doc
    nonempty(x.scope) && return x.scope
    for a in d.assocs
        if a.target == x.id && haskey(d.nodes, a.source)
            return d.nodes[a.source].scope
        end
    end
    if !isempty(d.pools)
        for p in d.pools
            p.blackbox || return process_id(p)
        end
    end
    return default_process_id(d)
end

has_child(e::XElem, name) = any(c -> c.ns == BPMN_NS && c.name == name, e.children)

function after_props(act::XElem)
    pos = 0
    for (k, c) in enumerate(act.children)
        if c.ns == BPMN_NS && c.name in ("extensionElements", "property", "dataInputAssociation", "dataOutputAssociation")
            pos = k
        end
    end
    return pos
end

function export_data!(ex::Exporter)
    d = ex.doc
    for x in ovalues(d.data)
        scope = data_scope(ex, x)
        cont = scope === nothing ? nothing : get(ex.containers, scope, nothing)
        cont === nothing && continue
        if x.kind == "data"
            obj = bsub!(cont, "dataObject", "id" => "$(x.id)-object", "name" => x.label,
                        "isCollection" => has(x, "collection") ? "true" : nothing)
            ref = bsub!(cont, "dataObjectReference", "id" => x.id, "name" => x.label,
                        "dataObjectRef" => getattr(obj, "id"))
            haskey(x.attrs, "state") && bsub!(ref, "dataState", "id" => "$(x.id)-state", "name" => x.attrs["state"])
        else
            name = nonempty(x.label) ? x.label : x.id
            haskey(ex.stores, name) || (ex.stores[name] = "$(x.id)-store")
            sid = ex.stores[name]
            bsub!(cont, "dataStoreReference", "id" => x.id, "name" => x.label, "dataStoreRef" => sid)
        end
    end
    n = 0
    for a in d.assocs
        act = find_id(ex, a.source)
        (act === nothing || !haskey(d.data, a.target)) && continue
        n += 1
        pos = has_child(act, "extensionElements") ? 1 : 0
        if a.kind == "reads"
            prop = XElem(BPMN_NS, "property")
            setattr!(prop, "id", "$(a.source)-in-$(n)")
            setattr!(prop, "name", "__targetRef_placeholder")
            assoc = XElem(BPMN_NS, "dataInputAssociation")
            setattr!(assoc, "id", "$(a.source)-dia-$(n)")
            bsub!(assoc, "sourceRef").text = a.target
            bsub!(assoc, "targetRef").text = getattr(prop, "id")
            insert_at!(act, pos, prop)
            insert_at!(act, after_props(act), assoc)
        else
            assoc = XElem(BPMN_NS, "dataOutputAssociation")
            setattr!(assoc, "id", "$(a.source)-doa-$(n)")
            bsub!(assoc, "targetRef").text = a.target
            insert_at!(act, after_props(act), assoc)
        end
    end
end

first_container(ex::Exporter) = isempty(ex.containers) ? nothing : ex.containers[ex.containers.keys[1]]

function export_artifacts!(ex::Exporter)
    d = ex.doc
    for note in d.notes
        target = get(note.attrs, "on", nothing)
        cont = nothing
        if target !== nothing && haskey(d.nodes, target)
            sc = d.nodes[target].scope
            cont = sc === nothing ? nothing : get(ex.containers, sc, nothing)
        end
        if cont === nothing
            cont = ex.collab !== nothing ? ex.collab : first_container(ex)
        end
        cont === nothing && continue
        t = bsub!(cont, "textAnnotation", "id" => note.id)
        bsub!(t, "text").text = something(note.label, "")
        if nonempty(target)
            bsub!(cont, "association", "id" => "$(note.id)-assoc", "sourceRef" => note.id, "targetRef" => target)
        end
    end
    if !isempty(d.groups)
        cat = XElem(BPMN_NS, "category")
        setattr!(cat, "id", "categories")
        for g in d.groups
            cv = bsub!(cat, "categoryValue", "id" => "$(g.id)-value", "value" => nonempty(g.label) ? g.label : g.id)
            cont = ex.collab !== nothing ? ex.collab : first_container(ex)
            bsub!(cont, "group", "id" => g.id, "categoryValueRef" => getattr(cv, "id"))
        end
        insert_at!(ex.root, 0, cat)
    end
end

# ------------------------------------------------------------------ writer

function escape_cdata(s::AbstractString)
    s = replace(s, "&" => "&amp;")
    s = replace(s, "<" => "&lt;")
    return replace(s, ">" => "&gt;")
end

function escape_attrib(s::AbstractString)
    s = escape_cdata(s)
    s = replace(s, "\"" => "&quot;")
    s = replace(s, "\r" => "&#13;")
    s = replace(s, "\n" => "&#10;")
    return replace(s, "\t" => "&#09;")
end

isblank(s) = s === nothing || isempty(pystrip(s))

function indent!(e::XElem, level::Int=0)
    pad = "\n" * "  "^level
    if !isempty(e.children)
        if isblank(e.text)
            e.text = pad * "  "
        end
        for c in e.children
            indent!(c, level + 1)
        end
        last = e.children[end]
        if isblank(last.tail)
            last.tail = pad
        end
    end
    if level > 0 && isblank(e.tail)
        e.tail = pad
    end
end

qname(ns, name) = ns == "" ? name : PREFIXES[ns] * ":" * name

function write_elem(io::IO, e::XElem, root::Bool)
    print(io, "<", qname(e.ns, e.name))
    if root
        used = Set{String}()
        iter_elems(e) do x
            x.ns == "" || push!(used, x.ns)
            for (ans, _, _) in x.attrs
                ans == "" || push!(used, ans)
            end
        end
        for ns in sort(collect(used); by=u -> PREFIXES[u])
            print(io, " xmlns:", PREFIXES[ns], "=\"", escape_attrib(ns), "\"")
        end
    end
    for (ans, an, v) in e.attrs
        print(io, " ", qname(ans, an), "=\"", escape_attrib(v), "\"")
    end
    if (e.text !== nothing && !isempty(e.text)) || !isempty(e.children)
        print(io, ">")
        (e.text !== nothing && !isempty(e.text)) && print(io, escape_cdata(e.text))
        for c in e.children
            write_elem(io, c, false)
        end
        print(io, "</", qname(e.ns, e.name), ">")
    else
        print(io, " />")
    end
    (e.tail !== nothing && !isempty(e.tail)) && print(io, escape_cdata(e.tail))
end

"""
    semantic_xml(doc; target=nothing) -> String

Semantic BPMN 2.0 XML of a document (attributes sorted, no layout), identical
to the output of the Python reference implementation.
"""
function semantic_xml(doc::Document; target=nothing)
    root = run!(Exporter(doc, target))
    iter_elems(root) do e
        sort!(e.attrs; by=t -> clark(t[1], t[2]))
    end
    indent!(root)
    io = IOBuffer()
    print(io, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
    write_elem(io, root, true)
    print(io, "\n")
    return String(take!(io))
end
