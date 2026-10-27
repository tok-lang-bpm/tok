# Intermediate representation. One mutable struct serves all statement kinds,
# as in the reference implementation; fields that a kind does not use stay empty.

const EVENT_KINDS = ("start", "end", "catch", "throw", "boundary")
const TASK_KINDS = ("task", "user", "service", "manual", "script", "rule", "send", "receive", "call")
const SUB_KINDS = ("sub", "eventsub", "adhoc", "transaction")
const GATEWAY_KINDS = ("xor", "and", "or", "eventgw", "complex")
const NODE_KINDS = (EVENT_KINDS..., TASK_KINDS..., SUB_KINDS..., GATEWAY_KINDS...)

const TRIGGERS = ("message", "timer", "signal", "error", "escalation", "condition", "link")
const TRIGGER_FLAGS = ("terminate", "compensate", "cancel")
# attribute keys that may repeat on one statement (language 0.5): the triggers of a
# multiple event and the errors of an operation
const REPEATABLE_KEYS = ("message", "timer", "signal", "error", "escalation", "condition", "link")
# catch-all flags (no name): catch every error or escalation (language 0.3)
const CATCH_ALL_FLAGS = ("error", "escalation")

const EVENT_KEYS = ("on", "message", "timer", "signal", "error", "escalation", "code", "condition", "link",
                    "correlation", "form", "binding", "version", "activity", "events",
                    "type", "retries", "headers", "outputs", "listeners", "delay", "assign")
const EVENT_FLAGS = ("noninterrupting", "terminate", "compensate", "cancel", "error", "escalation", "parallel")
const TASK_KEYS = ("message", "operation", "process", "task", "form", "decision", "expr", "result", "type",
                   "binding", "version", "child-vars", "parent-vars", "business-id", "while", "loop-max",
                   "collection", "item", "output", "output-item", "until", "active",
                   "assignee", "candidate-users", "candidate-groups", "due", "follow-up", "priority",
                   "retries", "headers", "connector", "method", "url", "query", "headers-in", "body",
                   "inputs", "outputs", "compensate", "listeners",
                   "by", "duration", "assign")
const TASK_FLAGS = ("loop", "test-before", "parallel", "sequential", "compensation", "wait", "instantiate")
# attributes of a complex gateway and flags of an event-based gateway (language 0.5)
const COMPLEX_KEYS = ("activation",)
const EVENTGW_FLAGS = ("instantiate", "parallel")
# values of `method=` on a transaction and their BPMN form
const TRANSACTION_METHODS = Dict("compensate" => "##Compensate", "store" => "##Store", "image" => "##Image")
# values of `kind=` on `global` and the BPMN element of each
const GLOBAL_KINDS = Dict("task" => "globalTask", "user" => "globalUserTask", "manual" => "globalManualTask",
                          "script" => "globalScriptTask", "rule" => "globalBusinessRuleTask")

const LANGUAGE_VERSIONS = ("0.1", "0.3", "0.5")

is_activity(kind) = kind in TASK_KINDS || kind in SUB_KINDS

function node_keys(kind)
    kind in EVENT_KINDS && return EVENT_KEYS
    is_activity(kind) && return TASK_KEYS
    kind == "complex" && return COMPLEX_KEYS
    return ()
end

function node_flags(kind)
    kind in EVENT_KINDS && return EVENT_FLAGS
    is_activity(kind) && return TASK_FLAGS
    kind == "eventgw" && return EVENTGW_FLAGS
    return ()
end

const HEADER_KEYS = ("id", "process", "target", "namespace", "collaboration", "version-tag")
const HEADER_FLAGS = ("executable",)
const POOL_KEYS = ("process", "version-tag")
const POOL_FLAGS = ("executable",)
const FLOW_KEYS = ("id", "when", "p")
const FLOW_FLAGS = ("default",)
const MESSAGE_KEYS = ("id", "message", "delay")
const DATA_KEYS = ("state", "type")
const DATA_FLAGS = ("collection",)
const NOTE_KEYS = ("on",)
const GROUP_KEYS = ("members",)
const DECISION_KEYS = ("hit",)
const FIELD_KEYS = ("type", "options", "min", "max", "default")
const FIELD_FLAGS = ("required",)
const SIMULATE_KEYS = ("cases", "seed", "arrival", "start")
const RESOURCE_KEYS = ("count",)
# process data of language 0.5: `input`, `output`, `property`
const IO_KINDS = ("input", "output", "property")
const IO_KEYS = ("type", "on")
const IO_FLAGS = ("collection",)
const ITEM_KEYS = ("structure", "kind")
const ITEM_FLAGS = ("collection",)
const GLOBAL_KEYS = ("kind",)
const OPERATION_KEYS = ("in", "out", "error")
const PROP_KEYS = ("message", "path")

