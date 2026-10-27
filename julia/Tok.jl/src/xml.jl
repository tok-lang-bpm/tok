# Semantic BPMN 2.0 export (no diagram interchange) and an XML writer that
# reproduces the output of Python's xml.etree.ElementTree byte for byte.



const BPMN_NS = "http://www.omg.org/spec/BPMN/20100524/MODEL"
const XSI_NS = "http://www.w3.org/2001/XMLSchema-instance"
const ZEEBE_NS = "http://camunda.org/schema/zeebe/1.0"
const MODELER_NS = "http://camunda.org/schema/modeler/1.0"
const XSD_NS = "http://www.w3.org/2001/XMLSchema"
const PREFIXES = Dict(BPMN_NS => "bpmn", XSI_NS => "xsi", ZEEBE_NS => "zeebe", MODELER_NS => "modeler",
                      XSD_NS => "xsd")

const TAGS = Dict(
    "start" => "startEvent", "end" => "endEvent", "catch" => "intermediateCatchEvent",
    "throw" => "intermediateThrowEvent", "boundary" => "boundaryEvent",
    "task" => "task", "user" => "userTask", "service" => "serviceTask", "manual" => "manualTask",
    "script" => "scriptTask", "rule" => "businessRuleTask", "send" => "sendTask", "receive" => "receiveTask",
    "call" => "callActivity", "sub" => "subProcess", "eventsub" => "subProcess", "adhoc" => "adHocSubProcess",
    "transaction" => "transaction",
    "xor" => "exclusiveGateway", "and" => "parallelGateway", "or" => "inclusiveGateway",
    "eventgw" => "eventBasedGateway", "complex" => "complexGateway",
)
const DEFINITION = Dict(
    "message" => "messageEventDefinition", "timer" => "timerEventDefinition", "signal" => "signalEventDefinition",
    "error" => "errorEventDefinition", "escalation" => "escalationEventDefinition",
    "condition" => "conditionalEventDefinition", "link" => "linkEventDefinition",
    "terminate" => "terminateEventDefinition", "compensate" => "compensateEventDefinition",
    "cancel" => "cancelEventDefinition",
)

# ------------------------------------------------------------------ tiny DOM

mutable struct XElem
    ns::String
    name::String
    attrs::Vector{Tuple{String,String,String}}   # (namespace, local name, value)
    children::Vector{XElem}
    text::Union{Nothing,String}
    tail::Union{Nothing,String}
    # namespaces declared on this element although no tag or attribute name uses them
    declare::Vector{String}
    # further namespaces with their prefixes, declared on the root (`structure="feel:string"
    # namespace="…"` of language 0.5)
    prefixed::Vector{Tuple{String,String}}
end

XElem(ns, name) = XElem(ns, name, Tuple{String,String,String}[], XElem[], nothing, nothing, String[],
                        Tuple{String,String}[])

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

const REST_CONNECTOR_TYPE = "io.camunda:http-json:1"
const BINDINGS = Dict("latest" => "latest", "deployment" => "deployment", "version-tag" => "versionTag")

"Pairs `left SEP right` separated by `;` (inputs, outputs, headers)."
function split_pairs(text, sep::AbstractString)
    out = Tuple{String,String}[]
    text === nothing && return out
    for item in split(text, ";")
        item = pystrip(item)
        if !isempty(item) && occursin(sep, item)
            left, right = split(item, sep; limit=2)
            push!(out, (pystrip(left), pystrip(right)))
        end
    end
    return out
end

"Listener entries `event: type (retries)` separated by `;`."
function parse_listeners(text)
    out = Tuple{String,String,MaybeStr}[]
    text === nothing && return out
    for item in split(text, ";")
        m = match(r"^([a-z]+)\s*:\s*(.+?)(?:\s*\((\d+)\))?$", pystrip(item))
        if m !== nothing
            push!(out, (String(m.captures[1]), pystrip(m.captures[2]),
                        m.captures[3] === nothing ? nothing : String(m.captures[3])))
        end
    end
    return out
end

function timer_kind(v::AbstractString)
    v = pystrip(v)
    startswith(v, "=") && return "timeDuration"
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
    # data inputs and outputs of activities made for `reads` and `writes` (language 0.5):
    # activity -> (input or output, identifier, name)
    implicit_io::ODict{Vector{Tuple{String,String,String}}}
end

function Exporter(doc::Document, target=nothing)
    t = target !== nothing ? target : get(doc.attrs, "target", "camunda")
    camunda = executable_any(doc) && t == "camunda"
    decls = Dict(k => ODict{Decl}() for k in ("message", "signal", "error", "escalation"))
    Exporter(doc, camunda, decls, ODict{String}(), ODict{XElem}(), XElem(BPMN_NS, "definitions"), nothing,
             ODict{Vector{Tuple{String,String,String}}}())
end

bsub!(parent, name, attrs::Pair...) = subelem!(parent, BPMN_NS, name, attrs...)

function ext!(e::XElem)
    for c in e.children
        c.ns == BPMN_NS && c.name == "extensionElements" && return c
    end
    x = XElem(BPMN_NS, "extensionElements")
    insert_at!(e, docs_len(e), x)
    return x
end

"Number of leading `documentation` children (they come first in every BPMN element)."
function docs_len(e::XElem)
    k = 0
    for c in e.children
        (c.ns == BPMN_NS && c.name == "documentation") || break
        k += 1
    end
    return k
end

