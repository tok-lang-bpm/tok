# The Rust engine of tok through its C interface (the shared library `tokengine`).

"""
    Tok.Engine

The whole tok language through the Rust engine: the checker of every layer, canonical
text, BPMN, SVG, DMN and form export, BPMN and DMN import, decisions and simulation.
The functions mirror the Python package (`tok.api`) and the R package (`tokr`).

The shared library is found, in this order, at the path in the environment variable
`TOK_LIBTOK`, in the binary package `tok_jll` when it is installed in the active
environment, and in the build directory `rust/target/release` of the repository.
[`available`](@ref) tells whether it was found. Errors of the engine are thrown as
[`TokError`](@ref).
"""
module Engine

using ..JSON: parse_json, write_json

export TokError

"""
    TokError(kind, message, diagnostics)

An error reported by the engine. `kind` is one of `"invalid_argument"`, `"document"`
(the document has errors; `diagnostics` holds them), `"render"`, `"import"`,
`"unknown_decision"`, `"decision"`, `"feel"`, `"distribution"`, `"simulation"`, `"io"`
and `"internal"`.
"""
struct TokError <: Exception
    kind::String
    message::String
    diagnostics::Vector{Any}
end

Base.showerror(io::IO, e::TokError) = print(io, "TokError (", e.kind, "): ", e.message)

"Version of the C interface this module is written for."
const ABI_VERSION = 1

const LIBNAME = Sys.iswindows() ? "tokengine.dll" : Sys.isapple() ? "libtokengine.dylib" : "libtokengine.so"
const REPO_LIB = normpath(joinpath(@__DIR__, "..", "..", "..", "rust", "target", "release", LIBNAME))

mutable struct Lib
    path::String
    handle::Ptr{Cvoid}
    syms::Dict{Symbol,Ptr{Cvoid}}
end

const LIB = Ref{Union{Nothing,Lib}}(nothing)
const LIB_LOCK = ReentrantLock()

"The path of the library from the binary package `tok_jll`, or `nothing`."
function jll_path()
    id = Base.identify_package("tok_jll")
    id === nothing && return nothing
    mod = try
        Base.require(id)
    catch
        return nothing
    end
    for name in (:libtokengine_path, :libtokengine)
        if isdefined(mod, name)
            p = getfield(mod, name)
            p isa AbstractString && isfile(p) && return String(p)
        end
    end
    return nothing
end

"""
    find_library() -> path or nothing

Where the engine library is: `TOK_LIBTOK`, the package `tok_jll`, or the build
directory of the repository.
"""
function find_library()
    p = get(ENV, "TOK_LIBTOK", "")
    if !isempty(p)
        isfile(p) || error("TOK_LIBTOK is set to $(repr(p)), which is not a file")
        return p
    end
    p = jll_path()
    p !== nothing && return p
    isfile(REPO_LIB) && return REPO_LIB
    return nothing
end

function lib()
    l = LIB[]
    l !== nothing && return l
    lock(LIB_LOCK) do
        LIB[] !== nothing && return LIB[]
        path = find_library()
        path === nothing && error("the tok engine library was not found: set TOK_LIBTOK to the path of " *
                                  "$LIBNAME, install the package tok_jll, or build it with " *
                                  "`cargo build --release -p tok-capi` in the rust folder of the repository")
        handle = Base.Libc.Libdl.dlopen(path)
        # every function is looked up once, so that calls from several threads only read
        syms = Dict{Symbol,Ptr{Cvoid}}(s => Base.Libc.Libdl.dlsym_e(handle, s) for s in SYMBOLS)
        abi = syms[:tok_abi_version] == C_NULL ? 0 : Int(ccall(syms[:tok_abi_version], UInt32, ()))
        abi == ABI_VERSION || error("$path implements version $abi of the C interface of tok; " *
                                    "Tok.jl needs version $ABI_VERSION")
        LIB[] = Lib(path, handle, syms)
        return LIB[]
    end
end

const SYMBOLS = (:tok_version, :tok_abi_version, :tok_free, :tok_check, :tok_fmt, :tok_bpmn, :tok_svg,
                 :tok_export, :tok_import, :tok_eval, :tok_sim, :tok_sim_model, :tok_sim_dist, :tok_sim_eval,
                 :tok_cli)

function sym(name::Symbol)
    p = lib().syms[name]
    p == C_NULL && error("the engine library $(library()) has no function $name")
    return p
end

"""
    available() -> Bool

Whether the engine library can be found and loaded.
"""
function available()
    try
        lib()
        return true
    catch
        return false
    end
