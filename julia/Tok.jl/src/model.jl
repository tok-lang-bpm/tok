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
# triggers written as flags without a value (language 0.5): an event definition without a
# message, signal, time or condition
const NAMELESS_TRIGGERS = ("message", "timer", "signal", "condition")

const EVENT_KEYS = ("on", "message", "timer", "signal", "error", "escalation", "code", "condition", "link",
                    "correlation", "form", "binding", "version", "activity", "events",
                    "type", "retries", "headers", "outputs", "listeners", "delay", "assign", "doc")
const EVENT_FLAGS = ("noninterrupting", "terminate", "compensate", "cancel", "error", "escalation",
                     "message", "timer", "signal", "condition", "parallel")
const TASK_KEYS = ("message", "operation", "process", "task", "form", "decision", "expr", "result", "type",
                   "binding", "version", "child-vars", "parent-vars", "business-id", "while", "loop-max",
                   "collection", "item", "output", "output-item", "until", "active",
                   "assignee", "candidate-users", "candidate-groups", "due", "follow-up", "priority",
                   "retries", "headers", "connector", "method", "url", "query", "headers-in", "body",
                   "inputs", "outputs", "compensate", "listeners",
                   "by", "duration", "assign", "doc")
const TASK_FLAGS = ("loop", "test-before", "parallel", "sequential", "compensation", "wait", "instantiate")
# attributes of a complex gateway and flags of an event-based gateway (language 0.5)
const COMPLEX_KEYS = ("activation", "doc")
const GATEWAY_KEYS = ("doc",)
const EVENTGW_FLAGS = ("instantiate", "parallel")
# values of `method=` on a transaction and their BPMN form
const TRANSACTION_METHODS = Dict("compensate" => "##Compensate", "store" => "##Store", "image" => "##Image")
# values of `kind=` on `global` and the BPMN element of each
const GLOBAL_KINDS = Dict("task" => "globalTask", "user" => "globalUserTask", "manual" => "globalManualTask",
                          "script" => "globalScriptTask", "rule" => "globalBusinessRuleTask")

const LANGUAGE_VERSIONS = ("0.1", "0.3", "0.5", "0.6", "0.7", "1.0")

# language 0.6: conversation nodes of a collaboration and choreography activities
const CONVERSATION_KINDS = ("conversation", "subconversation", "callconversation")
const CONVERSATION_KEYS = ("pools", "messages", "doc")
const CALLCONVERSATION_KEYS = ("pools", "messages", "collaboration", "map", "doc")
const CHOREO_KINDS = ("choreo", "subchoreo", "callchoreo")
const CHOREOGRAPHY_KEYS = ("doc",)
const CHOREO_KEYS = ("from", "to", "message", "reply", "doc")
const SUBCHOREO_KEYS = ("pools", "from", "doc")
const CALLCHOREO_KEYS = ("pools", "from", "choreography", "map", "doc")
const CHOREO_FLAGS = ("loop", "parallel", "sequential")
conversation_keys(kind) = kind == "callconversation" ? CALLCONVERSATION_KEYS : CONVERSATION_KEYS
"Words of a list attribute (`pools=\"a b\"`)."
words(text::AbstractString) = [String(w) for w in split(text, r"[\s,]+") if !isempty(w)]

is_activity(kind) = kind in TASK_KINDS || kind in SUB_KINDS

function node_keys(kind)
    kind in EVENT_KINDS && return EVENT_KEYS
    is_activity(kind) && return TASK_KEYS
    kind == "complex" && return COMPLEX_KEYS
    kind in GATEWAY_KINDS && return GATEWAY_KEYS
    kind == "choreo" && return CHOREO_KEYS
    kind == "subchoreo" && return SUBCHOREO_KEYS
    kind == "callchoreo" && return CALLCHOREO_KEYS
    return ()
end

function node_flags(kind)
    kind in EVENT_KINDS && return EVENT_FLAGS
    is_activity(kind) && return TASK_FLAGS
    kind == "eventgw" && return EVENTGW_FLAGS
    kind in CHOREO_KINDS && return CHOREO_FLAGS
    return ()
end