"`documentation` of the element from `doc=` (language 0.5), as its first child."
function documentation!(e::XElem, text)
    nonempty(text) || return
    d = XElem(BPMN_NS, "documentation")
    d.text = text
    insert_at!(e, 0, d)
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
            documentation!(part, get(p.attrs, "doc", nothing))
            m = multiplicity(d, p)
            m === nothing || push!(part.children, m)
            if !p.blackbox
                setattr!(part, "processRef", process_id(p))
                pname = if haskey(p.attrs, "process-name")
                    nonempty(p.attrs["process-name"]) ? p.attrs["process-name"] : nothing
                elseif length(inner) == 1 && nonempty(d.title)
                    d.title
                else
                    p.label
                end
                push!(processes, (process_id(p), pname, has(p, "executable") || is_executable(d), p))
            end
        end
        for m in d.messages
            mf = bsub!(collab, "messageFlow", "id" => m.id, "name" => m.label, "sourceRef" => m.source,
                       "targetRef" => m.target)
            name = nonempty(get(m.attrs, "message", nothing)) ? m.attrs["message"] : m.label
            nonempty(name) && setattr!(mf, "messageRef", declare!(ex, "message", name).id)
            documentation!(mf, get(m.attrs, "doc", nothing))
        end
        ex.collab = collab
    else
        ex.collab = nothing
        name = if haskey(d.attrs, "process-name")
            nonempty(d.attrs["process-name"]) ? d.attrs["process-name"] : nothing
        else
            d.title
        end
        push!(processes, (default_process_id(d), name, is_executable(d), nothing))
    end

    for (pid, name, executable, pool) in processes
        proc = bsub!(root, "process", "id" => pid, "name" => name, "isExecutable" => executable ? "true" : "false")
        ex.containers[pid] = proc
        # the header documents the process of a document without pools
        pool === nothing && documentation!(proc, get(d.attrs, "doc", nothing))
        tag = pool !== nothing ? get(pool.attrs, "version-tag", nothing) : nothing
        if !nonempty(tag)
            tag = (pool === nothing || length(processes) == 1) ? get(d.attrs, "version-tag", nothing) : nothing
        end
        if ex.camunda && executable && nonempty(tag)
            subelem!(ext!(proc), ZEEBE_NS, "versionTag", "value" => tag)
        end
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

    export_choreographies!(ex)
    export_flows!(ex)
    export_data!(ex)
    export_artifacts!(ex)
    compensation_links!(ex)
    order_adhoc!(ex)
    process_data!(ex)
    performers!(ex)
    supported_interfaces!(ex)
    first_decls, last_decls = declarations_05!(ex)
    export_conversations!(ex)

    decl = first_decls
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
    append!(decl, last_decls)
    for (k, e) in enumerate(decl)
        insert_at!(root, decl_slot + k - 1, e)
    end
    # the header documents the definitions of a document with pools
    isempty(d.pools) || documentation!(root, get(d.attrs, "doc", nothing))
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
    documentation!(e, get(lane.attrs, "doc", nothing))
    sublanes = [d.lanes[c] for c in lane.children if haskey(d.lanes, c)]
    for c in lane.children
        if haskey(d.nodes, c)
            r = bsub!(e, "flowNodeRef")
            r.text = c
            if haskey(d.nodes[c].attrs, "compensate") && compensation_boundary(d, c) === nothing
                bsub!(e, "flowNodeRef").text = compensation_id(c)
            end
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
    documentation!(e, get(a, "doc", nothing))
    if n.kind == "boundary"
        setattr!(e, "attachedToRef", get(a, "on", ""))
        has(n, "noninterrupting") && setattr!(e, "cancelActivity", "false")
    end
    n.kind == "start" && has(n, "noninterrupting") && setattr!(e, "isInterrupting", "false")
    n.kind == "eventsub" && setattr!(e, "triggeredByEvent", "true")
    if n.kind == "call" && haskey(a, "process") && !startswith(a["process"], "=")
        setattr!(e, "calledElement", a["process"])
    end
    n.kind == "call" && haskey(a, "task") && setattr!(e, "calledElement", a["task"])
    if n.kind == "transaction" && haskey(TRANSACTION_METHODS, get(a, "method", ""))
        setattr!(e, "method", TRANSACTION_METHODS[a["method"]])
    end
    n.kind in ("receive", "eventgw") && has(n, "instantiate") && setattr!(e, "instantiate", "true")
    n.kind == "eventgw" && has(n, "parallel") && setattr!(e, "eventGatewayType", "Parallel")
    n.kind in ("start", "catch", "boundary") && has(n, "parallel") && setattr!(e, "parallelMultiple", "true")
    if n.kind in ("service", "send", "receive") && haskey(a, "operation")
        setattr!(e, "operationRef", a["operation"])
    end
    if n.kind == "adhoc"
        has(n, "sequential") && setattr!(e, "ordering", "Sequential")
        ex.camunda && has(n, "wait") && setattr!(e, "cancelRemainingInstances", "false")
    end
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
    if n.kind == "complex" && haskey(a, "activation")
        push!(e.children, formal(ex, "activationCondition", a["activation"]))
    end
    if has(n, "loop")
        lc = bsub!(e, "standardLoopCharacteristics")
        has(n, "test-before") && setattr!(lc, "testBefore", "true")
        haskey(a, "loop-max") && setattr!(lc, "loopMaximum", a["loop-max"])
        haskey(a, "while") && push!(lc.children, formal(ex, "loopCondition", a["while"]))
    elseif is_activity(n.kind) && n.kind != "adhoc" && (has(n, "parallel") || has(n, "sequential"))
        mi = bsub!(e, "multiInstanceLoopCharacteristics", "isSequential" => has(n, "sequential") ? "true" : "false")
        if ex.camunda && haskey(a, "collection")
            lc = Pair{String,String}["inputCollection" => feel(ex, a["collection"])]
            haskey(a, "item") && push!(lc, "inputElement" => a["item"])
            haskey(a, "output") && push!(lc, "outputCollection" => a["output"])
            haskey(a, "output-item") && push!(lc, "outputElement" => feel(ex, a["output-item"]))
            subelem!(ext!(mi), ZEEBE_NS, "loopCharacteristics", lc...)
        end
        if ex.camunda && haskey(a, "until")
            c = bsub!(mi, "completionCondition")
            setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            c.text = feel(ex, a["until"])
        end
    end
    if n.kind in SUB_KINDS
        ex.containers[n.id] = e
        export_block!(ex, e, n.children)
        if n.kind == "adhoc" && ex.camunda && haskey(a, "until")
            c = bsub!(e, "completionCondition")
            setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            c.text = feel(ex, a["until"])
        end
    end
    if haskey(a, "compensate") && compensation_boundary(ex.doc, n.id) === nothing
        cid = compensation_id(n.id)
        b = bsub!(container, "boundaryEvent", "id" => cid, "attachedToRef" => n.id)
        bsub!(b, "compensateEventDefinition", "id" => "$(cid)-def")
    end