const MaybeStr = Union{Nothing,String}

mutable struct Rule
    inputs::Vector{String}
    outputs::Vector{String}
    annotation::MaybeStr
    line::Int
    comments::Vector{String}
    trailing::MaybeStr
end

mutable struct Elem
    kind::String
    id::MaybeStr
    label::MaybeStr
    attrs::Dict{String,String}
    # further values of repeated keys (REPEATABLE_KEYS) in written order; the first
    # value of each key is in attrs (language 0.5)
    extra::Vector{Tuple{String,String}}
    flags::Vector{String}
    line::Int
    comments::Vector{String}
    trailing::MaybeStr
    # flow nodes, lanes, pools, data
    scope::MaybeStr
    lane::MaybeStr
    pool::MaybeStr
    parent::MaybeStr
    block::MaybeStr
    children::Vector{String}
    blackbox::Bool
    # flows, message flows, associations
    source::String
    target::String
    explicit_id::Bool
    chain_start::Bool
    # decisions
    inputs::Vector{Tuple{String,String}}
    outputs::Vector{Tuple{String,String}}
    rules::Vector{Rule}
    has_annotations::Bool
    annotation_title::MaybeStr
    header_done::Bool
    # form items, operations of an interface, props of a correlation key
    items::Vector{Elem}
end

function Elem(kind, id; label=nothing, attrs=Dict{String,String}(), flags=String[], line=0)
    Elem(String(kind), id === nothing ? nothing : String(id), label, attrs, Tuple{String,String}[], flags, line,
         String[], nothing,
         nothing, nothing, nothing, nothing, nothing, String[], true,
         "", "", false, true,
         Tuple{String,String}[], Tuple{String,String}[], Rule[], false, nothing, false,
         Elem[])
end

has(e::Elem, flag) = flag in e.flags

"Every value of `key`: the one in attrs, then the repeated ones."
function values_of(e::Elem, key)
    out = String[]
    haskey(e.attrs, key) && push!(out, e.attrs[key])
    for (k, v) in e.extra
        k == key && push!(out, v)
    end
    return out
end

"Process identifier of a pool."
process_id(p::Elem) = get(p.attrs, "process", p.id * "-process")

mutable struct Document
    version::MaybeStr
    title::MaybeStr
    attrs::Dict{String,String}
    flags::Vector{String}
    header_line::Int
    header_comments::Vector{String}
    header_trailing::MaybeStr
    top::Vector{String}
    nodes::ODict{Elem}
    flows::Vector{Elem}
    messages::Vector{Elem}
    data::ODict{Elem}
    assocs::Vector{Elem}
    notes::Vector{Elem}
    groups::Vector{Elem}
    pools::Vector{Elem}
    lanes::ODict{Elem}
    decisions::ODict{Elem}
    forms::ODict{Elem}
    simulate::Union{Nothing,Elem}
    resources::ODict{Elem}
    variables::Vector{Elem}
    items::ODict{Elem}
    globals::ODict{Elem}
    interfaces::ODict{Elem}
    keys::ODict{Elem}
    end_comments::Vector{String}
    ids::Dict{String,Elem}
end

Document() = Document(nothing, nothing, Dict{String,String}(), String[], 0, String[], nothing,
                      String[], ODict{Elem}(), Elem[], Elem[], ODict{Elem}(), Elem[], Elem[], Elem[], Elem[],
                      ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), nothing, ODict{Elem}(), Elem[],
                      ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), String[],
                      Dict{String,Elem}())

"The header declares language 0.5 (or later): several triggers per event, BPMN resources."
lang_05(d::Document) = !(d.version === nothing || d.version in ("0.1", "0.3"))

is_executable(d::Document) = "executable" in d.flags
executable_any(d::Document) = is_executable(d) || any(p -> has(p, "executable"), d.pools)
default_process_id(d::Document) = get(d.attrs, "process", "process")