const HEADER_KEYS = ("id", "process", "target", "namespace", "collaboration", "version-tag", "process-name", "doc")
const HEADER_FLAGS = ("executable",)
const POOL_KEYS = ("process", "version-tag", "process-name", "doc")
const POOL_FLAGS = ("executable",)
# pools of a document of language 0.6 have a participant multiplicity
const POOL_KEYS_06 = ("process", "version-tag", "process-name", "min", "max", "doc")
const LANE_KEYS = ("doc",)
const FLOW_KEYS = ("id", "when", "p", "doc")
const FLOW_FLAGS = ("default",)
const MESSAGE_KEYS = ("id", "message", "delay", "doc")
const DATA_KEYS = ("state", "type", "object", "doc")
const DATA_FLAGS = ("collection",)
# `reads ACTIVITY DATA input=ID`, `writes ACTIVITY DATA output=ID` (language 0.5)
const READS_KEYS = ("input",)
const WRITES_KEYS = ("output",)
const NOTE_KEYS = ("on", "doc")
const GROUP_KEYS = ("members", "doc")
const DECISION_KEYS = ("hit",)
const FIELD_KEYS = ("type", "options", "min", "max", "default")
const FIELD_FLAGS = ("required",)
const SIMULATE_KEYS = ("cases", "seed", "arrival", "start")
const RESOURCE_KEYS = ("count",)
# process data of language 0.5: `input`, `output`, `property`
const IO_KINDS = ("input", "output", "property")
const IO_KEYS = ("type", "on", "doc")
const IO_FLAGS = ("collection",)
const ITEM_KEYS = ("structure", "namespace", "kind")
const ITEM_FLAGS = ("collection",)
const GLOBAL_KEYS = ("kind", "doc")
const INTERFACE_KEYS = ("doc",)
const OPERATION_KEYS = ("in", "out", "error", "doc")
const PROP_KEYS = ("message", "path")

# language 0.7: decision requirements graphs, boxed expressions and DMN types
const HIT_POLICIES_07 = ("unique", "first", "any", "priority", "collect", "rule-order", "output-order")
# statements of language 0.7; in documents of earlier versions they are unknown keywords
const KEYWORDS_07 = ("inputdata", "bkm", "source", "entry", "result", "element")
const DECISION_KEYS_07 = ("name", "type", "requires", "question", "answers", "hit", "aggregation", "default",
                          "expr", "invoke", "with", "doc")
const BKM_KEYS = ("name", "params", "type", "requires", "hit", "aggregation", "default", "expr", "invoke",
                  "with", "doc")
const INPUTDATA_KEYS = ("name", "type", "doc")
const SOURCE_KEYS = ("name", "requires", "kind", "uri", "doc")
# `entry`, `result` and `element` lines of a box
const BOX_LINE_KEYS = ("type", "params", "hit", "aggregation", "default", "expr", "invoke", "with")
const ITEM_KEYS_07 = ("name", "structure", "namespace", "kind", "type", "allowed")
# components of a DMN type
const COMPONENT_KEYS = ("type", "allowed")

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
    # form items, operations of an interface, props of a correlation key; the entry,
    # result and element lines of a box and the components of a DMN type (language 0.7)
    items::Vector{Elem}
    # the choreography of a flow node (language 0.6)
    choreography::MaybeStr
    # language 0.7: values of the input and output columns (`name: type = values`),
    # parallel to inputs and outputs; a header without `||` makes a relation whose
    # columns are in inputs
    input_values::Vector{MaybeStr}
    output_values::Vector{MaybeStr}
    relation::Bool
end

function Elem(kind, id; label=nothing, attrs=Dict{String,String}(), flags=String[], line=0)
    Elem(String(kind), id === nothing ? nothing : String(id), label, attrs, Tuple{String,String}[], flags, line,
         String[], nothing,
         nothing, nothing, nothing, nothing, nothing, String[], true,
         "", "", false, true,
         Tuple{String,String}[], Tuple{String,String}[], Rule[], false, nothing, false,
         Elem[], nothing,
         MaybeStr[], MaybeStr[], false)
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
    conversations::ODict{Elem}
    choreographies::ODict{Elem}
    end_comments::Vector{String}
    ids::Dict{String,Elem}
    # language 0.7: input data, knowledge sources and business knowledge models
    inputdata::ODict{Elem}
    sources::ODict{Elem}
    bkms::ODict{Elem}
end

Document() = Document(nothing, nothing, Dict{String,String}(), String[], 0, String[], nothing,
                      String[], ODict{Elem}(), Elem[], Elem[], ODict{Elem}(), Elem[], Elem[], Elem[], Elem[],
                      ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), nothing, ODict{Elem}(), Elem[],
                      ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), ODict{Elem}(), ODict{Elem}(),
                      String[],
                      Dict{String,Elem}(),
                      ODict{Elem}(), ODict{Elem}(), ODict{Elem}())

"The header declares language 0.5 (or later): several triggers per event, BPMN resources."
lang_05(d::Document) = !(d.version === nothing || d.version in ("0.1", "0.3"))

"The header declares language 0.6 (or later): pools have a multiplicity."
lang_06(d::Document) = !(d.version === nothing || d.version in ("0.1", "0.3", "0.5"))
pool_keys(d::Document) = lang_06(d) ? POOL_KEYS_06 : POOL_KEYS

"The header declares language 0.7 (or later): decision requirements graphs and boxed expressions."
lang_07(d::Document) = !(d.version === nothing || d.version in ("0.1", "0.3", "0.5", "0.6"))

is_executable(d::Document) = "executable" in d.flags
executable_any(d::Document) = is_executable(d) || any(p -> has(p, "executable"), d.pools)
default_process_id(d::Document) = get(d.attrs, "process", "process")