end

compensation_id(activity_id) = "$(activity_id)-compensate"

"""
A boundary event with the flag `compensate` attached to the activity: it is associated with
the activity's handler (`compensate=`) instead of a generated one.
"""
function compensation_boundary(d::Document, activity_id)
    for b in ovalues(d.nodes)
        if b.kind == "boundary" && get(b.attrs, "on", nothing) == activity_id && has(b, "compensate")
            return b.id
        end
    end
    return nothing
end

"A child `NAME` with xsi:type tFormalExpression and the expression as text."
function formal(ex::Exporter, name::String, expr::AbstractString)
    c = XElem(BPMN_NS, name)
    setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
    c.text = feel(ex, expr)
    return c
end

function binding!(attrs::Vector{Pair{String,String}}, a::Dict{String,String})
    haskey(a, "binding") && push!(attrs, "bindingType" => get(BINDINGS, a["binding"], a["binding"]))
    haskey(a, "version") && push!(attrs, "versionTag" => a["version"])
    return attrs
end

function node_executable(ex::Exporter, n::Elem)
    d = ex.doc
    is_executable(d) && return true
    return any(p -> p.id == n.pool && has(p, "executable"), d.pools)
end

function io_mapping!(e::XElem, inputs, outputs)
    x = ext!(e)
    box = nothing
    for c in x.children
        if c.ns == ZEEBE_NS && c.name == "ioMapping"
            box = c
            break
        end
    end
    box === nothing && (box = subelem!(x, ZEEBE_NS, "ioMapping"))
    for (target, source) in inputs
        subelem!(box, ZEEBE_NS, "input", "source" => source, "target" => target)
    end
    for (target, source) in outputs
        subelem!(box, ZEEBE_NS, "output", "source" => source, "target" => target)
    end
end

function export_zeebe!(ex::Exporter, e::XElem, n::Elem)
    node_executable(ex, n) || return export_zeebe_01!(ex, e, n)
    a = n.attrs
    if n.kind == "user"
        x = ext!(e)
        subelem!(x, ZEEBE_NS, "userTask")
        if haskey(a, "form")
            subelem!(x, ZEEBE_NS, "formDefinition", binding!(Pair{String,String}["formId" => a["form"]], a)...)
        end
        assign = Pair{String,String}[]
        for (key, attr) in (("assignee", "assignee"), ("candidate-users", "candidateUsers"),
                            ("candidate-groups", "candidateGroups"))
            haskey(a, key) && push!(assign, attr => a[key])
        end
        isempty(assign) || subelem!(x, ZEEBE_NS, "assignmentDefinition", assign...)
        sched = Pair{String,String}[]
        for (key, attr) in (("due", "dueDate"), ("follow-up", "followUpDate"))
            haskey(a, key) && push!(sched, attr => a[key])
        end
        isempty(sched) || subelem!(x, ZEEBE_NS, "taskSchedule", sched...)
        haskey(a, "priority") && subelem!(x, ZEEBE_NS, "priorityDefinition", "priority" => a["priority"])
    elseif n.kind == "start" && haskey(a, "form")
        subelem!(ext!(e), ZEEBE_NS, "formDefinition", binding!(Pair{String,String}["formId" => a["form"]], a)...)
    elseif n.kind == "rule" && haskey(a, "decision")
        res = nonempty(get(a, "result", nothing)) ? a["result"] : default_result(ex, a["decision"])
        subelem!(ext!(e), ZEEBE_NS, "calledDecision",
                 binding!(Pair{String,String}["decisionId" => a["decision"], "resultVariable" => res], a)...)
    elseif n.kind == "script" && haskey(a, "expr")
        subelem!(ext!(e), ZEEBE_NS, "script", "expression" => feel(ex, a["expr"]),
                 "resultVariable" => get(a, "result", "result"))
    elseif n.kind == "call" && haskey(a, "process")
        ce = binding!(Pair{String,String}["processId" => a["process"]], a)
        get(a, "child-vars", nothing) == "none" && push!(ce, "propagateAllChildVariables" => "false")
        get(a, "parent-vars", nothing) == "none" && push!(ce, "propagateAllParentVariables" => "false")
        haskey(a, "business-id") && push!(ce, "businessId" => a["business-id"])
        subelem!(ext!(e), ZEEBE_NS, "calledElement", ce...)
    elseif n.kind == "adhoc" && haskey(a, "active")
        subelem!(ext!(e), ZEEBE_NS, "adHoc", "activeElementsCollection" => feel(ex, a["active"]))
    end
    job_kinds = (haskey(a, "message") || has(n, "message")) ? ("service", "send", "throw", "end") : ("service", "send")
    inputs = split_pairs(get(a, "inputs", nothing), "=")
    headers = split_pairs(get(a, "headers", nothing), ":")
    if n.kind == "service" && get(a, "connector", nothing) == "rest"
        td = Pair{String,String}["type" => REST_CONNECTOR_TYPE]
        haskey(a, "retries") && push!(td, "retries" => a["retries"])
        subelem!(ext!(e), ZEEBE_NS, "taskDefinition", td...)
        conn = Tuple{String,String}[("authentication.type", "noAuth"), ("method", get(a, "method", "GET")),
                                    ("url", get(a, "url", ""))]
        for (key, target) in (("headers-in", "headers"), ("query", "queryParameters"), ("body", "body"))
            haskey(a, key) && push!(conn, (target, a[key]))
        end
        io_mapping!(e, conn, Tuple{String,String}[])
        if haskey(a, "result")
            headers = vcat(Tuple{String,String}[("resultVariable", a["result"])], headers)
        end
    elseif n.kind in job_kinds && haskey(a, "type")
        td = Pair{String,String}["type" => a["type"]]
        haskey(a, "retries") && push!(td, "retries" => a["retries"])
        subelem!(ext!(e), ZEEBE_NS, "taskDefinition", td...)
    end
    if !isempty(inputs) || haskey(a, "outputs")
        io_mapping!(e, [(t, feel(ex, src)) for (t, src) in inputs],
                    [(t, feel(ex, src)) for (t, src) in split_pairs(get(a, "outputs", nothing), "=")])
    end
    if !isempty(headers)
        th = subelem!(ext!(e), ZEEBE_NS, "taskHeaders")
        for (k, v) in headers
            subelem!(th, ZEEBE_NS, "header", "key" => k, "value" => v)
        end
    end
    if haskey(a, "events") && n.kind in ("start", "catch", "boundary")
        subelem!(ext!(e), ZEEBE_NS, "conditionalFilter", "variableEvents" => a["events"])
    end
    if haskey(a, "listeners")
        execl = Vector{Pair{String,String}}[]
        taskl = Vector{Pair{String,String}}[]
        for (ev, typ, retries) in parse_listeners(a["listeners"])
            item = Pair{String,String}["eventType" => ev, "type" => typ]
            retries === nothing || push!(item, "retries" => retries)
            push!(ev in ("start", "end") ? execl : taskl, item)
        end
        if !isempty(execl)
            box = subelem!(ext!(e), ZEEBE_NS, "executionListeners")
            for item in execl
                subelem!(box, ZEEBE_NS, "executionListener", item...)
            end
        end
        if !isempty(taskl)
            box = subelem!(ext!(e), ZEEBE_NS, "taskListeners")
            for item in taskl
                subelem!(box, ZEEBE_NS, "taskListener", item...)
            end
        end
    end