end

"Path of the loaded engine library."
library() = lib().path

"Version of the engine, for example `\"0.7.0\"`."
version() = unsafe_string(ccall(sym(:tok_version), Cstring, ()))

"Version of the C interface of the loaded library."
abi_version() = Int(ccall(sym(:tok_abi_version), UInt32, ()))

"""
    call(fn, input, options) -> value

Call one function of the C interface with a text and a dictionary of options; return
the value of `{"ok": …}` or throw the [`TokError`](@ref) of `{"error": …}`.
"""
function call(fn::Symbol, input::AbstractString, options=Dict{String,Any}())
    f = sym(fn)
    free = sym(:tok_free)
    opts = write_json(options)
    p = ccall(f, Ptr{UInt8}, (Cstring, Cstring), String(input), opts)
    p == C_NULL && throw(TokError("internal", "the engine returned no result", Any[]))
    text = try
        unsafe_string(p)
    finally
        ccall(free, Cvoid, (Ptr{UInt8},), p)
    end
    r = parse_json(text)
    if haskey(r, "error")
        e = r["error"]
        throw(TokError(e["kind"], e["message"], get(e, "diagnostics", Any[])))
    end
    return r["ok"]
end

# options without the keys left at `nothing`
opts(pairs...) = Dict{String,Any}(String(k) => v for (k, v) in pairs if v !== nothing)

"""
    check(text; lang="en", level="full", bpmn=true, class=nothing) -> Vector{Dict}

Diagnostics of `tok check`: dictionaries with the keys `code`, `severity`, `line`, `id`
and `message`, sorted by line and code. `class` checks against a BPMN conformance class
(`"descriptive"`, `"analytic"`, `"common-executable"`, `"full"`).
"""
check(text; lang="en", level="full", bpmn=true, class=nothing) =
    Vector{Any}(call(:tok_check, text, opts(:lang => lang, :level => level, :bpmn => bpmn, :class => class))["diagnostics"])

"""
    bpmn_class(text) -> String

The smallest BPMN 2.0.2 conformance class the document fits (as `tok check --json`).
"""
bpmn_class(text) = call(:tok_check, text, opts(:bpmn => false))["bpmn_class"]

"""
    fmt(text) -> String

Canonical text (`tok fmt`); a document with syntax errors throws a [`TokError`](@ref).
"""
fmt(text; lang="en") = call(:tok_fmt, text, opts(:lang => lang))

"""
    bpmn(text; layout=true, sidecar=nothing) -> String

BPMN 2.0 XML with diagram layout (`tok bpmn`). `sidecar` is the text of a layout
sidecar (`model.tok.layout`) whose geometry is kept.
"""
bpmn(text; layout=true, sidecar=nothing, lang="en") =
    call(:tok_bpmn, text, opts(:layout => layout, :sidecar => sidecar, :lang => lang))

"""
    svg(text; title=nothing, sidecar=nothing) -> String

SVG picture of the diagram (`tok svg`).
"""
svg(text; title=nothing, sidecar=nothing, lang="en") =
    call(:tok_svg, text, opts(:title => title, :sidecar => sidecar, :lang => lang))

"""
    export_files(text; name="model", sidecar=nothing) -> Dict{String,String}

Every file `tok export` writes, as file name => content: `NAME.bpmn`, `NAME.svg`, the
decision files and one `.form` per form.
"""
export_files(text; name="model", sidecar=nothing, lang="en") =
    Dict{String,String}(call(:tok_export, text, opts(:name => name, :sidecar => sidecar, :lang => lang)))

"""
    import_file(xml; format="auto", keep_ids=true, layout=true) -> Dict

A BPMN 2.0 or DMN file as tok (`tok import`): a dictionary with `format` (`"bpmn"` or
`"dmn"`), `text`, `warnings` and `layout` (the text of the layout sidecar of a BPMN
file, or `nothing`).
"""
import_file(xml; format="auto", keep_ids=true, layout=true) =
    call(:tok_import, xml, opts(:format => format, :flow_ids => keep_ids, :layout => layout))

"""
    import_bpmn(xml; keep_ids=true) -> (text, warnings)

BPMN 2.0 XML as a tok document (`tok import FILE.bpmn`).
"""
function import_bpmn(xml; keep_ids=true)
    r = call(:tok_import, xml, opts(:format => "bpmn", :flow_ids => keep_ids, :layout => false))
    return (r["text"], Vector{String}(r["warnings"]))
end