end

"The executable profile of language 0.1 (used where the process itself is not executable)."
function export_zeebe_01!(ex::Exporter, e::XElem, n::Elem)
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

"""
Event definitions of an event: every trigger in a document of language 0.5 (ids
ID-def, ID-def-2, ...), only the first one in documents of language 0.1 and 0.3.
"""
function export_event_definition!(ex::Exporter, e::XElem, n::Elem)
    a = n.attrs
    multiple = lang_05(ex.doc)
    count = 0
    function next_id()
        count += 1
        return count == 1 ? "$(n.id)-def" : "$(n.id)-def-$(count)"
    end
    for key in TRIGGERS, v in values_of(n, key)
        count > 0 && !multiple && return
        df = bsub!(e, DEFINITION[key], "id" => next_id())
        if key == "message"
            info = declare!(ex, "message", v)
            haskey(a, "correlation") && (info.correlation = a["correlation"])
            setattr!(df, "messageRef", info.id)
        elseif key == "signal"
            setattr!(df, "signalRef", declare!(ex, "signal", v).id)
        elseif key == "error"
            setattr!(df, "errorRef", declare!(ex, "error", v, get(a, "code", nothing)).id)
        elseif key == "escalation"
            setattr!(df, "escalationRef", declare!(ex, "escalation", v, get(a, "code", nothing)).id)
        elseif key == "timer"
            t = bsub!(df, timer_kind(v))
            setattr!(t, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            t.text = v
        elseif key == "condition"
            c = bsub!(df, "condition")
            setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            c.text = feel(ex, v)
        elseif key == "link"
            setattr!(df, "name", v)
        end
    end
    # triggers without a value (language 0.5): an event definition without a reference
    for flag in NAMELESS_TRIGGERS
        count > 0 && !multiple && return
        if has(n, flag)
            df = bsub!(e, DEFINITION[flag], "id" => next_id())
            if flag == "condition"
                # a conditional event definition always has a condition
                c = bsub!(df, "condition")
                setattr!(c, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
            end
        end
    end
    for flag in (TRIGGER_FLAGS..., CATCH_ALL_FLAGS...)
        count > 0 && !multiple && return
        if has(n, flag)
            df = bsub!(e, DEFINITION[flag], "id" => next_id())
            flag == "compensate" && haskey(a, "activity") && setattr!(df, "activityRef", a["activity"])
        end
    end
end

"Associations from generated compensation boundary events to their handlers."
function compensation_links!(ex::Exporter)
    d = ex.doc
    for n in ovalues(d.nodes)
        h = get(n.attrs, "compensate", nothing)
        (h === nothing || !haskey(d.nodes, h)) && continue
        cont = n.scope === nothing ? nothing : get(ex.containers, n.scope, nothing)
        cont === nothing && continue
        explicit = compensation_boundary(d, n.id)
        cid = explicit === nothing ? compensation_id(n.id) : explicit
        bsub!(cont, "association", "id" => "$(cid)-assoc", "associationDirection" => "One",
              "sourceRef" => cid, "targetRef" => h)
    end
end

"completionCondition is the last child of an ad-hoc sub-process (BPMN schema order)."
function order_adhoc!(ex::Exporter)
    iter_elems(ex.root) do e
        if e.ns == BPMN_NS && e.name == "adHocSubProcess"
            conds = [c for c in e.children if c.ns == BPMN_NS && c.name == "completionCondition"]
            filter!(c -> !(c.ns == BPMN_NS && c.name == "completionCondition"), e.children)
            append!(e.children, conds)
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
        documentation!(e, get(f.attrs, "doc", nothing))
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
    pos = docs_len(act)
    for (k, c) in enumerate(act.children)
        if c.ns == BPMN_NS && c.name in ("extensionElements", "property", "dataInputAssociation", "dataOutputAssociation")
            pos = k
        end
    end
    return pos
end

function export_data!(ex::Exporter)
    d = ex.doc
    shared = Dict{Tuple{String,String},String}()
    for x in ovalues(d.data)
        x.kind in IO_KINDS && continue
        scope = data_scope(ex, x)
        cont = scope === nothing ? nothing : get(ex.containers, scope, nothing)
        cont === nothing && continue
        if x.kind == "data"
            # `object=` (language 0.5) names a data object shared by the references with the
            # same object name in one process or sub-process
            object = get(x.attrs, "object", nothing)
            key = (scope, something(object, ""))
            if object !== nothing && haskey(shared, key)
                oid = shared[key]
            else
                oid = "$(x.id)-object"
                object === nothing || (shared[key] = oid)
                oname = object === nothing ? x.label : (nonempty(object) ? object : nothing)
                bsub!(cont, "dataObject", "id" => oid, "name" => oname,
                      "itemSubjectRef" => get(x.attrs, "type", nothing),
                      "isCollection" => has(x, "collection") ? "true" : nothing)
            end
            ref = bsub!(cont, "dataObjectReference", "id" => x.id, "name" => x.label, "dataObjectRef" => oid)
            documentation!(ref, get(x.attrs, "doc", nothing))
            haskey(x.attrs, "state") && bsub!(ref, "dataState", "id" => "$(x.id)-state", "name" => x.attrs["state"])
        else
            name = nonempty(x.label) ? x.label : x.id
            haskey(ex.stores, name) || (ex.stores[name] = "$(x.id)-store")
            sid = ex.stores[name]
            ref = bsub!(cont, "dataStoreReference", "id" => x.id, "name" => x.label, "dataStoreRef" => sid,
                        "itemSubjectRef" => get(x.attrs, "type", nothing))
            documentation!(ref, get(x.attrs, "doc", nothing))
        end
    end
    n = 0
    for a in d.assocs
        act = find_id(ex, a.source)
        (act === nothing || !haskey(d.data, a.target)) && continue
        n += 1
        pos = docs_len(act) + (has_child(act, "extensionElements") ? 1 : 0)
        if lang_05(d)
            # the association connects the data element with a data input or output of
            # the activity's ioSpecification (BPMN 2.0.2, 10.4.1)
            dx = d.data[a.target]
            name = dx.label !== nothing ? dx.label : a.target
            reads = a.kind == "reads"
            # `input=` / `output=` name a declared data input or output of the activity
            declared = get(a.attrs, reads ? "input" : "output", nothing)
            io_id = declared !== nothing ? declared : "$(a.source)-$(reads ? "in" : "out")-$(n)"
            assoc = XElem(BPMN_NS, reads ? "dataInputAssociation" : "dataOutputAssociation")
            setattr!(assoc, "id", "$(a.source)-$(reads ? "dia" : "doa")-$(n)")
            bsub!(assoc, "sourceRef").text = reads ? a.target : io_id
            bsub!(assoc, "targetRef").text = reads ? io_id : a.target
            insert_ranked!(act, assoc)
            if declared === nothing
                haskey(ex.implicit_io, a.source) || (ex.implicit_io[a.source] = Tuple{String,String,String}[])
                push!(ex.implicit_io[a.source], (reads ? "input" : "output", io_id, name))
            end
            continue
        end
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

"""
Position of a child in the BPMN schema order of activities, events and processes
(tBaseElement, tCallableElement, tProcess, tActivity, tEvent); elements of the
subtypes (event definitions, inner flow elements, scripts) come last.
"""
function schema_rank(name::AbstractString)
    name == "documentation" && return 0
    name == "extensionElements" && return 1
    name == "supportedInterfaceRef" && return 2
    name == "ioSpecification" && return 3
    name == "ioBinding" && return 4
    name == "property" && return 5
    # data inputs and outputs of events (activities have them in the ioSpecification)
    name in ("dataInput", "dataOutput") && return 6
    name == "dataInputAssociation" && return 7
    name == "dataOutputAssociation" && return 8
    name in ("inputSet", "outputSet") && return 9
    name == "laneSet" && return 10
    name in ("performer", "humanPerformer", "potentialOwner", "resourceRole") && return 11
    name in ("standardLoopCharacteristics", "multiInstanceLoopCharacteristics") && return 12
    return 13
end

"Insert `child` before the first child that comes later in the schema order."
function insert_ranked!(parent::XElem, child::XElem)
    r = schema_rank(child.name)
    pos = findfirst(c -> c.ns == BPMN_NS && schema_rank(c.name) > r, parent.children)
    pos === nothing ? push!(parent.children, child) : insert!(parent.children, pos, child)
    return child
end

function text_elem(name::String, text::AbstractString)
    e = XElem(BPMN_NS, name)
    e.text = String(text)
    return e
end

"A dataInput, dataOutput or property (item-aware elements of language 0.5)."
function item_aware(name::String, x::Elem)
    e = XElem(BPMN_NS, name)
    setattr!(e, "id", x.id)
    x.label === nothing || setattr!(e, "name", x.label)
    haskey(x.attrs, "type") && setattr!(e, "itemSubjectRef", x.attrs["type"])
    name != "property" && has(x, "collection") && setattr!(e, "isCollection", "true")
    documentation!(e, get(x.attrs, "doc", nothing))
    return e
end

"Identifier of the correlationProperty exported for a `prop` name."
correlation_property_id(name) = "CorrelationProperty_" * slug(name)

"The element (process, sub-process or activity) that owns an input, output or property."
function io_owner(ex::Exporter, x::Elem)
    d = ex.doc
    haskey(x.attrs, "on") && return x.attrs["on"]
    nonempty(x.scope) && return x.scope
    for p in d.pools
        p.blackbox || return process_id(p)
    end
    return default_process_id(d)
end

"ioSpecification with the data inputs and outputs, and property elements."
function process_data!(ex::Exporter)
    d = ex.doc
    owners = ODict{Vector{Elem}}()
    for x in ovalues(d.data)
        x.kind in IO_KINDS || continue
        o = io_owner(ex, x)
        haskey(owners, o) || (owners[o] = Elem[])
        push!(owners[o], x)
    end
    for act in ex.implicit_io.keys
        haskey(owners, act) || (owners[act] = Elem[])
    end
    for owner in owners.keys
        xs = owners[owner]
        implicit = get(ex.implicit_io, owner, Tuple{String,String,String}[])
        el = find_id(ex, owner)
        el === nothing && continue
        if endswith(el.name, "Event")
            # an event has its data inputs (throwing) or outputs (catching) and their set
            # directly, without an ioSpecification
            throwing = occursin("Throw", el.name) || el.name == "endEvent"
            kind, tag, set, refs = throwing ? ("input", "dataInput", "inputSet", "dataInputRefs") :
                                              ("output", "dataOutput", "outputSet", "dataOutputRefs")
            ids = String[]
            for x in xs
                x.kind == kind || continue
                push!(ids, x.id)
                insert_ranked!(el, item_aware(tag, x))
            end
            for (k, id, name) in implicit
                k == kind || continue
                push!(ids, id)
                insert_ranked!(el, setattr!(setattr!(XElem(BPMN_NS, tag), "id", id), "name", name))
            end
            if !isempty(ids)
                st = XElem(BPMN_NS, set)
                setattr!(st, "id", "$(owner)-$(throwing ? "inputs" : "outputs")")
                for i in ids
                    push!(st.children, text_elem(refs, i))
                end
                insert_ranked!(el, st)
            end
            for x in xs
                x.kind == "property" && insert_ranked!(el, item_aware("property", x))
            end
            continue
        end
        if any(x -> x.kind in ("input", "output"), xs) || !isempty(implicit)
            io = XElem(BPMN_NS, "ioSpecification")
            setattr!(io, "id", "$(owner)-io")
            ids = Dict("input" => String[], "output" => String[])
            for (kind, tag) in (("input", "dataInput"), ("output", "dataOutput"))
                for x in xs
                    x.kind == kind || continue
                    push!(io.children, item_aware(tag, x))
                    push!(ids[kind], x.id)
                end
                for (k, id, name) in implicit
                    k == kind || continue
                    push!(io.children, setattr!(setattr!(XElem(BPMN_NS, tag), "id", id), "name", name))
                    push!(ids[kind], id)
                end
            end
            ins = bsub!(io, "inputSet", "id" => "$(owner)-inputs")
            for i in ids["input"]
                push!(ins.children, text_elem("dataInputRefs", i))
            end
            outs = bsub!(io, "outputSet", "id" => "$(owner)-outputs")
            for i in ids["output"]
                push!(outs.children, text_elem("dataOutputRefs", i))
            end
            insert_ranked!(el, io)
        end
        for x in xs
            x.kind == "property" && insert_ranked!(el, item_aware("property", x))
        end
    end
end

"performer (humanPerformer on user and manual tasks) of activities with `by=` (language 0.5)."
function performers!(ex::Exporter)
    d = ex.doc
    lang_05(d) || return
    for n in ovalues(d.nodes)
        r = get(n.attrs, "by", nothing)
        (r === nothing || !haskey(d.resources, r) || !is_activity(n.kind)) && continue
        p = XElem(BPMN_NS, n.kind in ("user", "manual") ? "humanPerformer" : "performer")
        setattr!(p, "id", "$(n.id)-performer")
        push!(p.children, text_elem("resourceRef", r))
        el = find_id(ex, n.id)
        el === nothing || insert_ranked!(el, p)
    end
end

"The process (not sub-process) that contains a node."
function process_of(ex::Exporter, n::Elem)
    scope = n.scope
    while scope !== nothing && haskey(ex.doc.nodes, scope)
        scope = ex.doc.nodes[scope].scope
    end
    return scope
end

"supportedInterfaceRef of processes whose tasks use operations."
function supported_interfaces!(ex::Exporter)
    d = ex.doc
    refs = ODict{Vector{String}}()
    for n in ovalues(d.nodes)
        op = get(n.attrs, "operation", nothing)
        op === nothing && continue
        ifcs = [x for x in ovalues(d.interfaces) if any(o -> o.id == op, x.items)]
        isempty(ifcs) && continue
        pid = process_of(ex, n)
        pid === nothing && continue
        haskey(refs, pid) || (refs[pid] = String[])
        ifcs[1].id in refs[pid] || push!(refs[pid], ifcs[1].id)
    end
    for pid in refs.keys
        el = find_id(ex, pid)
        el === nothing && continue
        for i in refs[pid]
            insert_ranked!(el, text_elem("supportedInterfaceRef", i))
        end
    end
end

"""
Root elements of language 0.5: item definitions (placed before the other
declarations), and resources, correlation properties, interfaces and global tasks
(placed after them). Correlation keys go into the collaboration.
"""
function declarations_05!(ex::Exporter)
    d = ex.doc
    first = XElem[]
    last = XElem[]
    for it in ovalues(d.items)
        e = XElem(BPMN_NS, "itemDefinition")
        setattr!(e, "id", it.id)
        nonempty(it.label) && push!(e.children, text_elem("documentation", it.label))
        haskey(it.attrs, "structure") && setattr!(e, "structureRef", it.attrs["structure"])
        if haskey(it.attrs, "kind")
            setattr!(e, "itemKind", it.attrs["kind"] == "physical" ? "Physical" : "Information")
        end
        has(it, "collection") && setattr!(e, "isCollection", "true")
        if startswith(get(it.attrs, "structure", ""), "xsd:") && !(XSD_NS in ex.root.declare)
            push!(ex.root.declare, XSD_NS)
        end
        structure = get(it.attrs, "structure", "")
        if occursin(":", structure) && haskey(it.attrs, "namespace")
            prefix = String(split(structure, ":"; limit=2)[1])
            any(p -> p[1] == prefix, ex.root.prefixed) || push!(ex.root.prefixed, (prefix, it.attrs["namespace"]))
        end
        push!(first, e)
    end
    if lang_05(d)
        for r in ovalues(d.resources)
            e = XElem(BPMN_NS, "resource")
            setattr!(e, "id", r.id)
            setattr!(e, "name", nonempty(r.label) ? r.label : r.id)
            push!(last, e)
        end
    end
    props = ODict{XElem}()
    keys = XElem[]
    for k in ovalues(d.keys)
        ke = XElem(BPMN_NS, "correlationKey")
        setattr!(ke, "id", k.id)
        k.label === nothing || setattr!(ke, "name", k.label)
        seen = String[]
        for p in k.items
            name = p.id
            pid = correlation_property_id(name)
            if !haskey(props, name)
                e = XElem(BPMN_NS, "correlationProperty")
                setattr!(e, "id", pid)
                setattr!(e, "name", name)
                props[name] = e
            end
            if haskey(p.attrs, "message")
                rx = XElem(BPMN_NS, "correlationPropertyRetrievalExpression")
                setattr!(rx, "messageRef", declare!(ex, "message", p.attrs["message"]).id)
                mp = text_elem("messagePath", get(p.attrs, "path", ""))
                setattr!(mp, "type", "bpmn:tFormalExpression"; ns=XSI_NS)
                push!(rx.children, mp)
                push!(props[name].children, rx)
            end
            if !(pid in seen)
                push!(ke.children, text_elem("correlationPropertyRef", pid))
                push!(seen, pid)
            end
        end
        push!(keys, ke)
    end
    append!(last, ovalues(props))
    if !isempty(keys) && !isempty(d.pools)
        c = find_id(ex, get(d.attrs, "collaboration", "collaboration"))
        c === nothing || append!(c.children, keys)
    end
    for x in ovalues(d.interfaces)
        e = XElem(BPMN_NS, "interface")
        setattr!(e, "id", x.id)
        setattr!(e, "name", nonempty(x.label) ? x.label : x.id)
        documentation!(e, get(x.attrs, "doc", nothing))
        for op in x.items
            o = bsub!(e, "operation", "id" => op.id, "name" => nonempty(op.label) ? op.label : op.id)
            documentation!(o, get(op.attrs, "doc", nothing))
            if haskey(op.attrs, "in")
                push!(o.children, text_elem("inMessageRef", declare!(ex, "message", op.attrs["in"]).id))
            end
            if haskey(op.attrs, "out")
                push!(o.children, text_elem("outMessageRef", declare!(ex, "message", op.attrs["out"]).id))
            end
            for err in values_of(op, "error")
                push!(o.children, text_elem("errorRef", declare!(ex, "error", err).id))
            end
        end
        push!(last, e)
    end
    for g in ovalues(d.globals)
        e = XElem(BPMN_NS, get(GLOBAL_KINDS, get(g.attrs, "kind", ""), "globalTask"))
        setattr!(e, "id", g.id)
        g.label === nothing || setattr!(e, "name", g.label)
        documentation!(e, get(g.attrs, "doc", nothing))
        push!(last, e)
    end
    return first, last
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
        t = XElem(BPMN_NS, "textAnnotation")
        setattr!(t, "id", note.id)
        documentation!(t, get(note.attrs, "doc", nothing))
        bsub!(t, "text").text = something(note.label, "")
        # the artifacts of a choreography precede its flow elements (`tCollaboration`)
        at = length(cont.children)
        if cont.name == "choreography"
            k = findfirst(x -> !(x.name in ("documentation", "participant", "messageFlow", "textAnnotation",
                                            "association")), cont.children)
            k === nothing || (at = k - 1)
        end
        insert_at!(cont, at, t)
        if nonempty(target)
            a = XElem(BPMN_NS, "association")
            setattr!(a, "id", "$(note.id)-assoc")
            setattr!(a, "sourceRef", note.id)
            setattr!(a, "targetRef", target)
            insert_at!(cont, at + 1, a)
        end
    end
    if !isempty(d.groups)
        cat = XElem(BPMN_NS, "category")
        setattr!(cat, "id", "categories")
        for g in d.groups
            cv = bsub!(cat, "categoryValue", "id" => "$(g.id)-value", "value" => nonempty(g.label) ? g.label : g.id)
            cont = ex.collab !== nothing ? ex.collab : first_container(ex)
            ge = bsub!(cont, "group", "id" => g.id, "categoryValueRef" => getattr(cv, "id"))
            documentation!(ge, get(g.attrs, "doc", nothing))
        end
        insert_at!(ex.root, 0, cat)
    end
end

# ------------------------------------------------------------------ language 0.6

choreography_participant_id(choreography, pool) = "$(choreography)-$(pool)"
conversation_link_id(node, pool) = "$(node)-link-$(pool)"

"`participantMultiplicity` of a pool with `min=` or `max=` (minimum 2 when only the maximum is given)."
function multiplicity(d::Document, p::Elem)
    lang_06(d) || return nothing
    mn, mx = get(p.attrs, "min", nothing), get(p.attrs, "max", nothing)
    (mn === nothing && mx === nothing) && return nothing
    m = XElem(BPMN_NS, "participantMultiplicity")
    setattr!(m, "minimum", mn === nothing ? "2" : mn)
    mx === nothing || setattr!(m, "maximum", mx)
    return m
end

"`map=\"inner:outer; …\"` as pairs; `nothing` when an item is malformed."
function participant_map(text::AbstractString)
    out = Tuple{String,String}[]
    for item in split(text, ";")
        item = strip(item)
        isempty(item) && continue
        parts = split(item, ":")
        length(parts) == 2 || return nothing
        a, b = strip(parts[1]), strip(parts[2])
        (isempty(a) || isempty(b)) && return nothing
        push!(out, (String(a), String(b)))
    end
    return out
end

"Pools that take part in the activities of a choreography, in document order."
function choreography_participants(d::Document, cid)
    used = String[]
    for n in ovalues(d.nodes)
        (n.choreography == cid && n.kind in CHOREO_KINDS) || continue
        for k in ("from", "to")
            haskey(n.attrs, k) && push!(used, n.attrs[k])
        end
        append!(used, words(get(n.attrs, "pools", "")))
    end
    return [p.id for p in d.pools if p.id in used]
end

function loop_type(n::Elem)
    has(n, "loop") && return "Standard"
    has(n, "parallel") && return "MultiInstanceParallel"
    has(n, "sequential") && return "MultiInstanceSequential"
    return nothing
end

ref_elem(name, id) = text_elem(name, id)

function export_choreographies!(ex::Exporter)
    d = ex.doc
    for c in ovalues(d.choreographies)
        cid = c.id
        e = bsub!(ex.root, "choreography", "id" => cid, "name" => c.label)
        documentation!(e, get(c.attrs, "doc", nothing))
        for p in choreography_participants(d, cid)
            pool = d.pools[findfirst(x -> x.id == p, d.pools)]
            pe = bsub!(e, "participant", "id" => choreography_participant_id(cid, p), "name" => pool.label)
            m = multiplicity(d, pool)
            m === nothing || push!(pe.children, m)
        end
        for n in ovalues(d.nodes)
            (n.choreography == cid && n.kind == "choreo") || continue
            from, to = get(n.attrs, "from", ""), get(n.attrs, "to", "")
            for (suffix, src, tgt, key) in (("message", from, to, "message"), ("reply", to, from, "reply"))
                name = get(n.attrs, key, nothing)
                (key == "reply" && name === nothing) && continue
                mf = bsub!(e, "messageFlow", "id" => "$(n.id)-$(suffix)",
                           "sourceRef" => choreography_participant_id(cid, src),
                           "targetRef" => choreography_participant_id(cid, tgt))
                nonempty(name) && setattr!(mf, "messageRef", declare!(ex, "message", name).id)
            end
        end
        ex.containers[cid] = e
        export_choreography_block!(ex, e, cid, c.children)
    end
end

function export_choreography_block!(ex::Exporter, container::XElem, cid, ids)
    for i in ids
        n = get(ex.doc.nodes, i, nothing)
        n === nothing && continue
        if !(n.kind in CHOREO_KINDS)
            export_node!(ex, container, n)
            continue
        end
        e = choreography_activity(ex, cid, n)
        push!(container.children, e)
        if n.kind == "subchoreo"
            ex.containers[i] = e
            export_choreography_block!(ex, e, cid, n.children)
        end
    end
end

function choreography_activity(ex::Exporter, cid, n::Elem)
    tag = n.kind == "choreo" ? "choreographyTask" : n.kind == "subchoreo" ? "subChoreography" : "callChoreography"
    part(p) = choreography_participant_id(cid, p)
    e = XElem(BPMN_NS, tag)
    setattr!(e, "id", n.id)
    n.label === nothing || setattr!(e, "name", n.label)
    if n.kind == "callchoreo" && haskey(n.attrs, "choreography")
        setattr!(e, "calledChoreographyRef", n.attrs["choreography"])
    end
    haskey(n.attrs, "from") && setattr!(e, "initiatingParticipantRef", part(n.attrs["from"]))
    lt = loop_type(n)
    lt === nothing || setattr!(e, "loopType", lt)
    documentation!(e, get(n.attrs, "doc", nothing))
    pools = n.kind == "choreo" ? [n.attrs[k] for k in ("from", "to") if haskey(n.attrs, k)] :
            words(get(n.attrs, "pools", ""))
    for p in pools
        push!(e.children, ref_elem("participantRef", part(p)))
    end
    if n.kind == "choreo"
        push!(e.children, ref_elem("messageFlowRef", "$(n.id)-message"))
        haskey(n.attrs, "reply") && push!(e.children, ref_elem("messageFlowRef", "$(n.id)-reply"))
    end
    if n.kind == "callchoreo" && haskey(n.attrs, "map")
        called = get(n.attrs, "choreography", "")
        pairs = participant_map(n.attrs["map"])
        for (i, o) in (pairs === nothing ? Tuple{String,String}[] : pairs)
            pa = bsub!(e, "participantAssociation")
            push!(pa.children, ref_elem("innerParticipantRef", choreography_participant_id(called, i)))
            push!(pa.children, ref_elem("outerParticipantRef", part(o)))
        end
    end
    return e
end

"Conversation nodes of the collaboration before its correlation keys, conversation links last."
function export_conversations!(ex::Exporter)
    d = ex.doc
    (isempty(d.conversations) || ex.collab === nothing) && return
    collab = ex.collab
    nodes = XElem[]
    links = XElem[]
    for c in ovalues(d.conversations)
        c.parent === nothing || continue
        push!(nodes, conversation_node(ex, c))
        for p in words(get(c.attrs, "pools", ""))
            l = XElem(BPMN_NS, "conversationLink")
            setattr!(l, "id", conversation_link_id(c.id, p))
            setattr!(l, "sourceRef", p)
            setattr!(l, "targetRef", c.id)
            push!(links, l)
        end
    end
    k = findfirst(x -> x.name == "correlationKey", collab.children)
    at = k === nothing ? length(collab.children) : k - 1
    for n in nodes
        insert_at!(collab, at, n)
        at += 1
    end
    append!(collab.children, links)
end

function conversation_node(ex::Exporter, c::Elem)
    d = ex.doc
    tag = c.kind == "subconversation" ? "subConversation" : c.kind == "callconversation" ? "callConversation" :
          "conversation"
    e = XElem(BPMN_NS, tag)
    setattr!(e, "id", c.id)
    c.label === nothing || setattr!(e, "name", c.label)
    called = get(c.attrs, "collaboration", nothing)
    called === nothing || setattr!(e, "calledCollaborationRef", called)
    documentation!(e, get(c.attrs, "doc", nothing))
    for p in words(get(c.attrs, "pools", ""))
        push!(e.children, ref_elem("participantRef", p))
    end
    for m in words(get(c.attrs, "messages", ""))
        push!(e.children, ref_elem("messageFlowRef", m))
    end
    for k in c.children
        push!(e.children, conversation_node(ex, d.conversations[k]))
    end
    if c.kind == "callconversation" && haskey(c.attrs, "map")
        inner_of(i) = (called !== nothing && haskey(d.choreographies, called)) ?
                      choreography_participant_id(called, i) : i
        pairs = participant_map(c.attrs["map"])
        for (i, o) in (pairs === nothing ? Tuple{String,String}[] : pairs)
            pa = XElem(BPMN_NS, "participantAssociation")
            push!(pa.children, ref_elem("innerParticipantRef", inner_of(i)))
            push!(pa.children, ref_elem("outerParticipantRef", o))
            push!(e.children, pa)
        end
    end
    return e
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
            union!(used, x.declare)
            for (ans, _, _) in x.attrs
                ans == "" || push!(used, ans)
            end
        end
        decls = [(PREFIXES[ns], ns) for ns in used]
        for (p, ns) in e.prefixed
            any(d -> d[1] == p, decls) || push!(decls, (p, ns))
        end
        for (p, ns) in sort(decls; by=first)
            print(io, " xmlns:", p, "=\"", escape_attrib(ns), "\"")
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