"""
    import_dmn(xml) -> (text, warnings)

A DMN 1.1–1.5 file as a document of language 0.7 (`tok import FILE.dmn`).
"""
function import_dmn(xml)
    r = call(:tok_import, xml, opts(:format => "dmn"))
    return (r["text"], Vector{String}(r["warnings"]))
end

"""
    eval_decision(text, decision, values=Dict()) -> value

The value of a decision for the input values (`tok eval`). In a document of language
0.7 the decision is evaluated with everything it requires and the result is its value;
in earlier documents the decision table alone gives a dictionary of outputs, a vector of
them for hit policy `collect`, or `nothing`.
"""
eval_decision(text, decision::AbstractString, values=Dict{String,Any}(); lang="en") =
    call(:tok_eval, text, opts(:decision => decision, :values => Dict{String,Any}(string(k) => v for (k, v) in values),
                               :lang => lang))

"""
    SimResult

The result of [`simulate`](@ref): `kpi` (the indicators of `tok sim --json`), `report`
(the text `tok sim` prints), `log` (one dictionary per activity instance), `cases` (one
per case), and on request `xes`, `log_csv` and `cases_csv` (otherwise `nothing`).
"""
struct SimResult
    kpi::Dict{String,Any}
    report::String
    log::Vector{Any}
    cases::Vector{Any}
    xes::Union{Nothing,String}
    log_csv::Union{Nothing,String}
    cases_csv::Union{Nothing,String}
end

"The indicators of a simulation run."
kpi(r::SimResult) = r.kpi

Base.show(io::IO, r::SimResult) =
    print(io, "SimResult(", r.kpi["cases"]["started"], " cases, seed ", r.kpi["seed"], ")")

"""
    simulate(text; cases=nothing, seed=nothing, log=true, cases_table=true, xes=false,
             csv=false, lang="en") -> SimResult

Run the simulation of the Rust engine (`tok sim`). `cases` and `seed` override the
attributes of `simulate`. `xes=true` adds the event log as XES (IEEE 1849-2016),
`csv=true` the event log and the cases as the CSV text of `tok sim --log` and
`--cases-csv`.
"""
function simulate(text; cases=nothing, seed=nothing, log=true, cases_table=true, xes=false, csv=false, lang="en")
    r = call(:tok_sim, text, opts(:cases => cases, :seed => seed, :log => log, :cases_table => cases_table,
                                  :xes => xes, :csv => csv, :lang => lang))
    return SimResult(r["kpi"], r["report"], get(r, "log", Any[]), get(r, "cases", Any[]),
                     get(r, "xes", nothing), get(r, "log_csv", nothing), get(r, "cases_csv", nothing))
end

"""
    sim_xes(text; cases=nothing, seed=nothing) -> String

The event log of a simulation run as an XES document (`tok sim --xes`).
"""
sim_xes(text; cases=nothing, seed=nothing) =
    simulate(text; cases=cases, seed=seed, log=false, cases_table=false, xes=true).xes

"""
    sim_model(text; cases=nothing, seed=nothing) -> Dict

The simulation model of a document as the engine reads it: parameters of `simulate`,
resources, variables, nodes with parsed distributions and timers, and sequence flows.
The native simulator ([`Tok.Sim`](@ref)) is built from it.
"""
sim_model(text; cases=nothing, seed=nothing, lang="en") =
    call(:tok_sim_model, text, opts(:cases => cases, :seed => seed, :lang => lang))

"""
    sim_dist(text) -> Dict

A distribution of the simulation statements (`"tri(5m, 10m, 20m)"`) parsed by the
engine: `name`, `kind`, `args` (seconds for durations) and `labels`.
"""
sim_dist(text) = call(:tok_sim_dist, text)

"""
    sim_eval(expr, vars=Dict(); lang07=false) -> (value, truthy)

An expression evaluated as the simulator evaluates conditions: the value and its truth
value. An expression that cannot be evaluated throws a [`TokError`](@ref) of kind
`"feel"`.
"""
function sim_eval(expr, vars=Dict{String,Any}(); lang07=false)
    r = call(:tok_sim_eval, expr, opts(:vars => vars, :lang07 => lang07))
    return (r["value"], r["truthy"])
end

"""
    cli(args) -> Int

Run the `tok` command line of the engine with the arguments after the program name
(`cli(["sim", "model.tok", "--json"])`) and return its exit status. The command writes
to the standard output and error streams of the process.
"""
function cli(args::AbstractVector{<:AbstractString})
    flush(stdout)
    flush(stderr)
    r = call(:tok_cli, write_json(String.(args)))
    return Int(r["status"])
end

end # module Engine
